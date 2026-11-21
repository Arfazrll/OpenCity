//! Terrain rendering: generation, quadtree LOD and chunk streaming.
//!
//! # Design
//!
//! Three pieces, deliberately separated by how testable they are:
//!
//! - [`lod`] decides *which* nodes to draw. Pure arithmetic, no Bevy.
//! - [`mesh`] decides *what* each node looks like. Pure geometry, no Bevy.
//! - this module does the Bevy work: owning the heightfield, spawning and despawning chunk
//!   entities, and keeping the per-frame cost bounded.
//!
//! # Streaming budget
//!
//! A full selection is a few hundred nodes. Building them all in the frame the camera moves
//! would blow straight past the 50 ms stutter budget in blueprint 2.13, so new nodes go into a
//! queue and at most [`MESH_BUILDS_PER_FRAME`] are built per frame. Terrain fills in over a
//! handful of frames instead of hitching.
//!
//! Re-selection itself is also throttled: it only runs when the camera has moved far enough to
//! change the answer, not every frame.
//!
//! # Ownership note
//!
//! For M0 this module owns the [`Heightfield`]. From M3 the simulation owns the world and the
//! renderer reads it from a snapshot (blueprint 2.1); at that point [`TerrainWorld`] becomes a
//! view onto sim state rather than the storage itself. Everything that reads terrain already
//! goes through this resource, so that change stays local.

pub mod lod;
pub mod mesh;

use std::collections::{BTreeMap, VecDeque};
use std::time::Instant;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use procgen::terrain::{TerrainParams, TerrainStats};
use world_model::{Heightfield, QuadNode};

pub use lod::LodConfig;

/// Maximum node meshes built in a single frame.
///
/// Each build is a few hundred microseconds, so this is roughly a 2-3 ms slice — small enough
/// to stay inside the frame budget, large enough to fill a fresh view in under a second.
pub const MESH_BUILDS_PER_FRAME: usize = 16;

/// How far the camera must move horizontally before the LOD selection is recomputed, in metres.
///
/// A quarter of the finest node size: small enough that detail arrives before it is needed,
/// large enough that walking around does not re-select every frame.
pub const REEVALUATE_DISTANCE_M: f32 = 64.0;

/// How far the camera must move vertically before the LOD selection is recomputed, in metres.
pub const REEVALUATE_ALTITUDE_M: f32 = 100.0;

/// The generated terrain for the active city.
#[derive(Resource)]
pub struct TerrainWorld {
    /// Elevation for the whole plot.
    pub field: Heightfield,
    /// The parameters it was generated from. Enough to regenerate it exactly.
    pub params: TerrainParams,
    /// Statistics from the generation run.
    pub stats: TerrainStats,
    /// How long generation took, in seconds. Reported in the milestone report.
    pub generation_seconds: f32,
}

/// How aggressively terrain detail is drawn.
///
/// A separate resource rather than a read of the graphics preset, so that `render` does not
/// depend on the binary crate that owns presets. The application maps preset to these numbers.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct TerrainLodSettings(pub LodConfig);

/// Marks a spawned terrain chunk entity.
#[derive(Component, Debug, Clone, Copy)]
pub struct TerrainChunk {
    /// The quadtree node this entity draws.
    pub node: QuadNode,
}

/// Live streaming state.
#[derive(Resource, Default)]
struct TerrainStreaming {
    /// Nodes currently spawned, keyed by node so the diff against a new selection is cheap.
    ///
    /// `BTreeMap` rather than a hash map: iteration order is then deterministic, which keeps
    /// spawn and despawn order reproducible between runs.
    loaded: BTreeMap<QuadNode, Entity>,
    /// Nodes selected but not yet built.
    pending: VecDeque<QuadNode>,
    /// Camera position the current selection was computed for.
    last_eval: Option<Vec3>,
    /// Settings the current selection was computed for.
    last_settings: Option<TerrainLodSettings>,
    /// Shared material for every chunk.
    material: Option<Handle<StandardMaterial>>,
}

/// Statistics for the HUD and the milestone report.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct TerrainRenderStats {
    /// Chunk entities currently spawned.
    pub loaded_chunks: usize,
    /// Nodes selected but not yet built.
    pub pending_chunks: usize,
    /// Triangles in the currently loaded chunks.
    pub triangles: u64,
}

/// Generates and draws the terrain for one city plot.
#[derive(Default)]
pub struct TerrainPlugin {
    /// Terrain generation parameters.
    pub params: TerrainParams,
}

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        let params = self.params;
        app.init_resource::<TerrainLodSettings>()
            .init_resource::<TerrainStreaming>()
            .init_resource::<TerrainRenderStats>()
            .add_systems(
                Startup,
                move |commands: Commands,
                      materials: ResMut<Assets<StandardMaterial>>,
                      streaming: ResMut<TerrainStreaming>| {
                    generate_terrain(params, commands, materials, streaming);
                },
            )
            .add_systems(
                Update,
                (reevaluate_lod, build_pending_chunks, update_stats).chain(),
            );
    }
}

