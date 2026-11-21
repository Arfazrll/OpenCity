# OpenCity Blueprint — living Markdown version

Transcribed from `docs/OpenCity_Blueprint.pdf` (Project Blueprint v0.1, 21 November 2026).

From now on **this Markdown copy is the living version**. Changes happen through pull requests
and ADRs (`docs/adr/NNNN-title.md`), never silently. The PDF stays in the repository as the
immutable v0.1 snapshot.

| File | Contents | Binding level |
| --- | --- | --- |
| [part-i-product-overview.md](part-i-product-overview.md) | What we are building, for whom, why. Scope per release. | Directional |
| [part-ii-technical-design.md](part-ii-technical-design.md) | Architecture, simulation, rendering, content, data, performance, testing, roadmap. | Design. Items marked *Proposed default* may be changed through an ADR. |
| [part-iii-ai-implementation-guide.md](part-iii-ai-implementation-guide.md) | How Claude Code must work: rules, permissions, workflow, quality gates, reporting. | **Binding operating rules** |
| [decision-log.md](decision-log.md) | Appendix A — owner decisions A1–A42. | **Authoritative** |
| [appendices.md](appendices.md) | Appendices B–F: CLAUDE.md template, skill templates, report template, glossary, references. | Reference |

## Order of precedence when documents conflict

1. **Hard rules** in Part III section 3.2 (legal, licensing, safety). Never overridden.
2. **Decision log** in Appendix A (explicit choices made by the owner).
3. **Part II** technical design (including *Proposed defaults*, until replaced by an ADR).
4. **Part I** product overview.
5. Anything else (blog posts, examples, Claude's own preferences).

## Conventions

- All units are metric (metres, seconds, kilograms). **1 world unit = 1 metre.**
- **MUST / MUST NOT** are mandatory.
- **SHOULD** means strongly recommended; deviations need a short justification in the milestone report.
- ***Proposed default*** means a reasonable starting value that playtesting may change.

## Project metadata

- Owner: Azrll (GitHub: Arfazrll) — director and playtester
- Implementer: Claude Code (Claude Opus 5.5 / Claude Fable 5.2) — writes all code
- Engine: Rust + Bevy · License: MIT · Repo: `D:\app\Others\OpenCity`
- Status: approved for implementation of M0
