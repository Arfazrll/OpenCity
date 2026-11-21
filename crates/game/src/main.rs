//! OpenCity game binary: app setup, state machine, tools, cameras and save/load.
//!
//! # Design
//!
//! This is the only crate that wires everything together. It owns the Bevy `App`, drives the
//! simulation thread, translates input into undoable commands, and hosts the camera modes.
//!
//! Keeping the composition in one place means every other crate can stay a library that is
//! testable on its own.

fn main() {
    println!("OpenCity - app shell not implemented yet (M0 step 4)");
}
