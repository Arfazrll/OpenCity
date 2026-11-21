//! Road graph, routing and isochrones for OpenCity.
//!
//! # Design
//!
//! Routing is the single most expensive thing a city simulation does, so it is isolated here
//! where it can be benchmarked on its own. Three ideas keep it affordable at a million
//! citizens (blueprint 2.5.9):
//!
//! 1. **Hierarchy** - long trips route over a coarse graph of high-class roads, not every
//!    residential street.
//! 2. **Caching** - routes are keyed by `(origin district, destination district, time bucket)`
//!    rather than by individual citizen.
//! 3. **Incremental invalidation** - editing one road segment must not discard the whole cache.
//!
//! Like `world-model`, this crate stays free of engine dependencies so it can run inside the
//! deterministic simulation and under `criterion` benchmarks.

#![warn(missing_docs)]
