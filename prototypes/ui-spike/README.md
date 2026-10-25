<!--
SPDX-FileCopyrightText: 2026 OpenCabinet contributors
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# UI spike

> [!WARNING]
> **Throwaway code.** These prototypes exist only to gather evidence for the
> desktop user interface ADR. They will be deleted once the ADR is accepted.

Three prototypes implement the same screen with the same Rust core, so the
comparison measures the user interface toolkits, not the geometry:

| Directory | Toolkit | Rendering |
| --- | --- | --- |
| `slint/` | [Slint](https://slint.dev/) 1.18 | Native, declarative `.slint` markup |
| `egui/` | [egui](https://www.egui.rs/) / eframe 0.37 | Native, immediate mode |
| `tauri/` | [Tauri](https://tauri.app/) 2 | System web view (WebView2, WebKitGTK), HTML, CSS and SVG |

## The screen

- **Parameters:** width, height and depth in shop notation (`30`, `30.5`,
  `30-1/2`, `30 1/2"`) and the number of doors.
- **Elevation:** the base cabinet's front view with toe kick, doors, hinge
  cups and dimensions, fitted to the available space.
- **Cut list:** a table that updates on every keystroke. Invalid input shows
  an error and keeps the last valid result.

`core/` (`oc-ui-core`) holds the evaluator from the stack spike, inch
parsing, and the elevation as resolution-independent shapes in millimeters,
including label positions. Each front end only maps shapes to the screen.

## Evidence collected

- **Behavior:** headless tests drive the real UI and check that editing
  updates the cut list and that invalid input is rejected:
  `slint/tests/edit.rs` (Slint testing backend, through the accessibility
  tree) and `egui/tests/edit.rs` (egui_kittest, through keyboard events). The
  Tauri front end has no equivalent without a WebDriver setup, which is
  itself a finding.
- **Measurements:** `.github/workflows/ui-spike.yml` builds each front end
  cold in release mode on Linux and Windows and runs `measure-ui.sh`:
  executable size, launch to first presented frame (median of 5), memory of
  the whole process tree and build time.

With `OC_MEASURE=1`, each prototype prints `FIRST_FRAME` once its first frame
is presented and exits three seconds later.
