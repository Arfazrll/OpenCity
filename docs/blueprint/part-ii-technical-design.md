# Part II — Technical Design

*Architecture, simulation, rendering and content. Binding level: design. Items marked
**Proposed default** may be changed through an ADR.*

## 2.1 Architecture overview

OpenCity is split into a **headless, deterministic simulation** and a **presentation layer**
(rendering, audio, UI, input). They communicate only through two channels: a **command queue**
(player intent flowing into the sim) and **snapshots** (read-only state flowing out to
presentation).

```text
+------------------------------------------------------------------------+
|                      game (Bevy App, main thread)                      |
|   input -> tools (roads, placement, terraform, radial menu, cameras)   |
|         |                                    ^                         |
|   Command queue (undoable)          Snapshots (double-buffered)        |
|         v                                    |                         |
|  +------------------------------------------------------------------+  |
|  | sim-core (headless, fixed tick, deterministic, own thread)       |  |
|  | clock | citizens | utilities | services | traffic | transit      |  |
|  | pollution | land value | happiness | disasters | region          |  |
|  | uses: pathfinding, world-model, content (RON definitions)        |  |
|  +------------------------------------------------------------------+  |
|   render (terrain, buildings, roads, water, agents, overlays, post-fx) |
|   audio | ui (radial menu, HUD, panels, settings)                      |
|   procgen (terrain, buildings, roads, vegetation)                      |
+------------------------------------------------------------------------+
```

- **Why the split**: the simulation can be unit-tested and benchmarked without a GPU, can run
  faster than real time (2x/3x), can be replayed from a command log, and can later run in a Web
  Worker for the browser build.
- **Threading**: sim-core runs its fixed-tick loop on a dedicated thread and uses a `rayon` pool for
  parallel systems. Presentation reads the latest completed snapshot without blocking the sim.
- **Snapshots**: compact, render-oriented views (e.g., vehicle positions in the agent bubble,
  changed buildings, overlay textures). Large static data (terrain, road geometry) is shared through
  versioned, change-tracked buffers rather than copied each tick.
- **Commands**: every player action (place road, place building, terraform, toggle disaster) is a
  serialisable command with an inverse, giving undo/redo and deterministic replays for free.

## 2.2 Technology stack

Pin the latest stable versions at bootstrap (M0) and record them in
[`docs/TOOLING.md`](../TOOLING.md). Upgrade the engine only between milestones, through an ADR,
because Bevy APIs change between releases. As of November 2026 the latest Bevy major release is
**0.20** (shaders now use WESL, a module-capable extension of WGSL).

| Area | Choice | Notes / rationale |
| --- | --- | --- |
| Language | Rust (stable, edition 2024), MSVC toolchain on Windows | Performance, safe multithreading, strong compiler feedback for an AI-written codebase. |
| Engine | Bevy (latest stable, 0.20.x at time of writing) | Code-first (no editor needed), ECS-native, wgpu renderer (Vulkan / DX12 / Metal / WebGPU), MIT/Apache. |
| Simulation ECS | `bevy_ecs` used standalone inside sim-core | Same mental model as the app; no rendering deps in sim. |
| Bulk data | Struct-of-arrays tables (Vec-based) | Citizens and grid layers are too numerous for per-entity overhead. |
| Shaders | WESL / WGSL | Follow the conventions of the pinned Bevy version. |
| Game UI | `bevy_ui` (+ Bevy Feathers widgets if suitable) | Radial menu, HUD, panels. |
| Debug UI | `bevy_egui` | Inspectors, sim debug panels, perf graphs (dev builds only). |
| Noise / procgen | `noise` or `fastnoise-lite` | fBm, domain warping, ridged noise. |
| RNG | `rand` + `rand_chacha` | Seeded, deterministic, per-system streams. |
| Hashing / maps | `rustc-hash` (FxHash), `indexmap`, `smallvec` | Deterministic iteration; no random-state hashers in sim. |
| Parallelism | `rayon` (sim), Bevy task pools (app) | |
| Serialization | `serde`, `postcard`, `zstd`; RON for content | Compact saves; human-editable content definitions. |
| Errors / logs | `thiserror`, `anyhow` (app only), `tracing` | No `unwrap()` in sim code paths. |
| Audio | Bevy audio or `bevy_kira_audio` (evaluate in M8) | Music playlist + ambient layers. |
| Scripting (optional) | Rhai (evaluate) for mod scripts | Pure Rust, sandboxable; post-1.0. |
| Profiling | Tracy via Bevy's tracing integration | Frame and sim-tick profiling. |
| Testing | `cargo-nextest`, `proptest`, `insta`, `criterion` | Unit, property, snapshot, benchmark. |
| License hygiene | `cargo-deny` | Enforces the dependency license allowlist (3.2). |
| Asset tooling | Blender (headless, Python scripts) — optional | Offline generation / conversion to glTF. |
| Web (post-1.0) | `wasm32-unknown-unknown`, `wasm-bindgen`, WebGPU (WebGL2 fallback) | Reduced demo only. |

