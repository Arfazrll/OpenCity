//! Terrain chunk mesh construction.
//!
//! # Design
//!
//! This module deliberately contains **no Bevy types**. It turns a [`Heightfield`] and a
//! [`QuadNode`] into plain vectors of positions, normals, colours and indices; the caller
//! copies those into a `Mesh`. That keeps the geometry — the part that is easy to get subtly
//! wrong — unit-testable without a GPU, a window or an asset server.
//!
//! # Crack-free LOD: skirts, not morphing
//!
//! Adjacent quadtree nodes at different levels sample the heightfield at different spacings,
//! so their shared edge does not agree and a thin gap of background shows through. Blueprint
//! 2.6.2 allows either CDLOD-style vertex morphing or skirts.
//!
//! This uses **skirts**: each node's mesh is bordered by a vertical wall hanging below the
//! surface, which occludes any gap by construction. The reasons for choosing it over morphing:
//!
//! - Morphing needs a custom vertex shader. Bevy 0.20 moved shaders to WESL, and writing a
//!   custom material against a just-changed shader language is the largest avoidable risk in
//!   M0. Skirts are pure CPU geometry and use the stock `StandardMaterial`, so PBR, shadows
//!   and fog all work without custom pipeline code.
//! - A skirt cannot fail to cover a crack. Morphing can, if the morph factor is computed from
//!   a slightly different distance than the neighbour used.
//!
//! The cost is that a node changing level *pops* rather than blending. Popping is not a crack,
//! and it happens far from the camera where distance fog hides it. Morphing stays available as
//! an upgrade once there is a reason to write a custom material.
//!
//! # The winding trick
//!
//! Emitting skirt walls as a special case invites winding bugs: a back-facing skirt is
//! invisible and the crack reappears. Instead the grid is built two vertices larger on every
//! side, with the outer ring placed at the **same XZ** as the ring just inside it but lowered
//! in Y. The ordinary grid triangulation then produces the vertical walls automatically, with
//! the same winding as every other quad, and the four corner quads collapse to zero area,
//! which is harmless.
//!
//! # Local coordinates
//!
//! Positions are relative to the node's minimum corner, not the world origin. The entity's
//! `Transform` places the node. Keeping vertex coordinates in `0..node_size` instead of
//! +/-8,192 m is the chunk-origin scheme blueprint 2.4 asks for, and it keeps `f32` precision
//! near the node rather than near the plot corner.

use world_model::{Heightfield, QuadNode, plot};

/// Vertices along one axis of a node's surface grid, excluding the skirt ring.
///
/// 33 vertices means 32 quads, so a level-0 node (256 m) has 8 m vertex spacing. The
/// heightfield is sampled at 4 m, so this halves terrain detail at the finest level — a
/// deliberate trade for an integrated GPU, since 65 vertices per axis would quadruple the
/// triangle count for detail that is not visible from a camera 200 m up.
pub const NODE_VERTS_PER_AXIS: u32 = 33;

/// Vertices along one axis including the skirt ring on both sides.
pub const GRID_VERTS_PER_AXIS: u32 = NODE_VERTS_PER_AXIS + 2;

/// Vertices in one node mesh.
pub const VERTS_PER_NODE: u32 = GRID_VERTS_PER_AXIS * GRID_VERTS_PER_AXIS;

/// Triangles in one node mesh, including the degenerate corner quads.
pub const TRIS_PER_NODE: u32 = (GRID_VERTS_PER_AXIS - 1) * (GRID_VERTS_PER_AXIS - 1) * 2;

/// Minimum skirt depth, in metres.
///
/// Flat terrain still needs a skirt, because neighbouring levels disagree by the interpolation
/// error even where the ground is level.
const MIN_SKIRT_DEPTH_M: f32 = 12.0;

/// Spacing used for normal estimation, in metres.
///
/// Fixed at the heightfield resolution rather than the node's vertex spacing, so that a given
/// patch of ground gets the same normal at every LOD level. If normals changed with level, a
/// node changing LOD would visibly shift its shading even where the silhouette barely moved.
const NORMAL_SAMPLE_SPACING_M: f32 = plot::HEIGHTFIELD_RESOLUTION_M;

