//! The development HUD: frame rate, frame time, preset and vsync state.
//!
//! # Design
//!
//! Blueprint 2.19 step 4 asks for an "FPS + frame-time overlay", and 2.19 step 10 asks for an
//! FPS table with averages *and* 1% lows. The 1% low is the number that actually matters on an
//! integrated GPU: an average of 35 fps with a 12 fps 1% low feels broken, while a steady
//! 30 fps feels fine. So this overlay reports both from the start rather than making the
//! milestone measurement a separate exercise.
//!
//! Blueprint 2.13 also sets a stutter budget ("no frame above 50 ms"), so the overlay flags
//! the worst recent frame too.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use crate::clock::Clock;
use crate::preset::GraphicsPreset;
use crate::sky;
use crate::vsync::VsyncEnabled;

/// Frames of history kept for the 1% low calculation.
///
/// At 30 fps this is 20 seconds, which is long enough that one hitch does not dominate the
/// percentile but short enough to still reflect what the player is feeling now.
pub const HISTORY_FRAMES: usize = 600;

/// Adds the development HUD.
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin {
            max_history_length: HISTORY_FRAMES,
            // Bevy's own recommendation: 2 / (history + 1).
            smoothing_factor: 2.0 / (HISTORY_FRAMES as f64 + 1.0),
        })
        .add_systems(Startup, spawn_hud)
        .add_systems(Update, update_hud);
    }
}

/// Marks the text span that carries the live statistics.
#[derive(Component)]
struct HudText;

fn spawn_hud(mut commands: Commands) {
    commands
        .spawn((
            Text::new("OpenCity\n"),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            Node {
                position_type: PositionType::Absolute,
                top: px(8),
                left: px(8),
                ..default()
            },
        ))
        .with_child((
            TextSpan::default(),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::srgb(0.75, 0.85, 0.75)),
            HudText,
        ));
}

/// The 1% low frame rate, derived from the slowest 1% of recent frames.
///
/// Returns `None` until there is enough history to make the number mean anything, rather than
/// reporting a confident value computed from three frames.
fn one_percent_low_fps(diagnostics: &DiagnosticsStore) -> Option<f64> {
    let frame_time = diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)?;
    let mut times: Vec<f64> = frame_time.values().copied().collect();
    if times.len() < 100 {
        return None;
    }
    // Slowest first, then take the frame at the 1% mark. `total_cmp` avoids the NaN question
    // entirely.
    times.sort_by(|a, b| b.total_cmp(a));
    let index = times.len() / 100;
    let worst_percentile_ms = times.get(index).copied()?;
    if worst_percentile_ms <= 0.0 {
        return None;
    }
    Some(1_000.0 / worst_percentile_ms)
}

/// The slowest frame in recent history, in milliseconds.
fn worst_frame_ms(diagnostics: &DiagnosticsStore) -> Option<f64> {
    let frame_time = diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)?;
    frame_time
        .values()
        .copied()
        .fold(None, |acc: Option<f64>, v| {
            Some(acc.map_or(v, |a| if v > a { v } else { a }))
        })
}