> **Note.** Every third-party crate must be compatible with the pinned Bevy version. Prefer fewer,
> well-maintained dependencies. Check APIs against the pinned version's documentation (e.g., via the
> Context7 MCP server or docs.rs) instead of relying on memory — Bevy changes quickly.

## 2.3 Repository layout

```text
D:\app\Others\OpenCity\
|-- Cargo.toml                # workspace
|-- .cargo/config.toml        # fast-compile settings (linker, profiles)
|-- crates/
|   |-- world-model/          # IDs, coordinates, chunk math, shared types (no Bevy render deps)
|   |-- content/              # RON definitions: buildings, services, rules, presets
|   |-- pathfinding/          # road graph, hierarchical routing, route cache, isochrones
|   |-- sim-core/             # headless deterministic simulation (all systems)
|   |-- procgen/              # terrain, building, road and vegetation generators
|   |-- render/               # Bevy plugins: terrain, water, buildings, roads, agents, overlays, post-fx
|   |-- audio/                # music + ambient soundscape
|   |-- ui/                   # radial menu, HUD, panels, settings
|   `-- game/                 # binary: app setup, states, tools, cameras, save/load, modding hooks
|-- tools/
|   |-- bench/                # headless stress scenarios (e.g., 1M citizens)
|   |-- replay/               # deterministic replay runner
|   `-- blender/              # optional Python scripts for asset generation
|-- assets/                   # free-licensed assets + LICENSES.md registry
|-- mods/                     # optional data packs (ignored by default)
|-- docs/
|   |-- blueprint/            # Markdown copy of this document (living version)
|   |-- adr/                  # architecture decision records
|   |-- plans/                # per-milestone plans (M0.md, M1.md, ...)
|   |-- reports/              # per-milestone reports to the owner
|   `-- TOOLING.md            # every installed tool / skill / MCP, with version and reason
|-- .claude/skills/           # project skills (verify, bench, screenshot-review, ...)
|-- CLAUDE.md                 # agent operating rules (Appendix B)
|-- README.md, LICENSE (MIT), CHANGELOG.md, CONTRIBUTING.md
`-- .github/workflows/        # CI
```

## 2.4 World model and coordinates

| Concept | Specification (*Proposed defaults* where noted) |
| --- | --- |
| Units / axes | 1 unit = 1 m, SI units everywhere, Y-up (Bevy convention). |
| Region | Grid of city plots. ***Proposed default: 2 × 2 plots for v1.0*** (configurable up to 3 × 3). Only the active city is simulated in full; the others run an aggregate model (2.5.14). |
| City plot | **16,384 m × 16,384 m** = 268 km², centred on the world origin (−8,192 m … +8,192 m on X and Z). Low preset may cap the playable area to the central 8,192 m × 8,192 m. *Corrected by [ADR 0001](../adr/0001-plot-extent-16384-metres.md) from a literal "16 km": 16,000 m cannot be tiled by 256 m chunks.* |
| Terrain heightfield | 4 m resolution (**4,096 × 4,096** samples), stored as `u16` with 0.02 m steps covering −100 m to +1,210 m (33.6 MB per city). |
| Render chunks | 256 m × 256 m (64 × 64 = 4,096 chunks per city), quadtree LOD (CDLOD-style), streamed by distance. 7 levels, 0 (one chunk) through 6 (whole plot). |
| Simulation grid layers | 32 m cells (**512 × 512** = 262,144 cells per layer, `f32` = 1.05 MB/layer). Layers: air, ground and noise pollution, land value, crime/safety, coverage per service, happiness, traffic density. |
| Large-world precision | `f32` error at the plot edge is 0.98 mm with the plot centred on the origin, half what an un-centred plot would give (both verified by test). Enough for the city view, but it causes jitter in first-person and region views. Use camera-relative rendering / floating origin (evaluate the `big_space` crate or a custom chunk-origin scheme). Agents on networks store `(edge_id, distance_along_edge)` instead of absolute positions. |
| Identifiers | Stable generational IDs (`u32` index + `u32` generation) for buildings, road nodes/edges, citizens, vehicles, transit lines. |

## 2.5 Simulation core

### 2.5.1 Clock and speeds

- Fixed simulation tick: **10 Hz at 1x** (*Proposed default*). 2x and 3x run 20 and 30 ticks per real
  second; the tick size never changes, so results are identical at any speed. Pause stops ticking.
- **Game time**: 1 game day = 20 real minutes at 1x (*Proposed default*; tune in playtests). The
  day-night cycle, commute peaks and schedules follow the game clock.
- Systems run at different cadences (**time-slicing**): movement every tick, utilities every 5 ticks,
  pollution and land value every game hour, job matching every game hour, region exchange every game
  day.

### 2.5.2 Determinism rules

- No wall-clock time, no thread-timing dependence and no OS randomness inside sim-core.
- All randomness from ChaCha streams seeded by `(world_seed, system_id, tick)`.
- No iteration over random-state hash maps; use `Vec`, `BTreeMap`, `IndexMap` or FxHash with sorted
  keys.
- Parallel systems must produce order-independent results (e.g., reduce into per-thread buffers,
  then merge in ID order).
