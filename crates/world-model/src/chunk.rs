//! Quadtree addressing for terrain level of detail.
//!
//! The terrain is a quadtree over the 64 × 64 grid of 256 m render chunks. Level 0 is one
//! chunk; each coarser level merges 2 × 2 nodes, up to level 6 which is the whole plot as a
//! single node. Because 64 is 2^6 this is exact at every level, with no partial nodes — which
//! is the whole reason the plot is 16,384 m (ADR 0001).
//!
//! ```text
//! level   node size   nodes per axis   nodes
//!   0        256 m          64         4096
//!   1        512 m          32         1024
//!   2      1,024 m          16          256
//!   3      2,048 m           8           64
//!   4      4,096 m           4           16
//!   5      8,192 m           2            4
//!   6     16,384 m           1            1
//! ```
//!
//! A node's vertex grid halves with each coarser level, so a level-1 node renders the same
//! 65 × 65 vertices as a level-0 node but spanning twice the ground — this is what makes the
//! LOD affordable, and what makes neighbouring nodes of different levels disagree along their
//! shared edge. Closing that gap is the mesher's job (CDLOD morphing or skirts); this module
//! only provides the addressing and the neighbour queries it needs.

use crate::{ChunkCoord, plot};

/// Number of quadtree levels, 0 (finest) through 6 (the whole plot).
pub const QUADTREE_LEVELS: u8 = 7;

/// The coarsest level, a single node covering the entire plot.
pub const ROOT_LEVEL: u8 = QUADTREE_LEVELS - 1;

const _: () = assert!(
    plot::CHUNKS_PER_AXIS == 1 << ROOT_LEVEL,
    "quadtree depth must match the chunk grid size"
);

/// A node in the terrain quadtree.
///
/// `level` 0 corresponds one-to-one with a [`ChunkCoord`]; coarser levels cover
/// `2^level × 2^level` chunks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct QuadNode {
    /// Quadtree level, 0 (finest, 256 m) through [`ROOT_LEVEL`] (the whole plot).
    pub level: u8,
    /// Node index along X at this level, in `0..nodes_per_axis(level)`.
    pub x: u32,
    /// Node index along Z at this level, in `0..nodes_per_axis(level)`.
    pub z: u32,
}

/// Number of nodes along one axis at the given level.
///
/// Returns 0 for a level beyond [`ROOT_LEVEL`], so callers that forget to bound-check get an
/// empty range rather than a panic or a wrapped shift.
#[inline]
#[must_use]
pub const fn nodes_per_axis(level: u8) -> u32 {
    if level > ROOT_LEVEL {
        0
    } else {
        plot::CHUNKS_PER_AXIS >> level
    }
}

/// Edge length of a node at the given level, in metres.
#[inline]
#[must_use]
pub fn node_size_m(level: u8) -> f32 {
    plot::CHUNK_SIZE_M * (1u32 << level.min(ROOT_LEVEL)) as f32
}

impl QuadNode {
    /// The single node covering the whole plot.
    pub const ROOT: Self = Self {
        level: ROOT_LEVEL,
        x: 0,
        z: 0,
    };

    /// Creates a node without checking its range.
    #[inline]
    #[must_use]
    pub const fn new(level: u8, x: u32, z: u32) -> Self {
        Self { level, x, z }
    }

    /// Returns whether the level and both indices are in range.
    #[inline]
    #[must_use]
    pub const fn is_valid(self) -> bool {
        let per_axis = nodes_per_axis(self.level);
        per_axis > 0 && self.x < per_axis && self.z < per_axis
    }

    /// Edge length of this node, in metres.
    #[inline]
    #[must_use]
    pub fn size_m(self) -> f32 {
        node_size_m(self.level)
    }

    /// World-space XZ position of this node's minimum corner.
    #[inline]
    #[must_use]
    pub fn world_min(self) -> (f32, f32) {
        let size = self.size_m();
        (
            plot::PLOT_MIN_M + self.x as f32 * size,
            plot::PLOT_MIN_M + self.z as f32 * size,
        )
    }

    /// World-space XZ position of this node's centre.
    #[inline]
    #[must_use]
    pub fn world_center(self) -> (f32, f32) {
        let (x, z) = self.world_min();
        let half = self.size_m() / 2.0;
        (x + half, z + half)
    }

