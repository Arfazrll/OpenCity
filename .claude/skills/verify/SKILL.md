---
name: verify
description: Run all OpenCity quality gates before every commit (format, lint, tests, licenses, replay). Use before every commit, and whenever the user asks to verify, check the gates, or confirm the workspace is green.
---

# verify

Run from the repository root (`D:\app\Others\OpenCity`). Stop at the first failure, fix it, and
re-run from the top.

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo nextest run --workspace`
4. `cargo deny check licenses bans`
5. If files under `crates/sim-core` changed:
   `cargo run -p replay --release -- --check fixtures/replays`
6. If files under `crates/render` or `crates/procgen` changed: invoke the `screenshot-review` skill.

Finish with one line per gate: PASS/FAIL and the key number (tests run, warnings, regressions).

## Output format

```text
fmt      PASS
clippy   PASS   (0 warnings)
nextest  PASS   (37 tests, 0 failed)
deny     PASS   (licenses + bans)
replay   SKIP   (sim-core unchanged)
visual   SKIP   (render/procgen unchanged)
```

## Rules

- **Never report PASS for a gate that was not actually run.** Use `SKIP` with the reason.
- A gate that cannot run yet (tool missing, crate not created yet) is `BLOCKED`, not `PASS` —
  say what is missing.
- Do not work around a failure by relaxing the gate (no `#[allow]` sprees, no `--no-fail-fast`
  reinterpretation, no removing `-D warnings`). Fix the cause.
- `cargo deny` failures about licenses are a **hard rule** matter (blueprint 3.2 item 4): stop and
  ask the owner before adding anything outside the allowlist.
