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

/// Consecutive frames terrain must report nothing pending before capture may proceed.
///
/// One frame is not enough. On the frame the bookmark moves the camera, the camera's
/// `GlobalTransform` has not propagated yet, so the renderer's "nothing pending" still
/// describes the *previous* camera's selection. Requiring a run of stable frames lets the new
/// selection be computed and queued before the capture commits to it. See
/// docs/issues/2026-11-21-flaky-screenshot-capture.md.
const STABLE_FRAMES: u32 = 30;

/// Frames to wait after terrain has finished streaming before capturing.
///
/// Streaming finishing is not the same as the frame being finished: shadow cascades and the
/// atmosphere LUTs need a few frames to settle, and capturing earlier produces an image that
/// does not represent the build.
const SETTLE_FRAMES: u32 = 60;

/// Frames to wait after requesting the capture before exiting.
///
/// `save_to_disk` writes asynchronously once the GPU hands the image back; exiting immediately
/// would race it and produce no file.
const WRITE_FRAMES: u32 = 40;

/// Hard limit on how long to wait for terrain, in frames.
///
/// Without this a capture run would hang forever if streaming never drained.
const MAX_WAIT_FRAMES: u32 = 1_200;

/// Progress of an automated capture run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AutoCaptureState {
    /// Waiting for terrain streaming to drain and stay drained.
    WaitingForTerrain {
        /// Total frames spent waiting, for the timeout.
        frames: u32,
        /// Consecutive frames with nothing pending.
        stable: u32,
    },
    /// Waiting for lighting to settle after streaming finished.
    Settling { frames: u32 },
    /// Capture requested; waiting for the file to be written.
    Writing { frames: u32 },
}

/// Drives one automated screenshot and then exits.
///
/// Added only when the binary is started with `--capture <bookmark>`.
#[derive(Resource, Debug)]
pub struct AutoCapture {
    bookmark: &'static crate::bookmarks::Bookmark,
    state: AutoCaptureState,
    applied: bool,
}

/// Captures one bookmark and exits, for the `screenshot-review` workflow.
pub struct AutoCapturePlugin {
    /// The bookmark to capture.
    pub bookmark: &'static crate::bookmarks::Bookmark,
}

