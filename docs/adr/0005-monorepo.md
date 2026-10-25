<!--
SPDX-FileCopyrightText: 2026 OpenCabinet contributors
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# 0005 — Single monorepo

- **Status:** accepted
- **Date:** 2026-10-25
- **Deciders:** Andrés (@A-newGP)

## Context and problem statement

The suite will have an application, official modules, a file format
specification, documentation, ADRs and RFCs. Should they live in one
repository or several?

## Decision drivers

- One maintainer today: minimal overhead.
- Changes to the data model often touch the specification, the core and the
  modules at once; they should land atomically.
- "Structure grows with the community, not before."

## Considered options

- One monorepo
- One repository per component from the start

## Decision outcome

Chosen option: **one monorepo**. GitHub templates and workflows live in its
own `.github/` directory; a separate organization-wide `.github` repository
is created only when there is a second repository to share them with.

The website, the plugin registry and the plugin template move to their own
repositories when they exist and need independent releases.

### Consequences

- Good, because cross-cutting changes are reviewed and merged together.
- Good, because there is one place to search, one CI and one issue tracker.
- Bad, because per-module permissions need CODEOWNERS later, once there are
  module leads.

## Sources

- Project plan, section E (Spanish): [docs/es/plan.md](../es/plan.md).
