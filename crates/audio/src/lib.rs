//! Music playlist and ambient soundscape for OpenCity.
//!
//! # Design
//!
//! Two independent layers (blueprint 2.10): a crossfading music playlist on its own volume
//! slider, and an ambient bed whose mix follows camera altitude and the district type below it
//! - traffic hum, crowds, wind, sea, rain, construction.
//!
//! # Asset policy
//!
//! Every track and sample must be CC0, CC-BY or OFL and must have a row in
//! `assets/LICENSES.md` naming its source, author and license. Nothing ripped from another
//! game or product, ever (hard rule 3.2 item 2).

#![warn(missing_docs)]
