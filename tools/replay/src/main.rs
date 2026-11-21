//! Deterministic replay runner for OpenCity.
//!
//! # Design
//!
//! A replay is a save file plus a command log (blueprint 2.5.2). This tool reruns one and
//! compares a state hash per tick against the recorded run. Divergence means something in
//! `sim-core` picked up a dependency on wall-clock time, thread timing, OS randomness or
//! hash-map iteration order.
//!
//! It is quality gate 5 and runs whenever `sim-core` changes.

fn main() {
    println!("replay - no fixtures yet (M3+)");
}
