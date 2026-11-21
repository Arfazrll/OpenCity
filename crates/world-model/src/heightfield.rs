//! Terrain elevation storage.
//!
//! # Design
//!
//! One flat `Vec<u16>` of 4,096 × 4,096 fixed-point samples, row-major along Z (see
//! [`crate::height`] for the encoding). That is 33.6 MB, and it is the largest single
//! allocation in the game.
//!
//! It lives in `world-model` rather than in `procgen` because it is world *state*, not a
//! generator artefact: `procgen` writes it once, terraforming edits it, the renderer meshes
//! it, and the simulation reads it for buildability. Putting it here keeps all four honest
//! about using the same conversions.
//!
//! # Why not a quadtree or a sparse structure
//!
//! Terrain is dense by nature — every sample has a height — so a flat array is both smaller
//! and faster than any tree. Sparsity only appears in *edits*, which the save format stores as
//! a delta against the seed-generated base (blueprint 2.11) rather than as a sparse
//! heightfield.

use crate::{SampleCoord, height, plot};

/// A full-plot terrain heightfield.
#[derive(Clone)]
pub struct Heightfield {
    /// Row-major fixed-point samples, `HEIGHTFIELD_SAMPLES_PER_AXIS` squared.
    samples: Vec<u16>,
}

impl Heightfield {
    /// Creates a heightfield with every sample at the given height.
    ///
    /// Allocates 33.6 MB.
    #[must_use]
    pub fn flat(metres: f32) -> Self {
        Self {
            samples: vec![height::encode(metres); plot::HEIGHTFIELD_SAMPLE_COUNT as usize],
        }
    }

    /// Wraps an existing sample buffer.
    ///
    /// Returns `None` unless the buffer is exactly one plot's worth of samples, so a generator
    /// that produces the wrong size fails loudly instead of rendering a torn landscape.
    #[must_use]
    pub fn from_samples(samples: Vec<u16>) -> Option<Self> {
        if samples.len() == plot::HEIGHTFIELD_SAMPLE_COUNT as usize {
            Some(Self { samples })
        } else {
            None
        }
    }

    /// The raw sample buffer, for meshing and serialisation.
    #[inline]
    #[must_use]
    pub fn raw(&self) -> &[u16] {
        &self.samples
    }

    /// The raw sample buffer, for terraforming.
    #[inline]
    pub fn raw_mut(&mut self) -> &mut [u16] {
        &mut self.samples
    }

    /// The fixed-point sample at a coordinate, clamped to the plot edge.
    #[inline]
    #[must_use]
    pub fn raw_at(&self, coord: SampleCoord) -> u16 {
        let last = plot::HEIGHTFIELD_SAMPLES_PER_AXIS - 1;
        let clamped = SampleCoord {
            i: coord.i.min(last),
            j: coord.j.min(last),
        };
        // The clamp above guarantees a valid index, so the fallback is unreachable; it exists
        // because sim code may not panic (blueprint 3.9).
        clamped
            .to_index()
            .and_then(|index| self.samples.get(index).copied())
            .unwrap_or(0)
    }

    /// The height in metres at a sample coordinate, clamped to the plot edge.
    #[inline]
    #[must_use]
    pub fn height_at(&self, coord: SampleCoord) -> f32 {
        height::decode(self.raw_at(coord))
    }

    /// Sets the height at a sample coordinate. Out-of-range coordinates are ignored.
    #[inline]
    pub fn set_height_at(&mut self, coord: SampleCoord, metres: f32) {
        if let Some(index) = coord.to_index()
            && let Some(slot) = self.samples.get_mut(index)
        {
            *slot = height::encode(metres);
        }
    }

    /// The height in metres at an arbitrary world-space XZ position, bilinearly interpolated.
    ///
    /// Interpolation matters: samples are 4 m apart, so snapping to the nearest one would make
    /// roads and buildings sit up to 2 m off the visible surface on a slope.
    #[must_use]
    pub fn height_at_world(&self, x: f32, z: f32) -> f32 {
        let last = plot::HEIGHTFIELD_SAMPLES_PER_AXIS - 1;

        // Position in sample space, clamped so the +1 neighbours below stay in range.
        let fx = ((x - plot::PLOT_MIN_M) / plot::HEIGHTFIELD_RESOLUTION_M).clamp(0.0, last as f32);
        let fz = ((z - plot::PLOT_MIN_M) / plot::HEIGHTFIELD_RESOLUTION_M).clamp(0.0, last as f32);

        let i0 = fx.floor() as u32;
        let j0 = fz.floor() as u32;
        let i1 = (i0 + 1).min(last);
        let j1 = (j0 + 1).min(last);
        let tx = fx - i0 as f32;
        let tz = fz - j0 as f32;

        let h00 = self.height_at(SampleCoord { i: i0, j: j0 });
        let h10 = self.height_at(SampleCoord { i: i1, j: j0 });
        let h01 = self.height_at(SampleCoord { i: i0, j: j1 });
        let h11 = self.height_at(SampleCoord { i: i1, j: j1 });

        let top = h00 + (h10 - h00) * tx;
        let bottom = h01 + (h11 - h01) * tx;
        top + (bottom - top) * tz
    }

    /// The lowest and highest heights in the field, in metres.
    ///
    /// Used by the milestone report and by the renderer to size its height-based material
    /// blending.
    #[must_use]
    pub fn height_range_m(&self) -> (f32, f32) {
        let min = self.samples.iter().copied().min().unwrap_or(0);
        let max = self.samples.iter().copied().max().unwrap_or(0);
        (height::decode(min), height::decode(max))
    }

