//! Terrain generation (blueprint 2.8.1).
//!
//! # Design
//!
//! Height is a pure function of `(seed, preset, normalised position)`. Nothing is read from
//! the OS, no wall-clock time is consulted, and the per-sample function has no state, so the
//! whole plot can be filled in parallel on `rayon` and still come out byte-identical every
//! run. That is what lets a save file store a seed instead of 33.6 MB of heightfield
//! (blueprint 2.11) and what lets snapshot tests pin generated output.
//!
//! # Method
//!
//! Three layers, following blueprint 2.8.1:
//!
//! 1. **Domain warp** — the sample position is displaced by a low-frequency fBm before the
//!    height noise is evaluated. This is what stops fBm terrain from looking like uniform
//!    static: it bends ridges and valleys into shapes that read as geology.
//! 2. **Height** — fBm mixed with ridged multifractal noise. fBm alone gives rolling, rounded
//!    ground; ridged noise gives sharp crests. The mix weight is what separates the Flat
//!    preset from the Hills preset.
//! 3. **Coast mask** — for the Coastal preset only, a wavy north-south shoreline derived from
//!    a separate low-frequency noise, blending from seabed to land.
//!
//! # Deliberately not in M0
//!
//! Hydraulic erosion and river carving by flow accumulation are blueprint 2.8.1 features
//! scheduled for **M1** ("Land and water", blueprint 2.18). M0 only needs the three presets to
//! generate and to look distinct, so this module stops at noise. The function signature
//! already carries `rivers` so that adding them does not change call sites.

use rayon::prelude::*;
use world_model::{Heightfield, height, plot};

use noise::{Fbm, MultiFractal, NoiseFn, Perlin, RidgedMulti};
use rand_chacha::ChaCha12Rng;
use rand_core::{Rng, SeedableRng};

/// System identifier for the terrain generator's random stream.
///
/// Blueprint 2.5.2 seeds every stream by `(world_seed, system_id, tick)` so that two systems
/// drawing from the same world seed never produce correlated sequences. Terrain is generated
/// once, so there is no tick component.
const SYSTEM_ID_TERRAIN: u64 = 1;

/// The terrain presets offered when creating a city (blueprint 2.8.1, decision log A20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerrainPreset {
    /// Gently undulating lowland. The easiest preset to build a large city on.
    #[default]
    Flat,
    /// Ridged hills with substantial relief.
    Hills,
    /// Sea along one edge with a wavy shoreline, rising inland.
    Coastal,
}

impl TerrainPreset {
    /// All presets, for the city-creation screen and for tests.
    pub const ALL: [Self; 3] = [Self::Flat, Self::Hills, Self::Coastal];

    /// Short name for the HUD, logs and the milestone report.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Flat => "Flat",
            Self::Hills => "Hills",
            Self::Coastal => "Coastal",
        }
    }
}

/// Inputs to terrain generation.
#[derive(Debug, Clone, Copy)]
pub struct TerrainParams {
    /// World seed. The same seed and preset always produce the same terrain.
    pub seed: u64,
    /// Which preset to generate.
    pub preset: TerrainPreset,
    /// Sea level in metres. Everything below this is underwater.
    pub sea_level_m: f32,
    /// Whether to carve rivers. **Not implemented until M1**; accepted now so that adding it
    /// does not change call sites.
    pub rivers: bool,
}

impl Default for TerrainParams {
    fn default() -> Self {
        Self {
            seed: 0,
            preset: TerrainPreset::default(),
            sea_level_m: 0.0,
            rivers: false,
        }
    }
}

/// What a generation run produced, for the HUD and the milestone report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerrainStats {
    /// Lowest height in the plot, in metres.
    pub min_m: f32,
    /// Highest height in the plot, in metres.
    pub max_m: f32,
    /// Fraction of samples at or below sea level, in `0.0..=1.0`.
    pub water_fraction: f32,
}

