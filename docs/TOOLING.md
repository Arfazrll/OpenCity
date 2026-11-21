# TOOLING.md — installed tools, skills and MCP servers

Blueprint requirement: 3.3 — *"Log every installation in `docs/TOOLING.md` (tool, version, why, how
to uninstall)."*

Pinned versions here are authoritative. **Check this file before using any API** (CLAUDE.md session
checklist). Engine upgrades require an ADR (3.11).

Last updated: **2026-11-21** (M0 bootstrap).

---

## 1. Pinned versions

| Thing | Pinned version | Why this version |
| --- | --- | --- |
| **Bevy** | **0.20.1** | Latest stable on crates.io (published 2026-10-28), matching the blueprint's "0.20 as of November 2026" (2.2). MSRV 1.97.1 ≤ our 1.99.1. Upgrades need an ADR. |
| **Rust** | **1.99.1** stable (2026-10-13) | Already installed; satisfies Bevy 0.20.1's MSRV and edition 2024. |
| **Edition** | 2024 | Blueprint 3.9. |
| **Target** | `x86_64-pc-windows-msvc` | Blueprint 2.2 / 2.15. |

### Why Bevy 0.20.1 and not 0.20.0

0.20.1 is the patch release of the same minor version the blueprint names. Patch releases carry no
API breakage, so this is not an engine upgrade decision — it is taking the current stable 0.20.x.

---

## 2. Already present before bootstrap (nothing installed)

Verified on 2026-11-21 — see [`docs/reports/M0-environment.md`](reports/M0-environment.md) §3.

| Tool | Version | Note |
| --- | --- | --- |
| rustup | 1.29.3 | |
| rustc / cargo | 1.99.1 | |
| git | 2.55.0.windows.3 | |
| git-lfs | 3.7.1 | Needed for binaries > 1 MB (2.9). |
| cmake | 4.4.4 | Some crates' build scripts want it. |
| winget | v1.30.310 | Used for the installs below. |
| Python | 3.11.15 / 3.14.6 | Optional per 2.2; used here only to transcribe the blueprint PDF. |
| node | v24.18.0 | Not used by this project. |

---

## 3. Installed during M0 bootstrap

### 3.1 Visual Studio Build Tools 2022 — C++ workload

| | |
| --- | --- |
| **Version** | 17.14.45 (winget id `Microsoft.VisualStudio.2022.BuildTools`) |
| **Workload** | `Microsoft.VisualStudio.Workload.VCTools --includeRecommended` |
| **Delivered** | MSVC toolset **14.44.35207**, Windows SDK **10.0.26100.0** (482 libs in `um\x64`) |
| **Verified** | ✅ Compiled and ran a minimal Rust binary with `--target x86_64-pc-windows-msvc`. Printed its output. Linking works. |
| **Why** | **Nothing MSVC existed on this machine** — no `link.exe`, no Windows SDK. Without it the `x86_64-pc-windows-msvc` target cannot link at all. Blueprint 2.15 lists it as a Windows prerequisite and 3.3 permits installing it without asking. |
| **Installs to** | `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools` + Windows SDK under `C:\Program Files (x86)\Windows Kits\10` |
| **Disk cost** | Several GB on `C:`, which only had 29.8 GB free — minimal workload chosen deliberately. Tracked as a risk in the environment report. |
| **Uninstall** | `winget uninstall --id Microsoft.VisualStudio.2022.BuildTools`, or Visual Studio Installer → Build Tools → Modify/Uninstall. |

### 3.2 rustup: default toolchain switched to MSVC

| | |
| --- | --- |
| **Change** | `rustup default stable-x86_64-pc-windows-msvc` (was `stable-x86_64-pc-windows-gnu`) |
| **Why** | Blueprint 2.2 and 2.15 require the MSVC toolchain on Windows. The GNU toolchain was the default and would have produced non-conforming builds that diverge from CI. |
| **Revert** | `rustup default stable-x86_64-pc-windows-gnu` |

### 3.3 rustup components for the MSVC toolchain

| | |
| --- | --- |
| **Added** | `rust-src` (`clippy` and `rustfmt` were already present for msvc) |
| **Why** | `clippy` and `rustfmt` are quality gates 1 and 2 (3.8) and were already installed. `rust-src` was missing and is needed for IDE/analyzer support and std-dependent tooling. |
| **Revert** | `rustup component remove <name>` |

### 3.4 cargo-nextest

| | |
| --- | --- |
| **Version** | **0.9.152** (2026-11-13), host `x86_64-pc-windows-msvc` |
| **Install** | `cargo install cargo-nextest --locked` (compiled from source in 6m 42s on this CPU) |
| **Why** | Quality gate 3 (3.8): `cargo nextest run --workspace`. Blueprint 2.14 names it as the unit-test runner. |
| **License** | Apache-2.0 / MIT — inside the allowlist. |
| **Uninstall** | `cargo uninstall cargo-nextest` |

### 3.5 cargo-deny

| | |
| --- | --- |
| **Version** | **0.22.0** |
| **Install** | `cargo install cargo-deny --locked` |
| **Why** | Quality gate 4 (3.8): `cargo deny check licenses bans`. It is the mechanism that enforces hard rule 3.2 item 4 (license allowlist). |
| **License** | Apache-2.0 / MIT — inside the allowlist. |
| **Uninstall** | `cargo uninstall cargo-deny` |

