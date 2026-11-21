//! Graphics presets (blueprint 2.6.8).
//!
//! # Design
//!
//! A preset is **data**, not behaviour: [`PresetConfig`] is a plain struct of numbers that the
//! render systems read. That keeps the preset table in one readable place next to the
//! blueprint's own table, and it means switching preset at runtime is just swapping a
//! resource — no plugin rebuild, no window recreation.
//!
//! Every field stays individually adjustable under Advanced later (blueprint 2.6.8); the
//! preset only supplies the starting values.
//!
//! # Honesty about what is wired up
//!
//! M0 implements the app shell, so only the fields the shell can act on are applied today:
//! render scale and anti-aliasing mode are read by the camera, and the rest are carried so
//! that later milestones have one place to read them from. Each field below says which
//! milestone consumes it. Nothing here pretends to do more than it does.

use bevy::prelude::*;

/// The four graphics presets from blueprint 2.6.8.
///
/// `Low` is the default on this project's dev laptop, which is a Minimum-tier machine
/// (integrated Xe-LPG graphics sharing single-channel memory with the CPU).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GraphicsPreset {
    /// Integrated GPU class. 1080p/30, central 8 km only.
    #[default]
    Low,
    /// Entry discrete GPU class. 1080p/60.
    Medium,
    /// Recommended tier (GTX 1660 / RTX 3050 class). 1080p/60, full plot.
    High,
    /// RTX 3070 class or better. 1440p/60.
    Ultra,
}

/// Anti-aliasing method selected by a preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntiAliasing {
    /// Cheapest, blurriest. Low preset.
    Fxaa,
    /// Better edge quality at modest cost. Medium preset.
    Smaa,
    /// Temporal; best quality, needs motion vectors and a stable frame rate.
    Taa,
}

/// Reflection technique selected by a preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reflections {
    /// Reflection probes and sky reflection only. Low and Medium.
    ProbeOrSky,
    /// Screen-space reflections on water surfaces. High.
    SsrOnWater,
    /// Screen-space reflections on everything. Ultra.
    SsrOnAll,
}

/// Screen-space ambient occlusion quality selected by a preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsaoQuality {
    /// Disabled. Low preset.
    Off,
    /// Reduced sample count. Medium preset.
    Low,
    /// Default sample count. High preset.
    On,
    /// Increased sample count. Ultra preset.
    High,
}

/// The resolved numbers behind a [`GraphicsPreset`].
///
/// Mirrors the table in blueprint 2.6.8 one field per row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PresetConfig {
    /// Fraction of the window resolution the 3D scene renders at. Applied from M0.
    pub render_scale: f32,
    /// Shadow map resolution in pixels per cascade. Applied from M0 step 8.
    pub shadow_map_size: u32,
    /// Number of shadow cascades for the sun. Applied from M0 step 8.
    pub shadow_cascades: usize,
    /// Whether contact shadows are added on top of the cascades. M2.
    pub contact_shadows: bool,
    /// Ambient occlusion quality. M2.
    pub ssao: SsaoQuality,
    /// Reflection technique. M1, when water arrives.
    pub reflections: Reflections,
    /// Anti-aliasing method. Applied from M0 step 8.
    pub anti_aliasing: AntiAliasing,
    /// Distance at which buildings stop being drawn, in metres. M2.
    pub building_view_distance_m: f32,
    /// Maximum number of fully simulated Tier 2 agents (blueprint 2.5.10). M4.
    pub tier2_agent_cap: u32,
    /// Half-extent of the active city area, in metres. Applied from M0 step 5.
    ///
    /// `Low` caps this to the central quarter of the plot, which is the escape hatch that
    /// makes a 16 km world tractable on an integrated GPU.
    pub active_half_extent_m: f32,
    /// How aggressively terrain subdivides: a node splits when the camera is nearer than
    /// `node_size * this`. The "LOD bias per preset" of blueprint 2.6.7. Applied from M0.
    pub terrain_subdivide_factor: f32,
    /// Distance at which terrain stops being drawn, in metres. Applied from M0.
    pub terrain_view_distance_m: f32,
    /// Human-readable target this preset aims at, for the HUD and the milestone report.
    pub target: &'static str,
}