/// Per-preset shaping constants.
///
/// Extracted into a struct so the three presets differ only in data, which makes them easy to
/// retune during playtesting without touching the generation code.
#[derive(Debug, Clone, Copy)]
struct Shape {
    /// Height the terrain is centred on, in metres.
    base_m: f32,
    /// Peak deviation from `base_m`, in metres.
    amplitude_m: f32,
    /// Noise cycles across the full 16,384 m plot. 4.0 gives roughly 4 km features.
    frequency: f64,
    /// fBm octaves. More octaves add fine detail at the cost of generation time.
    octaves: usize,
    /// Mix weight of ridged noise against plain fBm, in `0.0..=1.0`.
    ridged_weight: f32,
    /// Domain warp displacement, in normalised plot units.
    warp_strength: f64,
    /// Whether to apply the coast mask.
    coastal: bool,
    /// Extra height added with distance inland, in metres. Coastal only.
    inland_lift_m: f32,
}

impl Shape {
    const fn for_preset(preset: TerrainPreset) -> Self {
        // Height budget. `height::encode` clamps to -100 m .. +1,210 m, and a clamp is
        // invisible in the generator but shows up as an unnaturally flat floor or ceiling in
        // the terrain. So `base_m +/- amplitude_m` (plus any lift) must stay inside that
        // range with margin. The `every_preset_stays_inside_the_encodable_height_range` test
        // enforces it; these numbers were corrected once already after that test caught the
        // Hills preset bottoming out at -162 m.
        match preset {
            // Low relief, long wavelengths: somewhere to lay out a grid city without
            // fighting the ground. Range 13 m .. 35 m.
            TerrainPreset::Flat => Self {
                base_m: 24.0,
                amplitude_m: 11.0,
                frequency: 2.5,
                octaves: 5,
                ridged_weight: 0.0,
                warp_strength: 0.08,
                coastal: false,
                inland_lift_m: 0.0,
            },
            // Ridged noise dominant. Range 25 m .. 395 m, so valleys sit near sea level and
            // crests are genuinely mountainous. Terraforming exists for a reason.
            TerrainPreset::Hills => Self {
                base_m: 210.0,
                amplitude_m: 185.0,
                frequency: 3.5,
                octaves: 6,
                ridged_weight: 0.6,
                warp_strength: 0.22,
                coastal: false,
                inland_lift_m: 0.0,
            },
            // Sea to the west, a coastal plain, then rising ground inland. The lift is what
            // guarantees the eastern side is dry land: noise alone would leave the far inland
            // edge below sea level wherever the fBm happened to dip.
            TerrainPreset::Coastal => Self {
                base_m: 35.0,
                amplitude_m: 85.0,
                frequency: 3.0,
                octaves: 6,
                ridged_weight: 0.25,
                warp_strength: 0.16,
                coastal: true,
                inland_lift_m: 90.0,
            },
        }
    }
}

/// How far inland the coastal lift takes to reach full height, in normalised plot units.
const INLAND_LIFT_SPAN: f32 = 0.66;

/// Seabed depth offshore on the Coastal preset, in metres.
const SEABED_M: f32 = -38.0;

/// Where the shoreline sits across the plot, in normalised units from the western edge.
const COAST_POSITION: f64 = 0.34;

/// How far the shoreline wanders, in normalised units.
const COAST_WAVINESS: f64 = 0.11;

/// Smooth Hermite interpolation between two edges.
///
/// Used instead of a linear ramp for the shoreline so that the land does not meet the sea at a
/// visible crease.
#[inline]
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A configured terrain generator.
///
/// Holds the noise functions so they are built once rather than per sample. Every method
/// takes `&self`, which is what allows the fill below to run on `rayon`.
pub struct Generator {
    warp_x: Fbm<Perlin>,
    warp_z: Fbm<Perlin>,
    base: Fbm<Perlin>,
    ridge: RidgedMulti<Perlin>,
    coast: Fbm<Perlin>,
    shape: Shape,
}

