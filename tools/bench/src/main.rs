//! Headless benchmark scenarios for OpenCity.
//!
//! # Design
//!
//! Runs the simulation without a window so that tick time, memory and generation time can be
//! measured in CI as well as on the dev laptop (blueprint 2.13). Scenarios scale from a
//! hundred thousand to a million citizens; a regression over 10% fails the check unless the
//! milestone report justifies it.

fn main() {
    println!("bench - no scenarios yet (M1+)");
}
