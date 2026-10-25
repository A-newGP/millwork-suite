<!--
SPDX-FileCopyrightText: 2026 OpenCabinet contributors
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# 0008 — Imperial units by default, metric as an option

- **Status:** accepted
- **Date:** 2026-10-25
- **Deciders:** Andrés (@A-newGP)

## Context and problem statement

Millwork shops work either in inches with fractions (1/16", 1/32", 1/64")
or in millimeters. Which unit system does the suite present to users?

## Decision drivers

- The first target users work in imperial units.
- Metric shops must be fully supported, not as an afterthought.
- Fractional inches must be exact: 23-7/32" must never display as
  23.21875" or round-trip to 23-15/64".

## Considered options

- Imperial by default, metric as a per-project option
- Metric only
- Imperial only

## Decision outcome

Chosen option: **imperial (fractional inches) by default, metric as a
first-class option**, selectable per project and per user preference.

This ADR decides **only the user-facing behavior**. How lengths are stored
internally (for example integers in a base unit that represents both 1 µm
and 1/128" exactly, or rational numbers) and the precision of each
operation are decided in a separate ADR during phase 1b, with tests in
fractional inches from day one.

### Consequences

- Good, because the default matches the first users' shops.
- Good, because supporting both from the start forces a unit-safe data
  model, preventing a whole class of bugs.
- Bad, because every input, output, report and export must handle both
  systems and their formatting rules.
