# Part I — Product Overview

*What, why and for whom. Binding level: directional.*

## 1.1 Vision

OpenCity (working title) is a free, open-source (MIT) 3D city-building sandbox for Windows and
Linux, with a lighter browser demo planned later. It recreates the *feel and flow* of modern
agent-based city builders — most notably the 2013 SimCity reboot — but is built entirely from
scratch, with original or free-licensed assets and no code, art or trademarks from Electronic Arts
or any other company.

OpenCity is made for fun, not for sale. Its four promises are:

1. **Total creative freedom** — sandbox only: unlimited money, no failure state, no taxes, place
   anything you like.
2. **Massive scale** — each city plot is 16 km × 16 km (256 km²), inside a region of several
   connected cities. For comparison, a SimCity 2013 plot is only about 2 km × 2 km.
3. **A living city** — more than one million citizens; cars and people near the camera are
   individual agents that the player can click and follow.
4. **A beautiful city** — semi-realistic PBR graphics, real-time shadows, day-night cycle, rain,
   water reflections, glowing night lights and an optional tilt-shift miniature look.

## 1.2 Goals

- Ship a playable preview (M0) as fast as possible (target: 1–3 working days of agent time) so the
  owner can see and steer the foundation.
- Reach v1.0 with every system the owner requested (see 1.7 and Appendix A).
- Hold 60 fps at 1080p and 1440p on recommended / high-end hardware, with graphics presets that
  scale down to ordinary laptops.
- Keep the codebase clean, tested and well documented so it can become a respected open-source
  project and a flagship portfolio piece for the owner.
- Be legally clean: clean-room design, permissive licenses only, full asset provenance.

## 1.3 Non-goals

- Multiplayer of any kind.
- Monetisation, paid DLC, ads, telemetry.
- Taxes, budgets or any money-management gameplay (money is unlimited).
- Automatic zoning / RCI growth (the owner chose manual placement only).
- Mobile, console, touch or gamepad support (mouse + keyboard only).
- Photorealistic AAA parity in v1 (target is high-quality semi-realistic PBR).
- Any use of EA / SimCity assets, code, decompiled material, names or logos.

## 1.4 Inspirations and legal boundaries

OpenCity is a **spiritual successor, not a clone**. Game mechanics and genre conventions (placing
roads, utilities carried by networks, agent-based citizens, data overlays, radial build menus) are
not protected and may be implemented. **Concrete expression is protected and MUST NOT be copied.**

| Allowed (public knowledge) | Not allowed |
| --- | --- |
| Concepts from Maxis' public talks and articles about the GlassBox engine (resources, units, maps, agents). | Decompiling or reverse-engineering EA game binaries to copy code or exact formulas. |
| General city-simulation and traffic-engineering literature (IDM, BPR, gravity models, isochrones). | Any EA / SimCity asset: models, textures, UI art, icons, sounds, music, fonts. |
| Genre UX conventions: radial menu, overlays, follow-cam, time controls. | The names "SimCity", "Sims", "GlassBox" or look-alike logos in the product. |
| Free-licensed assets (CC0, CC-BY) and our own procedural assets. | Recreating, tracing or lightly modifying assets from any other game. |
| Reading GPL projects (e.g., Micropolis) to understand ideas. | Copying GPL code into this MIT codebase (license incompatible). |

> **Important — name check.** "OpenCity" is already the name of a discontinued GPLv2 3D
> city-building game started in 2003 (still packaged in Debian and Ubuntu). The owner accepted it
> as a temporary working title. A final name MUST be chosen and checked (GitHub, crates.io,
> Steam/itch.io search) before the first public announcement.

## 1.5 Target players and hardware tiers

Players: city-builder fans who want a relaxing, unlimited sandbox at a scale no commercial game
offers, plus developers interested in large-scale simulation and Rust/Bevy.

| Tier | Reference hardware (examples) | Target |
| --- | --- | --- |
| Minimum (Low preset) | Integrated GPU (Intel Iris Xe / AMD Radeon 680M class), 8 GB RAM, SSD | 1080p, 30 fps; active city may be capped at 8 km × 8 km; reduced agents |
| Recommended (High preset) | GTX 1660 / RTX 3050 class, 16 GB RAM, SSD | 1080p, 60 fps, full 16 km city |
| Ultra | RTX 3070 class or better, 16–32 GB RAM | 1440p, 60 fps |
| Owner's dev laptop | Windows, personal laptop, 8–16 GB RAM (exact value TBD), GPU TBD, ~200 GB free SSD | Primary test machine; Claude MUST detect and record its specs in M0 |

