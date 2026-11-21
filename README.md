# OpenCity

> **Working title.** The name "OpenCity" belongs to an unrelated GPLv2 city-building game from 2003.
> A final name will be chosen before any public announcement (decision A1, open question Q1).

A free, open-source 3D city-building sandbox written in Rust with [Bevy](https://bevy.org).
Built from scratch, clean-room, with original and free-licensed assets only.

**Status: pre-alpha.** Milestone M0 (preview foundation) is in planning. Nothing is playable yet.

## What it aims to be

- **Total creative freedom** — sandbox only: unlimited money, no failure state, no taxes.
- **Massive scale** — 16 km × 16 km city plots (256 km²) inside a region of connected cities.
- **A living city** — 1,000,000+ citizens; cars and people near the camera are individual agents you
  can click and follow.
- **A beautiful city** — semi-realistic PBR, real-time shadows, day-night cycle, rain, water
  reflections, night lights, optional tilt-shift.

Citizen **happiness** is the core metric — there is no money management to lose at.

## Documentation

The blueprint is the single source of truth. Start here:

- [`docs/blueprint/`](docs/blueprint/) — the living specification
  - [Part I — Product Overview](docs/blueprint/part-i-product-overview.md)
  - [Part II — Technical Design](docs/blueprint/part-ii-technical-design.md)
  - [Part III — AI Implementation Guide](docs/blueprint/part-iii-ai-implementation-guide.md)
  - [Decision log](docs/blueprint/decision-log.md) — authoritative owner decisions
- [`docs/plans/`](docs/plans/) — per-milestone plans
- [`docs/reports/`](docs/reports/) — per-milestone reports and measurements
- [`docs/adr/`](docs/adr/) — architecture decision records
- [`docs/TOOLING.md`](docs/TOOLING.md) — pinned versions of everything

## Building

Requires the Rust toolchain (stable, `x86_64-pc-windows-msvc` on Windows) and Visual Studio Build
Tools with the C++ workload. See [`docs/TOOLING.md`](docs/TOOLING.md) for exact pinned versions and
[Part II §2.15](docs/blueprint/part-ii-technical-design.md#215-build-tooling-and-ci) for the full
prerequisite list.

```powershell
cargo run -p game --release
```

For faster iteration during development:

```powershell
cargo run -p game --features bevy/dynamic_linking
```

## Non-goals

No multiplayer, no monetisation, no telemetry, no network calls at runtime, no automatic zoning, no
mobile/console/gamepad support.

## Legal

OpenCity is a **spiritual successor, not a clone**. It contains no code, art, audio, data or
trademarks from Electronic Arts, Maxis or any other company, and nothing decompiled or extracted from
any game. Genre mechanics are not copyrightable; concrete expression is, and none has been copied.

- **Code**: MIT — see [LICENSE](LICENSE).
- **Assets**: CC0, CC-BY or SIL OFL only, each recorded in `assets/LICENSES.md` with its source,
  author and license.

## Credits

Owner, director and playtester: **Azrll** ([Arfazrll](https://github.com/Arfazrll)).
Implementation: Claude Code.