/// Geometry for one terrain node, ready to be copied into a GPU mesh.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainMeshData {
    /// Vertex positions, local to the node's minimum corner.
    pub positions: Vec<[f32; 3]>,
    /// Vertex normals, estimated from the heightfield gradient.
    pub normals: Vec<[f32; 3]>,
    /// Vertex colours, standing in for material splatting until textures arrive.
    pub colors: Vec<[f32; 4]>,
    /// Triangle indices, counter-clockwise when seen from above.
    pub indices: Vec<u32>,
    /// Lowest surface height in this node, in metres. Used for the bounding box.
    pub min_height_m: f32,
    /// Highest surface height in this node, in metres. Used for the bounding box.
    pub max_height_m: f32,
    /// How far the skirt hangs below the surface, in metres.
    pub skirt_depth_m: f32,
}

/// Vertex spacing of a node's surface grid, in metres.
#[must_use]
pub fn vertex_spacing_m(node: QuadNode) -> f32 {
    node.size_m() / (NODE_VERTS_PER_AXIS - 1) as f32
}

/// Picks a terrain colour from height and steepness.
///
/// Stands in for the texture splatting of blueprint 2.6.2. M0 ships **no binary assets at
/// all**, which keeps `assets/LICENSES.md` empty and the licensing gate trivially green, so
/// the "material" is a per-vertex colour chosen by the same rules a splat map would use:
/// height bands for biome, slope for exposed rock.
///
/// `slope_y` is the vertical component of the surface normal: 1.0 is level ground, 0.0 is a
/// vertical face.
#[must_use]
pub fn terrain_color(height_m: f32, slope_y: f32) -> [f32; 4] {
    // Steep ground is bare rock whatever its height: that is what makes relief readable
    // without a texture.
    const STEEP: f32 = 0.78;
    const VERY_STEEP: f32 = 0.55;

    if height_m < -1.0 {
        // Submerged ground, shaded as water rather than as seabed.
        //
        // Strictly the seabed is sand and silt, and the blue belongs to the water surface
        // above it (blueprint 2.6.3) — which arrives in M1. Until then, colouring submerged
        // terrain as water is what makes the Coastal preset read as a coast instead of a
        // sandy basin. M1 replaces this branch with a real water surface and restores the
        // seabed to its natural colour.
        let t = ((-height_m) / 40.0).clamp(0.0, 1.0);
        return lerp_rgb([0.22, 0.42, 0.45], [0.07, 0.13, 0.26], t);
    }

    if slope_y < VERY_STEEP {
        return [0.38, 0.36, 0.34, 1.0];
    }

    if height_m < 4.0 {
        // Beach, but only where it is flat enough to be a beach.
        return if slope_y < STEEP {
            [0.46, 0.43, 0.38, 1.0]
        } else {
            [0.76, 0.70, 0.54, 1.0]
        };
    }

    if slope_y < STEEP {
        // Exposed rock on hillsides, greying with altitude.
        let t = (height_m / 400.0).clamp(0.0, 1.0);
        return lerp_rgb([0.44, 0.40, 0.34], [0.55, 0.54, 0.53], t);
    }

    // Vegetated ground: lush low down, drier and paler with altitude.
    let t = (height_m / 360.0).clamp(0.0, 1.0);
    lerp_rgb([0.29, 0.42, 0.22], [0.52, 0.50, 0.40], t)
}

#[inline]
fn lerp_rgb(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        1.0,
    ]
}

