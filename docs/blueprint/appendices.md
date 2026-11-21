# Appendices B–F

*Appendix A (decision log) lives in its own file: [decision-log.md](decision-log.md).*

---

## Appendix B — CLAUDE.md template

> Transcribed as-is from the blueprint. The live version is [`/CLAUDE.md`](../../CLAUDE.md) at the
> repository root; it may carry extra project-specific notes but must never weaken anything below.

```markdown
# OpenCity - Agent Operating Rules

Source of truth: docs/blueprint/ (transcribed from docs/OpenCity_Blueprint.pdf).
Precedence: Hard rules > Decision log > Technical design > Overview.

## Hard rules (never break)
- Clean room: no EA/Maxis/SimCity code, binaries, data, assets or leaks.
- No ripped, recreated or lightly modified assets from other games.
- No trademarks (SimCity, Sims, Maxis, GlassBox, brand logos) in code/UI/assets.
- MIT code: never copy GPL/LGPL/AGPL code. cargo-deny allowlist only.
- Every asset recorded in assets/LICENSES.md (CC0, CC-BY, OFL only).
- No secrets, no telemetry, no runtime network calls.
- sim-core is deterministic and has no render/audio/UI/wall-clock dependencies.
- Work only inside D:\app\Others\OpenCity.

## Session start checklist
1. Read this file, docs/plans/<current milestone>.md, the latest docs/reports/*.md, docs/issues/*.md.
2. Check pinned versions in docs/TOOLING.md before using any API.

## Workflow
Plan (docs/plans) -> implement small steps -> tests -> /verify -> commit (Conventional Commits).
Render changes: screenshot-review. Hot paths: bench (fail on >10% regression).
Milestone done -> milestone-report (Bahasa Indonesia) -> wait for owner playtest -> tag.

## Ask the owner before
- Changing hard rules, decision log or architecture (write an ADR draft).
- New dependency outside the license allowlist; paid tools; new accounts.
- Bevy upgrade; major feature add/cut; GitHub remote or visibility changes.

## Stuck?
Same error 3 times -> stop, write docs/issues/<date>-<topic>.md, change approach or ask.

## Style
Rust 2024, rustfmt, clippy -D warnings, no unwrap in sim-core, tracing logs,
data-driven rules in crates/content (RON), regression test for every bug fix.
Code/docs/commits in English; reports to owner in Bahasa Indonesia.
```

---

## Appendix C — Project skill templates

The live skills are in [`.claude/skills/`](../../.claude/skills/) and are committed to git.

### C.1 `.claude/skills/verify/SKILL.md`

```markdown
---
name: verify
description: Run all OpenCity quality gates before every commit (format, lint, tests, licenses, replay).
---

# verify

Run from the repository root. Stop at the first failure, fix it, and re-run from the top.

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo nextest run --workspace`
4. `cargo deny check licenses bans`
5. If files under crates/sim-core changed: `cargo run -p replay --release -- --check fixtures/replays`
6. If files under crates/render or crates/procgen changed: invoke the screenshot-review skill.

Finish with one line per gate: PASS/FAIL and the key number (tests run, warnings, regressions).
```

### C.2 `.claude/skills/milestone-report/SKILL.md`

```markdown
---
name: milestone-report
description: Write the end-of-milestone report for the owner in Bahasa Indonesia.
---

# milestone-report

1. Gather: plan checklist status, verify results, bench numbers, screenshots, open issues.
2. Fill docs/reports/<milestone>.md using the template in docs/blueprint (Appendix D).
3. Keep it short: at most 3 decisions, each with a recommended option.
4. Never claim something works unless it was run and observed.
```

### C.3 `.claude/skills/asset-license-check/SKILL.md`

```markdown
---
name: asset-license-check
description: Verify every file under assets/ has an allowed license entry in assets/LICENSES.md.
---

# asset-license-check

