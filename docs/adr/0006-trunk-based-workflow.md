<!--
SPDX-FileCopyrightText: 2026 PROJECT_NAME contributors
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# 0006 — Trunk-based workflow, Conventional Commits and SemVer

- **Status:** accepted
- **Date:** 2026-10-25
- **Deciders:** Andrés (@A-newGP)

## Context and problem statement

How do changes reach the main branch, how are they described, and how are
releases numbered?

## Decision drivers

- A readable, linear history on `main`.
- Release notes that can be generated from the history.
- Version numbers that tell users and plugin authors what changed.

## Considered options

- Trunk-based development with short-lived branches and squash merges
- Git Flow (long-lived `develop` and release branches)

## Decision outcome

Chosen option: **trunk-based development**.

- `main` is protected; every change enters through a pull request with
  green CI.
- Pull requests are **squash-merged**; the pull request title becomes the
  commit and must follow
  [Conventional Commits 1.0](https://www.conventionalcommits.org/en/v1.0.0/)
  (checked in CI).
- Releases follow [Semantic Versioning 2.0](https://semver.org/). The plugin
  API is versioned separately from the application.
- A `CHANGELOG.md` in the [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
  format starts with the first release; there is nothing to log before it.

### Consequences

- Good, because history on `main` is one meaningful commit per change.
- Good, because commit types map directly to SemVer bumps and changelog
  sections.
- Bad, because contributors' intermediate commits are not kept on `main`.

## Sources

- Conventional Commits 1.0.0, <https://www.conventionalcommits.org/en/v1.0.0/>.
- Semantic Versioning 2.0.0, <https://semver.org/>.
- Keep a Changelog 1.1.0, <https://keepachangelog.com/en/1.1.0/>.
- Trunk Based Development, <https://trunkbaseddevelopment.com/>.
