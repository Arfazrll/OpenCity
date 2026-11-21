//! Headless, deterministic city simulation for OpenCity.
//!
//! # Design
//!
//! This is the half of the game that does not draw anything. It runs a fixed 10 Hz tick on its
//! own thread and communicates with the presentation layer through exactly two channels
//! (blueprint 2.1):
//!
//! - a **command queue** carrying player intent inward, where every command has an inverse;
//! - **snapshots** carrying read-only state outward, double-buffered so reading never blocks
//!   the tick.
//!
//! # Why it is separate
//!
//! Because it has no GPU dependency it can be unit-tested, property-tested and benchmarked
//! headlessly; it can run faster than real time for 2x and 3x speeds; it can be replayed from
//! a command log; and it could later run in a Web Worker for the browser demo.
//!
//! # Determinism rules (blueprint 2.5.2) - these are not optional
//!
//! - No wall-clock time, no thread-timing dependence, no OS randomness.
//! - All randomness from ChaCha streams seeded by `(world_seed, system_id, tick)`.
//! - Never iterate a random-state hash map. Use `Vec`, `BTreeMap`, `IndexMap`, or FxHash with
//!   sorted keys.
//! - Parallel systems must produce order-independent results: reduce into per-thread buffers,
//!   then merge in ID order.
//!
//! # Error handling
//!
//! Blueprint 3.9 forbids `unwrap()` and `expect()` anywhere in this crate outside tests.
//! Fallible operations return `Result` with `thiserror` types.

#![warn(missing_docs)]