/// Builds the mesh geometry for one quadtree node.
///
/// Positions are local to the node's minimum corner; place the result with a `Transform` at
/// [`QuadNode::world_min`].
#[must_use]
pub fn build_node_mesh(field: &Heightfield, node: QuadNode) -> TerrainMeshData {
    let grid = GRID_VERTS_PER_AXIS as usize;
    let spacing = vertex_spacing_m(node);
    let (origin_x, origin_z) = node.world_min();
    let node_size = node.size_m();

    let mut positions = Vec::with_capacity(grid * grid);
    let mut normals = Vec::with_capacity(grid * grid);
    let mut colors = Vec::with_capacity(grid * grid);

    // First pass over the surface ring only, to size the skirt from the node's actual relief.
    // The vertical gap between two LOD levels along a shared edge cannot exceed the relief
    // along that edge, so half the relief plus a floor is a safe depth without being wasteful
    // on flat ground.
    let (surface_min, surface_max) = surface_height_range(field, node);
    let skirt_depth_m = ((surface_max - surface_min) * 0.5).max(MIN_SKIRT_DEPTH_M);

    for gz in 0..grid {
        // Map the padded grid index onto the surface grid, clamping the outer ring onto the
        // ring just inside it. That is what makes the skirt vertical.
        let (sz, z_is_skirt) = clamp_to_surface(gz);
        let local_z = sz as f32 * spacing;
        let world_z = origin_z + local_z;

        for gx in 0..grid {
            let (sx, x_is_skirt) = clamp_to_surface(gx);
            let local_x = sx as f32 * spacing;
            let world_x = origin_x + local_x;

            let surface_y = field.height_at_world(world_x, world_z);
            let is_skirt = x_is_skirt || z_is_skirt;
            let y = if is_skirt {
                surface_y - skirt_depth_m
            } else {
                surface_y
            };

            positions.push([local_x, y, local_z]);

            let normal = estimate_normal(field, world_x, world_z);
            normals.push(normal);
            colors.push(terrain_color(surface_y, normal[1]));
        }
    }

    let indices = grid_indices(grid);

    debug_assert_eq!(positions.len(), VERTS_PER_NODE as usize);
    debug_assert_eq!(indices.len(), TRIS_PER_NODE as usize * 3);
    debug_assert!(node_size > 0.0);

    TerrainMeshData {
        positions,
        normals,
        colors,
        indices,
        min_height_m: surface_min,
        max_height_m: surface_max,
        skirt_depth_m,
    }
}

/// Maps a padded grid index to a surface grid index, reporting whether it is a skirt ring.
#[inline]
fn clamp_to_surface(padded: usize) -> (usize, bool) {
    let last = NODE_VERTS_PER_AXIS as usize - 1;
    if padded == 0 {
        (0, true)
    } else if padded > NODE_VERTS_PER_AXIS as usize {
        (last, true)
    } else {
        (padded - 1, false)
    }
}

/// The lowest and highest surface heights across a node's surface grid.
fn surface_height_range(field: &Heightfield, node: QuadNode) -> (f32, f32) {
    let spacing = vertex_spacing_m(node);
    let (origin_x, origin_z) = node.world_min();
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;

    for j in 0..NODE_VERTS_PER_AXIS {
        let world_z = origin_z + j as f32 * spacing;
        for i in 0..NODE_VERTS_PER_AXIS {
            let world_x = origin_x + i as f32 * spacing;
            let h = field.height_at_world(world_x, world_z);
            min = min.min(h);
            max = max.max(h);
        }
    }

    // A node entirely outside the plot would leave these at infinity; clamp so the caller
    // never sees a non-finite bound.
    if min.is_finite() && max.is_finite() {
        (min, max)
    } else {
        (0.0, 0.0)
    }
}

/// Estimates a surface normal from the heightfield gradient by central difference.
fn estimate_normal(field: &Heightfield, world_x: f32, world_z: f32) -> [f32; 3] {
    let s = NORMAL_SAMPLE_SPACING_M;
    let left = field.height_at_world(world_x - s, world_z);
    let right = field.height_at_world(world_x + s, world_z);
    let back = field.height_at_world(world_x, world_z - s);
    let front = field.height_at_world(world_x, world_z + s);

    // Gradient of a heightfield: n = (-dh/dx, 1, -dh/dz), scaled by the sample spacing.
    let nx = left - right;
    let ny = 2.0 * s;
    let nz = back - front;
    let length = (nx * nx + ny * ny + nz * nz).sqrt();

    if length > f32::EPSILON {
        [nx / length, ny / length, nz / length]
    } else {
        [0.0, 1.0, 0.0]
    }
}