- **Replay = save file + command log.** Goal: bit-identical replay on the same build and platform.
  Cross-platform bit-identical results are not required in v1.

### 2.5.3 GlassBox-inspired model (public concepts only)

Maxis described their 2013 engine publicly in terms of four concepts. OpenCity uses the same
vocabulary, **implemented independently**:

| Concept | In OpenCity |
| --- | --- |
| **Resources** | Quantities that are produced, stored, moved and consumed: power (MW), water (m³/day), sewage, garbage (t/day), workers, students, patients, goods, visitors. |
| **Units** | Buildings and facilities with data-driven rule sets: each tick they consume and produce resources, request agents, and emit effects (pollution, noise, beauty). |
| **Maps** | The 32 m grid layers (pollution, land value, coverage, …) that units read and write. |
| **Agents** | Citizens, vehicles and transit units that carry resources between units along networks (roads, rails, water routes, sky corridors). |

Rules live in RON files under `crates/content/` (and can be extended by mod data packs). Example:

```ron
BuildingDef(
  id: "office_tower_supertall_a",
  category: Office,
  footprint_m: (60.0, 60.0),
  floors: (60, 110),
  style: SupertallCbd,
  jobs_per_m2: 0.08,            // ~12.5 m2 per job (Proposed default)
  consumes: { Power: 0.012, Water: 0.03, Garbage: 0.0004 },  // per job per tick-day
  emits:    { Noise: 0.2, Beauty: 0.6 },
  services_needed: [Fire, Police, Health],
)
```

### 2.5.4 Citizens and households

- Stored as a **struct-of-arrays table**: `id`, `household`, `home_building`,
  `work_or_school_building`, `age_group`, `education_level`, `wealth_band`, `happiness`, `health`,
  `current_state`, `schedule_offset`. About **48 bytes each** (1 million citizens ≈ 48 MB).
- **Move-in**: residential capacity derives from floor area (*Proposed default: 50 m² per
  household*). Vacant homes fill at a rate scaled by attractiveness (happiness factors at that
  location, land value, utilities, road connection). Unhappy households may move out.
- **Jobs and schools**: matched every game hour at district level with a gravity model weighted by
  travel time buckets (avoids O(citizens × jobs)). Individual assignments are then sampled within
  districts.
- **Daily schedules**: commute peaks around 07:00–09:00 and 16:00–19:00 game time, with per-citizen
  offsets.
- **Tourists/visitors**: generated by landmarks, marinas and the airport; they increase commercial
  demand and traffic.

### 2.5.5 Happiness model (replaces taxes)

Each citizen's happiness (0–100) is a weighted sum of factors, smoothed over time. Weights are
data-driven (RON) so playtesting can tune them without code changes. City happiness is the
population-weighted mean and drives move-in/out and visual building upkeep.

| Factor | Source | *Proposed weight* |
| --- | --- | --- |
| Utilities | Power, water, sewage and garbage service at home | 0.20 |
| Safety | Police coverage, crime layer | 0.10 |
| Health | Clinic/hospital coverage and response time | 0.10 |
| Education | School/university access for the household | 0.08 |
| Commute | Travel time to work/school (all modes) | 0.15 |
| Environment | Air, ground and noise pollution at home | 0.15 |
| Beauty and leisure | Parks, water views, landmarks, marinas nearby | 0.10 |
| Transit access | Distance to stops/stations, frequency | 0.07 |
| Shocks | Recent earthquake damage, outages | 0.05 |

### 2.5.6 Utilities: power, water, sewage, garbage

- ***Proposed default network model***: power cables, water pipes and sewer pipes run **under every
  road automatically** (sandbox-friendly). Separate power-line and pipe tools exist for links across
  land without roads.
- Each connected network component balances supply and demand every 5 ticks. If supply < demand, the
  coverage ratio is applied in breadth-first order from producers (nearest consumers served first);
  shortages show on overlays.
- Producers (examples, modern/futuristic set): solar farm, wind turbines, gas plant, nuclear plant,
  fusion plant; water pump, desalination plant, water tower; sewage treatment plant, outflow;
  landfill, incinerator, recycling centre.
- **Garbage**: generated per building; collected by trucks dispatched from facilities (micro agents
  near the camera, statistical elsewhere). Uncollected garbage lowers happiness and raises ground
  pollution.

### 2.5.7 City services: police, fire, health, education

- Coverage is computed as **road-network isochrones** (multi-source Dijkstra on travel time from each
  facility), cached and recomputed incrementally only for road edges that changed.
- Events (crimes, fires, illnesses) are generated statistically from rates per cell. Inside the agent
  bubble they spawn visible vehicles (police car, fire truck, ambulance) that actually drive there;
  outside, response is resolved from the isochrone travel time.
- Facility tiers: police station / HQ; fire station / HQ; clinic / hospital; elementary school / high
  school / university.

### 2.5.8 Pollution and land value

- **Air**: advection-diffusion on the 32 m grid with a slowly changing wind vector; sources: industry,
  power plants, traffic density, airport; exponential decay.