---

## 4. Not installed (and why)

| Tool | Decision |
| --- | --- |
| **Blender** | Optional in 2.2, used only for offline asset generation. M0 generates everything from parametric code. Skipped to save `C:` space; install when M2 needs glTF tooling. |
| **Tracy profiler** | Blueprint 2.2 lists it for frame/sim profiling. Not needed to *build* M0; wire it in when the first real performance problem appears (likely M1/M2), so there is something worth profiling. |
| **cargo-watch** | Permitted by 3.3 but redundant here — the agent drives builds directly. |
| **cargo-binstall** | Would speed up installing the cargo tools above, but adds a tool that fetches prebuilt binaries from third parties. Compiling from source with `--locked` is the more conservative choice. |
| **Vulkan SDK** | Not needed: `wgpu` uses the system Vulkan/DX12 drivers at runtime; the SDK is only for validation layers during driver-level debugging. |

---

## 5. Claude Code skills

### 5.1 Project skills (committed to git, `.claude/skills/`)

Created during M0 bootstrap from blueprint Appendix C:

| Skill | Status | Purpose |
| --- | --- | --- |
| [`verify`](../.claude/skills/verify/SKILL.md) | ✅ created | All quality gates (3.8). Run before every commit. |
| [`milestone-report`](../.claude/skills/milestone-report/SKILL.md) | ✅ created | Owner report in Bahasa Indonesia (Appendix D). |
| [`asset-license-check`](../.claude/skills/asset-license-check/SKILL.md) | ✅ created | Every `assets/` file has an allowed `LICENSES.md` entry. |
| `bench` | ⏳ M1+ | Headless benchmark scenarios vs budgets (2.13). Nothing to benchmark yet. |
| `screenshot-review` | ⏳ M0 step 10 | Fixed camera bookmarks → capture → visual review. Needs the app shell to exist first. |
| `new-building-type` | ⏳ M2 | Scaffolds a `BuildingDef` + generator rules + test. Needs the generator to exist first. |

The three deferred skills are listed in blueprint 3.4 as "create during M0–M1". They are deliberately
not written yet because a skill that calls commands against crates that do not exist would be a skill
that cannot be run — and the rule is never to claim something works unless it was observed working.

### 5.2 No third-party skills or plugins installed

Nothing from a marketplace was added. The blueprint permits it (3.3) but requires reading the
`SKILL.md` and any scripts first. Nothing in M0 needs one, so none were added — this keeps the
attack surface and the audit burden at zero. Record any future addition here.

---

## 6. MCP servers

### 6.1 Documentation server — already available, nothing installed

Blueprint 2.2 and 3.4 strongly recommend a documentation server so the agent can check APIs of the
**pinned** Bevy version instead of relying on memory, and name Context7 as the example.

A **Context7 documentation server is already connected in this Claude Code session**. Verified on
2026-11-21 by resolving the library:

| | |
| --- | --- |
| Library ID | `/bevyengine/bevy` |
| Versions available | v0.15.3, v0.16.1, v0.18.0, v0.18.1, v0.20.0, **v0.20.1** |
| Code snippets | 5,647 |
| Supplementary | `/websites/rs_bevy` (docs.rs mirror, 132,640 snippets), `/websites/bevy-cheatbook_github_io` (2,305 snippets) |

**It has v0.20.1 — exactly our pinned version.** So the blueprint's requirement is satisfied with no
installation, no new account and no project data leaving the machine beyond the API names being
looked up.

**How to use it (mandatory before writing against an unfamiliar Bevy API):**

1. `resolve-library-id` with `libraryName: "Bevy"` and a query describing the task.
2. `query-docs` against `/bevyengine/bevy`, **passing the version `v0.20.1`**, never unversioned.

Bevy 0.20 moved shaders to WESL and changed renderer APIs; answering from memory is how wrong code
gets written. Check the docs.

### 6.2 Other MCP servers in the session

Many unrelated MCP servers are connected in this environment (cloud platforms, finance, calendar,
mail, spreadsheets, and so on). **None of them is used by this project and none was added for it.**
Per hard rule 3.2 item 5 and the 3.3 permission table, no server that needs secrets, broad file
access outside the project, or that sends project data to third parties may be used here.

---

## 7. Configuration applied to the repository

| File | Purpose |
| --- | --- |
| `.cargo/config.toml` | Fast-compile settings from 2.15 / 3.13: `rust-lld` linker, dev `opt-level = 1` for our crates, `opt-level = 3` for dependencies. Created in M0 step 3. |
| `deny.toml` | `cargo-deny` allowlist exactly as hard rule 3.2 item 4 states. Created in M0 step 3. |
| `.gitignore` | Excludes `target/`, screenshots, saves, profiling output and any stray secret files. |

---

## 8. Change log for this file

| Date | Change |
| --- | --- |
| 2026-11-21 | Created at M0 bootstrap. Pinned Bevy 0.20.1 / Rust 1.99.1. Installed VS Build Tools 2022 (VCTools), switched rustup default from gnu to MSVC, added `rust-src` for MSVC, installed cargo-nextest and cargo-deny. Recorded the pre-existing Context7 MCP server as the documentation source. |
