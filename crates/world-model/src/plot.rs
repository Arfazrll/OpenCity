//! City plot dimensions and conversions between the four coordinate layers.
//!
//! See [ADR 0001](../../../docs/adr/0001-plot-extent-16384-metres.md) for why the plot is
//! 16,384 m rather than a literal 16,000 m: it is the only extent that lets 256 m render
//! chunks, 4 m heightfield samples and 32 m simulation cells all tile it exactly.
//!
//! # Axes
//!
//! The plot is **centred on the world origin**, so X and Z both run from
//! [`PLOT_MIN_M`] to [`PLOT_MAX_M`] (−8,192 m … +8,192 m). Y is up, following Bevy's
//! convention. Centring halves the largest coordinate magnitude and therefore halves `f32`
//! rounding error at the plot corners (about 1.95 mm instead of 3.9 mm).
//!
//! Sample, cell and chunk indices are unsigned and count from the minimum corner, so index 0
//! is at −8,192 m.

use crate::{CellCoord, ChunkCoord, SampleCoord};

/// Edge length of a city plot, in metres. 2^14.
pub const PLOT_EXTENT_M: f32 = 16_384.0;

/// Half the plot extent. The plot spans `-PLOT_HALF_EXTENT_M ..= +PLOT_HALF_EXTENT_M`.
pub const PLOT_HALF_EXTENT_M: f32 = PLOT_EXTENT_M / 2.0;

/// Minimum world coordinate on the X and Z axes.
pub const PLOT_MIN_M: f32 = -PLOT_HALF_EXTENT_M;

/// Maximum world coordinate on the X and Z axes.
pub const PLOT_MAX_M: f32 = PLOT_HALF_EXTENT_M;

/// Spacing between terrain heightfield samples, in metres.
pub const HEIGHTFIELD_RESOLUTION_M: f32 = 4.0;

/// Heightfield samples along one axis: 16,384 / 4.
pub const HEIGHTFIELD_SAMPLES_PER_AXIS: u32 = 4_096;

/// Total heightfield samples in a plot.
pub const HEIGHTFIELD_SAMPLE_COUNT: u64 =
    HEIGHTFIELD_SAMPLES_PER_AXIS as u64 * HEIGHTFIELD_SAMPLES_PER_AXIS as u64;

/// Edge length of a render chunk, in metres.
pub const CHUNK_SIZE_M: f32 = 256.0;

/// Render chunks along one axis: 16,384 / 256.
pub const CHUNKS_PER_AXIS: u32 = 64;

/// Total render chunks in a plot.
pub const CHUNK_COUNT: u32 = CHUNKS_PER_AXIS * CHUNKS_PER_AXIS;

/// Heightfield samples spanned by one chunk along one axis: 256 / 4.
///
/// A chunk mesh at the finest level therefore has
/// `HEIGHTFIELD_SAMPLES_PER_CHUNK_AXIS + 1` = 65 vertices per axis, sharing its edge vertices
/// with its neighbours.
pub const HEIGHTFIELD_SAMPLES_PER_CHUNK_AXIS: u32 = 64;

/// Edge length of a simulation grid cell, in metres (blueprint 2.4).
pub const SIM_CELL_SIZE_M: f32 = 32.0;

/// Simulation cells along one axis: 16,384 / 32.
pub const SIM_CELLS_PER_AXIS: u32 = 512;

/// Total simulation cells per grid layer.
pub const SIM_CELL_COUNT: u32 = SIM_CELLS_PER_AXIS * SIM_CELLS_PER_AXIS;

/// Half-extent of the playable area on the Low graphics preset (blueprint 2.6.8).
///
/// The Low preset caps the active city to the central quarter of the plot, which is what makes
/// a 16 km world tractable on an integrated GPU.
pub const LOW_PRESET_HALF_EXTENT_M: f32 = PLOT_HALF_EXTENT_M / 2.0;

/// Lowest camera altitude for the city camera, in metres (blueprint 2.6.9).
pub const CAMERA_MIN_ALTITUDE_M: f32 = 20.0;

/// Highest camera altitude for the city camera, in metres (decision log A12).
pub const CAMERA_MAX_ALTITUDE_M: f32 = 5_000.0;

// ---------------------------------------------------------------------------
// Compile-time checks.
//
// These are the invariants ADR 0001 exists to guarantee. Breaking one of the constants above
// now fails the build rather than producing terrain seams that take a day to track down.
// ---------------------------------------------------------------------------