- **Ground**: slow accumulation from industry, landfills and sewage outflows; very slow decay.
- **Noise**: distance falloff from roads (by traffic flow), rail, airport and industry.
- **Land value**: f(service coverage, transit access, water proximity, parks/beauty, pollution, noise,
  crime), updated every game hour, shown as an overlay and used in happiness and move-in
  attractiveness.

### 2.5.9 Traffic and routing

- **Road graph**: nodes (intersections, dead ends) and edges (segments) built from road splines. Edge
  data: lanes per direction, speed limit, capacity (vehicles/hour), length, class (street, avenue,
  highway, one-way), elevation flag (bridges/tunnels later).
- **Routing**: hierarchical routing (contraction hierarchies or road-class hierarchy with HPA*-style
  abstraction). Route cache keyed by `(origin district, destination district, time bucket)`. Road
  edits invalidate only affected cache entries.
- **Macro model (all trips)**: trips are assigned per district origin-destination matrix and
  time-of-day bucket using incremental assignment. Congestion uses the BPR function
  `t = t0 × (1 + 0.15 × (v/c)^4)`. Resulting travel times feed commute happiness and job matching.
- **Micro model (near camera)**: individual vehicles using the **Intelligent Driver Model (IDM)** for
  car following, simple lane choice, and intersection control (signals and priority rules). Vehicles
  spawn so that local density matches the macro flow (`density = flow / speed`) and despawn when they
  leave the bubble.
- **Pedestrians**: near the camera only, walking between buildings and transit stops on sidewalks.

### 2.5.10 Agent level of detail (LOD) — the key to scale

Rendering only what is visible (like Minecraft's chunks) solves only half the problem. The 2013
game's city-size limit came mainly from **simulation cost** (agent routing and engine data
processing over a larger area), and a city, unlike a Minecraft world, cannot be frozen far from the
player. OpenCity therefore uses **simulation LOD**:

| Tier | Where | What is simulated | Cost |
| --- | --- | --- | --- |
| **Tier 0 — Statistical** | Everywhere, always | Every citizen exists as a compact record (home, job, state, happiness). Trips are aggregated into district flows on road edges. | Very low |
| **Tier 1 — Meso** | Mid distance from the focus point | Vehicles move along edges at the macro congestion speed, no interactions, simplified visuals. | Low |
| **Tier 2 — Micro** | Inside the agent bubble (about 1–2 km radius) | Full agents: car following (IDM), lane changes, signals, pedestrians, service dispatch, clickable. | High |

- **Consistency rule**: when the bubble moves, Tier 0 flows are turned into Tier 2 agents whose count
  and destinations match the statistics, and agents leaving the bubble are folded back into the
  flows. **The player must not see the seam.**
- **Click and follow**: any visible car, bus, boat, helicopter or pedestrian maps back to its citizen
  or vehicle ID. The info panel shows home, workplace, happiness and current trip. When following,
  the bubble centre moves with the followed agent, so it never degrades to a statistic.
- **Agent budgets per preset** (Tier 2 maximum): Low 5,000; Medium 20,000; High 50,000; Ultra 100,000.

### 2.5.11 Public transport and the air layer

- **Bus**: routes on roads, stops, depots; buses are traffic agents.
- **MRT**: rail lines (elevated or underground), stations, trains with capacity and frequency.
- **Ferries / ships**: water routes between piers; cargo ships as ambient traffic at ports.
- **Yachts and marina**: leisure boats (mostly ambient) that add beauty, tourism and happiness nearby.
- **Airport**: terminal + runways; passenger and tourist flows to and from outside the region; planes
  fly visual approach/departure paths.
- **Air layer**: helicopters and flying cars use 3D sky corridors at fixed altitudes (*Proposed
  default: 120 m and 200 m*) between helipads and vertiports. In v0.5 they are a premium mode used by
  high-wealth citizens.
- **Mode choice**: each trip chooses car, transit, walk or air with a logit model over generalised
  cost (time + comfort penalties). No ticket money is simulated (unlimited-money sandbox).

### 2.5.12 Disasters: earthquake (toggleable)

- Global toggle: disasters on/off (**default off** for a relaxing sandbox).
- Player-triggered or random (if enabled). Parameters: magnitude M4–M8 and epicentre.
- Damage probability per building = f(intensity at distance, building resilience class). Damaged
  buildings become rubble, utilities break locally, and happiness receives a temporary shock.
- Sandbox options: instant repair, auto-rebuild over time, or leave ruins. Effects: camera shake,
  dust particles, rumble audio.

### 2.5.13 Weather: rain

- Rain events with intensity; visual rain, wet road and terrain materials, puddle reflections and
  splashes.
- Gameplay effect: vehicle desired speed × 0.9 in rain (*Proposed default*). No flooding in v1.

### 2.5.14 Region

- The region view shows all plots; the player picks the active city. Switching saves the active city
  and loads the next one (only one full-fidelity city in memory).
- Inactive cities run an **aggregate model**: population, jobs, utility surplus/deficit, tourism.
- Cities connect through highways, rail, sea routes and air corridors at plot borders. Cross-border
  commuters, power and water exchange are aggregated flows that enter through border nodes.

### 2.5.15 Performance strategy (summary)

