<!--
SPDX-FileCopyrightText: 2026 OpenCabinet contributors
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# 0003 — Declare licensing with the REUSE specification

- **Status:** accepted
- **Date:** 2026-10-25
- **Deciders:** Andrés (@A-newGP)

## Context and problem statement

With several licenses in one repository ([ADR-0002](0002-license-per-component.md)),
every file must state unambiguously who holds the copyright and which
license applies. How do we declare and verify that?

## Decision drivers

- Unambiguous, machine-readable licensing for every file.
- Automatic verification in CI.
- Support for files that cannot hold comments (images, JSON, fixtures).

## Considered options

- The [REUSE specification](https://reuse.software/) (SPDX headers,
  `LICENSES/` directory, `REUSE.toml`)
- Ad-hoc SPDX headers plus a hand-written license file per component
- One license file per top-level directory

## Decision outcome

Chosen option: **REUSE 3.3**.

- Every file starts with `SPDX-FileCopyrightText` and
  `SPDX-License-Identifier` tags.
- Files that cannot carry a header are declared in `REUSE.toml`.
- `LICENSES/` holds the verbatim official text of each license **in use**;
  a license text is added together with the first file that uses it.
- The root `LICENSE` keeps the GPL-3.0 text so that GitHub detects the
  project's main license.
- CI runs `reuse lint` on every push and pull request.

### Consequences

- Good, because licensing is verifiable by tools and by lawyers.
- Good, because it is a recognized standard, used by projects such as KDE.
- Bad, because every new file needs a header; CI catches omissions.

## Sources

- REUSE Specification 3.3, <https://reuse.software/spec-3.3/>.
- SPDX License List, <https://spdx.org/licenses/>.
