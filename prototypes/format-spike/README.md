<!--
SPDX-FileCopyrightText: 2026 OpenCabinet contributors
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Format spike

> [!WARNING]
> **Throwaway code.** This prototype exists to test the `.ocab` format of
> [RFC-0001](../../docs/rfcs/README.md) before it is accepted, and to meet the
> phase 1 exit gate: *the prototype saves and reopens a project in our own
> format*. It will be deleted once RFC-0001 is accepted.

`oc-format` reads and writes the authored data of RFC-0001: the project,
its settings, rooms, walls with openings, placements on a wall or free,
products with their front split tree, hardware adjustments, overrides,
revisions, extensions and the library snapshots. It does not evaluate
products into parts.

```bash
cargo run -- check tests/fixtures/kitchen.ocab
```

```bash
cargo run -- convert tests/fixtures/kitchen.ocab kitchen.ocab
```

```bash
cargo run --release -- bench 1000
```

## The container

```text
kitchen.ocab               a ZIP file, or a directory with the same entries
├── mimetype               application/vnd.opencabinet.project+zip, first and uncompressed
├── manifest.json          formatVersion, generator, entries with media types
├── project.json           the project (RFC-0001 §4–§6, §10, §11)
├── library.json           snapshots of materials, edge bands, hardware, standards (§7, §8)
└── attachments/…          anything else, kept byte for byte
```

## Evidence collected

`tests/format.rs` checks, against two fixtures written by hand:

- **Lossless round trip.** Loading and saving reproduces both fixtures byte
  for byte, in the ZIP and the directory form.
- **Forward compatibility.** `future-minor.ocab` imitates a file from a
  newer minor version: unknown fields at every level, a new library
  section, a new insert kind (`tambour`) and a plugin's entry. All of it
  survives a save, and the product with the unknown insert is reported as
  an error instead of being dropped or misread.
- **A newer major version is rejected** before the rest is parsed.
- **Deterministic output.** The same project always produces the same
  bytes: fixed field order, sorted maps, fixed ZIP timestamps. Projects diff
  cleanly in version control.
- **Hostile files.** Entry names that could escape the project (`..`,
  absolute paths, drive letters, backslashes), links in the directory form,
  too many entries and oversized contents are rejected.
- **Validation** of identity (UUIDv7, used once), references, placement
  forms, the quantity rule, openings outside walls and the front tree.

## Findings for RFC-0001

1. **Open and closed enumerations.** Keeping unknown fields is easy (one
   catch-all per object), but an unknown *value* of an enumeration, such as
   a new insert kind, breaks a strict reader. The specification should say
   which enumerations may grow in a minor version (insert kinds, front
   styles, override actions, wall opening kinds) and how a reader treats an
   unknown value: keep it, and block evaluation of that product.
2. **Never relabel a newer file.** When a reader saves a file from a newer
   minor version, it must keep the newer `formatVersion`. Otherwise a newer
   reader would take the preserved fields for something an older writer
   produced.
3. **Angles are not defined.** Free placements need a rotation. The spike
   stores integer microdegrees, exact like lengths; RFC-0001 or ADR-0009
   should choose.
4. **Parameters cannot be typed from the file alone.** A `length` and a
   `count` are both JSON integers. Either the product type's declaration
   gives the types (the spike's choice), or each value carries its type.
5. **Saving over a directory is undefined.** The directory form needs a rule
   for files a user added by hand, and for line endings when it is kept in
   Git.
6. **Strict writers, lenient readers.** A project re-zipped by an operating
   system tool loses the `mimetype`-first rule. The spike reads it with a
   warning; the specification should say so explicitly.
7. **The manifest's entry list** duplicates the ZIP directory. It earns its
   place by giving each attachment a media type.

## Measurements

From CI (GitHub-hosted runners, run 40226820700, release build). The
sample kitchen is 13.9 KB of JSON and 2.4 KB as ZIP.

| Products | ZIP | Unpacked | Linux save | Linux load | Windows save | Windows load |
| --- | --- | --- | --- | --- | --- | --- |
| 10 | 1 KB | 14 KB | 0.2 ms | 0.1 ms | 1.3 ms | 0.3 ms |
| 100 | 5 KB | 131 KB | 0.9 ms | 0.5 ms | 1.6 ms | 1.3 ms |
| 1,000 | 37 KB | 1,303 KB | 6.7 ms | 3.3 ms | 8.5 ms | 5.5 ms |
| 10,000 | 345 KB | 13,036 KB | 63.1 ms | 34.2 ms | 75.4 ms | 51.2 ms |

Times are the median of five runs. The synthetic projects repeat one
cabinet, so they compress far better than real ones. Even a project of ten
thousand cabinets saves and loads in well under a tenth of a second, so the
format needs no incremental or binary encoding.