- Data-oriented storage; parallel systems on `rayon`; time-slicing by cadence.
- Incremental recomputation with dirty flags (only re-route, re-cover or re-balance what changed).
- Spatial hashing on the 32 m grid; network-distance caches.
- Benchmarks in CI with budgets (2.13) so regressions are caught immediately.

## 2.6 Rendering

### 2.6.1 Pipeline

- Bevy PBR in HDR with filmic tonemapping, bloom, SSAO, cascaded shadow maps for the sun, Bevy's
  procedural atmosphere / sky, distance fog and anti-aliasing (FXAA, SMAA or TAA by preset).
- Use Bevy's GPU-driven rendering and culling features where the pinned version supports them;
  evaluate occlusion culling and contact shadows for High/Ultra.

### 2.6.2 Terrain

- Quadtree LOD over 256 m chunks with crack-free transitions (CDLOD-style morphing or skirts).
- Material splatting by slope, height and paint masks (grass, rock, sand, soil, asphalt-adjacent
  dirt) using CC0 PBR materials.
- Terraforming updates the heightfield, then re-meshes only the affected chunks.

### 2.6.3 Water

- Sea and river surfaces with scrolling normal maps, depth-based colour and shoreline foam.
- Reflections: screen-space reflections on High/Ultra; reflection probes / sky reflection on
  Low/Medium.
- Rain ripples and wet-surface response.

### 2.6.4 Buildings, roads and props

- Procedural building meshes with LOD0–LOD2 plus a far impostor; GPU instancing for repeated modules;
  static merging of far LODs per chunk.
- **Night lights**: emissive windows with a per-building random lit pattern that changes with the
  hour; emissive streetlights; only a limited number of real point/spot lights near the camera.
- **Roads**: spline extrusion with lane-marking decals, generated intersection meshes, sidewalks,
  wetness parameter.
- **Vehicles and pedestrians**: instanced meshes; far vehicles at night become headlight/taillight
  sprites.

### 2.6.5 Post-processing

- **Tilt-shift** (toggle + strength): depth-based blur band around the focus distance plus mild
  saturation/contrast boost.
- Colour grading LUT per time of day; optional vignette.

### 2.6.6 Data overlays (required for v1.0, available from v0.1)

- Sim grid layers are uploaded as textures and projected onto terrain and building roofs with a
  legend.
- Overlays: power, water, sewage, garbage, police, fire, health, education coverage,
  air/ground/noise pollution, land value, happiness, traffic congestion, transit coverage,
  crime/safety.

### 2.6.7 Culling and streaming

- Frustum culling (Bevy), distance culling per category (props < trees < buildings), chunk streaming
  with load and unload radii, LOD bias per preset.

### 2.6.8 Graphics presets

| Setting | Low | Medium | High | Ultra |
| --- | --- | --- | --- | --- |
| Render scale (default) | 75% | 100% | 100% | 100% |
| Shadows | 1,024 px, 2 cascades | 2,048 px, 3 cascades | 4,096 px, 4 cascades | 4,096 px, 4 cascades + contact |
| SSAO | Off | Low | On | High |
| Reflections | Probe / sky | Probe / sky | SSR on water | SSR on all |
| Anti-aliasing | FXAA | SMAA | TAA | TAA |
| Building view distance | 4 km | 8 km | 16 km | Whole region |
| Tier 2 agent cap | 5,000 | 20,000 | 50,000 | 100,000 |
| Active city size | 8 km (central) | 16 km | 16 km | 16 km |
| Target | 1080p / 30 fps | 1080p / 60 fps | 1080p / 60 fps | 1440p / 60 fps |

An **Auto** option picks a preset from the detected GPU adapter and VRAM on first launch. Every
setting stays individually adjustable under **Advanced**.

### 2.6.9 Camera modes

| Mode | Controls | Limits |
| --- | --- | --- |
| City camera | Pan: WASD / edge scroll / middle-drag; rotate: Q/E or right-drag; zoom: wheel; tilt: R/F or Alt+drag | Altitude 20 m – 5,000 m; smooth inertia |
| Street camera | Orbit around a clicked point at low altitude | 2 m – 30 m |
| First person (v1.0) | WASD walk, mouse look; when following a vehicle: ride-along view | Collision with buildings/terrain |
| Follow cam | Click an agent, press Follow; cycle views with C | Bubble follows the agent |

Bevy has provided basic built-in fly and pan camera controllers since 0.18; they may be used as a
starting point but the final cameras are custom.

## 2.7 UI and UX

- **Radial menu** (original design, mouse-first): right-click or Space opens a ring with categories —
  Roads, Buildings (Residential, Commercial, Office, Industrial, Landmarks), Utilities, Services,
  Transport, Parks, Terraform, Disasters, Overlays. Choosing a category opens a sub-ring of items
  with thumbnails.
- **HUD**: top bar with population, happiness, date/time and speed controls (pause / 1x / 2x / 3x);
  overlay toggle bar; notification feed.
- **Placement**: ghost preview, rotate with R or wheel, snapping (grid / road edge / 15 degree
  angles), green/red validity tint, undo/redo (Ctrl+Z / Ctrl+Y). Productivity tools that still
  respect manual placement: stamp the same building along a line or across an area, copy/paste a
  block.