/// Triangulates a square grid of `grid` by `grid` vertices.
///
/// Winding is counter-clockwise seen from `+Y`, matching Bevy's convention for an upward
/// facing surface.
fn grid_indices(grid: usize) -> Vec<u32> {
    let quads = grid - 1;
    let mut indices = Vec::with_capacity(quads * quads * 6);

    for j in 0..quads {
        for i in 0..quads {
            let a = (j * grid + i) as u32; // (-x, -z)
            let b = a + 1; // (+x, -z)
            let d = a + grid as u32; // (-x, +z)
            let c = d + 1; // (+x, +z)

            // a, d, b and b, d, c both wind counter-clockwise from above.
            indices.extend_from_slice(&[a, d, b, b, d, c]);
        }
    }

    indices
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_model::SampleCoord;

    fn flat_field(height: f32) -> Heightfield {
        Heightfield::flat(height)
    }

    /// A field with a steady slope along X, for gradient checks.
    fn sloped_field(rise_per_metre: f32) -> Heightfield {
        let mut field = Heightfield::flat(0.0);
        for j in 0..plot::HEIGHTFIELD_SAMPLES_PER_AXIS {
            for i in 0..plot::HEIGHTFIELD_SAMPLES_PER_AXIS {
                let (x, _) = plot::sample_to_world(SampleCoord::new(i, j));
                field.set_height_at(SampleCoord::new(i, j), 300.0 + x * rise_per_metre);
            }
        }
        field
    }

    #[test]
    fn mesh_has_the_documented_vertex_and_triangle_counts() {
        let field = flat_field(10.0);
        let data = build_node_mesh(&field, QuadNode::new(0, 0, 0));
        assert_eq!(data.positions.len(), VERTS_PER_NODE as usize);
        assert_eq!(data.normals.len(), VERTS_PER_NODE as usize);
        assert_eq!(data.colors.len(), VERTS_PER_NODE as usize);
        assert_eq!(data.indices.len(), TRIS_PER_NODE as usize * 3);
        // 35 x 35 grid, 34 x 34 quads.
        assert_eq!(GRID_VERTS_PER_AXIS, 35);
        assert_eq!(VERTS_PER_NODE, 1_225);
        assert_eq!(TRIS_PER_NODE, 2_312);
    }

    #[test]
    fn every_index_is_in_range() {
        let field = flat_field(0.0);
        let data = build_node_mesh(&field, QuadNode::new(2, 5, 5));
        let count = data.positions.len() as u32;
        assert!(
            data.indices.iter().all(|i| *i < count),
            "index out of range"
        );
    }

    #[test]
    fn vertex_spacing_matches_the_node_level() {
        // A level-0 node is 256 m across 32 quads.
        assert_eq!(vertex_spacing_m(QuadNode::new(0, 0, 0)), 8.0);
        assert_eq!(vertex_spacing_m(QuadNode::new(1, 0, 0)), 16.0);
        assert_eq!(vertex_spacing_m(QuadNode::ROOT), 512.0);
    }

    #[test]
    fn surface_vertices_sit_exactly_on_the_heightfield() {
        let field = flat_field(57.0);
        let node = QuadNode::new(0, 20, 20);
        let data = build_node_mesh(&field, node);
        let grid = GRID_VERTS_PER_AXIS as usize;

        // Interior vertices (not the skirt ring) must be at the surface.
        for gz in 1..grid - 1 {
            for gx in 1..grid - 1 {
                let y = data.positions[gz * grid + gx][1];
                assert!((y - 57.0).abs() < 0.05, "vertex ({gx}, {gz}) at y={y}");
            }
        }
    }

    #[test]
    fn positions_are_local_to_the_node_not_the_world_origin() {
        // This is the chunk-origin scheme: a node far from the world origin must still have
        // small local coordinates, or f32 precision is wasted.
        let field = flat_field(0.0);
        let far_node = QuadNode::new(0, 63, 63);
        let data = build_node_mesh(&field, far_node);
        let max_coord = data
            .positions
            .iter()
            .flat_map(|p| [p[0].abs(), p[2].abs()])
            .fold(0.0_f32, f32::max);
        assert!(
            max_coord <= far_node.size_m() + 0.01,
            "local coord reached {max_coord}"
        );
        assert!(max_coord > 0.0);
    }

    #[test]
    fn the_skirt_ring_hangs_below_the_surface() {
        let field = flat_field(100.0);
        let node = QuadNode::new(0, 10, 10);
        let data = build_node_mesh(&field, node);
        let grid = GRID_VERTS_PER_AXIS as usize;

        assert!(data.skirt_depth_m >= MIN_SKIRT_DEPTH_M);

        // Every vertex on the outer ring is below every interior vertex.
        for g in 0..grid {
            for (gx, gz) in [(0, g), (grid - 1, g), (g, 0), (g, grid - 1)] {
                let y = data.positions[gz * grid + gx][1];
                assert!(
                    (y - (100.0 - data.skirt_depth_m)).abs() < 0.05,
                    "skirt vertex ({gx}, {gz}) at y={y}, expected {}",
                    100.0 - data.skirt_depth_m
                );
            }
        }
    }

    #[test]
    fn the_skirt_ring_shares_xz_with_the_ring_inside_it() {
        // This is what makes the skirt a vertical wall rather than an outward flange, and it
        // is why the ordinary triangulation gives correct winding.
        let field = flat_field(0.0);
        let data = build_node_mesh(&field, QuadNode::new(0, 1, 1));
        let grid = GRID_VERTS_PER_AXIS as usize;

        for g in 1..grid - 1 {
            // West skirt column 0 matches surface column 1.
            assert_eq!(data.positions[g * grid][0], data.positions[g * grid + 1][0]);
            // East skirt column matches the one inside it.
            assert_eq!(
                data.positions[g * grid + grid - 1][0],
                data.positions[g * grid + grid - 2][0]
            );
            // North skirt row 0 matches surface row 1.
            assert_eq!(data.positions[g][2], data.positions[grid + g][2]);
            assert_eq!(
                data.positions[(grid - 1) * grid + g][2],
                data.positions[(grid - 2) * grid + g][2]
            );
        }
    }

    #[test]
    fn a_skirt_is_deeper_on_rugged_ground_than_on_flat_ground() {
        let flat = build_node_mesh(&flat_field(50.0), QuadNode::new(0, 30, 30));
        let steep = build_node_mesh(&sloped_field(0.5), QuadNode::new(0, 30, 30));
        assert_eq!(flat.skirt_depth_m, MIN_SKIRT_DEPTH_M);
        assert!(
            steep.skirt_depth_m > flat.skirt_depth_m,
            "steep skirt {} vs flat {}",
            steep.skirt_depth_m,
            flat.skirt_depth_m
        );
    }

    #[test]
    fn surface_triangles_face_upwards() {
        // A wrong winding here would make the whole terrain invisible from above, and is the
        // single easiest mistake to make when triangulating a grid by hand.
        let field = flat_field(20.0);
        let data = build_node_mesh(&field, QuadNode::new(0, 0, 0));
        let grid = GRID_VERTS_PER_AXIS as usize;

        // Check an interior quad, away from the degenerate skirt ring.
        let mid = grid / 2;
        let quad_start = ((mid * (grid - 1)) + mid) * 6;
        let tri = &data.indices[quad_start..quad_start + 3];

        let p = |i: u32| {
            let v = data.positions[i as usize];
            (v[0], v[1], v[2])
        };
        let (ax, ay, az) = p(tri[0]);
        let (bx, by, bz) = p(tri[1]);
        let (cx, cy, cz) = p(tri[2]);

        // Cross product of (b - a) x (c - a); its Y component must be positive.
        let (e1x, e1y, e1z) = (bx - ax, by - ay, bz - az);
        let (e2x, e2y, e2z) = (cx - ax, cy - ay, cz - az);
        let normal_y = e1z * e2x - e1x * e2z;
        let _ = (e1y, e2y);
        assert!(
            normal_y > 0.0,
            "interior triangle winds the wrong way (ny={normal_y})"
        );
    }

    #[test]
    fn normals_point_up_on_flat_ground() {
        let field = flat_field(5.0);
        let data = build_node_mesh(&field, QuadNode::new(0, 0, 0));
        for n in &data.normals {
            assert!((n[1] - 1.0).abs() < 1e-4, "normal {n:?} is not vertical");
        }
    }

    #[test]
    fn normals_tilt_against_the_slope() {
        // Height rises with X, so the normal must lean towards -X.
        let field = sloped_field(0.25);
        let data = build_node_mesh(&field, QuadNode::new(0, 32, 32));
        let grid = GRID_VERTS_PER_AXIS as usize;
        let n = data.normals[(grid / 2) * grid + grid / 2];
        assert!(n[0] < -0.1, "normal {n:?} should lean towards -X");
        assert!(n[1] > 0.0, "normal {n:?} should still point upwards");
        assert!(n[2].abs() < 0.05, "normal {n:?} should not lean along Z");
    }

    #[test]
    fn normals_do_not_change_with_lod_level() {
        // If they did, a node changing level would visibly shift its shading. This is why the
        // gradient is sampled at a fixed spacing instead of the node's vertex spacing.
        let field = sloped_field(0.1);
        // The same world position, reached through two different node levels.
        let fine = build_node_mesh(&field, QuadNode::new(0, 32, 32));
        let coarse = build_node_mesh(&field, QuadNode::new(2, 8, 8));
        assert_eq!(fine.normals[0], coarse.normals[0]);
    }

    #[test]
    fn height_range_is_reported_from_the_surface_not_the_skirt() {
        let field = flat_field(80.0);
        let data = build_node_mesh(&field, QuadNode::new(0, 5, 5));
        assert!((data.min_height_m - 80.0).abs() < 0.05);
        assert!((data.max_height_m - 80.0).abs() < 0.05);
        // The skirt is below that range, and the caller needs to know by how much to size a
        // bounding box.
        assert!(data.skirt_depth_m > 0.0);
    }

    #[test]
    fn colours_distinguish_water_beach_grass_and_rock() {
        let sea = terrain_color(-20.0, 1.0);
        let beach = terrain_color(2.0, 1.0);
        let grass = terrain_color(60.0, 1.0);
        let cliff = terrain_color(60.0, 0.3);

        // Sea is bluer than it is green.
        assert!(sea[2] > sea[1], "sea {sea:?} should be blue-dominant");
        // Beach is warm and bright.
        assert!(beach[0] > beach[2], "beach {beach:?} should be warm");
        // Grass is green-dominant.
        assert!(
            grass[1] > grass[0] && grass[1] > grass[2],
            "grass {grass:?}"
        );
        // A cliff at the same height is not green.
        assert!(
            cliff[1] < grass[1],
            "cliff {cliff:?} should not be vegetated"
        );
        // Everything is opaque.
        for c in [sea, beach, grass, cliff] {
            assert_eq!(c[3], 1.0);
        }
    }

    #[test]
    fn colours_stay_in_gamut_across_the_whole_height_and_slope_range() {
        let mut h = -100.0_f32;
        while h <= 1_210.0 {
            for slope in [0.0_f32, 0.3, 0.6, 0.8, 1.0] {
                let c = terrain_color(h, slope);
                for (channel, value) in c.iter().enumerate() {
                    assert!(
                        (0.0..=1.0).contains(value),
                        "height {h}, slope {slope}, channel {channel} = {value}"
                    );
                }
            }
            h += 7.3;
        }
    }

    #[test]
    fn deeper_water_is_darker() {
        let shallow = terrain_color(-2.0, 1.0);
        let deep = terrain_color(-40.0, 1.0);
        let brightness = |c: [f32; 4]| c[0] + c[1] + c[2];
        assert!(
            brightness(deep) < brightness(shallow),
            "deep {deep:?} shallow {shallow:?}"
        );
    }

    #[test]
    fn building_the_same_node_twice_gives_the_same_mesh() {
        let field = sloped_field(0.05);
        let node = QuadNode::new(1, 10, 20);
        assert_eq!(build_node_mesh(&field, node), build_node_mesh(&field, node));
    }

    #[test]
    fn nodes_at_every_level_build_without_panicking() {
        let field = flat_field(30.0);
        for level in 0..=world_model::chunk::ROOT_LEVEL {
            let node = QuadNode::new(level, 0, 0);
            let data = build_node_mesh(&field, node);
            assert_eq!(data.positions.len(), VERTS_PER_NODE as usize);
            assert!(
                data.positions
                    .iter()
                    .all(|p| p.iter().all(|c| c.is_finite()))
            );
            assert!(data.normals.iter().all(|n| n.iter().all(|c| c.is_finite())));
        }
    }
}