impl GraphicsPreset {
    /// All presets in ascending order of cost, for cycling and for the settings screen.
    pub const ALL: [Self; 4] = [Self::Low, Self::Medium, Self::High, Self::Ultra];

    /// Short name for the HUD and for log lines.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Ultra => "Ultra",
        }
    }

    /// The numbers behind this preset (blueprint 2.6.8).
    #[must_use]
    pub fn config(self) -> PresetConfig {
        // `world_model::plot` owns the plot geometry; read the half-extents from it rather
        // than repeating 8192.0 here, so ADR 0001 stays the single source of truth.
        use world_model::plot::{LOW_PRESET_HALF_EXTENT_M, PLOT_HALF_EXTENT_M};

        match self {
            Self::Low => PresetConfig {
                render_scale: 0.75,
                shadow_map_size: 1_024,
                shadow_cascades: 2,
                contact_shadows: false,
                ssao: SsaoQuality::Off,
                reflections: Reflections::ProbeOrSky,
                anti_aliasing: AntiAliasing::Fxaa,
                building_view_distance_m: 4_000.0,
                tier2_agent_cap: 5_000,
                active_half_extent_m: LOW_PRESET_HALF_EXTENT_M,
                terrain_subdivide_factor: 2.0,
                terrain_view_distance_m: 8_000.0,
                target: "1080p / 30 fps",
            },
            Self::Medium => PresetConfig {
                render_scale: 1.0,
                shadow_map_size: 2_048,
                shadow_cascades: 3,
                contact_shadows: false,
                ssao: SsaoQuality::Low,
                reflections: Reflections::ProbeOrSky,
                anti_aliasing: AntiAliasing::Smaa,
                building_view_distance_m: 8_000.0,
                tier2_agent_cap: 20_000,
                active_half_extent_m: PLOT_HALF_EXTENT_M,
                terrain_subdivide_factor: 2.5,
                terrain_view_distance_m: 12_000.0,
                target: "1080p / 60 fps",
            },
            Self::High => PresetConfig {
                render_scale: 1.0,
                shadow_map_size: 4_096,
                shadow_cascades: 4,
                contact_shadows: false,
                ssao: SsaoQuality::On,
                reflections: Reflections::SsrOnWater,
                anti_aliasing: AntiAliasing::Taa,
                building_view_distance_m: PLOT_HALF_EXTENT_M * 2.0,
                tier2_agent_cap: 50_000,
                active_half_extent_m: PLOT_HALF_EXTENT_M,
                terrain_subdivide_factor: 3.0,
                terrain_view_distance_m: 24_000.0,
                target: "1080p / 60 fps",
            },
            Self::Ultra => PresetConfig {
                render_scale: 1.0,
                shadow_map_size: 4_096,
                shadow_cascades: 4,
                contact_shadows: true,
                ssao: SsaoQuality::High,
                reflections: Reflections::SsrOnAll,
                anti_aliasing: AntiAliasing::Taa,
                // "Whole region" in blueprint 2.6.8. The region is M7; until then this is the
                // whole plot, which is already further than anything is drawn.
                building_view_distance_m: f32::INFINITY,
                tier2_agent_cap: 100_000,
                active_half_extent_m: PLOT_HALF_EXTENT_M,
                terrain_subdivide_factor: 3.5,
                terrain_view_distance_m: 24_000.0,
                target: "1440p / 60 fps",
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_table_matches_blueprint_2_6_8() {
        // Render scale: only Low reduces it.
        assert_eq!(GraphicsPreset::Low.config().render_scale, 0.75);
        for p in [
            GraphicsPreset::Medium,
            GraphicsPreset::High,
            GraphicsPreset::Ultra,
        ] {
            assert_eq!(p.config().render_scale, 1.0, "{} render scale", p.name());
        }

        // Shadows: 1024/2, 2048/3, 4096/4, 4096/4 + contact.
        assert_eq!(
            (
                GraphicsPreset::Low.config().shadow_map_size,
                GraphicsPreset::Low.config().shadow_cascades
            ),
            (1_024, 2)
        );
        assert_eq!(
            (
                GraphicsPreset::Medium.config().shadow_map_size,
                GraphicsPreset::Medium.config().shadow_cascades
            ),
            (2_048, 3)
        );
        assert_eq!(
            (
                GraphicsPreset::High.config().shadow_map_size,
                GraphicsPreset::High.config().shadow_cascades
            ),
            (4_096, 4)
        );
        assert!(GraphicsPreset::Ultra.config().contact_shadows);
        assert!(!GraphicsPreset::High.config().contact_shadows);

        // Tier 2 agent caps from blueprint 2.5.10.
        assert_eq!(GraphicsPreset::Low.config().tier2_agent_cap, 5_000);
        assert_eq!(GraphicsPreset::Medium.config().tier2_agent_cap, 20_000);
        assert_eq!(GraphicsPreset::High.config().tier2_agent_cap, 50_000);
        assert_eq!(GraphicsPreset::Ultra.config().tier2_agent_cap, 100_000);
    }

    #[test]
    fn only_low_shrinks_the_active_city() {
        use world_model::plot::{LOW_PRESET_HALF_EXTENT_M, PLOT_HALF_EXTENT_M};
        assert_eq!(
            GraphicsPreset::Low.config().active_half_extent_m,
            LOW_PRESET_HALF_EXTENT_M
        );
        for p in [
            GraphicsPreset::Medium,
            GraphicsPreset::High,
            GraphicsPreset::Ultra,
        ] {
            assert_eq!(
                p.config().active_half_extent_m,
                PLOT_HALF_EXTENT_M,
                "{}",
                p.name()
            );
        }
        // And Low really is a quarter of the area, not half.
        assert_eq!(LOW_PRESET_HALF_EXTENT_M * 2.0, PLOT_HALF_EXTENT_M);
    }

    #[test]
    fn cost_increases_monotonically_across_presets() {
        // A preset that is cheaper in every respect than a lower one would be a table typo.
        let configs: Vec<PresetConfig> = GraphicsPreset::ALL.iter().map(|p| p.config()).collect();
        for pair in configs.windows(2) {
            assert!(pair[1].shadow_map_size >= pair[0].shadow_map_size);
            assert!(pair[1].shadow_cascades >= pair[0].shadow_cascades);
            assert!(pair[1].tier2_agent_cap > pair[0].tier2_agent_cap);
            assert!(pair[1].render_scale >= pair[0].render_scale);
            assert!(pair[1].terrain_subdivide_factor > pair[0].terrain_subdivide_factor);
            assert!(pair[1].terrain_view_distance_m >= pair[0].terrain_view_distance_m);
        }
    }

    #[test]
    fn terrain_lod_settings_are_plausible_for_every_preset() {
        for p in GraphicsPreset::ALL {
            let c = p.config();
            // A factor below 1 would never subdivide; above 6 would select thousands of nodes.
            assert!(
                (1.5..=6.0).contains(&c.terrain_subdivide_factor),
                "{} subdivide factor {}",
                p.name(),
                c.terrain_subdivide_factor
            );
            // Terrain must be drawn further than buildings, or the skyline floats on nothing.
            assert!(
                c.terrain_view_distance_m >= c.building_view_distance_m.min(16_384.0),
                "{}: terrain {} m, buildings {} m",
                p.name(),
                c.terrain_view_distance_m,
                c.building_view_distance_m
            );
        }
    }

    #[test]
    fn default_is_low_because_the_dev_laptop_is_minimum_tier() {
        assert_eq!(GraphicsPreset::default(), GraphicsPreset::Low);
    }

    #[test]
    fn every_preset_has_a_name_and_a_target() {
        for p in GraphicsPreset::ALL {
            assert!(!p.name().is_empty());
            assert!(p.config().target.contains("fps"), "{}", p.name());
        }
        assert_eq!(GraphicsPreset::ALL.len(), 4);
    }
}
