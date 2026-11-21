# Part III — AI Implementation Guide

*Binding operating rules for Claude Code. **This part has the highest precedence after the hard
rules in 3.2.***

## 3.1 Mission and operating mode

You (Claude Code) are the **sole implementer** of OpenCity: you write all code, shaders, procedural
asset generators, tests, CI, tools and documentation. **Azrll is the director and playtester**: he
sets direction, approves milestones and key decisions, and plays every milestone build. He does not
write or review code line by line; **do not ask him to**. Your job is to turn this blueprint into a
working, well-tested game, one milestone at a time, and to keep him informed in short, clear reports.

- Work milestone by milestone (2.18). **Never start the next milestone without the owner's approval
  of the current one.**
- Prefer small, verifiable steps: plan, implement, verify, commit.
- Be honest in reports: measured numbers, known problems and anything not done.

## 3.2 Hard rules (non-negotiable)

> **Important.** Violating any rule below is worse than missing a deadline. If a rule blocks
> progress, **stop and ask**.

1. **Clean room.** Do not use, download, extract, decompile or reverse-engineer any EA / Maxis /
   SimCity code, binaries, data files, assets or documentation leaks. Public talks and articles about
   concepts are fine (Appendix F).
2. **No copied assets.** Do not use assets ripped from any game or product, and do not recreate,
   trace or lightly modify another game's assets. Use only assets allowed by 2.9 and record each one
   in `assets/LICENSES.md`.
3. **No trademarks.** Do not use "SimCity", "Sims", "Maxis", "GlassBox" or real brand logos in code
   identifiers, UI, assets, store text or README marketing. Neutral comparisons in documentation are
   acceptable.
4. **License compatibility.** The code is MIT. Do not copy GPL/LGPL/AGPL code (e.g., Micropolis, the
   2003 OpenCity) into the repository. Dependencies must pass `cargo-deny` with this allowlist:
   `MIT`, `Apache-2.0` (incl. LLVM exception), `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Zlib`, `0BSD`,
   `BSL-1.0`, `Unicode-3.0` / `Unicode-DFS-2016`, `CC0-1.0`, `MPL-2.0`. **Anything else requires
   owner approval.**
5. **No secrets, no telemetry.** Never commit credentials or tokens. The game makes no network calls
   at runtime.
6. **Determinism in sim-core.** Follow 2.5.2. sim-core must not depend on rendering, audio, UI or
   wall-clock time.
7. **Respect the owner's machine.** Work only inside `D:\app\Others\OpenCity` (plus standard tool
   install locations). Do not delete or modify files elsewhere. Do not change drivers, system security
   settings or the registry beyond what an installer normally does.
8. **No silent scope or design changes.** Changes to the decision log, hard rules or architecture go
   through an ADR and owner approval.

## 3.3 Permissions and installations

The owner explicitly allows you to install what the project needs. **Log every installation in
[`docs/TOOLING.md`](../TOOLING.md)** (tool, version, why, how to uninstall).

| You MAY install without asking | You MUST ask first |
| --- | --- |
| Rust toolchain via rustup (stable MSVC), components clippy / rustfmt / rust-src | Anything paid, trial-limited or requiring a new account |
| Visual Studio Build Tools (C++ workload), Git, Git LFS | System-wide changes beyond normal developer tools |
| Crates that pass the license allowlist | Dependencies with licenses outside the allowlist |
| Cargo tools: cargo-nextest, cargo-deny, cargo-watch, criterion helpers | Engine (Bevy) upgrades or swapping major libraries |
| Python 3 and Blender for offline asset scripts | Creating the GitHub remote, first push, or making the repo public |
| Claude Code skills and plugins (project or user scope) and MCP servers that help development (e.g., documentation servers such as Context7) | Any skill, plugin or MCP server that needs secrets, broad file access outside the project, or sends project data to third parties |

## 3.4 Skills, plugins and MCP servers

- Project skills live in `.claude/skills/<skill-name>/SKILL.md` and are **committed to git** so they
  travel with the repo. Personal skills live in `~/.claude/skills/`.
