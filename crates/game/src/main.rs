//! OpenCity game binary: app setup, state machine, tools, cameras and save/load.
//!
//! # Design
//!
//! This is the only crate that wires everything together. It owns the Bevy `App`, drives the
//! simulation thread, translates input into undoable commands, and hosts the camera modes.
//!
//! Keeping the composition in one place means every other crate can stay a library that is
//! testable on its own.
//!
//! # State of play: M0 step 4 (app shell)
//!
//! What exists: window, adapter logging, FPS and frame-time overlay with 1% lows, vsync
//! toggle, `F12` screenshots, graphics presets switchable at runtime, and a game clock.
//!
//! What does not exist yet: terrain (step 5), the city camera (step 7), sky and shadows
//! (step 8), roads (step 9) and buildings (step 10). The placeholder camera below is a
//! stand-in so the window is not empty; step 7 replaces it.

mod capture;
mod clock;
mod hud;
mod preset;
mod vsync;

use bevy::diagnostic::DiagnosticsStore;
use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;
use bevy::window::{PresentMode, PrimaryWindow, WindowResolution};

use preset::GraphicsPreset;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "OpenCity - M0 preview".into(),
                        present_mode: PresentMode::AutoVsync,
                        // The dev laptop runs Windows at 150% scaling. Without this override
                        // Bevy sizes the surface in logical pixels, so a window that looks
                        // 1920x1200 would actually render 1280x800 and every FPS number in
                        // the milestone report would be measured at the wrong resolution.
                        resolution: WindowResolution::default().with_scale_factor_override(1.0),
                        ..default()
                    }),
                    ..default()
                })
                // Bevy's own log line already carries the adapter, but at DEBUG level in some
                // builds. Keep INFO so `log_adapter_info` below is not the only record.
                .set(bevy::log::LogPlugin {
                    level: bevy::log::Level::INFO,
                    filter: "wgpu=warn,naga=warn".into(),
                    ..default()
                }),
        )
        .init_resource::<GraphicsPreset>()
        .add_plugins((clock::ClockPlugin::default(), vsync::VsyncPlugin))
        .add_plugins((hud::HudPlugin, capture::CapturePlugin))
        .add_systems(Startup, (log_environment, spawn_placeholder_scene))
        .add_systems(Update, switch_preset)
        .run();
}

/// Logs the graphics adapter actually chosen by wgpu (blueprint 2.19 step 1).
///
/// `docs/reports/M0-environment.md` records what Windows reports; this records what the
/// renderer really picked, which is the authoritative value and can differ — a machine with
/// two adapters, or a driver that exposes DX12 and Vulkan both, will not always choose what
/// `dxdiag` lists first.
fn log_environment(adapter: Res<RenderAdapterInfo>, windows: Query<&Window, With<PrimaryWindow>>) {
    info!("--- OpenCity environment ---");
    info!("adapter      {}", adapter.name);
    info!("backend      {:?}", adapter.backend);
    info!("device type  {:?}", adapter.device_type);
    info!("driver       {} {}", adapter.driver, adapter.driver_info);

    for window in &windows {
        info!(
            "window       {:.0}x{:.0} physical, scale factor {:.2}",
            window.resolution.physical_width(),
            window.resolution.physical_height(),
            window.resolution.scale_factor(),
        );
    }

    let preset = GraphicsPreset::default();
    info!(
        "preset       {} ({})",
        preset.name(),
        preset.config().target
    );
    info!("--- press F1-F4 to switch preset, V for vsync, F12 to screenshot ---");
}

/// A placeholder camera and light so the window shows something before terrain exists.
///
/// Step 7 replaces this with the real city camera, and step 8 with the real sun.
fn spawn_placeholder_scene(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 400.0, 600.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Switches graphics preset at runtime with `F1`-`F4` (blueprint 2.19 step 4).
fn switch_preset(
    keys: Res<ButtonInput<KeyCode>>,
    mut preset: ResMut<GraphicsPreset>,
    diagnostics: Res<DiagnosticsStore>,
) {
    /// `F1`-`F4`, in the same order as [`GraphicsPreset::ALL`].
    const PRESET_KEYS: [KeyCode; 4] = [KeyCode::F1, KeyCode::F2, KeyCode::F3, KeyCode::F4];

    let requested = PRESET_KEYS
        .iter()
        .zip(GraphicsPreset::ALL)
        .find(|(key, _)| keys.just_pressed(**key))
        .map(|(_, preset)| preset);

    if let Some(new) = requested
        && new != *preset
    {
        // Log the frame rate the old preset was achieving: switching presets to compare is
        // exactly what this key is for, and having both numbers in the log makes the
        // milestone FPS table something to transcribe rather than re-measure.
        let fps = diagnostics
            .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
            .and_then(bevy::diagnostic::Diagnostic::smoothed)
            .unwrap_or(f64::NAN);
        info!(
            "preset {} -> {} (was {:.1} fps)",
            preset.name(),
            new.name(),
            fps
        );
        *preset = new;
    }
}
