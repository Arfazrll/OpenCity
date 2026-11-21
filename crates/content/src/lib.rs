//! RON content definitions for OpenCity: buildings, services, rules and presets.
//!
//! # Design
//!
//! Blueprint 3.9 requires that rules and numbers live in data, not in code, so that
//! playtesting can retune the game without a recompile and so that mod data packs
//! (blueprint 2.12) can override the same fields the base game uses.
//!
//! This crate owns the *schema* (the Rust types and their `serde` derives) and the loader.
//! The actual values live in `.ron` files shipped alongside it. Nothing here evaluates game
//! logic - it only parses and validates.

#![warn(missing_docs)]
