# Asset license registry

Every file under `assets/` MUST have a row in this table before it is committed. The
`asset-license-check` skill enforces this and blocks commits that violate it.

**Allowed licenses:** `CC0-1.0`, `CC-BY-3.0`, `CC-BY-4.0`, `OFL-1.1`, `MIT`, `Apache-2.0`.

**Never allowed:** non-commercial (NC), no-derivatives (ND), "free for personal use", unclear
licenses, anything ripped or extracted from a game or commercial product, recreations or traced
copies of another game's assets, real brand logos, exact replicas of distinctive copyrighted
landmark designs.

The in-game Credits screen is generated from this file, so the attribution columns must be
accurate and complete.

## Registry

| File | Source URL | Author | License | Date added | Modifications |
| --- | --- | --- | --- | --- | --- |
| *(none yet)* | | | | | |

## Notes

**M0 ships zero binary assets.** Terrain and buildings are generated procedurally from
parametric code in `crates/procgen`, and materials use flat procedural colours. That is
deliberate: it keeps the repository small (blueprint 2.9) and it means the licensing gate is
trivially green while the foundation is being built.

Procedurally generated output does still need a row once it is *committed as a file* rather than
produced at runtime. Use source `procedural (crates/procgen)` and license `MIT`, matching the
code that produced it.

AI-generated assets are permitted only if the tool's terms grant output rights compatible with
public redistribution. The row must record the tool, its version and the prompt. If the terms
are unclear, the answer is no — ask the owner.
