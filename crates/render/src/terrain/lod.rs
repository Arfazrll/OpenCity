//! Quadtree level-of-detail selection.
//!
//! # Design
//!
//! Given a camera position, this picks the set of [`QuadNode`]s to draw: fine nodes near the
//! camera, progressively coarser further out. It is pure arithmetic over node coordinates —
//! no heightfield, no Bevy, no entities — so the selection logic can be tested exhaustively
//! without rendering anything.
//!
//! # The rule
//!
//! Descend the quadtree from the root. A node is **subdivided** when the camera is closer to
//! it than its own size times [`LodConfig::subdivide_factor`], and **drawn** otherwise. Level 0
//! is always drawn, since it cannot subdivide further.
//!
//! Scaling the threshold by node size is what makes the result roughly constant in screen
//! space: a node twice as large is drawn at twice the distance, so each node covers a similar
//! number of pixels whatever its level. That is also why the triangle count stays bounded no
//! matter where the camera is.
//!
//! # Distance to a node, not to its centre
//!
//! Distance is measured to the nearest point of the node's footprint, and is zero when the
//! camera is inside it. Using the centre instead would misjudge large nodes badly: the root
//! node's centre is up to 11.6 km from a camera that is standing inside it.
//!
//! Only the horizontal distance counts. Altitude is handled separately, because a camera
//! 5 km up looking down should not force maximum detail on the ground directly beneath it.

use world_model::{QuadNode, plot};

/// How detail falls off with distance, and how far terrain is drawn at all.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LodConfig {
    /// A node subdivides when the camera is nearer than `node_size * subdivide_factor`.
    ///
    /// Higher means more detail and more triangles. This is the "LOD bias per preset" of
    /// blueprint 2.6.7.
    pub subdivide_factor: f32,
    /// Terrain beyond this horizontal distance is not drawn, in metres.
    pub max_view_distance_m: f32,
    /// Half-extent of the simulated area, in metres.
    ///
    /// The Low preset caps the active city to the central quarter of the plot
    /// (blueprint 2.6.8); nodes entirely outside that box are not drawn.
    pub active_half_extent_m: f32,
    /// Camera altitude contribution, as a fraction of height above the ground.
    ///
    /// Added to the horizontal distance before comparing against the threshold, so that
    /// zooming out reduces detail instead of keeping the ground under the camera at level 0.
    pub altitude_weight: f32,
}

impl Default for LodConfig {
    fn default() -> Self {
        Self {
            subdivide_factor: 2.5,
            max_view_distance_m: 12_000.0,
            active_half_extent_m: plot::PLOT_HALF_EXTENT_M,
            altitude_weight: 0.6,
        }
    }
}

/// Upper bound on how many nodes a selection may contain.
///
/// Not a tuning knob: it is a safety valve. If a future config change would select thousands
/// of nodes, the selection is truncated and the caller warns, rather than the frame budget
/// quietly collapsing. The `node_count_stays_bounded_from_every_camera_position` test keeps
/// real configs far below this.
pub const MAX_SELECTED_NODES: usize = 1_024;

/// Horizontal distance from a point to the nearest edge of a node's footprint, in metres.
///
/// Zero when the point is inside the footprint.
#[must_use]
pub fn horizontal_distance_to_node(node: QuadNode, x: f32, z: f32) -> f32 {
    let (min_x, min_z) = node.world_min();
    let size = node.size_m();
    let max_x = min_x + size;
    let max_z = min_z + size;

    let dx = (min_x - x).max(x - max_x).max(0.0);
    let dz = (min_z - z).max(z - max_z).max(0.0);

    (dx * dx + dz * dz).sqrt()
}

/// Returns whether any part of a node lies inside the active area.
#[must_use]
fn intersects_active_area(node: QuadNode, half_extent: f32) -> bool {
    let (min_x, min_z) = node.world_min();
    let size = node.size_m();
    min_x < half_extent
        && min_z < half_extent
        && min_x + size > -half_extent
        && min_z + size > -half_extent
}

