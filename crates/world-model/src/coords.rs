//! Index types for the three discrete coordinate layers.
//!
//! These are separate types rather than bare `(u32, u32)` tuples because they are not
//! interchangeable: a sample index is 4 m apart, a cell index 32 m, a chunk index 256 m.
//! Passing one where another is expected is a class of bug the compiler can catch for free.

use crate::plot;

/// Index of a terrain heightfield sample, 4 m apart.
///
/// `i` runs along X, `j` along Z. Both count from the plot's minimum corner, so
/// `(0, 0)` is at −8,192 m, −8,192 m.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct SampleCoord {
    /// Index along the X axis, in `0..HEIGHTFIELD_SAMPLES_PER_AXIS`.
    pub i: u32,
    /// Index along the Z axis, in `0..HEIGHTFIELD_SAMPLES_PER_AXIS`.
    pub j: u32,
}

impl SampleCoord {
    /// Creates a sample coordinate without checking its range.
    #[inline]
    #[must_use]
    pub const fn new(i: u32, j: u32) -> Self {
        Self { i, j }
    }

    /// Returns whether both indices are within the plot.
    #[inline]
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.i < plot::HEIGHTFIELD_SAMPLES_PER_AXIS && self.j < plot::HEIGHTFIELD_SAMPLES_PER_AXIS
    }

    /// Flattens to a row-major offset into a full-plot heightfield buffer.
    ///
    /// Returns `None` if the coordinate is out of range, so a bad index cannot silently read
    /// a neighbouring row.
    #[inline]
    #[must_use]
    pub const fn to_index(self) -> Option<usize> {
        if self.is_valid() {
            Some(self.j as usize * plot::HEIGHTFIELD_SAMPLES_PER_AXIS as usize + self.i as usize)
        } else {
            None
        }
    }
}

/// Index of a simulation grid cell, 32 m apart (blueprint 2.4).
///
/// Used by every grid layer: pollution, land value, service coverage, happiness, traffic
/// density.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct CellCoord {
    /// Index along the X axis, in `0..SIM_CELLS_PER_AXIS`.
    pub i: u32,
    /// Index along the Z axis, in `0..SIM_CELLS_PER_AXIS`.
    pub j: u32,
}

impl CellCoord {
    /// Creates a cell coordinate without checking its range.
    #[inline]
    #[must_use]
    pub const fn new(i: u32, j: u32) -> Self {
        Self { i, j }
    }

    /// Returns whether both indices are within the plot.
    #[inline]
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.i < plot::SIM_CELLS_PER_AXIS && self.j < plot::SIM_CELLS_PER_AXIS
    }

    /// Flattens to a row-major offset into a grid layer buffer.
    #[inline]
    #[must_use]
    pub const fn to_index(self) -> Option<usize> {
        if self.is_valid() {
            Some(self.j as usize * plot::SIM_CELLS_PER_AXIS as usize + self.i as usize)
        } else {
            None
        }
    }
}

/// Index of a render chunk, 256 m apart.
///
/// This is the finest quadtree level; see [`crate::chunk::QuadNode`] for coarser levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct ChunkCoord {
    /// Index along the X axis, in `0..CHUNKS_PER_AXIS`.
    pub x: u32,
    /// Index along the Z axis, in `0..CHUNKS_PER_AXIS`.
    pub z: u32,
}

impl ChunkCoord {
    /// Creates a chunk coordinate without checking its range.
    #[inline]
    #[must_use]
    pub const fn new(x: u32, z: u32) -> Self {
        Self { x, z }
    }