const _: () = assert!(
    HEIGHTFIELD_SAMPLES_PER_AXIS as f32 * HEIGHTFIELD_RESOLUTION_M == PLOT_EXTENT_M,
    "heightfield samples must tile the plot exactly"
);
const _: () = assert!(
    CHUNKS_PER_AXIS as f32 * CHUNK_SIZE_M == PLOT_EXTENT_M,
    "render chunks must tile the plot exactly"
);
const _: () = assert!(
    SIM_CELLS_PER_AXIS as f32 * SIM_CELL_SIZE_M == PLOT_EXTENT_M,
    "simulation cells must tile the plot exactly"
);
const _: () = assert!(
    HEIGHTFIELD_SAMPLES_PER_CHUNK_AXIS as f32 * HEIGHTFIELD_RESOLUTION_M == CHUNK_SIZE_M,
    "a chunk must span a whole number of heightfield samples"
);
const _: () = assert!(
    CHUNKS_PER_AXIS.is_power_of_two(),
    "quadtree LOD requires a power-of-two chunk grid"
);
const _: () = assert!(
    HEIGHTFIELD_SAMPLES_PER_CHUNK_AXIS.is_power_of_two(),
    "crack-free LOD requires a power-of-two vertex grid per chunk"
);
const _: () = assert!(
    CHUNK_COUNT == 4_096,
    "blueprint 2.4 states 4,096 chunks per city"
);

// ---------------------------------------------------------------------------
// Containment
// ---------------------------------------------------------------------------

/// Returns whether a world-space XZ position lies inside the plot.
#[inline]
#[must_use]
pub fn contains(x: f32, z: f32) -> bool {
    (PLOT_MIN_M..=PLOT_MAX_M).contains(&x) && (PLOT_MIN_M..=PLOT_MAX_M).contains(&z)
}

/// Clamps a world-space XZ position into the plot.
#[inline]
#[must_use]
pub fn clamp_to_plot(x: f32, z: f32) -> (f32, f32) {
    (
        x.clamp(PLOT_MIN_M, PLOT_MAX_M),
        z.clamp(PLOT_MIN_M, PLOT_MAX_M),
    )
}

/// Returns whether a world-space XZ position lies inside the Low preset's active area.
#[inline]
#[must_use]
pub fn contains_low_preset(x: f32, z: f32) -> bool {
    let r = LOW_PRESET_HALF_EXTENT_M;
    (-r..=r).contains(&x) && (-r..=r).contains(&z)
}

// ---------------------------------------------------------------------------
// World <-> heightfield sample
// ---------------------------------------------------------------------------

/// Converts a world-space XZ position to the nearest heightfield sample index.
///
/// Positions outside the plot clamp to the edge sample, so this never produces an
/// out-of-bounds index.
#[inline]
#[must_use]
pub fn world_to_sample(x: f32, z: f32) -> SampleCoord {
    let last = HEIGHTFIELD_SAMPLES_PER_AXIS - 1;
    SampleCoord {
        i: axis_to_index(x, HEIGHTFIELD_RESOLUTION_M, last),
        j: axis_to_index(z, HEIGHTFIELD_RESOLUTION_M, last),
    }
}

/// Converts a heightfield sample index to the world-space XZ position of that sample.
#[inline]
#[must_use]
pub fn sample_to_world(sample: SampleCoord) -> (f32, f32) {
    (
        PLOT_MIN_M + sample.i as f32 * HEIGHTFIELD_RESOLUTION_M,
        PLOT_MIN_M + sample.j as f32 * HEIGHTFIELD_RESOLUTION_M,
    )
}

// ---------------------------------------------------------------------------
// World <-> simulation cell
// ---------------------------------------------------------------------------

/// Converts a world-space XZ position to the simulation cell containing it.
///
/// Positions outside the plot clamp to the edge cell.
#[inline]
#[must_use]
pub fn world_to_cell(x: f32, z: f32) -> CellCoord {
    let last = SIM_CELLS_PER_AXIS - 1;
    CellCoord {
        i: axis_to_index(x, SIM_CELL_SIZE_M, last),
        j: axis_to_index(z, SIM_CELL_SIZE_M, last),
    }
}

/// Converts a simulation cell index to the world-space XZ position of its centre.
#[inline]
#[must_use]
pub fn cell_to_world_center(cell: CellCoord) -> (f32, f32) {
    let half = SIM_CELL_SIZE_M / 2.0;
    (
        PLOT_MIN_M + cell.i as f32 * SIM_CELL_SIZE_M + half,
        PLOT_MIN_M + cell.j as f32 * SIM_CELL_SIZE_M + half,
    )
}

// ---------------------------------------------------------------------------
// World <-> render chunk
// ---------------------------------------------------------------------------

