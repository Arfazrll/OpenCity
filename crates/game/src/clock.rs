//! The game clock and speed controls (blueprint 2.5.1, decision log A30).
//!
//! # Design
//!
//! In the finished game the authoritative clock lives inside `sim-core`, advancing one fixed
//! 10 Hz tick at a time. M0 has no simulation yet, so this resource is the clock: it advances
//! [`GameTime`] from real elapsed time and drives the sun.
//!
//! It is written to be replaced rather than extended. When `sim-core` arrives in M3 the
//! presentation layer will read the clock out of a snapshot instead of advancing it here, and
//! the only thing that should need to change is where [`Clock::time`] comes from. Everything
//! that reads the clock already reads it through this resource.
//!
//! Speed is a player control, not a time scale on the simulation: the tick size never changes,
//! so results are identical at any speed. Here that distinction is simpler than it sounds —
//! there is nothing to tick — but the type is the same [`SimSpeed`] the simulation will use,
//! so the HUD and key bindings do not change later.

use bevy::prelude::*;
use world_model::{GameTime, Seconds, SimSpeed};

/// The current in-game time and speed.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct Clock {
    /// Elapsed in-game time.
    pub time: GameTime,
    /// The player's selected speed.
    pub speed: SimSpeed,
}

/// Adds the game clock and its speed controls.
///
/// Bindings: `Space` pauses and resumes, `1`/`2`/`3` select the speed directly.
pub struct ClockPlugin {
    /// In-game time the world starts at.
    ///
    /// M0 starts mid-morning so that the first thing the owner sees is lit terrain with
    /// visible shadows, rather than darkness.
    pub start: GameTime,
}

impl Default for ClockPlugin {
    fn default() -> Self {
        Self {
            start: GameTime::from_day_and_hour(0, 9.0),
        }
    }
}

impl Plugin for ClockPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Clock {
            time: self.start,
            speed: SimSpeed::default(),
        })
        .add_systems(Update, (advance_clock, speed_controls));
    }
}

fn advance_clock(time: Res<Time>, mut clock: ResMut<Clock>) {
    let delta = Seconds(time.delta_secs());
    let speed = clock.speed;
    clock.time.advance(delta, speed);
}

fn speed_controls(keys: Res<ButtonInput<KeyCode>>, mut clock: ResMut<Clock>) {
    let new_speed = if keys.just_pressed(KeyCode::Space) {
        // Space toggles: pause if running, resume at 1x if paused.
        Some(if clock.speed == SimSpeed::Paused {
            SimSpeed::X1
        } else {
            SimSpeed::Paused
        })
    } else if keys.just_pressed(KeyCode::Digit1) {
        Some(SimSpeed::X1)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(SimSpeed::X2)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(SimSpeed::X3)
    } else {
        None
    };

    if let Some(speed) = new_speed
        && speed != clock.speed
    {
        clock.speed = speed;
        info!("speed {:?}", speed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_start_is_mid_morning_so_the_first_view_is_lit() {
        let plugin = ClockPlugin::default();
        assert_eq!(plugin.start.day(), 0);
        assert!((plugin.start.hour_of_day() - 9.0).abs() < 1e-3);
    }

    #[test]
    fn clock_starts_at_1x_not_paused() {
        let clock = Clock::default();
        assert_eq!(clock.speed, SimSpeed::X1);
    }

    #[test]
    fn a_real_minute_at_1x_advances_three_quarters_of_a_game_day() {
        // 20 real minutes is one game day, so 1 real minute is 1/20 of a day = 1.2 game hours.
        let mut clock = Clock::default();
        for _ in 0..60 {
            clock.time.advance(Seconds(1.0), clock.speed);
        }
        assert!(
            (clock.time.hour_of_day() - 1.2).abs() < 1e-2,
            "{}",
            clock.time.hour_of_day()
        );
    }
}