    /// Memory held by the sample buffer, in bytes.
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        self.samples.len() * size_of::<u16>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flat_field_is_flat_everywhere() {
        let field = Heightfield::flat(42.0);
        assert!((field.height_at(SampleCoord::new(0, 0)) - 42.0).abs() < 0.01);
        assert!((field.height_at(SampleCoord::new(4_095, 4_095)) - 42.0).abs() < 0.01);
        assert!((field.height_at_world(0.0, 0.0) - 42.0).abs() < 0.01);
        assert!((field.height_at_world(-1_234.5, 6_789.0) - 42.0).abs() < 0.01);
        let (min, max) = field.height_range_m();
        assert!((min - 42.0).abs() < 0.01 && (max - 42.0).abs() < 0.01);
    }

    #[test]
    fn memory_is_the_expected_thirty_four_megabytes() {
        let field = Heightfield::flat(0.0);
        assert_eq!(field.memory_bytes(), 16_777_216 * 2);
        let mib = field.memory_bytes() as f64 / (1024.0 * 1024.0);
        assert!((32.0..34.0).contains(&mib), "{mib} MiB");
    }

    #[test]
    fn from_samples_rejects_a_wrongly_sized_buffer() {
        assert!(Heightfield::from_samples(vec![0; 10]).is_none());
        assert!(Heightfield::from_samples(vec![]).is_none());
        let right = vec![0; plot::HEIGHTFIELD_SAMPLE_COUNT as usize];
        assert!(Heightfield::from_samples(right).is_some());
    }

    #[test]
    fn setting_and_reading_a_sample_round_trips() {
        let mut field = Heightfield::flat(0.0);
        field.set_height_at(SampleCoord::new(100, 200), 250.0);
        assert!((field.height_at(SampleCoord::new(100, 200)) - 250.0).abs() < 0.01);
        // Neighbours untouched.
        assert!((field.height_at(SampleCoord::new(101, 200))).abs() < 0.01);
    }

    #[test]
    fn out_of_range_writes_are_ignored_rather_than_panicking() {
        let mut field = Heightfield::flat(5.0);
        field.set_height_at(SampleCoord::new(99_999, 0), 900.0);
        let (min, max) = field.height_range_m();
        assert!((min - 5.0).abs() < 0.01 && (max - 5.0).abs() < 0.01);
    }

    #[test]
    fn out_of_range_reads_clamp_to_the_edge() {
        let mut field = Heightfield::flat(0.0);
        field.set_height_at(SampleCoord::new(4_095, 4_095), 700.0);
        // Reading past the edge returns the edge sample, not zero or a panic.
        assert!((field.height_at(SampleCoord::new(99_999, 99_999)) - 700.0).abs() < 0.01);
        assert!((field.height_at_world(1.0e9, 1.0e9) - 700.0).abs() < 0.01);
    }

    #[test]
    fn world_lookup_interpolates_between_samples() {
        let mut field = Heightfield::flat(0.0);
        // Two adjacent samples 4 m apart: 0 m and 100 m.
        let a = SampleCoord::new(2_000, 2_000);
        let b = SampleCoord::new(2_001, 2_000);
        field.set_height_at(a, 0.0);
        field.set_height_at(b, 100.0);

        let (ax, az) = plot::sample_to_world(a);
        let (bx, _) = plot::sample_to_world(b);

        // At the samples themselves.
        assert!((field.height_at_world(ax, az) - 0.0).abs() < 0.05, "at a");
        assert!((field.height_at_world(bx, az) - 100.0).abs() < 0.05, "at b");
        // Halfway between must be halfway up, not snapped to either end.
        let mid = field.height_at_world((ax + bx) / 2.0, az);
        assert!((mid - 50.0).abs() < 1.0, "midpoint was {mid}");
        // A quarter of the way along.
        let quarter = field.height_at_world(ax + 1.0, az);
        assert!((quarter - 25.0).abs() < 1.0, "quarter point was {quarter}");
    }

    #[test]
    fn interpolation_is_continuous_across_a_sample_boundary() {
        // A discontinuity here would show up as a visible crease in the terrain mesh.
        let mut field = Heightfield::flat(0.0);
        for i in 2_000..2_004 {
            field.set_height_at(SampleCoord::new(i, 2_000), (i - 2_000) as f32 * 10.0);
            field.set_height_at(SampleCoord::new(i, 2_001), (i - 2_000) as f32 * 10.0);
        }
        let (x0, z0) = plot::sample_to_world(SampleCoord::new(2_000, 2_000));
        let mut prev = field.height_at_world(x0, z0);
        let mut step = 0.25_f32;
        while step < 12.0 {
            let h = field.height_at_world(x0 + step, z0);
            assert!(
                (h - prev).abs() < 1.0,
                "jump of {} at +{step} m",
                (h - prev).abs()
            );
            prev = h;
            step += 0.25;
        }
    }

    #[test]
    fn height_range_reports_the_extremes() {
        let mut field = Heightfield::flat(10.0);
        field.set_height_at(SampleCoord::new(0, 0), -50.0);
        field.set_height_at(SampleCoord::new(1, 1), 800.0);
        let (min, max) = field.height_range_m();
        assert!((min + 50.0).abs() < 0.01, "min {min}");
        assert!((max - 800.0).abs() < 0.01, "max {max}");
    }
}
