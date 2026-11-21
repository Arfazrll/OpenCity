//! Radial menu, HUD, panels and settings screens for OpenCity.
//!
//! # Design
//!
//! Mouse-first and original (blueprint 2.7). The radial menu is the primary build interface:
//! right-click or Space opens a ring of categories, and picking one opens a sub-ring of items
//! with thumbnails.
//!
//! All user-visible strings go through a localisation layer even though v1.0 ships English
//! only, so that translation later does not require touching every call site.

#![warn(missing_docs)]