fn generate_terrain(
    params: TerrainParams,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut streaming: ResMut<TerrainStreaming>,
) {
    info!(
        "generating {} terrain, seed {} ({} m plot)",
        params.preset.name(),
        params.seed,
        world_model::plot::PLOT_EXTENT_M
    );

    let started = Instant::now();
    let (field, stats) = procgen::terrain::generate(params);
    let elapsed = started.elapsed();

    info!(
        "terrain generated in {:.2} s: {:.1} m to {:.1} m, {:.1}% water, {:.1} MiB",
        elapsed.as_secs_f32(),
        stats.min_m,
        stats.max_m,
        stats.water_fraction * 100.0,
        field.memory_bytes() as f32 / (1024.0 * 1024.0),
    );

    // One material for every chunk. Vertex colours carry the terrain appearance, so the base
    // colour is white; blueprint 2.6.2's texture splatting arrives in M1, when CC0 PBR
    // materials get registered in assets/LICENSES.md.
    streaming.material = Some(materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.92,
        reflectance: 0.06,
        ..default()
    }));

    commands.insert_resource(TerrainWorld {
        field,
        params,
        stats,
        generation_seconds: elapsed.as_secs_f32(),
    });
}

/// Recomputes the node selection when the camera has moved enough to change it.
fn reevaluate_lod(
    settings: Res<TerrainLodSettings>,
    mut streaming: ResMut<TerrainStreaming>,
    mut commands: Commands,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
) {
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let position = camera.translation();

    let settings_changed = streaming.last_settings != Some(*settings);
    let moved_enough = match streaming.last_eval {
        None => true,
        Some(previous) => {
            let horizontal =
                ((position.x - previous.x).powi(2) + (position.z - previous.z).powi(2)).sqrt();
            horizontal > REEVALUATE_DISTANCE_M
                || (position.y - previous.y).abs() > REEVALUATE_ALTITUDE_M
        }
    };

    if !settings_changed && !moved_enough {
        return;
    }

    let selection = lod::select_nodes(position.x, position.y, position.z, &settings.0);

    // Despawn whatever is no longer selected. Dropping the entity drops its mesh handle, which
    // releases the mesh asset.
    let mut to_despawn = Vec::new();
    for (node, entity) in &streaming.loaded {
        if selection.binary_search(node).is_err() {
            to_despawn.push((*node, *entity));
        }
    }
    for (node, entity) in to_despawn {
        commands.entity(entity).despawn();
        streaming.loaded.remove(&node);
    }

    // Queue whatever is newly selected, nearest first so detail appears where the player is
    // looking before it appears at the horizon.
    streaming.pending.clear();
    let mut newly_needed: Vec<QuadNode> = selection
        .into_iter()
        .filter(|node| !streaming.loaded.contains_key(node))
        .collect();
    newly_needed.sort_by(|a, b| {
        let da = lod::horizontal_distance_to_node(*a, position.x, position.z);
        let db = lod::horizontal_distance_to_node(*b, position.x, position.z);
        da.total_cmp(&db).then_with(|| a.cmp(b))
    });
    streaming.pending.extend(newly_needed);

    streaming.last_eval = Some(position);
    streaming.last_settings = Some(*settings);
}

/// Builds up to [`MESH_BUILDS_PER_FRAME`] queued chunk meshes.
fn build_pending_chunks(
    world: Option<Res<TerrainWorld>>,
    mut streaming: ResMut<TerrainStreaming>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    let Some(world) = world else {
        return;
    };
    let Some(material) = streaming.material.clone() else {
        return;
    };

    for _ in 0..MESH_BUILDS_PER_FRAME {
        let Some(node) = streaming.pending.pop_front() else {
            break;
        };
        if streaming.loaded.contains_key(&node) {
            continue;
        }

        let data = mesh::build_node_mesh(&world.field, node);
        let handle = meshes.add(to_bevy_mesh(&data));
        let (origin_x, origin_z) = node.world_min();

        let entity = commands
            .spawn((
                Mesh3d(handle),
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(origin_x, 0.0, origin_z),
                TerrainChunk { node },
            ))
            .id();

        streaming.loaded.insert(node, entity);
    }
}

fn update_stats(streaming: Res<TerrainStreaming>, mut stats: ResMut<TerrainRenderStats>) {
    stats.loaded_chunks = streaming.loaded.len();
    stats.pending_chunks = streaming.pending.len();
    stats.triangles = streaming.loaded.len() as u64 * u64::from(mesh::TRIS_PER_NODE);
}

/// Copies generated geometry into a Bevy mesh.
fn to_bevy_mesh(data: &mesh::TerrainMeshData) -> Mesh {
    // MAIN_WORLD is kept alongside RENDER_WORLD so Bevy can compute the bounding box used for
    // frustum culling, and so terraforming can mutate the mesh in place from M1.
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, data.positions.clone())
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, data.normals.clone())
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, data.colors.clone())
    .with_inserted_indices(Indices::U32(data.indices.clone()))
}
