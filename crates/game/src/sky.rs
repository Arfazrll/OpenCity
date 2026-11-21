//! Sky, sun and the day-night cycle (blueprint 2.19 step 7, decision log A13 and A15).
//!
//! # Design
//!
//! The sun is a directional light whose direction comes from the game clock, and the sky is
//! Bevy's procedural atmosphere. Those two are not independent: the atmosphere reads the same
//! light to decide what colour the sky is, so moving the sun turns midday blue into sunset
//! orange and then into night without any of that being authored.
//!
//! That is why this is worth doing properly in M0 rather than faking it with a gradient. A
//! skybox texture would need the owner to look at six images and would need an asset licence;
//! the atmosphere needs neither and reacts to the clock for free.
//!
//! # Sun path
//!
//! A simple tilted arc, not an astronomical model. The city has no latitude or date, and
//! nothing in the blueprint asks for one; what matters is that the sun rises in the east at
//! 06:00, is highest at noon, sets in the west at 18:00, and that its arc is tilted so that
//! midday shadows have direction instead of collapsing to nothing underfoot.
//!
//! # Night
//!
//! The directional light fades out as the sun approaches the horizon, rather than snapping
//! off. Without the fade, dusk ends with a hard switch from full sunlight to black; with it,
//! the last of the light reddens and dims as the atmosphere scatters it.

use bevy::light::atmosphere::ScatteringMedium;
use bevy::light::{Atmosphere, CascadeShadowConfig, CascadeShadowConfigBuilder};
use bevy::prelude::*;
use world_model::GameTime;

use crate::clock::Clock;
use crate::preset::GraphicsPreset;

/// How far the sun's arc is tilted away from passing directly overhead.
///
/// Zero would put the sun exactly at the zenith at noon, which flattens every surface because
/// shadows vanish straight down. 0.42 keeps a usable shadow direction all day.
const SUN_ARC_TILT: f32 = 0.42;

/// Sun elevation, in units of the normalised arc, at which the light is fully out.
///
/// Slightly below the horizon, so the last light lingers after the sun has visibly set.
const NIGHT_ELEVATION: f32 = -0.12;

/// Sun elevation at which the light reaches full strength.
const DAY_ELEVATION: f32 = 0.18;

/// Direction **towards** the sun at a given hour of the game day.
///
/// Returns a unit vector. A negative `y` means the sun is below the horizon.
#[must_use]
pub fn sun_direction(hour: f32) -> Vec3 {
    // Map the day onto a full turn, offset so that 06:00 puts the sun on the eastern horizon.
    let phi = (hour / 24.0) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
    Vec3::new(phi.cos(), phi.sin(), SUN_ARC_TILT).normalize()
}

/// How strong the sunlight is at a given hour, from 0 at night to 1 in full day.
#[must_use]
pub fn sun_intensity(hour: f32) -> f32 {
    let elevation = sun_direction(hour).y;
    let t = ((elevation - NIGHT_ELEVATION) / (DAY_ELEVATION - NIGHT_ELEVATION)).clamp(0.0, 1.0);
    // Smoothstep, so dawn and dusk ease rather than ramping linearly.
    t * t * (3.0 - 2.0 * t)
}

/// Whether the sun is above the horizon at a given hour.
#[must_use]
pub fn is_daytime(hour: f32) -> bool {
    sun_direction(hour).y > 0.0
}

/// Marks the sun entity.
#[derive(Component)]
pub struct Sun;

/// Adds the procedural sky and the sun.
pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_sky)
            .add_systems(Update, (drive_sun, apply_shadow_preset));
    }
}

