//! Screenshot capture, bound to `F12` (blueprint 2.19 step 4).
//!
//! # Design
//!
//! Files land in `screenshots/` beside the executable's working directory, named by capture
//! order and by the in-game hour, because the six screenshots the milestone report needs
//! (dawn, noon, dusk, night, 5 km altitude, street level) are distinguished by time of day.
//! Having the hour in the filename means the report can be assembled without opening each
//! image to work out which is which.
//!
//! `screenshots/` is git-ignored: these are build output, not assets, so they need no row in
//! `assets/LICENSES.md`.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use world_model::GameTime;

/// Directory screenshots are written to, relative to the working directory.
pub const SCREENSHOT_DIR: &str = "screenshots";

/// Adds `F12` screenshot capture.
pub struct CapturePlugin;

impl Plugin for CapturePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, capture_on_f12);
    }
}

/// Builds the path for the next screenshot.
///
/// Separated from the system so it can be tested without a window or a GPU.
#[must_use]
pub fn screenshot_path(counter: u32, clock: GameTime) -> PathBuf {
    let hour = clock.hour_of_day();
    let hours = hour.trunc() as u32;
    let minutes = ((hour - hour.trunc()) * 60.0) as u32;
    PathBuf::from(SCREENSHOT_DIR).join(format!(
        "opencity-{counter:03}-d{day}-{hours:02}h{minutes:02}.png",
        day = clock.day()
    ))
}

fn capture_on_f12(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    clock: Res<crate::clock::Clock>,
    mut counter: Local<u32>,
) {
    if !keys.just_pressed(KeyCode::F12) {
        return;
    }

    // Create the directory up front: `save_to_disk` reports a failure through an error log
    // that is easy to miss, and a missing directory is the one failure that is certain to
    // happen on a fresh clone.
    if let Err(error) = std::fs::create_dir_all(SCREENSHOT_DIR) {
        error!("could not create {SCREENSHOT_DIR}: {error}");
        return;
    }

    let path = screenshot_path(*counter, clock.time);
    *counter += 1;

    info!("capturing screenshot to {}", path.display());
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_carries_the_counter_day_and_in_game_time() {
        let path = screenshot_path(7, GameTime::from_day_and_hour(2, 13.5));
        let name = path
            .file_name()
            .expect("has a filename")
            .to_string_lossy()
            .to_string();
        assert_eq!(name, "opencity-007-d2-13h30.png");
        assert_eq!(
            path.parent().expect("has a parent"),
            std::path::Path::new(SCREENSHOT_DIR)
        );
    }

    #[test]
    fn filenames_sort_in_capture_order() {
        // Zero-padding matters: without it, 10 sorts before 2 and the report's screenshots
        // end up in the wrong order.
        let clock = GameTime::from_day_and_hour(0, 6.0);
        let mut names: Vec<String> = [2_u32, 10, 1]
            .iter()
            .map(|c| {
                screenshot_path(*c, clock)
                    .file_name()
                    .expect("has a filename")
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                "opencity-001-d0-06h00.png",
                "opencity-002-d0-06h00.png",
                "opencity-010-d0-06h00.png",
            ]
        );
    }

    #[test]
    fn times_of_day_produce_distinguishable_names() {
        // The six report screenshots must not collide.
        let hours = [6.0, 12.0, 18.5, 23.0];
        let names: Vec<String> = hours
            .iter()
            .enumerate()
            .map(|(i, h)| {
                screenshot_path(i as u32, GameTime::from_day_and_hour(0, *h))
                    .file_name()
                    .expect("has a filename")
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        assert_eq!(names[0], "opencity-000-d0-06h00.png");
        assert_eq!(names[1], "opencity-001-d0-12h00.png");
        assert_eq!(names[2], "opencity-002-d0-18h30.png");
        assert_eq!(names[3], "opencity-003-d0-23h00.png");
    }

    #[test]
    fn midnight_does_not_produce_hour_24() {
        let name = screenshot_path(0, GameTime::from_day_and_hour(1, 0.0))
            .file_name()
            .expect("has a filename")
            .to_string_lossy()
            .to_string();
        assert!(name.contains("-00h00"), "{name}");
        assert!(name.contains("-d1-"), "{name}");
    }
}