- **Road tools**: straight, curved (Bezier), freeform; grid-snap toggle (G); upgrade/downgrade;
  one-way; demolish.
- **Info panels**: click a building or agent; Follow button; per-building stats (occupancy, jobs,
  services, happiness).
- **Settings**: graphics preset + Advanced, render scale, audio volumes, key bindings, autosave
  interval, tilt-shift.
- **Language**: English only, but all strings go through a localisation layer (e.g., Fluent) so
  translations are possible later.

## 2.8 Procedural content

### 2.8.1 Terrain generation

- Inputs: seed, preset (Flat, Hills, Coastal), river on/off, sea level.
- Method: fBm + domain-warped noise, ridged noise for hills; hydraulic erosion pass; rivers carved by
  flow accumulation; coastlines from a large-scale mask.
- Outputs: heightfield, material masks, water bodies, buildability mask.
- Budget: generate a 16 km plot in **under 30 s** on recommended hardware (parallel CPU or GPU
  compute).

### 2.8.2 Terraforming tools

- Raise, lower, flatten, smooth and level-to-height brushes, 8 m to 512 m radius.
- Updates simulation (buildability, water) and render chunks incrementally; undoable.

### 2.8.3 Building generator

A rule/grammar-based generator turns a `BuildingDef` + seed into a complete building:

1. **Footprint**: rectangle, L, U, rounded or chamfered, sized within the definition's range.
2. **Massing**: extrude floors; podium + tower; setbacks; taper, twist or stepped crowns for
   supertall towers.
3. **Facade**: module grid of bays — curtain wall, punched windows, mullions, balconies, spandrels.
4. **Roof**: flat with equipment, crowns, spires, helipads (enables the air layer).
5. **Materials**: CC0 PBR glass, concrete, metal, stone, with per-building tint variation.
6. **LODs**: LOD0–LOD2 meshes + impostor; collision box; metadata (capacity, jobs) derived from floor
   area.

| Style family | Description |
| --- | --- |
| Supertall CBD | Generic glass-and-steel supertall towers inspired by modern Asian business districts such as Jakarta's Sudirman and Shanghai's Lujiazui. **Generic forms only — no replicas of specific famous buildings.** |
| International modern | Mid-rise offices and mixed-use blocks with modern facades. |
| Residential | Apartment towers, mid-rise blocks, low-rise housing. |
| Industrial | Sheds, factories, warehouses, ports, power plants. |
| Civic | Police, fire, clinic, hospital, schools, university, transit stations, airport terminal, marina. |

### 2.8.4 Roads and vegetation

- **Roads**: cubic Bezier splines, extruded cross-section profiles per class, automatically generated
  intersections and crossings, grid mode with configurable cell size (*Proposed default 16 m*).
- **Vegetation**: CC0 tree and shrub models, GPU-instanced; procedural placement in parks, along
  avenues and in natural areas.

## 2.9 Assets and licensing policy

The game is non-commercial, but the repository is public and MIT-licensed, which means everything in
it is redistributed. **Every asset license MUST allow redistribution.**

| Allowed | Not allowed |
| --- | --- |
| CC0 / public domain | Non-commercial (NC) or no-derivatives (ND) licenses |
| CC-BY 3.0 / 4.0 (credit in LICENSES.md and in-game credits) | "Free for personal use" or unclear licenses |
| SIL Open Font License (fonts) | Anything ripped or extracted from a game or commercial product |
| MIT / Apache-2.0 / BSD / Zlib code and shaders | Recreations, traces or light modifications of another game's assets |
| Our own procedural output | Real brand logos; exact replicas of distinctive copyrighted landmark designs |

- **Registry**: `assets/LICENSES.md` lists every file: name, source URL, author, license, date
  downloaded, modifications. The in-game Credits screen is generated from it.
- **Suggested sources** (verify the license at download time): Poly Haven and ambientCG (CC0
  materials, HDRIs, models), Kenney and Quaternius (CC0 model kits), OpenGameArt (filter CC0/CC-BY),
  Freesound (filter CC0), CC-BY or CC0 music libraries (e.g., Kevin MacLeod's incompetech, CC-BY 4.0
  with attribution).
- **AI 3D / audio generators (optional)**: only if the tool's terms grant output rights compatible
  with public redistribution; record tool, version and prompt in the registry.
- **Repository size**: prefer generating content procedurally at runtime or build time; use Git LFS
  for unavoidable binaries larger than 1 MB.

## 2.10 Audio

- **Music**: playlist of CC0/CC-BY tracks with crossfades; separate volume slider. Music is required
  for v0.1.
- **Ambient soundscape** layered by camera altitude and district type: traffic hum, crowds, wind,
  sea, rain, construction.
- UI sounds; earthquake rumble; aircraft and helicopter flyovers (3D positional near camera).

## 2.11 Save/load, autosave and timelapse

- **Save format**: header (magic bytes, schema version, world seed, metadata, thumbnail) + sections
  (terraform edits relative to the seed-generated base, roads, buildings, citizens table, transit
  lines, settings), serialised with `postcard` and compressed with `zstd`. Derived layers are
  recomputed on load.
