# Appendix A — Decision log (authoritative)

Decisions made by the owner on **21 November 2026**. **Changes require an ADR approved by the
owner** (`docs/adr/NNNN-title.md`).

Precedence: this log sits directly below the hard rules (Part III, 3.2) and **above** the Part II
technical design.

| # | Topic | Decision |
| --- | --- | --- |
| **A1** | Project name | "OpenCity" as a temporary working title; rename before public launch (name already used by a 2003 GPL game). |
| **A2** | Purpose | For fun, non-commercial, open source; flagship portfolio project. |
| **A3** | License | MIT for code. Assets: CC0 / CC-BY / OFL only. |
| **A4** | Repository | Local path `D:\app\Others\OpenCity`; hosted on GitHub; docs and README in English. |
| **A5** | Dev machine | Personal Windows laptop, 8–16 GB RAM (exact TBD), GPU TBD, ~200 GB free SSD; Claude Code installed; owner has tried Rust. |
| **A6** | Platforms | Windows (primary) and Linux; browser as a later, lighter demo. |
| **A7** | Hardware target | Scalable presets Low–Ultra; minimum spec chosen by Claude (see 1.5). |
| **A8** | Performance | 60 fps at 1080p and 1440p on suitable hardware. |
| **A9** | Controls | Mouse + keyboard only. |
| **A10** | Visual style | Realistic, implemented as high-quality semi-realistic PBR. |
| **A11** | Camera | Free 3D city camera + street level + first person (first person after the foundation is stable). |
| **A12** | Zoom limit | Maximum altitude about 5 km. |
| **A13** | Time of day | Day-night cycle: yes. |
| **A14** | Weather | Rain only. |
| **A15** | Lighting | Real-time shadows. |
| **A16** | Visual effects | Tilt-shift miniature effect, water reflections, city lights at night. |
| **A17** | Data overlays | Required (v1.0), available from v0.1. |
| **A18** | UI | English; radial menu. |
| **A19** | World | A region of several connected cities; 16 km × 16 km city plots. |
| **A20** | Terrain | Presets flat / hills, rivers, sea + terraforming tools; procedural random generation. |
| **A21** | Roads | Both free curves and grid-snapped. |
| **A22** | Building placement | Manual placement only (no zoning). |
| **A23** | Game mode | Sandbox only: unlimited money, cannot lose. |
| **A24** | Taxes | None. Citizen happiness is the core metric. |
| **A25** | Systems | Traffic, power, water, sewage, garbage, police, fire, health, education, pollution, land value, public transport. |
| **A26** | Transport | Bus, MRT, ships/ferries, airport, yachts, helicopters, flying cars and other modern modes. |
| **A27** | Agents | Individual cars and citizens, clickable and followable. |
| **A28** | Population | More than 1 million, using agent LOD (statistical everywhere, full agents near camera). |
| **A29** | Disasters | Earthquake only for now; can be turned off. |
| **A30** | Time controls | Pause, 1x, 2x, 3x. |
| **A31** | Building editor | Not answered — not planned before v1.0 (open question Q2). |
| **A32** | Mod support | Provided but optional (off by default). |
| **A33** | Extras | Autosave and city-growth timelapse. |
| **A34** | Multiplayer | No. |
| **A35** | Asset sources | Mix: procedural code + CC0 packs + optional AI generators, all with open licenses. |
| **A36** | Building styles | Supertall CBD (Sudirman / Shanghai inspired, generic) and international modern. |
| **A37** | Audio | Music required (normal background music). |
| **A38** | Owner involvement | Playtest each milestone; approve key decisions only. |
| **A39** | Reporting | Report per milestone (in Bahasa Indonesia). |
| **A40** | Budget | Rp0 for now (free tools and assets only). |
| **A41** | Implementation | Claude Code (Opus 5.5 / Fable 5.2) writes everything; Claude may install needed tools, skills and MCP servers. |
| **A42** | Preview timing | Foundation preview as fast as possible (target 1–3 agent working days). |

## Amendments

Record every change below with a link to the approving ADR. Nothing here may be edited silently.

| Date | Decision changed | ADR | Summary |
| --- | --- | --- | --- |
| — | — | — | *(none yet)* |

### Notes added during M0 (measurement, not a decision change)

- **A5 is now measured**, not TBD: Lenovo laptop, Intel Core Ultra 5 125U, 15.5 GB RAM, Intel
  Graphics (integrated, Meteor Lake Xe-LPG), 1920 × 1200 panel, 214.9 GB free on `D:`. Full detail in
  [`docs/reports/M0-environment.md`](../reports/M0-environment.md). This resolves open question **Q3**
  and fixes the dev laptop at the **Minimum / Low preset tier** (1080p, 30 fps target).
