//! Shared world geometry for OpenCity: units, coordinates, chunk math and stable IDs.
//!
//! # Design
//!
//! This crate is the single authority on *where things are*. Terrain generation, render
//! chunking, the simulation grid and the camera all convert through the functions here, so
//! that an off-by-one in one subsystem cannot disagree with another.
//!
//! It deliberately has **no engine dependency**. Nothing here touches Bevy, rendering, audio
//! or wall-clock time, which keeps it usable from `sim-core` (blueprint 3.2 rule 6) and makes
//! every conversion unit-testable without a GPU.
//!
//! # Coordinate layers (blueprint 2.4)
//!
//! A city plot is 16,384 m square, centred on the world origin, and is addressed at four
//! granularities:
//!
//! | Layer | Cell size | Count per axis | Type |
//! | --- | --- | --- | --- |
//! | World space | 1 m (`f32`) | - | plain `f32`, Y up |
//! | Heightfield sample | 4 m | 4,096 | [`SampleCoord`] |
//! | Render chunk | 256 m | 64 | [`ChunkCoord`], [`chunk::QuadNode`] |
//! | Simulation cell | 32 m | 512 | [`CellCoord`] |
//!
//! All units are metric and 1 world unit is exactly 1 metre.
//!
//! The extent is 16,384 m rather than a literal 16,000 m because that is the only value that
//! lets all three grids tile the plot exactly. See
//! [ADR 0001](../../../docs/adr/0001-plot-extent-16384-metres.md).
//!
//! # Modules
//!
//! - [`units`] - [`Meters`], [`Seconds`], [`GameTime`] and [`SimSpeed`]
//! - [`plot`] - plot constants and conversions between every coordinate layer
//! - [`coords`] - the index types for the three discrete layers
//! - [`chunk`] - quadtree addressing for terrain level of detail
//! - [`height`] - fixed-point terrain height encoding
//! - [`heightfield`] - the full-plot elevation buffer and its interpolated lookups
//! - [`ids`] - generational identifiers and the arena that issues them

#![warn(missing_docs)]

pub mod chunk;
pub mod coords;
pub mod height;
pub mod heightfield;
pub mod ids;
pub mod plot;
pub mod units;

pub use chunk::QuadNode;
pub use coords::{CellCoord, ChunkCoord, SampleCoord};
pub use heightfield::Heightfield;
pub use ids::{Id, IdArena};
pub use units::{GameTime, Meters, Seconds, SimSpeed};
