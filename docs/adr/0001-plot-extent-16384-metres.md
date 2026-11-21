# ADR 0001 — City plot extent is 16,384 m, not 16,000 m

- **Status**: Draft — awaiting owner approval
- **Date**: 2026-11-21
- **Milestone**: M0, step 3 (`world-model`)
- **Affects**: blueprint 2.4 (world model and coordinates)
- **Does not affect**: decision log A19 ("16 km × 16 km city plots") — see "Decision" below

## Context

Blueprint 2.4 specifies four coordinate layers for a city plot. Taken literally they contradict
each other:

| Layer | Blueprint says | Implied plot extent |
| --- | --- | --- |
| City plot | 16 km × 16 km | 16,000 m |
| Terrain heightfield | 4 m resolution, 4,000 × 4,000 samples | 4,000 × 4 = **16,000 m** ✅ |
| Simulation grid | 32 m cells, 500 × 500 | 500 × 32 = **16,000 m** ✅ |
| Render chunks | 256 m × 256 m, **64 × 64 = 4,096 chunks** | 64 × 256 = **16,384 m** ❌ |

16,000 / 256 = **62.5**. A plot of exactly 16,000 m cannot be tiled by 256 m chunks. One of
these four numbers has to move.

This is not a detail that can be deferred. Every subsystem in the game converts between these
layers, and `world-model` exists precisely so that they cannot disagree (blueprint 2.4). The
constant has to be right before terrain generation, chunk streaming or the camera is written.

## Options considered

### Option A — Keep 16,000 m, accept a partial chunk row

62 full chunks plus a 128 m remainder strip on two edges.

Rejected. The remainder strip would need its own mesh path, its own LOD rules and its own
seam-stitching logic, forever, on the one step of M0 that is already the hardest (crack-free
LOD). It also contradicts the "4,096 chunks" figure the blueprint states explicitly.

### Option B — Keep 16,000 m, shrink chunks to 250 m

64 × 250 = 16,000 m, and 64 × 64 = 4,096 chunks as stated.

Rejected. 250 m / 4 m = 62.5 heightfield samples per chunk — not an integer, so no chunk has a
clean vertex grid. Quadtree LOD wants power-of-two vertex counts per node so that a coarser
level can drop every other vertex exactly. 250 m breaks that at every level.

### Option C — Plot extent becomes 16,384 m (chosen)

16,384 = 2¹⁴. Every layer then divides exactly, and every count is a power of two:

| Layer | Cell size | Count per axis | Exact? |
| --- | --- | --- | --- |
| City plot | — | 16,384 m | 2¹⁴ |
| Heightfield sample | 4 m | **4,096** | 2¹² |
| Render chunk | 256 m | **64** (4,096 chunks) | 2⁶ |
| Simulation cell | 32 m | **512** | 2⁹ |
| Heightfield samples per chunk | — | 64 (65 × 65 vertices) | 2⁶ |

## Decision

**The city plot is 16,384 m × 16,384 m**, centred on the world origin, spanning
−8,192 m … +8,192 m on both X and Z.

This keeps every number the blueprint states *as a count* (4,096 chunks, 256 m chunks, 4 m
heightfield, 32 m sim cells) and adjusts only the figures it states as approximations.

### Why this honours the blueprint rather than overriding it

- Decision log **A19** says "16 km × 16 km city plots". 16,384 m **is** 16 km to three
  significant figures (16.384 km, 2.4% over). A19 is satisfied.
- Blueprint 2.4's own memory estimates were already approximations and they still hold:
  - Heightfield: 4,096² × 2 B = **33.6 MB**, versus "about 32 MB". ✅
  - Sim layer: 512² × 4 B = **1.05 MB**, versus "1 MB/layer". ✅
- Blueprint 2.4's precision note says "f32 error near 16 km is about 1–2 mm". Centring the plot
  on the origin caps |coordinate| at 8,192 m = 2¹³, where the f32 ulp is 2¹³ × 2⁻²³ = 2⁻¹⁰ ≈
  **0.98 mm** — half the blueprint's estimate, so comfortably inside it. ✅
  (An un-centred 0…16,384 m plot reaches 2¹⁴ and so doubles the ulp to 1.95 mm, which is the
  second reason to centre it.) Both figures are asserted by a unit test in `plot.rs`.
- The "4,000 × 4,000" and "500 × 500" figures were the round-number approximations; "64 × 64 =
  4,096 chunks" is the one stated with an exact product, and it is the one that forces 16,384.

### Secondary decision: the plot is centred on the origin

Beyond the precision gain above, centring makes the Low preset's "central 8 km × 8 km" cap
(blueprint 2.6.8) a symmetric −4,096 … +4,096 box instead of an awkward offset window.

## Consequences

**Good**

- Every conversion in `world-model` is an exact integer shift or mask, not a division with a
  remainder. Cheap and impossible to get subtly wrong.
- Quadtree LOD gets 7 clean levels: level 0 = one 256 m chunk, level 6 = the whole plot.
  `64 >> level` nodes per axis, exactly.
- A chunk is 64 heightfield samples across → 65 × 65 vertices at LOD0, halving cleanly at each
  coarser level. This is what makes crack-free CDLOD morphing tractable.
- 2.4% more playable area than specified, for free.

**Costs**

- Blueprint 2.4's sample counts (4,000 → 4,096) and sim grid counts (500 → 512) must be
  corrected in `docs/blueprint/part-ii-technical-design.md`.
- 1.6 MB more heightfield memory per city than the literal reading (33.6 vs 32.0 MB).
  Irrelevant against the 3 GB Low-preset budget.

**Risks**

- None identified. No decision-log entry changes meaning, and no hard rule is touched.

## Implementation

Constants live in `crates/world-model/src/plot.rs` and are the single source of truth. Unit
tests assert the exact divisibility relationships above, so that a future edit to one constant
that breaks another fails the test suite rather than producing silent seams.

## Owner ratification

Flagged in the M0 report for Azrll. If he prefers a literal 16,000 m plot, Option A is the
fallback and the cost is a permanent special case in the terrain mesher — which is why it is not
the recommendation.