1. List all files under assets/ (exclude LICENSES.md).
2. For each file, find its row in assets/LICENSES.md (source URL, author, license, date, changes).
3. Allowed licenses: CC0-1.0, CC-BY-3.0, CC-BY-4.0, OFL-1.1, MIT, Apache-2.0.
4. Report missing or disallowed entries and block the commit until fixed.
```

### Skills still to be created (M1+, per 3.4)

| Skill | Purpose |
| --- | --- |
| `bench` | Runs headless benchmark scenarios and compares with budgets (2.13). |
| `screenshot-review` | Launches the build at fixed camera bookmarks, captures images, reviews them visually. |
| `new-building-type` | Scaffolds a `BuildingDef` + generator rules + test for a new building type. |

---

## Appendix D — Milestone report template (Bahasa Indonesia)

```markdown
# Laporan Milestone <Mx> - <judul>

## Ringkasan (maks. 3 kalimat)

## Cara menjalankan
1. ...

## Yang selesai (sesuai Definition of Done)
- [x] ...
- [ ] ... (belum, alasannya)

## Screenshot
(fajar, siang, senja, malam, ketinggian 5 km, level jalan)

## Performa (laptop dev)
| Preset | Resolusi | FPS rata-rata | FPS 1% low | Tick sim |

## Masalah yang diketahui

## Keputusan yang dibutuhkan (maks. 3, dengan rekomendasi)

## Rencana milestone berikutnya
```

---

## Appendix E — Glossary

| Term | Meaning |
| --- | --- |
| **ADR** | Architecture Decision Record: a short document describing a design decision, options and rationale. |
| **Agent bubble** | Area around the camera focus (or followed agent) where full Tier 2 agents are simulated. |
| **BPR function** | Bureau of Public Roads link-performance function relating travel time to volume/capacity. |
| **CDLOD** | Continuous Distance-Dependent Level of Detail — a quadtree terrain LOD technique without cracks. |
| **Definition of Done (DoD)** | Checklist that must be true before a milestone counts as finished. |
| **ECS** | Entity Component System — Bevy's data-oriented architecture. |
| **Floating origin** | Rendering relative to the camera to avoid floating-point jitter far from the world origin. |
| **IDM** | Intelligent Driver Model — a standard microscopic car-following model. |
| **Impostor** | A flat, camera-facing image that stands in for a distant 3D object. |
| **Isochrone** | Area reachable within a given travel time over the road network. |
| **Instancing** | Drawing many copies of the same mesh in a single GPU call. |
| **LOD** | Level of detail — simpler representations for distant or less important things (render or simulation). |
| **OD matrix** | Origin-destination matrix of trips between districts. |
| **PBR** | Physically Based Rendering — materials described by albedo, roughness, metallic, normal maps. |
| **SoA** | Struct of arrays — storing each field in its own array for cache-friendly bulk processing. |
| **SSR / SSAO** | Screen-space reflections / screen-space ambient occlusion. |
| **Tick** | One fixed simulation step (100 ms of real time at 1x). |

---

## Appendix F — References (public sources)

- Game Developer: "GDC 2012: Breaking down SimCity's Glassbox engine" —
  `gamedeveloper.com/design/gdc-2012-breaking-down-em-simcity-em-s-glassbox-engine`
  (**concepts only**).
- EA: "SimCity Insider's Look: GlassBox Game Engine" video series —
  `ea.com/en-ca/news/insiders-look-at-simcity-new-simulation-engine` (resources, units, maps, agents;
  **concepts only**).
- Bevy release notes — `bevy.org/news` (pin the version you build against).
- Claude Code skills documentation — `code.claude.com/docs/en/skills`; Anthropic skills marketplace —
  `github.com/anthropics/skills`.
- Treiber, Hennecke, Helbing (2000), "Congested traffic states in empirical observations and
  microscopic simulations", *Physical Review E* 62 (IDM).
- Bureau of Public Roads (1964), *Traffic Assignment Manual* (BPR function).
- Geisberger, Sanders, Schultes, Delling (2008), "Contraction Hierarchies: Faster and Simpler
  Hierarchical Routing in Road Networks".
- Strugar (2010), "Continuous Distance-Dependent Level of Detail for Rendering Heightmaps" (CDLOD).
- Micropolis (GPL) — `github.com/SimHacker/micropolis`: **read-only reference for ideas; never copy
  code into this MIT project.**
- OpenGlassBox — `github.com/Lecrapouille/OpenGlassBox`: a C++ GlassBox-style simulation based on the
  GDC 2012 slides; **check its license before reusing anything; ideas only by default.**