- **Versioning**: every schema change bumps the version and adds a migration; old saves must keep
  loading.
- **Autosave**: every 10 real minutes (configurable), three rotating slots, written on a background
  thread from a snapshot so gameplay never stutters.
- **Timelapse**: the player places a camera bookmark; once per game day the game captures a
  1280 × 720 frame plus key stats. An in-game viewer plays the sequence; export as a PNG sequence
  (video export only if the user has ffmpeg installed).

## 2.12 Mod support (optional, off by default)

- **Phase 1 (v1.0)**: data packs in `mods/<name>/` with `mod.ron` (name, version, author, license)
  plus building definitions, rule-weight overrides and glTF/texture assets. A mod manager screen
  enables/disables packs; vanilla is the default.
- **Phase 2 (post-1.0)**: scripting via a sandboxed engine (Rhai is the leading candidate): no file
  or network access, rate-limited hooks (`on_tick`, `on_building_placed`, custom overlays).
- Mods must declare a license; the game warns before loading a mod without one.

## 2.13 Performance budgets

| Metric | Budget / target |
| --- | --- |
| Frame time | 16.6 ms at 60 fps (Medium/High 1080p on the recommended tier; Ultra 1440p on the Ultra tier) |
| Sim tick at 1x | 100 ms available per tick; target under 40 ms per tick on the sim thread at 1M citizens (so 3x can keep up on 4+ cores) |
| Memory | Low under 3 GB total; High under 6 GB at 1M citizens |
| New city generation | Under 30 s |
| Save / load | Under 5 s / under 10 s |
| Stutter | No frame above 50 ms during normal play (except loading) |

Benchmarks run headlessly in `tools/bench` (e.g., scenario `million_city`) and in CI with thresholds;
a regression over 10% fails the check unless the report justifies it.

## 2.14 Testing and QA

| Level | What | Tooling |
| --- | --- | --- |
| Unit | Rules of every sim system, procgen invariants, math | `cargo-nextest` |
| Property | Invariants: resources never negative, citizens always housed or homeless-counted, graph connectivity | `proptest` |
| Snapshot | Generated meshes/stats for fixed seeds; RON parsing | `insta` |
| Determinism | Replay a recorded session twice and compare state hashes per tick | `tools/replay` |
| Benchmark | Tick time, memory, generation time at 100k / 1M citizens | `criterion` + `tools/bench` |
| Visual | Headless or scripted screenshots of fixed camera bookmarks; reviewed by Claude and diffed | in-app F12 + scripts |
| Smoke | Release build launches, loads a save, runs 1 game day without panic | CI (Windows + Linux) |
| Playtest | Owner plays each milestone build and gives feedback | Milestone report |

## 2.15 Build, tooling and CI

- **Windows prerequisites**: rustup (stable, `x86_64-pc-windows-msvc`), Visual Studio Build Tools with
  the "Desktop development with C++" workload, Git (with Git LFS), optionally Python 3 and Blender
  for asset scripts.
- **Fast compiles**: use Bevy's `dynamic_linking` feature in dev builds only; use the `rust-lld`
  linker where supported; dev profile `opt-level = 1` for our crates and `opt-level = 3` for
  dependencies.
- **CI (GitHub Actions)**: fmt, clippy (warnings = errors), nextest, cargo-deny, benchmarks (on a
  schedule), release builds for Windows and Linux on tags with artifacts attached to GitHub Releases.

## 2.16 Browser demo (post-1.0)

- Target `wasm32-unknown-unknown` with WebGPU (WebGL2 fallback).
- Constraints: wasm memory limits and weaker multithreading (SharedArrayBuffer requires COOP/COEP
  headers). Therefore: 8 km plot, Low preset, Tier 2 cap 3,000, sim in a Web Worker.
- Hosted on GitHub Pages from CI.

## 2.17 Risks and mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Scope is very large | High | Strict milestones with Definition of Done; owner approves each before the next. |
| Bevy API churn | Medium | Pin version; upgrade only between milestones via ADR; check docs for the pinned version. |
| Weak dev laptop GPU / 8 GB RAM | High | Presets, render scale, agent caps, 8 km mode; measure in M0. |
| Agent simulation cost at 1M+ | High | Three-tier LOD, aggregate first, micro only in the bubble; benchmarks in CI. |
| Semi-realistic look with free assets | Medium | Invest in lighting, materials and post-fx; CC0 PBR libraries; procedural variety. |
| License contamination | High | Allowlist, LICENSES.md registry, cargo-deny, review of every new asset. |
| Name conflict (OpenCity) | Low now / High at launch | Rename before public launch (Q1). |
| Agent gets stuck in loops | Medium | Stop after 3 failed attempts on the same error; write an issue note; change approach or ask the owner. |
| Long compile times on Windows | Medium | Dynamic linking, fast linker, small crates, incremental builds. |
| Float precision in large worlds | Medium | Floating origin / camera-relative rendering; edge-relative agent positions. |

## 2.18 Roadmap and milestones

