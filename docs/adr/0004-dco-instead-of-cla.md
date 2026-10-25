<!--
SPDX-FileCopyrightText: 2026 PROJECT_NAME contributors
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# 0004 — DCO instead of a CLA

- **Status:** accepted
- **Date:** 2026-10-25
- **Deciders:** Andrés (@A-newGP)

## Context and problem statement

The project must know that each contribution can legally be distributed
under its licenses. Should contributors sign a Contributor License Agreement
(CLA) or certify the Developer Certificate of Origin (DCO)?

## Decision drivers

- Low barrier for new contributors.
- Contributors keep their copyright; no single party can relicense the
  project as proprietary.
- A clear, per-commit record of provenance.

## Considered options

- DCO 1.1 sign-off on every commit
- A CLA assigning or broadly licensing rights to the project
- No formal mechanism

## Decision outcome

Chosen option: **DCO 1.1**. Every commit carries a
`Signed-off-by: Name <email>` trailer matching its author (`git commit -s`).
CI rejects pull requests with unsigned commits; bot accounts such as
Dependabot are exempt because they cannot certify anything.

Pull requests are squash-merged; GitHub keeps the commits' trailers in the
squash message only when the repository's default squash message includes
the commit details (see the GitHub setup guide).

### Consequences

- Good, because there is no paperwork and contributors keep their rights.
- Good, because copyright stays spread across contributors, which protects
  the copyleft promise.
- Bad, because relicensing later (for example GPL "-or-later" to "-only")
  requires every contributor's agreement.

## Sources

- Developer Certificate of Origin 1.1, <https://developercertificate.org/>.