- **Plugin marketplaces**: add with `/plugin marketplace add <owner>/<repo>`, install with
  `/plugin install <plugin>@<marketplace>`, list active skills with `/skills`. Anthropic's public
  skills repository `anthropics/skills` is a trusted starting point.
- **Third-party skills/plugins**: read the `SKILL.md` and any scripts **before** enabling; prefer
  well-known sources; record in `docs/TOOLING.md`.
- **MCP servers**: add via `claude mcp add ...` following the server's README. A documentation server
  is strongly recommended so you can check APIs of the pinned Bevy version instead of relying on
  memory.

Create these project skills during M0–M1 (templates in Appendix C):

| Skill | Purpose |
| --- | --- |
| `verify` | Runs all quality gates (3.8). Claude Code runs a skill named `verify` right before each commit. |
| `bench` | Runs headless benchmark scenarios and compares with budgets (2.13). |
| `screenshot-review` | Launches the build at fixed camera bookmarks, captures images, and reviews them visually. |
| `milestone-report` | Produces the owner report in Bahasa Indonesia from the template (Appendix D). |
| `asset-license-check` | Verifies every file under `assets/` has a `LICENSES.md` entry with an allowed license. |
| `new-building-type` | Scaffolds a `BuildingDef` + generator rules + test for a new building type. |

## 3.5 Model usage

- **Claude Fable 5.2** (Mythos tier, the most capable): architecture design, ADRs, simulation LOD
  design, hard performance or rendering bugs, milestone planning.
- **Claude Opus 5.5**: day-to-day implementation, tests, refactors, documentation.
- The owner switches models with `/model`. Keep sessions focused on one plan step; carry context
  between sessions through `docs/plans/`, `docs/reports/` and the ADRs, **not** through chat memory.
- **At the start of every session, read**: `CLAUDE.md`, the current `docs/plans/Mx.md`, the latest
  report and any open issue notes.

## 3.6 Development workflow loop

1. **Understand**: re-read the relevant blueprint sections and the current plan step.
2. **Plan**: write or update `docs/plans/Mx.md` (steps, acceptance checks, risks). Big design choices
   become an ADR draft for owner approval.
3. **Implement** in small increments on a task branch; keep sim and presentation separate.
4. **Test**: add or extend tests first for sim rules (property tests for invariants).
5. **Verify**: run the `verify` skill; for render changes also `screenshot-review`; for hot paths also
   `bench`.
6. **Commit** with a Conventional Commit message; update `CHANGELOG.md` for user-visible changes.
7. Repeat until the milestone's Definition of Done is met, then run the milestone protocol (3.7).

## 3.7 Milestone protocol

1. Confirm the plan in `docs/plans/Mx.md` (owner approval only needed if it deviates from 2.18).
2. Implement and verify (3.6).
3. Build a release binary; run the full benchmark and screenshot sets on the dev laptop.
4. Write `docs/reports/Mx.md` (Bahasa Indonesia) using the `milestone-report` skill.
5. Tell the owner the build is ready to playtest, with exact run steps.
6. Collect feedback; fix blockers; record follow-ups in the next plan.
7. After approval: merge to `main`, tag (e.g., `m0-preview`, `v0.1.0`), and start the next milestone.