> Measured in M0: see [`docs/reports/M0-environment.md`](../reports/M0-environment.md).

## 1.6 Experience pillars

| Pillar | What it means in practice |
| --- | --- |
| **Freedom** | Unlimited money, no failure, terraform anything, curved or grid roads, place any building anywhere valid, disasters optional, speed control 1x/2x/3x. |
| **Scale** | 16 km × 16 km city plots in a multi-city region; zoom out to 5 km altitude and see the whole skyline. |
| **Life** | Millions of citizens with homes, jobs, schools and commutes; nearby cars, buses, boats, helicopters and flying cars move individually; click anyone and follow them, even in first person. |
| **Beauty** | Semi-realistic lighting, day-night, rain, reflective water, city lights, tilt-shift, timelapse of your city's growth. |

## 1.7 Core gameplay loop

1. Generate a region and pick a city plot (terrain preset: Flat, Hills, Coastal; optional river).
2. Terraform the land (raise, lower, flatten, smooth).
3. Draw roads (free curves or grid-snapped); utilities travel under roads automatically.
4. Place buildings manually: residential, commercial, office, industrial, civic, landmarks.
5. Citizens move into homes, find jobs and schools, commute, and consume services.
6. Happiness and land value respond to services, pollution, commute time, beauty and safety.
7. Expand: add public transport (bus, MRT, ferries, airport, helicopters, flying cars), parks and
   landmarks.
8. Enjoy: follow citizens, watch day turn to night, record a timelapse, connect neighbouring cities.

## 1.8 Feature scope by release

| Release | Milestones | Contents |
| --- | --- | --- |
| **Preview** | M0 | Window, 16 km procedural terrain with chunk LOD streaming, city camera, sun + sky with day-night and real-time shadows, road tool v0 (straight + curved, grid snap), manual placement of 3 procedural building types, FPS counter, graphics presets. |
| **Alpha v0.1** | M1–M5 | Terrain presets + terraforming, sea/river water with reflections, procedural building generator v1, radial menu, night lights, citizens + happiness, power, water, sewage, garbage, police, fire, health, education, pollution, land value, data overlays, traffic (macro + micro), click-and-follow, autosave, music. |
| **Beta v0.5** | M6–M8 | Public transport (bus, MRT, ferries/ships, yachts, airport) and the air layer (helicopters, flying cars), multi-city region, earthquake disaster, rain, timelapse, street-level camera. |
| **v1.0** | M9–M10 | First-person camera, street props, performance at 1M+ population, Linux build, polish, mod data-packs (optional, off by default), public release on GitHub. |
| **Post-1.0** | M11+ | Browser demo (small map, Low preset), mod scripting (optional), in-game building editor (if approved), more disasters and styles. |

## 1.9 Success criteria

- **M0**: runs on the owner's laptop; whole 16 km terrain navigable without visible LOD cracks;
  roads and buildings can be placed; day-night works; presets switch at runtime; all quality gates
  pass.
- **v0.1**: a city of 100,000 citizens runs at 1x with stable frame rate on the dev laptop at its
  best preset; all v0.1 systems visible through overlays.
- **v1.0**: a city of 1,000,000+ citizens on the recommended tier at 60 fps (High, 1080p) with
  simulation keeping up at 3x; save/load under 5 s; no crashes in a 2-hour soak test.

## 1.10 Open questions

| # | Question | Owner of answer | Default until answered |
| --- | --- | --- | --- |
| Q1 | Final project name (OpenCity is taken) | Azrll | Keep "OpenCity" internally |
| Q2 | In-game building editor — wanted? | Azrll | Not planned before v1.0 |
| Q3 | Exact laptop specs (GPU model, RAM 8 or 16 GB, screen resolution) | Claude detects in M0 | Assume 8 GB + integrated GPU (worst case) |
| Q4 | Which Claude plan (affects daily throughput) | Azrll | Work in small, resumable tasks |
| Q5 | GitHub repository name and visibility | Azrll | Private repo under Arfazrll until v0.1 |

> **Q3 is now answered** — measured in M0, see
> [`docs/reports/M0-environment.md`](../reports/M0-environment.md).
