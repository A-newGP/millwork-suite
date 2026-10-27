<!--
SPDX-FileCopyrightText: 2026 OpenCabinet contributors
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Changelog

All notable changes to OpenCabinet are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions
follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0-beta.1] - 2026-10-26

The first test release of OpenCabinet Designer, for shops willing to try
it on a real job and tell us what is wrong. It is a beta: check every
size before cutting.

### Added

- **Cabinets:** base, wall and tall cabinets, frameless or with a face
  frame. Fronts are slab or five-piece doors, drawers and false fronts,
  with drawer boxes and adjustable shelves. Every size comes from the
  shop's construction standard and is exact to the nanometer, in inches
  or millimeters.
- **Several cabinets per project:** add, duplicate, rename and delete
  them; the cut list covers the whole project.
- **Cut list** on screen, as CSV (with commas, or semicolons for Excel in
  Spanish) and as PDF.
- **Machining:** the back's groove, 32 mm line boring for shelf pins and
  hinge plates, hinge cups, and the drawer bottom's groove.
- **Parts as DXF** for CAM programs: one file per part at its cut size,
  with layers named after the tool and depth, and one file with every
  part.
- **Walls:** walls at right angles with windows and doors, and cabinets
  placed on them, with warnings when a cabinet runs past a wall, covers
  an opening or collides with another, also around a corner.
- **Elevations** of every wall: on screen, as PDF at a standard scale, one
  wall to a page, and as DXF at full size.
- **Windows installer** that installs for the current user, without
  administrator rights.

### Known limitations

- Not yet: a bill of materials (sheets, edge band length, hardware), and
  blind corner, sink, filler and vanity cabinets, which show as "cannot
  be evaluated yet".
- The window edits the first room of a project; walls run at right
  angles; free-standing islands are not checked for collisions.
- Slide, pull and joinery holes, face frame pocket holes and five-piece
  profiles are not machined yet.
- Windows only. The installer is not signed yet: Windows warns that it
  protected your PC; choose **More info**, then **Run anyway**, after
  checking the installer against `SHA256SUMS.txt`.

[Unreleased]: https://github.com/OpenCabinet/opencabinet/compare/v0.1.0-beta.1...HEAD
[0.1.0-beta.1]: https://github.com/OpenCabinet/opencabinet/releases/tag/v0.1.0-beta.1
