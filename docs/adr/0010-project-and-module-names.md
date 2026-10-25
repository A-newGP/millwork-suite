<!--
SPDX-FileCopyrightText: 2026 OpenCabinet contributors
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# 0010 — Project and module names

- **Status:** accepted
- **Date:** 2026-10-25
- **Deciders:** Andrés (@A-newGP)

## Context and problem statement

The project needs a name that users understand, that can be protected by
the trademark policy, and that does not collide with existing software. How
are the suite and its modules named?

## Decision drivers

- Users (shop owners) should understand what the software does.
- The name must be available on GitHub and as a domain.
- No collision with an established project or trademark in software.
- The suite will outgrow cabinets (shop drawings, PDF review, CNC, 2D CAD).

## Considered options

- **Openwork** — a real woodworking term, but an established open source
  desktop application already uses the name, and the GitHub account and the
  main domains are taken.
- **Joinwright** — an invented, highly distinctive name; less obvious to
  users.
- **OpenJoinery** — fully available, but descriptive.
- **OpenCabinet** — immediately understood; GitHub organization and the
  `.org`, `.dev` and `.app` domains available on 2026-10-25.

## Decision outcome

Chosen option: **OpenCabinet** for the suite, with **descriptive module
names prefixed by the suite name**: "OpenCabinet Designer", "OpenCabinet
Shop Drawings", "OpenCabinet CNC". Module identifiers in code and packages
use lowercase with hyphens (`opencabinet-designer`).

The suite name is the brand; module names describe functions and are not
brands of their own.

### Consequences

- Good, because users and search engines immediately associate the name
  with cabinet software.
- Good, because the prefix scheme keeps one brand while modules stay
  self-explanatory.
- Bad, because a descriptive name is a weaker trademark; the trademark
  policy may be harder to enforce. Legal review is pending.
- Bad, because a small project created in October 2026,
  `xiamingyang70/opencabinet` (a parametric wardrobe generator, MIT), uses
  the same name in the same field; the GitHub organization and domain should
  be secured promptly.
- Bad, because "cabinet" understates the scope of the suite; the module
  names carry that scope instead.

## Sources

- Availability checks on 2026-10-25: GitHub user and organization API, and
  RDAP lookups for `.com`, `.org`, `.dev` and `.app`.
