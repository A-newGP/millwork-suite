<!--
SPDX-FileCopyrightText: 2026 OpenCabinet contributors
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Stack spike

> [!WARNING]
> **Throwaway code.** These prototypes exist only to gather evidence for the
> technical stack ADR. They are not the start of the product, are not
> maintained, and will be deleted once the ADR is accepted.

Two prototypes, one in **Rust** and one in **C#/.NET**, implement the same
small slice of [RFC-0001](../../docs/rfcs/0001-project-data-model.md) so
they can be compared on equal terms:

> Load a project in the `.ocab` format → evaluate frameless cabinets into
> parts and machining operations, with lengths as integer nanometers
> ([ADR-0009](../../docs/adr/0009-length-representation.md)) → write a cut
> list and an operations list → save the project as a `.ocab` ZIP, reopen
> it, and obtain the same result.

The rules below are deliberately simplified and are **not** a shop
standard. What matters is that every implementation follows them exactly.

## Layout

| Path | Contents |
| --- | --- |
| `fixtures/sample.ocab/` | Input project, in the unpacked directory form |
| `expected/` | Golden outputs every implementation must reproduce byte for byte |
| `reference/evaluate.py` | Independent reference implementation that generated the golden outputs |
| `rust/` | Rust prototype |
| `csharp/` | C#/.NET prototype |

## Command-line contract

Every prototype is a single executable with three commands:

```text
<exe> evaluate  <input> <output-dir>     # writes cutlist.csv and operations.csv
<exe> roundtrip <input> <temp.ocab>      # load, save as ZIP, reload, compare
<exe> bench     <input> <iterations>     # load once, evaluate N times, print ns per evaluation
```

`<input>` is either a `.ocab` ZIP file or a `.ocab` directory. Exit codes:
`0` success, `1` usage error, `2` invalid project, `3` round trip mismatch.

## Container

- A `.ocab` is a ZIP archive or a directory with the same entries.
- `mimetype`: exactly `application/vnd.opencabinet.project+zip`, without a
  trailing newline. In a ZIP it is the first entry, stored uncompressed.
- `manifest.json`: `{"formatVersion": "0.1", ...}`. Readers reject a major
  version other than `0`.
- `project.json`: the project. All lengths are JSON integers in nanometers.

## Evaluation rules

Notation for a product: `W`, `H`, `D` (width, height, depth), `T` the actual
thickness of the `case` material, `Td` of the `doors` material, `Tb` of the
`back` material. From the standard: `K` toe kick height, `G` front gap, `gd`
back groove depth, `S` stretcher width.

Box height: `Hb = H − K`.

Parts are produced in this order, each with **length** (grain direction),
**width** and **thickness**:

| Key | Material | Length | Width | Banded edges |
| --- | --- | --- | --- | --- |
| `case/side.left` | case | `Hb` | `D` | `y1` (case band) |
| `case/side.right` | case | `Hb` | `D` | `y1` (case band) |
| `case/bottom` | case | `W − 2T` | `D` | `y1` (case band) |
| `case/stretcher.front` | case | `W − 2T` | `S` | `y1` (case band) |
| `case/stretcher.back` | case | `W − 2T` | `S` | none |
| `case/back` | back | `Hb − T + gd` | `W − 2T + 2·gd` | none |
| `front/door.N` | doors | `Hb − G` | see below | all four (fronts band) |

**Doors.** The front insert declares `count` (1 or 2). The doors cover
`W − G` horizontally; with `n` doors the total door width is
`W − G − (n − 1)·G`. It is divided with integer arithmetic: each door gets
`floor(total / n)` and the remainder `r` is distributed one nanometer each to
the first `r` doors from the left. Doors are numbered from 1, left to right.

**Cut size.** `cut length = length − band(x0) − band(x1)` and
`cut width = width − band(y0) − band(y1)`, where `band(e)` is the thickness
of the edge band on edge `e` (0 if none).

**Hinge holes.** Door part frame: `x` runs along the length from the bottom
end (`x0`), `y` across the width from the left edge as seen from the front
(`y0`). A single door is hinged on the insert's `hingeSide`; with two doors,
door 1 is hinged left and door 2 right. Each hinge is a hole on face `B`
with the standard's cup diameter and depth, centered at
`y = edgeDistance` (left hinge side) or `y = width − edgeDistance` (right),
and at `x = endDistance` and `x = length − endDistance`. If the door length
exceeds `thirdHingeAbove`, a third hole is added at `x = floor(length / 2)`.
Holes are listed by ascending `x`.

**Quantity.** Every part row carries the product's `quantity`.

## Output

CSV, comma-separated, LF line endings, a header row, a final newline and no
quoting (names never contain commas). Rows follow the product order of the
file, then the part order above.

`cutlist.csv`:

```text
product,part,material,qty,length,width,thickness,length_nm,width_nm,thickness_nm,cut_length_nm,cut_width_nm,band_x0,band_x1,band_y0,band_y1
```

`length`, `width` and `thickness` are displayed in fractional inches rounded
to the nearest 1/32", ties away from zero, reduced: `30"`, `7/32"`,
`23-7/32"`, `0"`. Band columns hold the edge band name, or are empty.

`operations.csv`:

```text
product,part,face,kind,x_nm,y_nm,diameter_nm,depth_nm
```

## Measurements

The CI workflow `stack-spike.yml` builds both prototypes in release mode on
Linux and Windows, checks the outputs against `expected/` and the round
trip, and reports: executable size, start-up plus evaluation time of the
`evaluate` command, evaluation time from `bench`, peak memory (Linux), build
time, and lines of code.