    /// Returns whether both indices are within the plot.
    #[inline]
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.x < plot::CHUNKS_PER_AXIS && self.z < plot::CHUNKS_PER_AXIS
    }

    /// Flattens to a row-major offset into a per-chunk array.
    #[inline]
    #[must_use]
    pub const fn to_index(self) -> Option<usize> {
        if self.is_valid() {
            Some(self.z as usize * plot::CHUNKS_PER_AXIS as usize + self.x as usize)
        } else {
            None
        }
    }

    /// Chebyshev (chessboard) distance to another chunk, in chunks.
    ///
    /// This is the right metric for streaming radii: it makes the load region a square, which
    /// matches how a camera frustum covers a chunk grid better than a circle does.
    #[inline]
    #[must_use]
    pub const fn chebyshev_distance(self, other: Self) -> u32 {
        let dx = self.x.abs_diff(other.x);
        let dz = self.z.abs_diff(other.z);
        if dx > dz { dx } else { dz }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_coordinates_flatten_and_invalid_ones_do_not() {
        assert_eq!(SampleCoord::new(0, 0).to_index(), Some(0));
        assert_eq!(SampleCoord::new(1, 0).to_index(), Some(1));
        assert_eq!(
            SampleCoord::new(0, 1).to_index(),
            Some(plot::HEIGHTFIELD_SAMPLES_PER_AXIS as usize)
        );
        assert_eq!(
            SampleCoord::new(plot::HEIGHTFIELD_SAMPLES_PER_AXIS, 0).to_index(),
            None
        );

        assert_eq!(CellCoord::new(0, 0).to_index(), Some(0));
        assert_eq!(
            CellCoord::new(0, 1).to_index(),
            Some(plot::SIM_CELLS_PER_AXIS as usize)
        );
        assert_eq!(CellCoord::new(0, plot::SIM_CELLS_PER_AXIS).to_index(), None);

        assert_eq!(ChunkCoord::new(0, 0).to_index(), Some(0));
        assert_eq!(ChunkCoord::new(63, 63).to_index(), Some(4_095));
        assert_eq!(ChunkCoord::new(64, 0).to_index(), None);
    }

    #[test]
    fn last_valid_index_is_the_last_buffer_slot() {
        let last = SampleCoord::new(
            plot::HEIGHTFIELD_SAMPLES_PER_AXIS - 1,
            plot::HEIGHTFIELD_SAMPLES_PER_AXIS - 1,
        );
        assert_eq!(
            last.to_index(),
            Some(plot::HEIGHTFIELD_SAMPLE_COUNT as usize - 1)
        );

        let last_cell = CellCoord::new(plot::SIM_CELLS_PER_AXIS - 1, plot::SIM_CELLS_PER_AXIS - 1);
        assert_eq!(
            last_cell.to_index(),
            Some(plot::SIM_CELL_COUNT as usize - 1)
        );
    }

    #[test]
    fn chebyshev_distance_is_symmetric_and_square() {
        let a = ChunkCoord::new(10, 10);
        assert_eq!(a.chebyshev_distance(ChunkCoord::new(10, 10)), 0);
        assert_eq!(a.chebyshev_distance(ChunkCoord::new(13, 10)), 3);
        assert_eq!(a.chebyshev_distance(ChunkCoord::new(10, 14)), 4);
        // Diagonal neighbours are distance 1, which is what makes the region a square.
        assert_eq!(a.chebyshev_distance(ChunkCoord::new(11, 11)), 1);
        let b = ChunkCoord::new(2, 40);
        assert_eq!(a.chebyshev_distance(b), b.chebyshev_distance(a));
    }

    #[test]
    fn coordinate_types_order_consistently_for_deterministic_iteration() {
        // Blueprint 2.5.2 forbids iteration over random-state maps. Ord lets these be used as
        // BTreeMap keys instead, so the derive must actually be present and sane.
        let mut v = vec![
            ChunkCoord::new(1, 0),
            ChunkCoord::new(0, 1),
            ChunkCoord::new(0, 0),
        ];
        v.sort();
        assert_eq!(
            v,
            vec![
                ChunkCoord::new(0, 0),
                ChunkCoord::new(0, 1),
                ChunkCoord::new(1, 0)
            ]
        );
    }
}
