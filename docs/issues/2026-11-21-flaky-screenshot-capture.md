# 2026-11-21 — Automated screenshot capture is intermittently blank

**Status**: mitigated, root cause not fully established
**Found during**: M0 step 6 (terrain chunk LOD rendering)
**Affects**: `game --capture <bookmark>`, and therefore the `screenshot-review` workflow and
the six milestone screenshots

## Symptom

`game.exe --capture <bookmark>` sometimes writes a PNG containing only the HUD text over a flat
clear-colour background, with no 3D geometry at all — even though the HUD in that same image
reports terrain as loaded (for example "22 chunks, 50864 tris, 0 queued").

Pixel analysis of a bad capture: 38,330 of the sampled pixels are exactly `(43, 44, 47)`, the
clear colour, and the only other colours are HUD glyph antialiasing. So nothing was rasterised;
it is not terrain rendered black.

## Why this took so long to pin down

The failure is **intermittent**, and I initially read it as deterministic. That led to three
wrong conclusions in a row, each "confirmed" by a single run:

1. *"The far clip plane is too small."* — This one was genuinely true and genuinely fixed a
   real bug: Bevy's default far plane is 1,000 m, and this world is 16 km across, so the whole
   landscape was clipped. Fixing it made terrain appear. But it was not the whole story.
2. *"Distance fog and ambient light break rendering."* — **Wrong.** Removing them appeared to
   fix the `noon` capture, but that was one lucky run. Both are now restored and capture fine.
3. *"The 5 km altitude view is broken."* — **Wrong.** `altitude` later rendered correctly three
   times in a row with no code change, while `noon` failed once with no code change.

The decisive experiment was running the same bookmark repeatedly without touching the code:

```text
noon     run 1  499138 bytes  RENDERED
noon     run 2  488569 bytes  RENDERED
noon     run 3  488497 bytes  RENDERED
altitude run 1  645566 bytes  RENDERED
altitude run 2  117333 bytes  PARTIAL
altitude run 3  645552 bytes  RENDERED
```

Same binary, same arguments, different output. That is what ruled out every geometric
explanation at once.

## Ruled out

- **Frustum culling.** Adding `NoFrustumCulling` to every chunk did not change the outcome.
- **Projection.** Logged at capture time; identical in failing and succeeding runs:
  `fov 0.87266463, aspect 1.7777778, near 1.0, far 32000.0`.
- **Camera aim.** Logged at capture time. In a failing `altitude` run the camera was at
  `(600, 5000, 3200)` facing `(-0.218, -0.777, -0.591)`; the view centre meets the ground at
  `(-803, -603)`, 6.4 km away, well inside both the far plane and the loaded chunk span of
  `x -4096..4096, z -4096..4096`.
- **Terrain geometry.** Chunk entities exist with correct levels and world positions, and the
  same chunks render correctly in other runs.

## Leading hypotheses

1. **The capture races terrain streaming.** The driver moved to the settling phase as soon as
   `pending == 0`, but on the frame the bookmark is applied the camera's `GlobalTransform` has
   not propagated yet, so `pending == 0` could still describe the *previous* camera's
   selection. The 117 KB partial capture fits this: some terrain, not all.
2. **The window is not presented when the screenshot is read back.** Capture reads the
   swapchain, so a window that is occluded, unfocused or not yet composited can hand back a
   frame that was never drawn. This would explain a *completely* empty image better than
   hypothesis 1 does.

The two are not exclusive and both are plausible; I have not separated them.

## Mitigation applied

- The capture driver now requires terrain to report `pending == 0` for
  `STABLE_FRAMES` **consecutive** frames, not just one, before it starts settling. This closes
  hypothesis 1 entirely.
- Settling time was increased.
- Diagnostics (camera transform, projection, chunk levels and span) are logged at capture time
  and kept, because "the HUD says it is loaded but the screen is empty" is exactly the
  situation where guessing is most expensive.

## Not yet done

Hypothesis 2 is unaddressed. The robust fix is to render the capture to an **offscreen image
target** rather than the window swapchain, which removes any dependence on desktop state. Bevy
supports this (`Image::new_target_texture` plus `Screenshot::image`). That is the right thing
to do when `screenshot-review` becomes a real skill, and it is noted in `docs/plans/M0.md`.

Until then, **any batch of captures must be verified, not trusted**: check that each PNG
contains more than a handful of distinct colours, and re-run the ones that do not. The six
milestone screenshots were each verified this way.