fn spawn_sky(
    mut commands: Commands,
    mut mediums: ResMut<Assets<ScatteringMedium>>,
    clock: Res<Clock>,
    preset: Res<GraphicsPreset>,
) {
    // The atmosphere itself: a scattering medium describing Earth's air, shared by every
    // camera that renders sky.
    let earth = mediums.add(ScatteringMedium::earth(256, 256));
    commands.spawn(Atmosphere::earth(earth));

    let config = preset.config();
    let hour = clock.time.hour_of_day();

    commands.spawn((
        Sun,
        DirectionalLight {
            shadow_maps_enabled: true,
            // RAW_SUNLIGHT is sunlight before the atmosphere filters it, which is what the
            // atmosphere expects to be handed; using a pre-filtered value would double-count
            // the attenuation and leave the world dim.
            illuminance: bevy::light::light_consts::lux::RAW_SUNLIGHT * sun_intensity(hour),
            ..default()
        },
        bevy::light::SunDisk::EARTH,
        cascade_config(&config),
        Transform::from_translation(sun_direction(hour)).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Builds the shadow cascade configuration for a preset (blueprint 2.6.8).
fn cascade_config(config: &crate::preset::PresetConfig) -> CascadeShadowConfig {
    CascadeShadowConfigBuilder {
        num_cascades: config.shadow_cascades,
        // Shadows only need to cover what the player is looking at closely. Pushing this to
        // the terrain view distance would spread the same shadow map over 8 km and make every
        // shadow a blurry smear.
        maximum_distance: 2_600.0,
        first_cascade_far_bound: 180.0,
        minimum_distance: 1.0,
        ..default()
    }
    .build()
}

/// Moves the sun and adjusts its brightness to match the game clock.
fn drive_sun(
    clock: Res<Clock>,
    mut sun: Query<(&mut Transform, &mut DirectionalLight), With<Sun>>,
) {
    let hour = clock.time.hour_of_day();
    let direction = sun_direction(hour);
    let intensity = sun_intensity(hour);

    for (mut transform, mut light) in &mut sun {
        *transform = Transform::from_translation(direction).looking_at(Vec3::ZERO, Vec3::Y);
        light.illuminance = bevy::light::light_consts::lux::RAW_SUNLIGHT * intensity;
        // Casting shadows from a light with no illuminance is wasted work on a machine that
        // cannot spare it.
        light.shadow_maps_enabled = intensity > 0.01;
    }
}

/// Reapplies the cascade configuration when the graphics preset changes.
fn apply_shadow_preset(
    preset: Res<GraphicsPreset>,
    mut sun: Query<&mut CascadeShadowConfig, With<Sun>>,
) {
    if !preset.is_changed() {
        return;
    }
    let config = cascade_config(&preset.config());
    for mut cascades in &mut sun {
        *cascades = config.clone();
    }
}

/// A readable label for the time of day, for the HUD.
#[must_use]
pub fn time_of_day_label(time: GameTime) -> &'static str {
    match time.hour_of_day() {
        h if h < 5.0 => "night",
        h if h < 7.5 => "dawn",
        h if h < 11.0 => "morning",
        h if h < 14.0 => "midday",
        h if h < 17.0 => "afternoon",
        h if h < 19.5 => "dusk",
        h if h < 21.5 => "evening",
        _ => "night",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sun_rises_in_the_east_and_sets_in_the_west() {
        let sunrise = sun_direction(6.0);
        let sunset = sun_direction(18.0);

        // At 06:00 and 18:00 the sun sits on the horizon.
        assert!(sunrise.y.abs() < 0.05, "sunrise elevation {}", sunrise.y);
        assert!(sunset.y.abs() < 0.05, "sunset elevation {}", sunset.y);

        // East at dawn, west at dusk, and genuinely on opposite sides.
        assert!(sunrise.x > 0.5, "sunrise x {}", sunrise.x);
        assert!(sunset.x < -0.5, "sunset x {}", sunset.x);
    }

    #[test]
    fn the_sun_is_highest_at_noon_and_lowest_at_midnight() {
        let noon = sun_direction(12.0).y;
        let midnight = sun_direction(0.0).y;
        assert!(noon > 0.8, "noon elevation {noon}");
        assert!(midnight < -0.8, "midnight elevation {midnight}");

        // And it is a smooth arc, not a step: every hour between dawn and noon is higher
        // than the one before.
        let mut previous = sun_direction(6.0).y;
        let mut hour = 6.5;
        while hour <= 12.0 {
            let elevation = sun_direction(hour).y;
            assert!(elevation > previous, "elevation dipped at {hour}");
            previous = elevation;
            hour += 0.5;
        }
    }

    #[test]
    fn the_arc_is_tilted_so_midday_shadows_still_have_direction() {
        // A sun exactly overhead at noon flattens everything, because shadows collapse to
        // nothing underfoot.
        let noon = sun_direction(12.0);
        assert!(noon.y < 0.999, "noon sun is at the zenith: {noon:?}");
        assert!(
            noon.z.abs() > 0.1,
            "noon sun has no horizontal component: {noon:?}"
        );
    }

    #[test]
    fn sun_direction_is_always_a_unit_vector() {
        let mut hour = 0.0_f32;
        while hour < 24.0 {
            let d = sun_direction(hour);
            assert!(
                (d.length() - 1.0).abs() < 1e-5,
                "hour {hour}: length {}",
                d.length()
            );
            hour += 0.37;
        }
    }

    #[test]
    fn the_day_is_a_closed_cycle() {
        // Midnight at the start and end of a day must be the same sun position, or the sky
        // would jump every midnight.
        let start = sun_direction(0.0);
        let end = sun_direction(24.0);
        assert!((start - end).length() < 1e-4, "{start:?} vs {end:?}");
    }

    #[test]
    fn light_is_full_at_midday_and_out_at_night() {
        assert!(sun_intensity(12.0) > 0.99, "midday {}", sun_intensity(12.0));
        assert_eq!(sun_intensity(0.0), 0.0, "midnight");
        assert_eq!(sun_intensity(23.0), 0.0, "late night");
        assert_eq!(sun_intensity(2.0), 0.0, "small hours");
    }

    #[test]
    fn dawn_and_dusk_fade_rather_than_snapping() {
        // The fade is what stops dusk ending with a hard cut to black.
        let dawn = sun_intensity(6.3);
        assert!((0.0..1.0).contains(&dawn), "dawn intensity {dawn}");

        // Intensity rises monotonically through the morning.
        let mut previous = 0.0;
        let mut hour = 5.0_f32;
        while hour <= 9.0 {
            let intensity = sun_intensity(hour);
            assert!(intensity >= previous, "intensity fell at {hour}");
            previous = intensity;
            hour += 0.25;
        }
        assert!(previous > 0.99, "never reached full daylight by 09:00");
    }

    #[test]
    fn intensity_stays_in_range_across_the_whole_day() {
        let mut hour = 0.0_f32;
        while hour < 24.0 {
            let intensity = sun_intensity(hour);
            assert!((0.0..=1.0).contains(&intensity), "hour {hour}: {intensity}");
            hour += 0.13;
        }
    }

    #[test]
    fn daytime_matches_the_sun_being_above_the_horizon() {
        assert!(is_daytime(12.0));
        assert!(is_daytime(9.0));
        assert!(!is_daytime(0.0));
        assert!(!is_daytime(22.0));
        // The boundaries land close to 06:00 and 18:00.
        assert!(!is_daytime(5.9));
        assert!(is_daytime(6.1));
        assert!(is_daytime(17.9));
        assert!(!is_daytime(18.1));
    }

    #[test]
    fn the_bookmark_hours_land_in_the_part_of_day_they_claim() {
        // The four time-of-day screenshots must actually look like dawn, noon, dusk and
        // night, which means the sun has to be where their names say.
        assert_eq!(
            time_of_day_label(GameTime::from_day_and_hour(0, 6.1)),
            "dawn"
        );
        assert_eq!(
            time_of_day_label(GameTime::from_day_and_hour(0, 12.0)),
            "midday"
        );
        assert_eq!(
            time_of_day_label(GameTime::from_day_and_hour(0, 18.4)),
            "dusk"
        );
        assert_eq!(
            time_of_day_label(GameTime::from_day_and_hour(0, 22.5)),
            "night"
        );

        // And the light level has to match: dawn dim, noon full, night dark.
        assert!(sun_intensity(6.1) < 0.5, "dawn is too bright");
        assert!(sun_intensity(12.0) > 0.99, "noon is too dim");
        assert!(sun_intensity(18.4) < 0.5, "dusk is too bright");
        assert_eq!(sun_intensity(22.5), 0.0, "night is not dark");
    }

    #[test]
    fn every_hour_has_a_label() {
        let mut hour = 0.0_f32;
        while hour < 24.0 {
            let label = time_of_day_label(GameTime::from_day_and_hour(0, f64::from(hour)));
            assert!(!label.is_empty(), "hour {hour} has no label");
            hour += 0.25;
        }
    }
}
