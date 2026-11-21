//! Procedural generators for OpenCity: terrain, buildings, roads and vegetation.
//!
//! # Design
//!
//! Everything this crate produces is a pure function of a seed plus a definition. That matters
//! for three reasons: a save file can store the seed instead of 32 MB of heightfield, snapshot
//! tests can pin generated output, and the same terrain appears identically on every machine.
//!
//! Randomness therefore comes from seeded ChaCha streams, never from the OS, matching the
//! determinism rules in `sim-core`.
//!
//! # Asset policy (blueprint 2.9, hard rule 3.2 item 2)
//!
//! Generating content here is the preferred alternative to shipping binaries: it keeps the
//! repository small and sidesteps asset licensing entirely. Output of this crate is MIT, like
//! the code that produced it.

#![warn(missing_docs)]