/// Converts a world-space XZ position to the render chunk containing it.
///
/// Positions outside the plot clamp to the edge chunk.
#[inline]
#[must_use]
pub fn world_to_chunk(x: f32, z: f32) -> ChunkCoord {
    let last = CHUNKS_PER_AXIS - 1;
    ChunkCoord {
        x: axis_to_index(x, CHUNK_SIZE_M, last),
        z: axis_to_index(z, CHUNK_SIZE_M, last),
    }
}

/// Converts a chunk coordinate to the world-space XZ position of its minimum corner.
#[inline]
#[must_use]
pub fn chunk_to_world_min(chunk: ChunkCoord) -> (f32, f32) {
    (
        PLOT_MIN_M + chunk.x as f32 * CHUNK_SIZE_M,
        PLOT_MIN_M + chunk.z as f32 * CHUNK_SIZE_M,
    )
}

/// Converts a chunk coordinate to the world-space XZ position of its centre.
#[inline]
#[must_use]
pub fn chunk_to_world_center(chunk: ChunkCoord) -> (f32, f32) {
    let (x, z) = chunk_to_world_min(chunk);
    let half = CHUNK_SIZE_M / 2.0;
    (x + half, z + half)
}

/// Returns the heightfield sample index of a chunk's minimum corner.
///
/// Chunk boundaries always land exactly on a sample because a chunk spans
/// [`HEIGHTFIELD_SAMPLES_PER_CHUNK_AXIS`] samples (ADR 0001).
#[inline]
#[must_use]
pub fn chunk_to_sample_min(chunk: ChunkCoord) -> SampleCoord {
    SampleCoord {
        i: chunk.x * HEIGHTFIELD_SAMPLES_PER_CHUNK_AXIS,
        j: chunk.z * HEIGHTFIELD_SAMPLES_PER_CHUNK_AXIS,
    }
}

// ---------------------------------------------------------------------------
// Shared helper
// ---------------------------------------------------------------------------