    /// The coarser node containing this one, or `None` at the root.
    #[inline]
    #[must_use]
    pub const fn parent(self) -> Option<Self> {
        if self.level >= ROOT_LEVEL {
            None
        } else {
            Some(Self {
                level: self.level + 1,
                x: self.x / 2,
                z: self.z / 2,
            })
        }
    }

    /// The four finer nodes this one contains, or `None` at level 0.
    ///
    /// Ordered `(−x,−z), (+x,−z), (−x,+z), (+x,+z)`, i.e. row-major, so that iteration order
    /// is deterministic (blueprint 2.5.2).
    #[inline]
    #[must_use]
    pub const fn children(self) -> Option<[Self; 4]> {
        if self.level == 0 {
            None
        } else {
            let level = self.level - 1;
            let (x, z) = (self.x * 2, self.z * 2);
            Some([
                Self { level, x, z },
                Self { level, x: x + 1, z },
                Self { level, x, z: z + 1 },
                Self {
                    level,
                    x: x + 1,
                    z: z + 1,
                },
            ])
        }
    }

    /// The render chunk this node is, if it is a level-0 node.
    #[inline]
    #[must_use]
    pub const fn as_chunk(self) -> Option<ChunkCoord> {
        if self.level == 0 {
            Some(ChunkCoord {
                x: self.x,
                z: self.z,
            })
        } else {
            None
        }
    }

    /// The inclusive range of render chunks this node covers, as
    /// `(min_x, min_z, max_x, max_z)`.
    #[inline]
    #[must_use]
    pub const fn chunk_range(self) -> (u32, u32, u32, u32) {
        let span = 1u32 << self.level;
        let min_x = self.x * span;
        let min_z = self.z * span;
        (min_x, min_z, min_x + span - 1, min_z + span - 1)
    }

    /// Returns whether this node covers the given render chunk.
    #[inline]
    #[must_use]
    pub const fn contains_chunk(self, chunk: ChunkCoord) -> bool {
        let (min_x, min_z, max_x, max_z) = self.chunk_range();
        chunk.x >= min_x && chunk.x <= max_x && chunk.z >= min_z && chunk.z <= max_z
    }