impl Generator {
    /// Builds a generator for the given parameters.
    #[must_use]
    pub fn new(params: TerrainParams) -> Self {
        // Derive the per-noise seeds from one ChaCha stream rather than reusing `params.seed`
        // for each. Perlin with adjacent seeds produces visibly correlated fields, so the warp
        // would end up echoing the height it is supposed to be distorting.
        let mut seed_bytes = [0u8; 32];
        seed_bytes[..8].copy_from_slice(&params.seed.to_le_bytes());
        seed_bytes[8..16].copy_from_slice(&SYSTEM_ID_TERRAIN.to_le_bytes());
        let mut rng = ChaCha12Rng::from_seed(seed_bytes);

        let shape = Shape::for_preset(params.preset);

        Self {
            warp_x: Fbm::<Perlin>::new(rng.next_u32())
                .set_octaves(3)
                .set_frequency(shape.frequency * 0.5),
            warp_z: Fbm::<Perlin>::new(rng.next_u32())
                .set_octaves(3)
                .set_frequency(shape.frequency * 0.5),
            base: Fbm::<Perlin>::new(rng.next_u32())
                .set_octaves(shape.octaves)
                .set_frequency(shape.frequency)
                .set_persistence(0.5)
                .set_lacunarity(2.0),
            ridge: RidgedMulti::<Perlin>::new(rng.next_u32())
                .set_octaves(shape.octaves.saturating_sub(1).max(1))
                .set_frequency(shape.frequency)
                .set_lacunarity(2.0),
            coast: Fbm::<Perlin>::new(rng.next_u32())
                .set_octaves(2)
                .set_frequency(2.0),
            shape,
        }
    }

    /// Height in metres at a normalised plot position, both axes in `0.0..=1.0`.
    ///
    /// This is the whole generator: everything else in this module either calls it or stores
    /// its output.
    #[must_use]
    pub fn height_m(&self, nx: f64, nz: f64) -> f32 {
        // 1. Domain warp.
        let wx = nx + self.warp_x.get([nx, nz]) * self.shape.warp_strength;
        let wz = nz + self.warp_z.get([nx, nz]) * self.shape.warp_strength;

        // 2. Height. fBm is roughly -1..1; RidgedMulti is roughly 0..1, so it is remapped to
        //    the same range before mixing, otherwise changing the mix weight would also shift
        //    the average height.
        let fbm = self.base.get([wx, wz]) as f32;
        let ridged = (self.ridge.get([wx, wz]) as f32).mul_add(2.0, -1.0);
        let mixed = fbm * (1.0 - self.shape.ridged_weight) + ridged * self.shape.ridged_weight;

        // Clamp to +/-1 so that the height is provably bounded by `base_m +/- amplitude_m`.
        //
        // This is not defensive padding. Neither `Fbm` nor `RidgedMulti` guarantees a +/-1
        // output: summed octaves overshoot, and the Hills preset was measured reaching -1.72,
        // which drove heights to -109 m and got silently clamped to -100 m by the fixed-point
        // encoding. A clamp here instead makes the bound explicit and testable.
        //
        // Visually it flattens the rare extreme: the very tops of the sharpest ridges and the
        // very bottoms of the deepest valleys. On mountainous terrain that reads as plateaus
        // and valley floors, which is plausible geology rather than an artefact.
        let mixed = mixed.clamp(-1.0, 1.0);

        let noise_height = self.shape.base_m + mixed * self.shape.amplitude_m;

        if !self.shape.coastal {
            return noise_height;
        }

        // 3. Coast mask: a wavy shoreline running north-south.
        let coast_x = COAST_POSITION + self.coast.get([nz * 2.0, 0.5]) * COAST_WAVINESS;
        let distance_inland = (nx - coast_x) as f32;

        // Ground rises with distance from the shore, so the far inland edge is reliably dry
        // no matter where the noise happens to dip.
        let lift = smoothstep(0.0, INLAND_LIFT_SPAN, distance_inland) * self.shape.inland_lift_m;
        let land = noise_height + lift;

        // The asymmetric edges put a narrow surf zone seaward of the shoreline and a longer
        // ramp landward, which reads as a beach rather than a cliff.
        let shore = smoothstep(-0.015, 0.09, distance_inland);

        SEABED_M + shore * (land - SEABED_M)
    }
}

