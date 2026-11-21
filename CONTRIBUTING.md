# Contributing to OpenCity

OpenCity is currently written by a single implementing agent working from
[`docs/blueprint/`](docs/blueprint/), with Azrll as owner, director and playtester. External
contributions are not being accepted yet — the project is pre-alpha and the architecture is
still being laid down. Issues and discussion are welcome once the repository is public.

If you are working on this codebase, the rules below are not style preferences. They exist
because this is an MIT, publicly redistributed project with a legally clean-room design.

## Hard rules

1. **Clean room.** No code, binaries, data, assets or leaked documentation from EA, Maxis or
   SimCity. Public talks and articles about *concepts* are fine.
2. **No copied assets.** Nothing ripped, extracted, traced or lightly modified from any game or
   commercial product.
3. **No trademarks.** Not in code identifiers, UI, assets or marketing text.
4. **License compatibility.** The code is MIT. Never copy GPL/LGPL/AGPL code into this
   repository. Dependencies must pass `cargo deny check licenses bans`; the allowlist is in
   [`deny.toml`](deny.toml) and extending it needs the owner's approval.
5. **Every asset is registered** in [`assets/LICENSES.md`](assets/LICENSES.md) with its source,
   author and license. CC0, CC-BY and OFL only.
6. **No secrets, no telemetry, no runtime network calls.**

## Architecture boundary

The simulation and the presentation layer are separate on purpose
([blueprint 2.1](docs/blueprint/part-ii-technical-design.md#21-architecture-overview)).

- `sim-core` must not depend on rendering, audio, UI or wall-clock time.
- It must stay deterministic: seeded ChaCha streams only, no OS randomness, no iteration over
  random-state hash maps, and parallel systems must produce order-independent results.
- Player intent enters through commands that each have an inverse. State leaves through
  read-only snapshots. Nothing else crosses the line.

Breaking this boundary breaks replays, benchmarks, headless tests and the future browser build
all at once.

## Code standards

- Rust stable, edition 2024, `rustfmt` defaults, `clippy` clean with warnings as errors.
- No `unwrap()` or `expect()` in `sim-core` outside tests. Return `Result` with `thiserror`
  types.
- `tracing` for logs. No `println!` in library crates.
- Strong types for units where it prevents bugs (`Meters`, `Seconds`, `GameTime`).
- No per-tick heap allocation in hot paths. Profile before optimising.
- Every public item has a doc comment; every crate has a module-level design overview.
- Rules and numbers live in RON under `crates/content`, not hard-coded.
- Every bug fix comes with a regression test.
- Dev-only tooling sits behind the `dev` cargo feature.

## Workflow

Plan → implement in small steps → tests → run the quality gates → commit.

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo deny check licenses bans
```

Commits use [Conventional Commits](https://www.conventionalcommits.org/) in English, for
example `feat(render): add CDLOD terrain chunks`. `main` is always green. Never force-push
`main`.

Code, comments, documentation, commit messages and the README are in English. Reports to the
owner are in Bahasa Indonesia.

## Changing the design

The blueprint is the source of truth, and its decision log is authoritative. Changes to the
hard rules, the decision log or the architecture need an ADR in [`docs/adr/`](docs/adr/) and
the owner's approval. Do not change them silently.
