<!--
SPDX-FileCopyrightText: 2026 PROJECT_NAME contributors
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# 0001 — Record decisions with MADR

- **Status:** accepted
- **Date:** 2026-10-25
- **Deciders:** Andrés (@A-newGP)

## Context and problem statement

Many early decisions (licenses, contribution rules, data model, stack) are
hard to reverse and will be questioned by future contributors. How do we keep
the reasoning behind them available and reviewable?

## Decision drivers

- Newcomers must understand *why*, not only *what*.
- Independent development must be documented: each technical decision lists
  its public sources (clean-room policy).
- Low ceremony for a one-person project.

## Considered options

- Markdown ADRs in the [MADR](https://adr.github.io/madr/) format
- Free-form design notes in a wiki
- No written record

## Decision outcome

Chosen option: **MADR ADRs in `docs/adr/`**, one file per decision, reviewed
through pull requests like code. Large, open-ended proposals use RFCs in
`docs/rfcs/`; an accepted RFC usually produces one or more ADRs.

### Consequences

- Good, because decisions are versioned, reviewable and linkable.
- Good, because the "Sources" section documents independent development.
- Bad, because writing ADRs takes time; we keep them short.

## Sources

- Michael Nygard, "Documenting Architecture Decisions" (2011).
- MADR project, <https://adr.github.io/madr/>.