fn update_hud(
    diagnostics: Res<DiagnosticsStore>,
    preset: Res<GraphicsPreset>,
    vsync: Res<VsyncEnabled>,
    clock: Res<Clock>,
    terrain: Option<Res<render::TerrainRenderStats>>,
    world: Option<Res<render::TerrainWorld>>,
    mut query: Query<&mut TextSpan, With<HudText>>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(bevy::diagnostic::Diagnostic::smoothed);
    let frame_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(bevy::diagnostic::Diagnostic::smoothed);
    let low = one_percent_low_fps(&diagnostics);
    let worst = worst_frame_ms(&diagnostics);

    let config = preset.config();

    for mut span in &mut query {
        let mut out = String::with_capacity(256);

        match (fps, frame_ms) {
            (Some(f), Some(ms)) => {
                out.push_str(&format!("{f:>6.1} fps   {ms:>5.2} ms\n"));
            }
            _ => out.push_str("     -- fps      -- ms\n"),
        }

        match low {
            Some(l) => out.push_str(&format!("{l:>6.1} fps   1% low\n")),
            None => out.push_str("     -- fps   1% low (warming up)\n"),
        }

        if let Some(w) = worst {
            // Blueprint 2.13: no frame above 50 ms during normal play.
            let flag = if w > 50.0 { "  OVER BUDGET" } else { "" };
            out.push_str(&format!("{w:>6.2} ms    worst frame{flag}\n"));
        }

        out.push_str(&format!(
            "\npreset  {} ({})\n  render scale {:.0}%\n  target       {}\nvsync   {}\n",
            preset.name(),
            if preset.is_changed() {
                "changed"
            } else {
                "active"
            },
            config.render_scale * 100.0,
            config.target,
            if vsync.0 { "on" } else { "off" },
        ));

        if let Some(terrain) = terrain.as_deref() {
            out.push_str(&format!(
                "\nterrain {} chunks ({} queued)\n  {} tris\n",
                terrain.loaded_chunks, terrain.pending_chunks, terrain.triangles
            ));
        }
        if let Some(world) = world.as_deref() {
            out.push_str(&format!(
                "  {} seed {}\n  generated in {:.2} s\n  {:.0} m to {:.0} m, {:.0}% water\n",
                world.params.preset.name(),
                world.params.seed,
                world.generation_seconds,
                world.stats.min_m,
                world.stats.max_m,
                world.stats.water_fraction * 100.0,
            ));
        }

        // Blueprint 2.7 asks the HUD for date, time and speed controls. Population and
        // happiness join them once there is a simulation (M3).
        let time = clock.time;
        let hour = time.hour_of_day();
        let minutes = ((hour - hour.trunc()) * 60.0) as u32;
        out.push_str(&format!(
            "\nday {}  {:02}:{:02}  {} ({})\nspeed   {}\n",
            time.day(),
            hour.trunc() as u32,
            minutes,
            sky::time_of_day_label(time),
            if sky::is_daytime(hour) {
                "sun up"
            } else {
                "sun down"
            },
            match clock.speed {
                world_model::SimSpeed::Paused => "paused",
                world_model::SimSpeed::X1 => "1x",
                world_model::SimSpeed::X2 => "2x",
                world_model::SimSpeed::X3 => "3x",
            },
        ));

        out.push_str(
            "\nWASD pan   QE rotate   RF tilt   wheel zoom\n\
             F1-F4 preset   V vsync   Space pause   F12 screenshot\n",
        );

        **span = out;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::diagnostic::{Diagnostic, DiagnosticPath};

    /// Builds a diagnostics store holding the given frame times, newest last.
    fn store_with_frame_times(times: &[f64]) -> DiagnosticsStore {
        let mut store = DiagnosticsStore::default();
        let path: DiagnosticPath = FrameTimeDiagnosticsPlugin::FRAME_TIME;
        store.add(Diagnostic::new(path.clone()).with_max_history_length(HISTORY_FRAMES));
        let diagnostic = store.get_mut(&path).expect("just added");
        for t in times {
            diagnostic.add_measurement(bevy::diagnostic::DiagnosticMeasurement {
                time: bevy::platform::time::Instant::now(),
                value: *t,
            });
        }
        store
    }

    #[test]
    fn one_percent_low_needs_enough_history_before_reporting() {
        // Reporting a 1% low from a handful of frames would be a made-up number.
        let store = store_with_frame_times(&[16.6; 50]);
        assert_eq!(one_percent_low_fps(&store), None);
    }

    #[test]
    fn one_percent_low_reflects_the_slow_frames_not_the_average() {
        // 297 frames at a smooth 10 ms, plus 3 terrible 100 ms frames.
        let mut times = vec![10.0; 297];
        times.extend_from_slice(&[100.0, 100.0, 100.0]);
        let store = store_with_frame_times(&times);

        let low = one_percent_low_fps(&store).expect("300 frames is enough history");
        // index = 300/100 = 3, so the 4th-slowest frame: 10 ms -> 100 fps.
        assert!((low - 100.0).abs() < 1.0, "1% low was {low}");

        // The point of the metric: with one more bad frame the 1% low collapses, while a mean
        // would barely move.
        let mut worse = vec![10.0; 296];
        worse.extend_from_slice(&[100.0, 100.0, 100.0, 100.0]);
        let store = store_with_frame_times(&worse);
        let low = one_percent_low_fps(&store).expect("enough history");
        assert!((low - 10.0).abs() < 1.0, "1% low was {low}");
    }

    #[test]
    fn worst_frame_finds_the_single_slowest_frame() {
        let store = store_with_frame_times(&[10.0, 12.0, 83.0, 11.0]);
        let worst = worst_frame_ms(&store).expect("has measurements");
        assert!((worst - 83.0).abs() < 1e-9);
    }

    #[test]
    fn missing_diagnostics_do_not_panic() {
        let store = DiagnosticsStore::default();
        assert_eq!(one_percent_low_fps(&store), None);
        assert_eq!(worst_frame_ms(&store), None);
    }
}