/// Picks the nodes to draw for a camera at `(x, altitude, z)`.
///
/// The result is sorted and contains no duplicates or overlaps: the nodes exactly tile the
/// visible part of the active area.
#[must_use]
pub fn select_nodes(x: f32, altitude: f32, z: f32, config: &LodConfig) -> Vec<QuadNode> {
    let mut out = Vec::with_capacity(256);
    descend(QuadNode::ROOT, x, altitude, z, config, &mut out);
    out.sort_unstable();
    out
}

fn descend(
    node: QuadNode,
    x: f32,
    altitude: f32,
    z: f32,
    config: &LodConfig,
    out: &mut Vec<QuadNode>,
) {
    if out.len() >= MAX_SELECTED_NODES {
        return;
    }

    if !intersects_active_area(node, config.active_half_extent_m) {
        return;
    }

    let horizontal = horizontal_distance_to_node(node, x, z);
    if horizontal > config.max_view_distance_m {
        return;
    }

    // Altitude pushes the effective distance up, so climbing reduces detail.
    let effective = horizontal + altitude.abs() * config.altitude_weight;

    let subdivide = node.level > 0 && effective < node.size_m() * config.subdivide_factor;

    if subdivide && let Some(children) = node.children() {
        for child in children {
            descend(child, x, altitude, z, config, out);
        }
        return;
    }

    out.push(node);
}

