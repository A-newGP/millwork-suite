# SPDX-FileCopyrightText: 2026 OpenCabinet contributors
# SPDX-License-Identifier: GPL-3.0-or-later

"""Reference implementation of the stack spike rules (see ../README.md).

Written independently of the Rust and C# prototypes to generate the golden
outputs in ../expected/. Python integers are arbitrary precision, so every
length computation here is exact.

Usage: python evaluate.py <input.ocab dir or zip> <output-dir>
"""

import json
import sys
import zipfile
from math import gcd
from pathlib import Path

MIMETYPE = "application/vnd.opencabinet.project+zip"
NM_PER_INCH = 25_400_000

CUTLIST_HEADER = (
    "product,part,material,qty,length,width,thickness,length_nm,width_nm,"
    "thickness_nm,cut_length_nm,cut_width_nm,band_x0,band_x1,band_y0,band_y1"
)
OPERATIONS_HEADER = "product,part,face,kind,x_nm,y_nm,diameter_nm,depth_nm"


def read_entries(path: Path) -> dict[str, bytes]:
    if path.is_dir():
        return {p.name: p.read_bytes() for p in path.iterdir() if p.is_file()}
    with zipfile.ZipFile(path) as archive:
        return {name: archive.read(name) for name in archive.namelist()}


def load_project(path: Path) -> dict:
    entries = read_entries(path)
    if entries.get("mimetype") != MIMETYPE.encode():
        raise ValueError("missing or wrong mimetype entry")
    manifest = json.loads(entries["manifest.json"])
    if manifest["formatVersion"].split(".")[0] != "0":
        raise ValueError("unsupported format version")
    return json.loads(entries["project.json"])


def round_half_away(numerator: int, denominator: int) -> int:
    """Round numerator/denominator to the nearest integer, ties away from zero."""
    sign = -1 if (numerator < 0) != (denominator < 0) else 1
    n, d = abs(numerator), abs(denominator)
    return sign * ((2 * n + d) // (2 * d))


def format_inches(nm: int, denominator: int = 32) -> str:
    units = round_half_away(nm * denominator, NM_PER_INCH)
    sign = "-" if units < 0 else ""
    units = abs(units)
    whole, rest = divmod(units, denominator)
    if rest == 0:
        return f'{sign}{whole}"'
    common = gcd(rest, denominator)
    fraction = f"{rest // common}/{denominator // common}"
    return f'{sign}{whole}-{fraction}"' if whole else f'{sign}{fraction}"'


def band_thickness(band: dict | None) -> int:
    return band["thickness"] if band else 0


def split_evenly(total: int, count: int) -> list[int]:
    base, remainder = divmod(total, count)
    return [base + 1 if i < remainder else base for i in range(count)]


def evaluate(project: dict) -> tuple[list[str], list[str]]:
    library = project["library"]
    materials = {m["id"]: m for m in library["materials"]}
    bands = {b["id"]: b for b in library["edgeBands"]}
    standards = {s["id"]: s for s in library["standards"]}

    cutlist, operations = [CUTLIST_HEADER], [OPERATIONS_HEADER]
    for product in project["products"]:
        std = standards[product["standard"]]
        roles = product["materials"]
        case, back, doors = (materials[roles[r]] for r in ("case", "back", "doors"))
        case_band, front_band = bands[roles["edgeBandCase"]], bands[roles["edgeBandFronts"]]

        w, h, d = product["width"], product["height"], product["depth"]
        t = case["actualThickness"]
        hb = h - std["toeKick"]["height"]
        gap = std["fronts"]["gap"]
        gd = std["back"]["grooveDepth"]
        s = std["stretchers"]["width"]

        no_band = (None, None, None, None)
        y1_case = (None, None, None, case_band)
        all_front = (front_band,) * 4
        parts = [
            ("case/side.left", case, hb, d, y1_case),
            ("case/side.right", case, hb, d, y1_case),
            ("case/bottom", case, w - 2 * t, d, y1_case),
            ("case/stretcher.front", case, w - 2 * t, s, y1_case),
            ("case/stretcher.back", case, w - 2 * t, s, no_band),
            ("case/back", back, hb - t + gd, w - 2 * t + 2 * gd, no_band),
        ]

        insert = product["front"]["insert"]
        count = insert["count"]
        if insert["kind"] != "doors" or count not in (1, 2):
            raise ValueError(f"unsupported front insert in {product['label']}")
        door_length = hb - gap
        door_widths = split_evenly(w - gap - (count - 1) * gap, count)
        hinge_sides = [insert.get("hingeSide", "left")] if count == 1 else ["left", "right"]
        for number, door_width in enumerate(door_widths, start=1):
            parts.append((f"front/door.{number}", doors, door_length, door_width, all_front))

        for key, material, length, width, edge_bands in parts:
            if length <= 0 or width <= 0:
                raise ValueError(f"non-positive size for {product['label']} {key}")
            x0, x1, y0, y1 = edge_bands
            cut_length = length - band_thickness(x0) - band_thickness(x1)
            cut_width = width - band_thickness(y0) - band_thickness(y1)
            thickness = material["actualThickness"]
            cutlist.append(",".join(str(v) for v in (
                product["label"], key, material["name"], product["quantity"],
                format_inches(length), format_inches(width), format_inches(thickness),
                length, width, thickness, cut_length, cut_width,
                *((e["name"] if e else "") for e in edge_bands),
            )))

        hinges = std["hinges"]
        for (number, door_width), side in zip(enumerate(door_widths, start=1), hinge_sides):
            y = hinges["edgeDistance"] if side == "left" else door_width - hinges["edgeDistance"]
            xs = [hinges["endDistance"], door_length - hinges["endDistance"]]
            if door_length > hinges["thirdHingeAbove"]:
                xs.append(door_length // 2)
            for x in sorted(xs):
                operations.append(",".join(str(v) for v in (
                    product["label"], f"front/door.{number}", "B", "hole",
                    x, y, hinges["cupDiameter"], hinges["cupDepth"],
                )))

    return cutlist, operations


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__.strip().splitlines()[-1], file=sys.stderr)
        return 1
    project = load_project(Path(sys.argv[1]))
    cutlist, operations = evaluate(project)
    out = Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    for name, lines in (("cutlist.csv", cutlist), ("operations.csv", operations)):
        (out / name).write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