/// Maps a world-space axis value onto a clamped grid index of the given cell size.
///
/// `floor` rather than `round`: an index identifies the cell a position falls *inside*, which
/// is what every grid layer wants. Sample lookup also uses this, treating a sample as owning
/// the cell that starts at it; terrain interpolation between samples is the renderer's job,
/// not this function's.
#[inline]
fn axis_to_index(value: f32, cell_size_m: f32, last_index: u32) -> u32 {
    let offset = (value - PLOT_MIN_M) / cell_size_m;
    if offset <= 0.0 {
        0
    } else {
        // `offset` is positive here, so the cast truncates towards zero, i.e. floors.
        (offset as u32).min(last_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plot_is_tiled_exactly_by_every_layer() {
        // The compile-time asserts already cover this, but an explicit test documents the
        // relationships for a reader and fails with a readable message.
        assert_eq!(
            HEIGHTFIELD_SAMPLES_PER_AXIS as f32 * HEIGHTFIELD_RESOLUTION_M,
            PLOT_EXTENT_M
        );
        assert_eq!(CHUNKS_PER_AXIS as f32 * CHUNK_SIZE_M, PLOT_EXTENT_M);
        assert_eq!(SIM_CELLS_PER_AXIS as f32 * SIM_CELL_SIZE_M, PLOT_EXTENT_M);
        assert_eq!(CHUNK_COUNT, 4_096);
        assert_eq!(SIM_CELL_COUNT, 262_144);
        assert_eq!(HEIGHTFIELD_SAMPLE_COUNT, 16_777_216);
    }

    #[test]
    fn heightfield_memory_matches_the_blueprint_estimate() {
        // Blueprint 2.4 says "about 32 MB per city" for u16 samples.
        let bytes = HEIGHTFIELD_SAMPLE_COUNT * 2;
        let mib = bytes as f64 / (1024.0 * 1024.0);
        assert!((32.0..34.0).contains(&mib), "heightfield is {mib} MiB");
    }

    #[test]
    fn sim_layer_memory_matches_the_blueprint_estimate() {
        // Blueprint 2.4 says "f32 = 1 MB/layer".
        let bytes = u64::from(SIM_CELL_COUNT) * 4;
        let mib = bytes as f64 / (1024.0 * 1024.0);
        assert!((1.0..1.1).contains(&mib), "sim layer is {mib} MiB");
    }

    #[test]
    fn corner_positions_are_inside_the_plot() {
        assert!(contains(PLOT_MIN_M, PLOT_MIN_M));
        assert!(contains(PLOT_MAX_M, PLOT_MAX_M));
        assert!(contains(0.0, 0.0));
        assert!(!contains(PLOT_MIN_M - 1.0, 0.0));
        assert!(!contains(0.0, PLOT_MAX_M + 1.0));
    }

    #[test]
    fn low_preset_covers_the_central_quarter() {
        assert!(contains_low_preset(0.0, 0.0));
        assert!(contains_low_preset(4_000.0, -4_000.0));
        assert!(!contains_low_preset(5_000.0, 0.0));
        // Its area is a quarter of the full plot.
        let full = PLOT_EXTENT_M * PLOT_EXTENT_M;
        let low = (LOW_PRESET_HALF_EXTENT_M * 2.0) * (LOW_PRESET_HALF_EXTENT_M * 2.0);
        assert_eq!(low * 4.0, full);
    }

    #[test]
    fn sample_round_trips_from_its_own_world_position() {
        for i in [0_u32, 1, 1_000, 2_048, 4_095] {
            for j in [0_u32, 7, 2_048, 4_095] {
                let coord = SampleCoord { i, j };
                let (x, z) = sample_to_world(coord);
                assert_eq!(
                    world_to_sample(x, z),
                    coord,
                    "sample ({i}, {j}) at ({x}, {z})"
                );
            }
        }
    }

    #[test]
    fn cell_centre_round_trips() {
        for i in [0_u32, 1, 255, 511] {
            for j in [0_u32, 100, 511] {
                let coord = CellCoord { i, j };
                let (x, z) = cell_to_world_center(coord);
                assert_eq!(world_to_cell(x, z), coord, "cell ({i}, {j}) at ({x}, {z})");
            }
        }
    }

    #[test]
    fn chunk_centre_round_trips() {
        for x in [0_u32, 1, 32, 63] {
            for z in [0_u32, 17, 63] {
                let coord = ChunkCoord { x, z };
                let (wx, wz) = chunk_to_world_center(coord);
                assert_eq!(world_to_chunk(wx, wz), coord, "chunk ({x}, {z})");
            }
        }
    }

    #[test]
    fn chunk_minimum_corner_lands_on_a_sample() {
        for x in [0_u32, 1, 63] {
            let chunk = ChunkCoord { x, z: x };
            let (wx, wz) = chunk_to_world_min(chunk);
            let expected = chunk_to_sample_min(chunk);
            assert_eq!(world_to_sample(wx, wz), expected);
            // And the sample's own world position is identical, i.e. no rounding drift.
            assert_eq!(sample_to_world(expected), (wx, wz));
        }
    }

    #[test]
    fn positions_outside_the_plot_clamp_instead_of_overflowing() {
        let far = 1.0e9_f32;
        assert_eq!(world_to_sample(-far, -far), SampleCoord { i: 0, j: 0 });
        assert_eq!(
            world_to_sample(far, far),
            SampleCoord {
                i: HEIGHTFIELD_SAMPLES_PER_AXIS - 1,
                j: HEIGHTFIELD_SAMPLES_PER_AXIS - 1
            }
        );
        assert_eq!(world_to_chunk(-far, far).x, 0);
        assert_eq!(world_to_chunk(-far, far).z, CHUNKS_PER_AXIS - 1);
        assert_eq!(world_to_cell(far, -far).i, SIM_CELLS_PER_AXIS - 1);
        assert_eq!(world_to_cell(far, -far).j, 0);
    }

    #[test]
    fn clamping_a_position_keeps_it_inside() {
        let (x, z) = clamp_to_plot(1.0e9, -1.0e9);
        assert!(contains(x, z));
        assert_eq!((x, z), (PLOT_MAX_M, PLOT_MIN_M));
    }

    #[test]
    fn f32_resolution_at_the_plot_edge_is_about_one_millimetre() {
        // Blueprint 2.4 estimates "1-2 mm" of f32 error near 16 km. That figure describes an
        // un-centred 0..16,384 m plot. Centring on the origin caps |coordinate| at 8,192 m,
        // which is 2^13, so the ulp is 2^13 * 2^-23 = 2^-10 = 0.98 mm - half the estimate.
        //
        // This is not trivia: it is why the city camera needs no floating origin now, and why
        // first person will need one in M9, where sub-millimetre jitter at eye level shows.
        let edge = PLOT_HALF_EXTENT_M;
        let ulp = f32::from_bits(edge.to_bits() + 1) - edge;
        assert_eq!(ulp, 2.0_f32.powi(-10), "ulp at {edge} m is {ulp} m");
        assert!(
            (0.0005..0.0015).contains(&ulp),
            "ulp at {edge} m is {ulp} m"
        );

        // Confirm that centring is what buys this: an un-centred plot would reach 16,384 m,
        // doubling the ulp to the blueprint's 1.95 mm.
        let uncentred_ulp = f32::from_bits(PLOT_EXTENT_M.to_bits() + 1) - PLOT_EXTENT_M;
        assert_eq!(uncentred_ulp, ulp * 2.0);
    }
}
