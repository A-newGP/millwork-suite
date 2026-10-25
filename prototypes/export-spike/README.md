<!--
SPDX-FileCopyrightText: 2026 OpenCabinet contributors
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Export spike

> [!WARNING]
> **Throwaway code.** This prototype tests the Rust libraries for the
> exports version 0.1 promises: CSV, PDF and DXF. ADR-0011 named them the
> weakest part of the Rust ecosystem. It will be deleted once phase 2
> records its choice in an ADR.

`core/` is the UI spike's evaluator and elevation drawing. It turns the
sample kitchen (four cabinets) into a job: one elevation per cabinet, the
cut list and the doors with their hinge holes. `core/src/sheet.rs` lays
those out as lines, circles and text on named layers, so each exporter only
maps three primitives to its format and the comparison measures the
libraries, not the drawing code.

| Exporter | Library | Writes |
| --- | --- | --- |
| `plain/` | none (standard library) | DXF R12 elevations and machining, CSV cut list |
| `dxf/` | [`dxf`](https://crates.io/crates/dxf) 0.6 (MIT) | DXF 2018 elevations and machining |
| `krilla/` | [`krilla`](https://crates.io/crates/krilla) 0.8 (MIT or Apache-2.0) | PDF shop drawings: one page per elevation, then the cut list |
| `printpdf/` | [`printpdf`](https://crates.io/crates/printpdf) 0.12 (MIT) | The same PDF |

```bash
cargo run -p oc-export-krilla -- out
```

## How the files are checked

No proprietary software is involved. In CI, `ci/verify.py` reads every file
back with independent open source readers: **ezdxf** audits each DXF and
counts its entities and labels, **pypdf** extracts the PDF text and checks
that every label and cut length is present, **qpdf** checks the PDF
structure, and Python's `csv` module parses the CSV. CI also renders every
file to PNG and uploads all of them as the `exported-files` artifact, so
anyone can look at the results.

## Results

From a local release build on Windows; CI repeats them on Linux and Windows.

| Exporter | Formats | Lines of code | Crates | Executable | Run (median of 5) |
| --- | --- | --- | --- | --- | --- |
| plain | DXF R12, CSV | 125 | 15 | 429 KB | 48 ms |
| dxf | DXF 2018 | 81 | 46 | 1,373 KB | 52 ms |
| krilla | PDF | 141 | 87 | 3,188 KB | 78 ms |
| printpdf | PDF | 125 | 104 | 1,991 KB | 54 ms |

The 15 crates of `plain` are the core's (serde and JSON); run times are
mostly process start-up.

| File | Version | Units or fonts | Audit errors | Audit fixes | Size |
| --- | --- | --- | --- | --- | --- |
| plain-elevations.dxf | R12 (AC1009) | none declared | 0 | 0 | 12.4 KB |
| dxf-elevations.dxf | 2018 (AC1032) | millimeters | 0 | 7 | 38.0 KB |
| krilla-shop.pdf | PDF 1.7 | Arial, subset embedded | | | 33.5 KB |
| printpdf-shop.pdf | PDF 1.3 | Helvetica, built in | | | 7.5 KB |

Every dependency of every exporter is under a permissive license (MIT,
Apache-2.0, BSD, Zlib, Unicode-3.0), all compatible with GPL-3.0-or-later.

## Findings

### DXF

1. **Hand-written R12 is clean but cannot declare units.** It passes the
   audit with no issue and needs no dependency, but R12 has no units
   header: the CAM program assumes or asks. A part cut in inches instead of
   millimeters is scrap, so this is a real risk for shops.
2. **The `dxf` crate writes modern DXF with units in the fewest lines, with
   two defects.** Its default entity transparency (0) is invalid, which
   the exporter overrides in one line, and its table headers carry no
   handles, which ezdxf repairs (the 7 fixes). Whether strict readers
   reject such files is untested here. Its R12 output could not be read at
   all, so R12 would stay hand-written.
3. **Recommendation for phase 2:** the `dxf` crate for DXF 2000 or later,
   with fixes for both defects proposed upstream and the ezdxf audit kept
   in CI, so millimeters are always declared.

### PDF

1. **krilla embeds the font; printpdf uses the built-in Helvetica.**
   Embedding makes every reader show the same thing and covers all of
   Unicode, at the cost of a larger file and of needing a font. Built-in
   fonts are small but limited to the Western European character set, and
   each reader substitutes its own Helvetica.
2. **The product must ship its own font.** krilla here searches for a
   system font, which differs between machines. OpenCabinet should bundle
   one open font (for example under the SIL Open Font License) so drawings
   look the same everywhere.
3. **Neither library measures text.** Centering a dimension label needs
   the text's width; the spike estimates it. A text layout library
   (parley, which krilla supports, or rustybuzz) is needed for production
   quality labels.
4. **Recommendation for phase 2:** krilla. It is the one with embedded
   fonts, full Unicode, and support for archival PDF/A and tagged
   (accessible) PDF, which matters for drawings kept for years and for
   screen reader users. printpdf pulls in more crates even with its HTML
   layout feature turned off.

### CSV

1. **CSV needs no library, but Excel in Spanish reads it wrong.** The
   standard (RFC 4180) uses commas, and every inch mark forces quoting.
   Excel in locales that write decimals with a comma, Spanish among them,
   expects semicolons and splits the file wrongly. Version 0.1 should let
   the user choose the separator, or also write a spreadsheet file.
