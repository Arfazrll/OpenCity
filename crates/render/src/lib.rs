//! Bevy render plugins for OpenCity: terrain, water, buildings, roads, agents and overlays.
//!
//! # Design
//!
//! This crate turns simulation snapshots and procedural geometry into what the player sees. It
//! reads state; it never mutates the simulation.
//!
//! The scale is what makes it interesting. A 16 km plot is 4,096 render chunks of 256 m, and
//! the target hardware includes integrated GPUs, so the strategy throughout is to draw less
//! rather than to draw faster (blueprint 2.6):
//!
//! - **Quadtree chunk LOD** over the terrain with crack-free transitions.
//! - **Distance streaming** with separate load and unload radii, biased per graphics preset.
//! - **Instancing** for repeated building modules, vehicles and vegetation.
//! - **Impostors** for distant buildings, and headlight sprites for distant night traffic.

//! # Modules
//!
//! - [`terrain`] - terrain generation, quadtree LOD and chunk streaming

#![warn(missing_docs)]

pub mod terrain;

pub use terrain::{TerrainLodSettings, TerrainPlugin, TerrainRenderStats, TerrainWorld};