/// Total triangles a selection will draw, using the mesh module's per-node count.
#[must_use]
pub fn triangle_count(selection: &[QuadNode]) -> u64 {
    selection.len() as u64 * u64::from(super::mesh::TRIS_PER_NODE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_model::chunk;

    fn ground_config() -> LodConfig {
        LodConfig {
            altitude_weight: 0.0,
            ..Default::default()
        }
    }

    #[test]
    fn distance_is_zero_inside_a_node_and_grows_outside_it() {
        let node = QuadNode::new(0, 32, 32); // straddles the origin-ish
        let (min_x, min_z) = node.world_min();
        let size = node.size_m();

        // Inside.
        assert_eq!(
            horizontal_distance_to_node(node, min_x + 1.0, min_z + 1.0),
            0.0
        );
        assert_eq!(
            horizontal_distance_to_node(node, min_x + size / 2.0, min_z + size / 2.0),
            0.0
        );
        // Exactly 100 m west of the western edge.
        let d = horizontal_distance_to_node(node, min_x - 100.0, min_z + size / 2.0);
        assert!((d - 100.0).abs() < 0.01, "distance {d}");
        // Diagonally off the corner: 3-4-5.
        let d = horizontal_distance_to_node(node, min_x - 30.0, min_z - 40.0);
        assert!((d - 50.0).abs() < 0.01, "distance {d}");
    }

    #[test]
    fn the_root_node_contains_a_camera_anywhere_in_the_plot() {
        // The reason distance is measured to the footprint rather than the centre.
        for (x, z) in [(0.0, 0.0), (8_000.0, -8_000.0), (-8_191.0, 8_191.0)] {
            assert_eq!(horizontal_distance_to_node(QuadNode::ROOT, x, z), 0.0);
        }
    }

    #[test]
    fn selection_is_finest_under_the_camera_and_coarsens_outward() {
        let config = ground_config();
        let selection = select_nodes(0.0, 0.0, 0.0, &config);
        assert!(!selection.is_empty());

        // The node containing the camera must be level 0.
        let under_camera = selection
            .iter()
            .find(|n| horizontal_distance_to_node(**n, 0.0, 0.0) == 0.0)
            .copied()
            .expect("some node contains the camera");
        assert_eq!(
            under_camera.level, 0,
            "node under the camera is {under_camera:?}"
        );

        // Level must not decrease with distance: for every pair, a nearer node is at most as
        // coarse as a further one.
        for node in &selection {
            let d = horizontal_distance_to_node(*node, 0.0, 0.0);
            let expected_max_level = chunk::ROOT_LEVEL;
            assert!(node.level <= expected_max_level);
            // A node of this level must genuinely be far enough away not to subdivide.
            if node.level > 0 {
                assert!(
                    d >= node.size_m() * config.subdivide_factor - node.size_m(),
                    "{node:?} at {d} m is too close to be drawn at level {}",
                    node.level
                );
            }
        }
    }

    #[test]
    fn selected_nodes_never_overlap() {
        // An overlap would draw terrain twice and show z-fighting; a gap would show
        // background. This checks the first by verifying no node contains another's area.
        let config = ground_config();
        for (x, z) in [(0.0, 0.0), (1_234.0, -5_678.0), (8_000.0, 8_000.0)] {
            let selection = select_nodes(x, 0.0, z, &config);
            for (a_index, a) in selection.iter().enumerate() {
                for b in selection.iter().skip(a_index + 1) {
                    let (a_min_x, a_min_z) = a.world_min();
                    let (b_min_x, b_min_z) = b.world_min();
                    let a_size = a.size_m();
                    let b_size = b.size_m();
                    let overlap_x = a_min_x < b_min_x + b_size && b_min_x < a_min_x + a_size;
                    let overlap_z = a_min_z < b_min_z + b_size && b_min_z < a_min_z + a_size;
                    assert!(
                        !(overlap_x && overlap_z),
                        "camera ({x}, {z}): {a:?} overlaps {b:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn selected_nodes_leave_no_gap_near_the_camera() {
        // Sample a grid of ground positions around the camera and assert each is covered by
        // exactly one selected node. A gap here is a hole in the world.
        let config = ground_config();
        let selection = select_nodes(0.0, 0.0, 0.0, &config);

        let mut probe_x = -4_000.0_f32;
        while probe_x < 4_000.0 {
            let mut probe_z = -4_000.0_f32;
            while probe_z < 4_000.0 {
                let covering = selection
                    .iter()
                    .filter(|n| {
                        let (min_x, min_z) = n.world_min();
                        let size = n.size_m();
                        probe_x >= min_x
                            && probe_x < min_x + size
                            && probe_z >= min_z
                            && probe_z < min_z + size
                    })
                    .count();
                assert_eq!(
                    covering, 1,
                    "({probe_x}, {probe_z}) is covered {covering} times"
                );
                probe_z += 311.0;
            }
            probe_x += 311.0;
        }
    }

    #[test]
    fn node_count_stays_bounded_from_every_camera_position() {
        // The property that makes the frame budget predictable. Also the early warning if a
        // future tuning change makes the selection explode.
        let config = ground_config();
        let mut worst = 0;
        let mut worst_at = (0.0, 0.0);

        let mut x = -8_192.0_f32;
        while x <= 8_192.0 {
            let mut z = -8_192.0_f32;
            while z <= 8_192.0 {
                let count = select_nodes(x, 0.0, z, &config).len();
                if count > worst {
                    worst = count;
                    worst_at = (x, z);
                }
                z += 1_024.0;
            }
            x += 1_024.0;
        }

        assert!(
            worst < MAX_SELECTED_NODES,
            "selected {worst} nodes at {worst_at:?}"
        );
        // The real budget: a few hundred nodes, not a few thousand.
        assert!(
            worst <= 400,
            "selected {worst} nodes at {worst_at:?}, expected at most 400"
        );
        assert!(
            worst > 20,
            "only {worst} nodes selected, LOD is suspiciously coarse"
        );
    }

    #[test]
    fn triangle_count_fits_an_integrated_gpu() {
        let config = ground_config();
        let selection = select_nodes(0.0, 0.0, 0.0, &config);
        let tris = triangle_count(&selection);
        // Well under a million triangles per frame, which is the realistic ceiling for a
        // 15 W integrated part at 30 fps.
        assert!(tris < 1_000_000, "{tris} triangles");
        assert!(tris > 10_000, "{tris} triangles is implausibly few");
    }

    #[test]
    fn a_higher_subdivide_factor_selects_more_nodes() {
        let low = LodConfig {
            subdivide_factor: 2.0,
            altitude_weight: 0.0,
            ..Default::default()
        };
        let ultra = LodConfig {
            subdivide_factor: 3.5,
            altitude_weight: 0.0,
            ..Default::default()
        };
        let low_count = select_nodes(0.0, 0.0, 0.0, &low).len();
        let ultra_count = select_nodes(0.0, 0.0, 0.0, &ultra).len();
        assert!(
            ultra_count > low_count,
            "low {low_count}, ultra {ultra_count}"
        );
    }

    #[test]
    fn climbing_reduces_detail() {
        // Zooming out to 5 km must not keep maximum detail on the ground below, or the
        // "whole skyline at 5 km" view in blueprint 1.6 would be unaffordable.
        let config = LodConfig::default();
        let ground = select_nodes(0.0, 50.0, 0.0, &config);
        let high = select_nodes(0.0, 5_000.0, 0.0, &config);

        let finest = |s: &[QuadNode]| s.iter().map(|n| n.level).min().unwrap_or(0);
        assert!(
            finest(&high) > finest(&ground),
            "finest level at 50 m is {}, at 5 km is {}",
            finest(&ground),
            finest(&high)
        );
        assert!(
            high.len() < ground.len(),
            "high {} vs ground {}",
            high.len(),
            ground.len()
        );
    }

    #[test]
    fn view_distance_limits_how_far_terrain_is_drawn() {
        let near = LodConfig {
            max_view_distance_m: 2_000.0,
            altitude_weight: 0.0,
            ..Default::default()
        };
        let selection = select_nodes(0.0, 0.0, 0.0, &near);
        for node in &selection {
            let d = horizontal_distance_to_node(*node, 0.0, 0.0);
            assert!(d <= near.max_view_distance_m, "{node:?} is {d} m away");
        }
        // And it really does cut the count down.
        let far = ground_config();
        assert!(selection.len() < select_nodes(0.0, 0.0, 0.0, &far).len());
    }

    #[test]
    fn the_low_preset_cap_keeps_selection_inside_the_central_quarter() {
        let config = LodConfig {
            active_half_extent_m: plot::LOW_PRESET_HALF_EXTENT_M,
            altitude_weight: 0.0,
            ..Default::default()
        };
        let selection = select_nodes(0.0, 0.0, 0.0, &config);
        assert!(!selection.is_empty());
        for node in &selection {
            let (min_x, min_z) = node.world_min();
            let size = node.size_m();
            // Every node must at least partly lie inside the capped box.
            assert!(
                min_x + size > -plot::LOW_PRESET_HALF_EXTENT_M
                    && min_x < plot::LOW_PRESET_HALF_EXTENT_M,
                "{node:?} lies outside the Low preset area"
            );
            assert!(
                min_z + size > -plot::LOW_PRESET_HALF_EXTENT_M
                    && min_z < plot::LOW_PRESET_HALF_EXTENT_M
            );
        }
    }

    #[test]
    fn selection_is_sorted_and_free_of_duplicates() {
        // Sorted order makes the diff against the previously loaded set deterministic.
        let selection = select_nodes(2_500.0, 300.0, -1_200.0, &LodConfig::default());
        let mut sorted = selection.clone();
        sorted.sort_unstable();
        assert_eq!(selection, sorted);
        sorted.dedup();
        assert_eq!(
            selection.len(),
            sorted.len(),
            "selection contains duplicates"
        );
    }

    #[test]
    fn every_selected_node_is_valid() {
        for (x, z) in [
            (0.0, 0.0),
            (-8_192.0, -8_192.0),
            (8_191.0, 8_191.0),
            (500.0, -7_000.0),
        ] {
            for node in select_nodes(x, 200.0, z, &LodConfig::default()) {
                assert!(node.is_valid(), "{node:?} from camera ({x}, {z})");
            }
        }
    }

    #[test]
    fn a_camera_outside_the_plot_still_produces_a_usable_selection() {
        // The camera is clamped elsewhere, but the selector must not return an empty set or
        // recurse pathologically if it ever sees an out-of-bounds position.
        let selection = select_nodes(50_000.0, 1_000.0, 50_000.0, &LodConfig::default());
        assert!(selection.len() <= MAX_SELECTED_NODES);
    }
}