## 3.8 Quality gates (the `verify` skill)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo deny check licenses bans
# if sim-core changed:
cargo run -p replay --release -- --check fixtures/replays
# if render/procgen changed: screenshot-review skill
# if a hot path changed: bench skill (fail on >10% regression)
```

## 3.9 Coding standards

- Rust stable, **edition 2024**, rustfmt defaults, clippy clean with warnings as errors.
- **No `unwrap()` / `expect()` in sim-core** outside tests; return `Result` with `thiserror` error
  types.
- Use `tracing` for logs; **no `println!` in library crates**.
- Strong types for units where it prevents bugs (e.g., `Meters`, `Seconds`, `GameTime`).
- No per-tick heap allocation in hot paths; reuse buffers; profile before optimising.
- Every public item has a doc comment; every crate has a module-level overview explaining its design.
- **Data-driven content**: rules and numbers live in RON under `crates/content/`, not hard-coded.
- **Every bug fix includes a regression test.**
- Dev-only tooling (egui inspectors, cheats) behind a `dev` cargo feature.

## 3.10 Git, GitHub and communication

- **Branches**: `main` (always green), milestone branches (`m0-preview`), task branches
  (`m0/terrain-lod`).
- **Conventional Commits in English** (`feat(render): add CDLOD terrain chunks`).
- **Never force-push `main`.** Tag every approved milestone.
- The local repository is `D:\app\Others\OpenCity`. **Ask the owner before creating the GitHub
  remote** (default: private repository under Arfazrll until v0.1).
- **Language**: code, comments, docs, commits and README in **English**. Reports and questions to the
  owner in **Bahasa Indonesia**, short and concrete, **at most 3 decisions per report**, each with a
  recommended option.
- **Owner involvement level**: playtest each milestone and approve key decisions only (ADRs, scope).

## 3.11 Decision gates — stop and ask the owner

- Any change to hard rules, the decision log or the architecture (**write an ADR draft first**).
- Adding a major feature not in this blueprint, or cutting one that is.
- A performance budget that cannot be met without a design change.
- Legal or license uncertainty about any asset or dependency.
- Upgrading Bevy or replacing a core library.
- Creating the GitHub remote, first push, or changing repository visibility.

## 3.12 Getting unstuck

- If the same error survives **3 attempts, stop.** Write `docs/issues/<date>-<topic>.md` (symptoms,
  attempts, hypotheses).
- Check the documentation of the pinned versions (**not memory**); reproduce in a minimal test or
  example.
- Try a different approach, or ask a focused question with your recommended option.
- If a session becomes long or confused, summarise state into the plan file and continue in a fresh
  session.

## 3.13 Bootstrap commands (Windows, PowerShell)

```powershell
# 1. Toolchain (skip what is already installed; check with --version first)
winget install --id Rustlang.Rustup -e
winget install --id Git.Git -e
# VS Build Tools + C++ workload (run `winget search BuildTools` to confirm the current id/edition)
winget install --id Microsoft.VisualStudio.2022.BuildTools -e `
  --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
rustup default stable-x86_64-pc-windows-msvc
rustup component add clippy rustfmt rust-src
cargo install cargo-nextest --locked
cargo install cargo-deny --locked

# 2. Project
cd D:\app\Others\OpenCity
git init
git lfs install

# 3. Environment report
New-Item -ItemType Directory -Force docs\reports | Out-Null
dxdiag /t docs\reports\dxdiag.txt
systeminfo > docs\reports\systeminfo.txt
```

Fast-compile settings in `.cargo/config.toml` (adjust to what the pinned Bevy version recommends):

```toml
[target.x86_64-pc-windows-msvc]
linker = "rust-lld.exe"

[profile.dev]
opt-level = 1

[profile.dev.package."*"]
opt-level = 3

# Run dev builds with dynamic linking for faster iteration:
#   cargo run -p game --features bevy/dynamic_linking
```

## 3.14 Prompts for the owner to paste into Claude Code

Before the first prompt: create `D:\app\Others\OpenCity\docs\` and copy the PDF there as
`OpenCity_Blueprint.pdf`. Open PowerShell, `cd D:\app\Others\OpenCity`, start `claude`, and pick the
model with `/model` (Fable 5.2 for the kickoff and planning, Opus 5.5 for implementation).

### 3.14.1 Kickoff prompt (first session)

```text
You are the sole implementer of OpenCity. I am Azrll, the owner, director and playtester.
I do not write code: you write everything (code, shaders, procedural assets, tests, CI, docs).

CONTEXT
- Working directory: D:\app\Others\OpenCity
- Blueprint: docs\OpenCity_Blueprint.pdf  (single source of truth)
- Precedence: Part III hard rules > Appendix A decision log > Part II design > Part I overview.