Each milestone ends with: all quality gates green, a demo build, screenshots, measured performance on
the dev laptop, and a report to the owner (Part III, 3.10). **The next milestone starts only after
the owner's playtest approval.** Durations are rough estimates in agent working days and depend on
the owner's Claude usage limits.

| # | Milestone | Scope | Definition of Done (summary) | Est. |
| --- | --- | --- | --- | --- |
| **M0** | Preview foundation | Workspace, environment check, window, 16 km terrain (procedural, 3 presets), chunk LOD streaming, city camera, sky + day-night + shadows, road tool v0, 3 procedural building types placed manually, FPS overlay, presets. | Runs on dev laptop; no LOD cracks; roads + buildings placeable with undo; preset switch at runtime; specs and FPS recorded. | 1–3 d |
| **M1** | Land and water | Terrain erosion + rivers, terraforming brushes, sea/river water with reflections, terrain materials, road splines v1 with intersections and grid mode. | All 3 presets look natural; terraform + roads interact correctly; water reflects on High. | 3–5 d |
| **M2** | Buildings and UI | Building generator v1 (5 style families, LODs, impostors), radial menu, placement tools (stamping, copy/paste), night lights, HUD, settings screen. | Skyline at 5 km altitude holds target fps; night city glows; radial menu fully mouse-operable. | 4–6 d |
| **M3** | Living city core | Citizens table, move-in/out, jobs/schools matching, happiness, power, water, sewage, garbage, overlays v1, save/load + autosave, music. | 100k-citizen city stable at 1x; overlays correct; saves round-trip; property tests pass. | 5–7 d |
| **M4** | Traffic and agents | Road graph, hierarchical routing, macro assignment + BPR, meso + micro (IDM) tiers, bubble spawning, pedestrians, click-and-follow. | No visible seams when moving the bubble; follow cam works; benchmark budgets met at 100k. | 6–8 d |
| **M5** | Services and environment | Police, fire, health, education (isochrones + dispatch), air/ground/noise pollution, land value, crime layer. | **Alpha v0.1 tagged**: all v0.1 systems live with overlays. | 5–7 d |
| **M6** | Transport | Bus, MRT, ferries/ships, yachts/marina, airport, helicopters, flying cars (sky corridors), mode choice. | Transit changes commute times and happiness measurably; air layer visible and clickable. | 6–9 d |
| **M7** | Region | Region generation and view, plot switching, aggregate neighbour cities, border connections. | 2 × 2 region playable; cross-border flows visible. | 4–6 d |
| **M8** | Atmosphere and events | Rain, earthquake (toggle), ambient audio, timelapse, street camera. | **Beta v0.5 tagged.** | 4–6 d |
| **M9** | First person and scale | First-person mode, street props, optimisation to 1M+ citizens, memory work. | 1M-citizen benchmark meets budgets on the recommended tier. | 6–9 d |
| **M10** | Release v1.0 | Polish, Linux build, mod data packs (optional, off), credits screen, README with GIFs, public GitHub release. | **v1.0 tagged**; soak test 2 h without crash; licenses audited. | 4–6 d |
| **M11+** | Post-1.0 | Browser demo, mod scripting, building editor (if approved), more disasters/styles. | Per follow-up plan. | — |

## 2.19 M0 in detail (the preview)

1. **Environment check**: record OS version, CPU, RAM, GPU adapter name, VRAM, driver and backend
   (print the wgpu adapter info at startup; also `dxdiag /t dxdiag.txt`). Write them to
   `docs/reports/M0-environment.md`.
2. **Workspace**: create the Cargo workspace and crates from 2.3 (empty crates allowed), MIT LICENSE,
   README, `.gitignore`, `CLAUDE.md` (Appendix B), `docs/blueprint/` Markdown transcription.
3. **Fast compile config** (2.15) and a `verify` project skill (Appendix C).
4. **App shell**: window, vsync toggle, FPS + frame-time overlay, F12 screenshot to `screenshots/`,
   graphics presets Low/Medium/High/Ultra switchable at runtime (F1–F4 in dev builds).
5. **Terrain**: 16 km plot from seed with Flat / Hills / Coastal presets, sea level, basic splat
   material; 256 m chunks with quadtree LOD, streaming, crack-free seams.
6. **City camera**: pan, rotate, zoom, tilt, altitude 20 m – 5,000 m, smooth.
7. **Sky and time**: Bevy atmosphere, sun driven by a game clock (1 game day = 20 real minutes),
   cascaded shadows, night darkness with a simple sky.
8. **Road tool v0**: click to place straight and curved (Bezier) segments; grid snap toggle (G);
   extruded road mesh draped on terrain; demolish.
9. **Building placement v0**: a temporary radial menu with 3 procedural types (house, mid-rise,
   tower) built from parametric boxes with a window material; ghost preview, rotate, place, delete,
   undo/redo.
10. **Verify**: all gates pass; release build; capture 6 screenshots (dawn, noon, dusk, night, 5 km
    altitude, street level); measure FPS per preset at 1080p (and 1440p if the screen supports it).
11. **Report** (Bahasa Indonesia) with run instructions, screenshots, FPS table, known issues and
    decisions needed.
