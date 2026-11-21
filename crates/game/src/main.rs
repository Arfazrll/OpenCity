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

mod bookmarks;
mod camera;
mod capture;
mod clock;
mod hud;
mod preset;
mod sky;
mod vsync;

use bevy::diagnostic::DiagnosticsStore;
use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;
use bevy::window::{PresentMode, PrimaryWindow, WindowResolution};
use procgen::terrain::{TerrainParams, TerrainPreset};
use render::terrain::{LodConfig, TerrainLodSettings, TerrainPlugin};

use preset::GraphicsPreset;

/// Terrain the M0 preview starts with.
///
/// Coastal rather than the Flat default: it is the only preset that shows sea, beach, lowland
/// and hills in one view, which is what makes a preview build worth looking at. The other two
/// are reachable by regenerating with a different preset once the city-creation screen exists
/// (M1).
const PREVIEW_TERRAIN: TerrainParams = TerrainParams {
    seed: 20_261_121,
    preset: TerrainPreset::Coastal,
    sea_level_m: 0.0,
    rivers: false,
};

fn main() {
    // Command line, parsed by hand rather than with a dependency: there are exactly two flags
    // and both exist for the screenshot-review workflow, not for players.
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "--list-bookmarks") {
        for name in bookmarks::names() {
            println!("{name}");
        }
        return;
    }

    let capture_bookmark = match args.iter().position(|a| a == "--capture") {
        None => None,
        Some(index) => match args.get(index + 1) {
            None => {
                eprintln!(
                    "--capture needs a bookmark name. Available: {}",
                    bookmarks::names().join(", ")
                );
                std::process::exit(2);
            }
            Some(name) => match bookmarks::find(name) {
                Some(bookmark) => Some(bookmark),
                None => {
                    eprintln!(
                        "unknown bookmark '{name}'. Available: {}",
                        bookmarks::names().join(", ")
                    );
                    std::process::exit(2);
                }
            },
        },
    };

    let mut app = App::new();

    app.add_plugins(
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
    .add_plugins((camera::CityCameraPlugin, sky::SkyPlugin))
    .add_plugins((hud::HudPlugin, capture::CapturePlugin))
    .add_plugins(TerrainPlugin {
        params: PREVIEW_TERRAIN,
    })
    .add_systems(Startup, (log_environment, spawn_camera))
    .add_systems(Update, (switch_preset, sync_terrain_lod));

    if let Some(bookmark) = capture_bookmark {
        app.add_plugins(capture::AutoCapturePlugin { bookmark });
    }

    app.run();
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

/// Initial viewpoint for the preview: over the sea looking inland across the shoreline.
const PREVIEW_EYE: Vec3 = Vec3::new(-3_750.0, 620.0, 1_450.0);

/// Point the preview camera looks at.
const PREVIEW_TARGET: Vec3 = Vec3::new(2_600.0, 180.0, -1_000.0);

/// Spawns the camera and places it at the preview viewpoint.
///
/// The sun and sky live in [`sky`]; the camera's own transform is owned by
/// [`camera::CityCameraPlugin`] from the first frame onward.
fn spawn_camera(mut commands: Commands, mut camera: ResMut<camera::CityCamera>) {
    *camera = camera::CityCamera::from_eye_and_target(PREVIEW_EYE, PREVIEW_TARGET);

    commands.spawn((
        Camera3d::default(),
        // Bevy's default far plane is 1,000 m. This world is 16 km across and the camera
        // reaches 5 km altitude (decision log A12), so the default clips the entire landscape
        // away and renders an empty screen. The far plane has to cover the plot diagonal,
        // which is 23.2 km.
        //
        // The near plane goes to 1 m rather than the default 0.1 m: the street camera's
        // closest approach is 2 m (blueprint 2.6.9), so 10 cm of near precision buys nothing
        // and costs depth-buffer range.
        Projection::Perspective(PerspectiveProjection {
            near: 1.0,
            far: 32_000.0,
            fov: 50.0_f32.to_radians(),
            ..default()
        }),
        // Procedural sky. AtmosphereSettings requires Hdr, and the atmosphere's own aerial
        // perspective replaces the distance fog that stood in for it, so there is no separate
        // fog here any more. AtmosphereEnvironmentMapLight lights surfaces facing away from
        // the sun with scattered sky light, which is what the manual ambient light was
        // standing in for.
        bevy::pbr::AtmosphereSettings::default(),
        bevy::light::AtmosphereEnvironmentMapLight::default(),
        // RAW_SUNLIGHT is about 100,000 lux, so the camera needs a daylight exposure or
        // everything clips to white.
        bevy::camera::Exposure { ev100: 13.0 },
        bevy::core_pipeline::tonemapping::Tonemapping::AcesFitted,
        bevy::post_process::bloom::Bloom::NATURAL,
        // The transform is owned by `CityCameraPlugin` from here on; this is only the first
        // frame's value, before the camera system runs.
        Transform::default(),
    ));
}

/// Maps the active graphics preset onto the renderer's terrain LOD settings.
///
/// `render` deliberately does not know about [`GraphicsPreset`], so this is where the two meet.
fn sync_terrain_lod(preset: Res<GraphicsPreset>, mut settings: ResMut<TerrainLodSettings>) {
    if !preset.is_changed() && !settings.is_added() {
        return;
    }

    let config = preset.config();
    let wanted = TerrainLodSettings(LodConfig {
        subdivide_factor: config.terrain_subdivide_factor,
        max_view_distance_m: config.terrain_view_distance_m,
        active_half_extent_m: config.active_half_extent_m,
        ..LodConfig::default()
    });

    if *settings != wanted {
        *settings = wanted;
    }
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