    /// The node at `level` containing the given world-space XZ position.
    ///
    /// Positions outside the plot clamp to the edge node.
    #[inline]
    #[must_use]
    pub fn from_world(level: u8, x: f32, z: f32) -> Self {
        let level = level.min(ROOT_LEVEL);
        let chunk = plot::world_to_chunk(x, z);
        Self {
            level,
            x: chunk.x >> level,
            z: chunk.z >> level,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_table_matches_the_documented_values() {
        let expected = [
            (0_u8, 256.0_f32, 64_u32),
            (1, 512.0, 32),
            (2, 1_024.0, 16),
            (3, 2_048.0, 8),
            (4, 4_096.0, 4),
            (5, 8_192.0, 2),
            (6, 16_384.0, 1),
        ];
        for (level, size, per_axis) in expected {
            assert_eq!(node_size_m(level), size, "size at level {level}");
            assert_eq!(nodes_per_axis(level), per_axis, "count at level {level}");
            // Every level tiles the plot exactly: this is ADR 0001's whole point.
            assert_eq!(
                per_axis as f32 * size,
                plot::PLOT_EXTENT_M,
                "tiling at level {level}"
            );
        }
    }

    #[test]
    fn root_covers_the_whole_plot_as_one_node() {
        assert!(QuadNode::ROOT.is_valid());
        assert_eq!(QuadNode::ROOT.size_m(), plot::PLOT_EXTENT_M);
        assert_eq!(
            QuadNode::ROOT.world_min(),
            (plot::PLOT_MIN_M, plot::PLOT_MIN_M)
        );
        assert_eq!(QuadNode::ROOT.world_center(), (0.0, 0.0));
        assert_eq!(QuadNode::ROOT.parent(), None);
        assert_eq!(QuadNode::ROOT.chunk_range(), (0, 0, 63, 63));
    }

    #[test]
    fn level_zero_nodes_are_chunks() {
        let node = QuadNode::new(0, 17, 42);
        assert_eq!(node.as_chunk(), Some(ChunkCoord::new(17, 42)));
        assert_eq!(node.children(), None);
        assert_eq!(node.size_m(), plot::CHUNK_SIZE_M);
        // And its world position agrees with the plot module's own conversion.
        assert_eq!(
            node.world_min(),
            plot::chunk_to_world_min(ChunkCoord::new(17, 42))
        );
        assert_eq!(QuadNode::new(1, 0, 0).as_chunk(), None);
    }

    #[test]
    fn children_and_parent_are_inverses_everywhere() {
        for level in 1..=ROOT_LEVEL {
            let per_axis = nodes_per_axis(level);
            for x in [0, per_axis / 2, per_axis - 1] {
                for z in [0, per_axis - 1] {
                    let node = QuadNode::new(level, x, z);
                    let children = node.children().expect("level >= 1 has children");
                    for child in children {
                        assert!(child.is_valid(), "{child:?} from {node:?}");
                        assert_eq!(child.parent(), Some(node), "parent of {child:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn children_are_row_major_and_partition_the_parent() {
        let parent = QuadNode::new(3, 2, 5);
        let children = parent.children().expect("has children");
        assert_eq!(children[0], QuadNode::new(2, 4, 10));
        assert_eq!(children[1], QuadNode::new(2, 5, 10));
        assert_eq!(children[2], QuadNode::new(2, 4, 11));
        assert_eq!(children[3], QuadNode::new(2, 5, 11));

        // The four children exactly cover the parent's chunk range, with no overlap.
        let mut covered = 0_u32;
        let (pmin_x, pmin_z, pmax_x, pmax_z) = parent.chunk_range();
        for child in children {
            let (cmin_x, cmin_z, cmax_x, cmax_z) = child.chunk_range();
            assert!(cmin_x >= pmin_x && cmax_x <= pmax_x);
            assert!(cmin_z >= pmin_z && cmax_z <= pmax_z);
            covered += (cmax_x - cmin_x + 1) * (cmax_z - cmin_z + 1);
        }
        assert_eq!(covered, (pmax_x - pmin_x + 1) * (pmax_z - pmin_z + 1));
    }

    #[test]
    fn walking_from_a_chunk_to_the_root_takes_six_steps() {
        let mut node = QuadNode::new(0, 63, 63);
        let mut steps = 0;
        while let Some(parent) = node.parent() {
            node = parent;
            steps += 1;
        }
        assert_eq!(steps, ROOT_LEVEL);
        assert_eq!(node, QuadNode::ROOT);
    }

    #[test]
    fn a_node_contains_exactly_the_chunks_in_its_range() {
        let node = QuadNode::new(2, 1, 1); // covers chunks 4..=7 on both axes
        assert_eq!(node.chunk_range(), (4, 4, 7, 7));
        assert!(node.contains_chunk(ChunkCoord::new(4, 4)));
        assert!(node.contains_chunk(ChunkCoord::new(7, 7)));
        assert!(!node.contains_chunk(ChunkCoord::new(3, 4)));
        assert!(!node.contains_chunk(ChunkCoord::new(8, 4)));
        assert!(!node.contains_chunk(ChunkCoord::new(4, 8)));
    }

    #[test]
    fn from_world_agrees_with_contains_chunk() {
        let probes = [
            (0.0_f32, 0.0_f32),
            (plot::PLOT_MIN_M, plot::PLOT_MIN_M),
            (plot::PLOT_MAX_M, plot::PLOT_MAX_M),
            (-1_234.5, 6_789.0),
        ];
        for (x, z) in probes {
            let chunk = plot::world_to_chunk(x, z);
            for level in 0..=ROOT_LEVEL {
                let node = QuadNode::from_world(level, x, z);
                assert!(node.is_valid(), "{node:?} at ({x}, {z})");
                assert_eq!(node.level, level);
                assert!(
                    node.contains_chunk(chunk),
                    "{node:?} should contain {chunk:?}"
                );
            }
        }
    }

    #[test]
    fn out_of_range_levels_do_not_panic_or_wrap() {
        assert_eq!(nodes_per_axis(ROOT_LEVEL + 1), 0);
        assert_eq!(nodes_per_axis(255), 0);
        assert!(!QuadNode::new(ROOT_LEVEL + 1, 0, 0).is_valid());
        // Clamped rather than shifted into nonsense.
        assert_eq!(node_size_m(200), plot::PLOT_EXTENT_M);
        assert_eq!(QuadNode::from_world(200, 0.0, 0.0).level, ROOT_LEVEL);
    }

    #[test]
    fn invalid_indices_are_reported() {
        assert!(!QuadNode::new(0, plot::CHUNKS_PER_AXIS, 0).is_valid());
        assert!(!QuadNode::new(ROOT_LEVEL, 1, 0).is_valid());
        assert!(QuadNode::new(5, 1, 1).is_valid());
        assert!(!QuadNode::new(5, 2, 0).is_valid());
    }
}