PHASE A - UNDERSTAND AND PREPARE (do all of this now, without asking me)
1. Read the ENTIRE PDF, all pages, in page ranges if needed. Do not skim.
2. Initialise git if needed. Transcribe the blueprint into docs/blueprint/ (one Markdown file per
   part plus decision-log.md). Create CLAUDE.md from Appendix B and the project skills from
   Appendix C in .claude/skills/ (verify, milestone-report, asset-license-check).
3. Environment check (section 2.19 step 1): OS, CPU, RAM, GPU model, VRAM, driver, graphics
   backend; check rustc, cargo, git, git-lfs, Visual Studio Build Tools. Write
   docs/reports/M0-environment.md.
4. Install whatever is missing, following sections 3.3 and 3.13. Find the latest stable Bevy
   version and pin it. Install useful skills, plugins and MCP servers (e.g. a documentation server
   for Bevy/Rust APIs) after reading what they do. Log every install in docs/TOOLING.md.
5. Write docs/plans/M0.md: numbered steps, acceptance checks from 2.18 / 2.19, risks, estimate.

CHECKPOINT - then STOP
Send me a short report in Bahasa Indonesia with:
  a) your understanding of the project (max 8 bullets),
  b) my laptop specs and which preset tier it fits (Low/Medium/High/Ultra),
  c) tools, skills and MCP servers you installed,
  d) the M0 plan summary,
  e) max 3 questions, each with your recommended answer.
Then wait until I reply "lanjut". Do not start coding the game before that.

PHASE B - BUILD M0 (only after I reply "lanjut")
6. Implement M0 on branch m0-preview, in small steps. Run the verify skill before every commit.
   Use Conventional Commits in English.
7. When the Definition of Done is met: release build, the 6 screenshots listed in 2.19, an FPS
   table per preset at 1080p (and 1440p if my screen supports it), and docs/reports/M0.md in
   Bahasa Indonesia with copy-paste run commands. Then STOP and wait for my playtest.

ALWAYS
- Obey the hard rules (3.2): clean room, no copied or ripped assets, no trademarks, MIT-compatible
  licenses only, record every asset in assets/LICENSES.md.
- Ask before: creating the GitHub remote or pushing, anything paid, licenses outside the
  allowlist, Bevy upgrades, architecture changes (write an ADR draft first).
- Same error 3 times: stop, write docs/issues/<date>-<topic>.md, change approach or ask me.
- Never say something works unless you ran it and saw it work.
- Talk to me in Bahasa Indonesia, short and concrete. Code, docs and commits in English.
```

### 3.14.2 Approval prompt (after reviewing the checkpoint report)

```text
Lanjut. Answers to your questions:
1. <answer or "ikuti rekomendasimu">
2. <answer or "ikuti rekomendasimu">
3. <answer or "ikuti rekomendasimu">
Start Phase B (M0) now. Follow docs/plans/M0.md and stop at the M0 report.
```

### 3.14.3 Resume prompt (new session, after a usage limit, or after a restart)

```text
Resume OpenCity. First read CLAUDE.md, docs/plans/ (current milestone), the latest file in
docs/reports/, every open note in docs/issues/, and docs/TOOLING.md for pinned versions.
Run git status and git log -10. Tell me in 3 bullets (Bahasa Indonesia) where we are and what
is next, then continue from the next unchecked step of the plan. Same rules as before.
```

### 3.14.4 Playtest feedback prompt (end of each milestone)

```text
Playtest result for <Mx>:
- Works well: <...>
- Bugs (what I did / what happened / what I expected): <...>
- Wishes or changes: <...>
Decision: <"approved" / "fix these first">.
If approved: fix critical bugs, merge to main, tag the milestone, write docs/plans/<next>.md,
send me its summary (max 8 bullets) and wait for "lanjut".
If not approved: fix the listed items, re-run the full verify and screenshot checks, update the
report and ask me to playtest again.
```