impl Plugin for AutoCapturePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AutoCapture {
            bookmark: self.bookmark,
            state: AutoCaptureState::WaitingForTerrain {
                frames: 0,
                stable: 0,
            },
            applied: false,
        })
        .add_systems(Update, drive_auto_capture);
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "a one-shot driver system that needs the camera, clock, terrain and exit channel"
)]
fn drive_auto_capture(
    mut commands: Commands,
    mut auto: ResMut<AutoCapture>,
    mut clock: ResMut<crate::clock::Clock>,
    cameras: Query<&Transform, With<Camera3d>>,
    mut city_camera: ResMut<crate::camera::CityCamera>,
    terrain_stats: Option<Res<render::TerrainRenderStats>>,
    terrain_world: Option<Res<render::TerrainWorld>>,
    chunks: Query<(&render::terrain::TerrainChunk, &GlobalTransform)>,
    projections: Query<&Projection>,
    mut exit: MessageWriter<AppExit>,
) {
    // Park the clock at the bookmark's hour and move the camera there, once the terrain exists
    // so that ground-relative bookmarks can be resolved.
    if !auto.applied {
        let Some(world) = terrain_world.as_deref() else {
            return;
        };
        let transform = auto
            .bookmark
            .transform(|x, z| world.field.height_at_world(x, z));
        // Set the camera *state*, not the transform: `CityCameraPlugin` recomputes the
        // transform every frame and would overwrite it. `snap_to` also skips the easing, so
        // the capture is not taken mid-glide.
        let target = transform.translation + transform.forward() * 1_000.0;
        let aimed = crate::camera::CityCamera::from_eye_and_target(
            transform.translation,
            aim_at_ground(
                transform.translation,
                transform.forward().as_vec3(),
                |x, z| world.field.height_at_world(x, z),
            )
            .unwrap_or(target),
        );
        *city_camera = aimed;
        clock.time = auto.bookmark.game_time();
        // Freeze time so the capture is reproducible frame to frame.
        clock.speed = world_model::SimSpeed::Paused;
        auto.applied = true;
        info!(
            "auto-capture '{}': camera at {:.0?}, hour {:.1}",
            auto.bookmark.name, transform.translation, auto.bookmark.hour
        );
        return;
    }

    let pending = terrain_stats.as_deref().map_or(1, |s| s.pending_chunks);
    let loaded = terrain_stats.as_deref().map_or(0, |s| s.loaded_chunks);

    auto.state = match auto.state {
        AutoCaptureState::WaitingForTerrain { frames, stable } => {
            if frames > MAX_WAIT_FRAMES {
                warn!(
                    "auto-capture '{}': terrain still streaming after {frames} frames \
                     ({pending} pending); capturing anyway",
                    auto.bookmark.name
                );
                AutoCaptureState::Settling { frames: 0 }
            } else if pending == 0 && loaded > 0 {
                let stable = stable + 1;
                if stable >= STABLE_FRAMES {
                    info!(
                        "auto-capture '{}': {loaded} chunks, stable for {stable} frames \
                         after {frames} total",
                        auto.bookmark.name
                    );
                    AutoCaptureState::Settling { frames: 0 }
                } else {
                    AutoCaptureState::WaitingForTerrain {
                        frames: frames + 1,
                        stable,
                    }
                }
            } else {
                // Streaming restarted, so the stability run has to begin again.
                AutoCaptureState::WaitingForTerrain {
                    frames: frames + 1,
                    stable: 0,
                }
            }
        }

        AutoCaptureState::Settling { frames } if frames == 1 => {
            log_capture_diagnostics(auto.bookmark.name, &cameras, &chunks, &projections);
            AutoCaptureState::Settling { frames: frames + 1 }
        }

        AutoCaptureState::Settling { frames } if frames >= SETTLE_FRAMES => {
            if let Err(error) = std::fs::create_dir_all(SCREENSHOT_DIR) {
                error!("could not create {SCREENSHOT_DIR}: {error}");
                exit.write(AppExit::error());
                return;
            }
            let path = PathBuf::from(SCREENSHOT_DIR).join(format!("{}.png", auto.bookmark.name));
            info!(
                "auto-capture '{}': writing {}",
                auto.bookmark.name,
                path.display()
            );
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
            AutoCaptureState::Writing { frames: 0 }
        }
        AutoCaptureState::Settling { frames } => AutoCaptureState::Settling { frames: frames + 1 },

        AutoCaptureState::Writing { frames } if frames >= WRITE_FRAMES => {
            info!("auto-capture '{}': done", auto.bookmark.name);
            exit.write(AppExit::Success);
            AutoCaptureState::Writing { frames }
        }
        AutoCaptureState::Writing { frames } => AutoCaptureState::Writing { frames: frames + 1 },
    };
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

/// Finds where a view ray meets the ground, so a bookmark's focus point sits on the terrain.
///
/// Bookmarks are written as "stand here, look there", but the city camera orbits a focus point
/// on the ground. Marching the ray down to the surface converts one into the other, and keeps
/// the framing the bookmark author intended.
///
/// Returns `None` if the ray does not descend, or never reaches the ground within the plot.
fn aim_at_ground(
    eye: Vec3,
    direction: Vec3,
    ground_height_at: impl Fn(f32, f32) -> f32,
) -> Option<Vec3> {
    if direction.y >= -0.001 {
        return None;
    }

    // March in fixed steps, then refine. Fixed stepping is robust on a heightfield that the
    // ray may graze; a closed-form plane intersection would miss hills entirely.
    const STEP_M: f32 = 40.0;
    const MAX_DISTANCE_M: f32 = 40_000.0;

    let mut travelled = 0.0_f32;
    let mut previous = eye;
    while travelled < MAX_DISTANCE_M {
        travelled += STEP_M;
        let point = eye + direction * travelled;
        let ground = ground_height_at(point.x, point.z);
        if point.y <= ground {
            // Bisect between the last point above ground and this one below it.
            let mut low = previous;
            let mut high = point;
            for _ in 0..24 {
                let mid = (low + high) * 0.5;
                if mid.y <= ground_height_at(mid.x, mid.z) {
                    high = mid;
                } else {
                    low = mid;
                }
            }
            let hit = (low + high) * 0.5;
            return Some(Vec3::new(hit.x, ground_height_at(hit.x, hit.z), hit.z));
        }
        previous = point;
    }

    None
}

/// Logs what the camera and the terrain actually are at capture time.
///
/// Added while diagnosing an empty 5 km screenshot: the chunk count said terrain was loaded,
/// so the question was whether it was in front of the camera at all. Kept because "the HUD
/// says 22 chunks but the screen is empty" is exactly the situation where guessing wastes the
/// most time.
fn log_capture_diagnostics(
    name: &str,
    cameras: &Query<&Transform, With<Camera3d>>,
    chunks: &Query<(&render::terrain::TerrainChunk, &GlobalTransform)>,
    projections: &Query<&Projection>,
) {
    for camera in cameras.iter() {
        info!(
            "diag '{name}': camera at {:.0?}, forward {:.3?}",
            camera.translation,
            camera.forward().as_vec3()
        );
    }
    for projection in projections.iter() {
        info!("diag '{name}': projection {projection:?}");
    }

    let mut by_level = [0_u32; 8];
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_z = f32::INFINITY;
    let mut max_z = f32::NEG_INFINITY;

    for (chunk, transform) in chunks.iter() {
        let level = chunk.node.level as usize;
        if let Some(slot) = by_level.get_mut(level) {
            *slot += 1;
        }
        let position = transform.translation();
        let size = chunk.node.size_m();
        min_x = min_x.min(position.x);
        max_x = max_x.max(position.x + size);
        min_z = min_z.min(position.z);
        max_z = max_z.max(position.z + size);
    }

    info!(
        "diag '{name}': {} chunk entities, levels {:?}",
        chunks.iter().count(),
        by_level
    );
    info!("diag '{name}': chunks span x {min_x:.0}..{max_x:.0}, z {min_z:.0}..{max_z:.0}");
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