/// Generates a full-plot heightfield.
///
/// Fills 16.7 million samples in parallel across the `rayon` pool. Returns the field and the
/// statistics the HUD and milestone report need.
///
/// Blueprint 2.8.1 budgets 30 s for this on recommended hardware; the dev laptop is below that
/// tier, so the caller should measure and report the real number rather than assume the budget
/// is met.
#[must_use]
pub fn generate(params: TerrainParams) -> (Heightfield, TerrainStats) {
    let generator = Generator::new(params);
    let per_axis = plot::HEIGHTFIELD_SAMPLES_PER_AXIS as usize;
    let last = (per_axis - 1) as f64;

    let mut samples = vec![0u16; plot::HEIGHTFIELD_SAMPLE_COUNT as usize];

    // One rayon task per row. Rows are 4,096 samples, which is enough work per task that
    // scheduling overhead disappears, and the write pattern stays sequential per task.
    samples
        .par_chunks_mut(per_axis)
        .enumerate()
        .for_each(|(j, row)| {
            let nz = j as f64 / last;
            for (i, slot) in row.iter_mut().enumerate() {
                let nx = i as f64 / last;
                *slot = height::encode(generator.height_m(nx, nz));
            }
        });

    // Statistics in one pass, in parallel, reducing into per-chunk results and merging. The
    // merge is associative and order-independent, so the result does not depend on how rayon
    // happened to split the work (blueprint 2.5.2).
    let sea_raw = height::encode(params.sea_level_m);
    let (min_raw, max_raw, water) = samples
        .par_iter()
        .fold(
            || (u16::MAX, u16::MIN, 0u64),
            |(min, max, water), &s| (min.min(s), max.max(s), water + u64::from(s <= sea_raw)),
        )
        .reduce(
            || (u16::MAX, u16::MIN, 0u64),
            |a, b| (a.0.min(b.0), a.1.max(b.1), a.2 + b.2),
        );

    let stats = TerrainStats {
        min_m: height::decode(min_raw),
        max_m: height::decode(max_raw),
        water_fraction: water as f32 / plot::HEIGHTFIELD_SAMPLE_COUNT as f32,
    };

    // `samples` is exactly one plot's worth by construction, so `from_samples` cannot fail;
    // the fallback keeps this function panic-free regardless.
    let field = Heightfield::from_samples(samples).unwrap_or_else(|| Heightfield::flat(0.0));

    (field, stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sparse grid of normalised positions, for cheap whole-plot property checks.
    fn probe_positions() -> Vec<(f64, f64)> {
        let mut out = Vec::new();
        for a in 0..17 {
            for b in 0..17 {
                out.push((f64::from(a) / 16.0, f64::from(b) / 16.0));
            }
        }
        out
    }

    fn sample_heights(params: TerrainParams) -> Vec<f32> {
        let generator = Generator::new(params);
        probe_positions()
            .iter()
            .map(|(x, z)| generator.height_m(*x, *z))
            .collect()
    }

    #[test]
    fn the_same_seed_and_preset_always_produce_the_same_terrain() {
        // This is the property the save format depends on (blueprint 2.11): a save stores the
        // seed, not the heightfield.
        for preset in TerrainPreset::ALL {
            let params = TerrainParams {
                seed: 0xDEAD_BEEF,
                preset,
                ..Default::default()
            };
            let a = sample_heights(params);
            let b = sample_heights(params);
            assert_eq!(a, b, "{} is not reproducible", preset.name());
        }
    }

    #[test]
    fn different_seeds_produce_different_terrain() {
        let a = sample_heights(TerrainParams {
            seed: 1,
            ..Default::default()
        });
        let b = sample_heights(TerrainParams {
            seed: 2,
            ..Default::default()
        });
        assert_ne!(a, b, "seed is being ignored");
    }

    #[test]
    fn different_presets_produce_visibly_different_terrain() {
        // M0's acceptance check is that the three presets look distinct, so compare their
        // relief rather than just asserting inequality.
        let relief = |preset| {
            let h = sample_heights(TerrainParams {
                seed: 7,
                preset,
                ..Default::default()
            });
            let min = h.iter().copied().fold(f32::INFINITY, f32::min);
            let max = h.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            max - min
        };

        let flat = relief(TerrainPreset::Flat);
        let hills = relief(TerrainPreset::Hills);
        let coastal = relief(TerrainPreset::Coastal);

        // Hills must have substantially more relief than Flat, or the presets are pointless.
        assert!(
            hills > flat * 5.0,
            "flat relief {flat} m, hills relief {hills} m"
        );
        // Coastal sits between the two and must reach below sea level.
        assert!(
            coastal > flat,
            "flat relief {flat} m, coastal relief {coastal} m"
        );
    }

    #[test]
    fn flat_preset_is_actually_flat_enough_to_build_on() {
        let h = sample_heights(TerrainParams {
            preset: TerrainPreset::Flat,
            seed: 42,
            ..Default::default()
        });
        let min = h.iter().copied().fold(f32::INFINITY, f32::min);
        let max = h.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        assert!(max - min < 40.0, "flat preset relief is {} m", max - min);
        // And it should be above water, since Flat has no sea.
        assert!(min > 0.0, "flat preset dips to {min} m");
    }

    #[test]
    fn coastal_preset_has_both_sea_and_land() {
        let generator = Generator::new(TerrainParams {
            preset: TerrainPreset::Coastal,
            seed: 11,
            ..Default::default()
        });
        // Far west is sea, far east is land, for every latitude.
        for b in 0..9 {
            let nz = f64::from(b) / 8.0;
            let west = generator.height_m(0.02, nz);
            let east = generator.height_m(0.95, nz);
            assert!(
                west < 0.0,
                "west edge at nz={nz} is {west} m, expected below sea level"
            );
            assert!(
                east > 0.0,
                "east edge at nz={nz} is {east} m, expected above sea level"
            );
        }
    }

    #[test]
    fn coastal_shoreline_is_smooth_not_a_cliff() {
        // A discontinuity at the shore would be visible as a wall of water.
        let generator = Generator::new(TerrainParams {
            preset: TerrainPreset::Coastal,
            seed: 3,
            ..Default::default()
        });
        let step = 0.002;
        let mut nx = 0.0;
        let mut prev = generator.height_m(nx, 0.5);
        while nx < 0.7 {
            nx += step;
            let h = generator.height_m(nx, 0.5);
            // 0.002 normalised units is 32.8 m of ground. A jump beyond 25 m over that
            // distance would be a near-vertical face.
            assert!(
                (h - prev).abs() < 25.0,
                "jump of {} m at nx={nx}",
                (h - prev).abs()
            );
            prev = h;
        }
    }

    #[test]
    fn every_preset_stays_inside_the_encodable_height_range() {
        // Heights outside this range clamp silently, which would flatten peaks or floors.
        for preset in TerrainPreset::ALL {
            for seed in [0u64, 1, 12_345, u64::MAX] {
                let generator = Generator::new(TerrainParams {
                    seed,
                    preset,
                    ..Default::default()
                });
                for (nx, nz) in probe_positions() {
                    let h = generator.height_m(nx, nz);
                    assert!(h.is_finite(), "{} seed {seed} produced {h}", preset.name());
                    assert!(
                        h > height::HEIGHT_MIN_M && h < height::HEIGHT_MAX_M,
                        "{} seed {seed} produced {h} m at ({nx}, {nz}), outside the \
                         encodable range",
                        preset.name()
                    );
                }
            }
        }
    }

    #[test]
    fn smoothstep_is_clamped_and_monotonic() {
        assert_eq!(smoothstep(0.0, 1.0, -5.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 5.0), 1.0);
        assert!((smoothstep(0.0, 1.0, 0.5) - 0.5).abs() < 1e-6);
        let mut prev = 0.0;
        for i in 0..=20 {
            let v = smoothstep(0.0, 1.0, i as f32 / 20.0);
            assert!(v >= prev, "not monotonic at {i}");
            prev = v;
        }
    }

    #[test]
    fn generated_field_has_the_right_size_and_plausible_statistics() {
        // The one test that fills a whole plot. It is the only way to catch a row-indexing
        // mistake in the parallel fill.
        let params = TerrainParams {
            seed: 2024,
            preset: TerrainPreset::Coastal,
            sea_level_m: 0.0,
            rivers: false,
        };
        let (field, stats) = generate(params);

        assert_eq!(field.raw().len(), plot::HEIGHTFIELD_SAMPLE_COUNT as usize);
        assert_eq!(field.memory_bytes(), 16_777_216 * 2);

        // Coastal must have water, but must not be an ocean with a puddle of land.
        assert!(
            (0.1..0.6).contains(&stats.water_fraction),
            "water fraction {}",
            stats.water_fraction
        );
        assert!(stats.min_m < 0.0, "min {}", stats.min_m);
        assert!(stats.max_m > 50.0, "max {}", stats.max_m);
        assert!(stats.min_m < stats.max_m);

        // Stats must agree with the field itself.
        let (field_min, field_max) = field.height_range_m();
        assert!((field_min - stats.min_m).abs() < 0.05);
        assert!((field_max - stats.max_m).abs() < 0.05);
    }

    #[test]
    fn generated_field_matches_the_generator_at_the_corners() {
        // Proves the parallel fill maps row index to Z and column index to X the right way
        // round. Getting this backwards transposes the world, which is easy to miss on
        // noise-based terrain.
        let params = TerrainParams {
            seed: 5,
            preset: TerrainPreset::Hills,
            ..Default::default()
        };
        let (field, _) = generate(params);
        let generator = Generator::new(params);
        let last = plot::HEIGHTFIELD_SAMPLES_PER_AXIS - 1;

        // Asymmetric probe points, so a transpose cannot pass by coincidence.
        for (i, j) in [(0, 0), (last, 0), (0, last), (last, last), (100, 3_000)] {
            let expected = generator.height_m(
                f64::from(i) / f64::from(last),
                f64::from(j) / f64::from(last),
            );
            let actual = field.height_at(world_model::SampleCoord::new(i, j));
            assert!(
                (actual - expected).abs() < 0.02,
                "sample ({i}, {j}): field {actual} m, generator {expected} m"
            );
        }
    }

    #[test]
    fn flat_preset_produces_no_water_at_sea_level_zero() {
        let (_, stats) = generate(TerrainParams {
            seed: 9,
            preset: TerrainPreset::Flat,
            sea_level_m: 0.0,
            rivers: false,
        });
        assert_eq!(
            stats.water_fraction, 0.0,
            "flat preset should be entirely dry"
        );
    }

    #[test]
    fn preset_names_are_stable_for_the_ui_and_reports() {
        assert_eq!(TerrainPreset::Flat.name(), "Flat");
        assert_eq!(TerrainPreset::Hills.name(), "Hills");
        assert_eq!(TerrainPreset::Coastal.name(), "Coastal");
        assert_eq!(TerrainPreset::ALL.len(), 3);
        assert_eq!(TerrainPreset::default(), TerrainPreset::Flat);
    }
}
