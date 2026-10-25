<!--
SPDX-FileCopyrightText: 2026 OpenCabinet contributors
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Governance

> [!NOTE]
> **Draft.** This document describes how the project is run today and how
> that will change. Changes to it go through an RFC once the project has
> more than one maintainer.

## Current model: founder-maintainer

OpenCabinet starts with a single maintainer, its founder, who decides
following the public rules below. The model is honest about its size and is
designed to be replaced by collective governance.

| Person | Role | Since |
| --- | --- | --- |
| Andrés ([@A-newGP](https://github.com/A-newGP)) | Founder and maintainer | 2026 |

## Roles

| Role | How to get there | What they can do |
| --- | --- | --- |
| Contributor | First accepted contribution (code, docs, translation, testing) | Propose changes and RFCs |
| Reviewer | Sustained contributions in an area + nomination | Approve pull requests in that area |
| Module lead | Sustained reviewing + maintainers' agreement | Decide and release their module |
| Core maintainer | Elected by the core maintainers | Change the core and API; release the suite |
| Founder | — | Final tie-break, only until the technical council exists |

The exact criteria (number of contributions, time) will be published here
**before** anyone is appointed to a role.

## How decisions are made

1. Proposals start as a public issue or discussion.
2. Changes land through pull requests with DCO sign-off and green CI.
3. Large changes (plugin API, file format, new official modules,
   governance) need an accepted RFC after a public comment period.
4. Technical disagreements: discussion in the issue → the module lead
   decides → appeal to the founder (later, the technical council). Every
   decision is written down with its reasoning.

## Code of conduct enforcement

Conduct reports follow the [Code of Conduct](CODE_OF_CONDUCT.md) in a
private channel, separate from technical discussions. While the founder is
the only maintainer, the project will seek a second trusted moderator so no
one acts as both party and judge.

## Transition to a technical council

A technical council replaces the founder's tie-break when the project has
**three active maintainers**, ideally from different organizations.

## Continuity

- The GitHub organization will have two owners with 2FA and stored recovery
  codes.
- Domain, email and service accounts belong to the project, not to a person.
- The release process will be documented so that someone else can ship a
  version.
- If the founder steps down, access and responsibilities pass to the core
  maintainers; until there are any, a successor will be named here.
