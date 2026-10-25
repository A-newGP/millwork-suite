<!--
SPDX-FileCopyrightText: 2026 OpenCabinet contributors
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# 0002 — License per component

- **Status:** accepted
- **Date:** 2026-10-25
- **Deciders:** Andrés (@A-newGP)

## Context and problem statement

The mission requires that no distributed derivative of the suite can become
proprietary, while users' files must stay readable by any software. Which
license fits each kind of component?

## Decision drivers

- Derivatives that are distributed must stay open (copyleft).
- Modified versions offered over a network must also share their source.
- Users' data must be interoperable, even with closed software.
- Plugin authors should be able to start from templates freely.
- Every dependency of the official distribution must be GPLv3-compatible.

## Considered options

- GPL-3.0 for everything
- A copyleft license per component (GPL, AGPL, CC, MIT)
- MPL-2.0 or Apache-2.0 for the core

## Decision outcome

Chosen option: **a license per component**:

| Component | License | Reason |
| --- | --- | --- |
| Core and official desktop modules | GPL-3.0-or-later | Protects distributed derivatives |
| Server services | AGPL-3.0-or-later | Also covers use over a network |
| File format specification | CC-BY-4.0, published separately | Anyone, including closed software, can read users' files |
| Plugin templates and examples | MIT | Copying a template imposes nothing; the plugin is still subject to the GPL if combined with the core |
| Documentation and interface icons | CC-BY-SA-4.0 | Free, and derivatives stay free |
| Trivial configuration files | CC0-1.0 | Not meaningfully copyrightable; anyone may reuse them |
| Name, logo and "official" badge | Trademark policy | Code licenses do not grant the right to appear official |

**"Or later" is provisional.** It lets the project adopt future GPL versions
without asking every contributor, at the cost of trusting the Free Software
Foundation to draft them. A future ADR may switch to "-only" before external
contributions arrive; after that, the change needs every contributor's
agreement.

**Open question:** code samples inside CC-BY-SA documentation inherit that
license, which is awkward for readers who copy them into programs. A future
ADR should decide whether samples are additionally offered under MIT.

### Consequences

- Good, because no distributed or network-hosted derivative can be closed.
- Good, because the open file format favors interoperability.
- Bad, because GPLv3-incompatible dependencies (for example GPL-2.0-only
  libraries or proprietary SDKs such as the Open Design Alliance's) cannot
  be bundled; each dependency must be checked.
- Bad, because combining AGPL libraries (for example MuPDF) into the desktop
  app adds AGPL obligations to that combined binary.

## Pros and cons of the options

### GPL-3.0 for everything

- Good, because it is simple.
- Bad, because it leaves the network ("SaaS") gap open and makes the file
  format specification awkward to implement in other software.

### MPL-2.0 or Apache-2.0 for the core

- Good, because they attract companies.
- Bad, because large parts of a derivative could stay proprietary, which
  contradicts the mission.

## Sources

- GNU General Public License v3, <https://www.gnu.org/licenses/gpl-3.0.html>.
- GNU Affero General Public License v3, <https://www.gnu.org/licenses/agpl-3.0.html>.
- FSF, "Frequently Asked Questions about the GNU Licenses",
  <https://www.gnu.org/licenses/gpl-faq.html>.
- Creative Commons, "ShareAlike compatibility: GPLv3",
  <https://creativecommons.org/share-your-work/licensing-considerations/compatible-licenses/>.
- Project plan, section A (Spanish): [docs/es/plan.md](../es/plan.md).
