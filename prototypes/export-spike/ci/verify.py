# SPDX-FileCopyrightText: 2026 OpenCabinet contributors
# SPDX-License-Identifier: GPL-3.0-or-later

"""Checks the exporters' files with independent open source readers.

Usage: python -I verify.py <out-dir>. Needs ezdxf and pypdf. Prints a
Markdown table and exits non-zero on any failure.
"""

import csv
import sys
from collections import Counter
from pathlib import Path

import ezdxf
from pypdf import PdfReader

LABELS = ["B30", "B31", "B18", "T24"]
ELEVATION_ENTITIES = {"LINE": 92, "CIRCLE": 13, "TEXT": 16}
PAGES = 6  # four elevations and two cut list pages

out = Path(sys.argv[1])
failures = []
rows = []


def check(condition, message):
    if not condition:
        failures.append(message)


for name in ["plain", "dxf"]:
    for sheet in ["elevations", "machining"]:
        path = out / f"{name}-{sheet}.dxf"
        doc = ezdxf.readfile(path)
        auditor = doc.audit()
        kinds = Counter(e.dxftype() for e in doc.modelspace())
        check(not auditor.has_errors, f"{path.name}: {len(auditor.errors)} audit errors")
        if sheet == "elevations":
            check(dict(kinds) == ELEVATION_ENTITIES, f"{path.name}: entities {dict(kinds)}")
            texts = {e.dxf.text for e in doc.modelspace().query("TEXT")}
            check(set(LABELS) <= texts, f"{path.name}: labels missing")
        rows.append((path.name, doc.dxfversion, doc.units, len(auditor.errors),
                     len(auditor.fixes), path.stat().st_size))

with open(out / "cut-list.csv", newline="", encoding="utf-8") as f:
    table = list(csv.reader(f))
check(table[0][:2] == ["Product", "Part"], "cut-list.csv: header")
check(all(len(r) == 7 for r in table), "cut-list.csv: ragged rows")
check(table[1][4].endswith('"'), "cut-list.csv: inch marks lost")
cut_lengths = {r[4] for r in table[1:]}

for name in ["krilla", "printpdf"]:
    path = out / f"{name}-shop.pdf"
    reader = PdfReader(path)
    text = "\n".join(page.extract_text() for page in reader.pages)
    check(len(reader.pages) == PAGES, f"{path.name}: {len(reader.pages)} pages")
    check(all(label in text for label in LABELS), f"{path.name}: labels missing")
    missing = [length for length in cut_lengths if length not in text]
    check(not missing, f"{path.name}: cut lengths missing from text: {missing[:3]}")
    fonts = set()
    for page in reader.pages:
        for font in page["/Resources"].get("/Font", {}).values():
            font = font.get_object()
            embedded = "/FontDescriptor" in font or "/DescendantFonts" in font
            fonts.add(f"{font['/BaseFont']} ({'embedded' if embedded else 'built in'})")
    rows.append((path.name, reader.pdf_header, ", ".join(sorted(fonts)), "-", "-",
                 path.stat().st_size))

print("| File | Version | Units or fonts | Audit errors | Audit fixes | Bytes |")
print("| --- | --- | --- | --- | --- | --- |")
for row in rows:
    print("| " + " | ".join(str(c) for c in row) + " |")
for failure in failures:
    print(f"FAIL: {failure}", file=sys.stderr)
sys.exit(1 if failures else 0)
