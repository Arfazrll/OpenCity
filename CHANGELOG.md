# Changelog

All notable user-visible changes to OpenCity are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

This project is pre-alpha; version numbers begin at the first tagged milestone.

## [Unreleased]

### Added

- Cargo workspace with the nine crates and two tools from the blueprint's repository layout.
- Blueprint transcribed into `docs/blueprint/` as the living Markdown specification.
- Project agent rules (`CLAUDE.md`) and the `verify`, `milestone-report` and
  `asset-license-check` skills.
- Dev-machine environment report and pinned tooling (Bevy 0.20.1, Rust 1.99.1 MSVC).
- `cargo-deny` allowlist enforcing the dependency license policy.
