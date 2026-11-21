# OpenCity - Agent Operating Rules

Source of truth: `docs/blueprint/` (transcribed from `docs/OpenCity_Blueprint.pdf`).
Precedence: Hard rules > Decision log > Technical design > Overview.

## Hard rules (never break)

- Clean room: no EA/Maxis/SimCity code, binaries, data, assets or leaks.
- No ripped, recreated or lightly modified assets from other games.
- No trademarks (SimCity, Sims, Maxis, GlassBox, brand logos) in code/UI/assets.
- MIT code: never copy GPL/LGPL/AGPL code. cargo-deny allowlist only.
- Every asset recorded in `assets/LICENSES.md` (CC0, CC-BY, OFL only).
- No secrets, no telemetry, no runtime network calls.
- sim-core is deterministic and has no render/audio/UI/wall-clock dependencies.
- Work only inside `D:\app\Others\OpenCity`.

## Session start checklist

1. Read this file, `docs/plans/<current milestone>.md`, the latest `docs/reports/*.md`,
   `docs/issues/*.md`.
2. Check pinned versions in `docs/TOOLING.md` before using any API.

## Workflow

Plan (`docs/plans`) -> implement small steps -> tests -> `/verify` -> commit (Conventional Commits).
Render changes: `screenshot-review`. Hot paths: `bench` (fail on >10% regression).
Milestone done -> `milestone-report` (Bahasa Indonesia) -> wait for owner playtest -> tag.

## Ask the owner before

- Changing hard rules, decision log or architecture (write an ADR draft).
- New dependency outside the license allowlist; paid tools; new accounts.
- Bevy upgrade; major feature add/cut; GitHub remote or visibility changes.

## Stuck?

Same error 3 times -> stop, write `docs/issues/<date>-<topic>.md`, change approach or ask.

## Style

Rust 2024, rustfmt, clippy `-D warnings`, no `unwrap` in sim-core, `tracing` logs,
data-driven rules in `crates/content` (RON), regression test for every bug fix.
Code/docs/commits in English; reports to owner in Bahasa Indonesia.

---

## Project-specific notes (added during M0 — never weaken the rules above)

### Pinned versions

Authoritative list: [`docs/TOOLING.md`](docs/TOOLING.md). At M0 bootstrap:

- Bevy **0.20.1** (latest stable, 2026-10-28). Upgrades need an ADR.
- Rust stable **1.99.1**, edition 2024, target `x86_64-pc-windows-msvc`.

### Dev laptop reality (measured, see `docs/reports/M0-environment.md`)

Intel Core Ultra 5 125U, 15.5 GB shared RAM, **integrated Intel Graphics (Xe-LPG)**, 1920 × 1200
panel. This is the **Minimum / Low preset tier**: 1080p @ 30 fps, 8 km active city, Tier 2 agent cap
5,000. There is **no 1440p panel**, so the 1440p row of any FPS table is "not testable here".

Consequences for every milestone:

- Default the dev build to the **Low** preset; never assume headroom.
- The iGPU shares system RAM — memory budgets (2.13) and VRAM pressure are the same pool.
- `Low under 3 GB total` is the budget that actually matters on this machine.

### Reporting

Reports and questions to Azrll: **Bahasa Indonesia**, short and concrete, at most 3 decisions each
with a recommended option. Code, comments, docs, commits, README: **English**.

Never claim something works unless it was run and the result observed.
