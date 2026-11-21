//! The city camera (blueprint 2.6.9, decision log A11 and A12).
//!
//! # Design
//!
//! An orbit camera: the player manipulates a **focus point on the ground**, and the camera
//! sits at a distance from it given by yaw, pitch and zoom. That is the right model for a city
//! builder because every control the player reaches for — pan, rotate, zoom, tilt — is natural
//! in terms of "the place I am looking at", and because later features (click an agent and
//! follow it, jump to a notification) are just a change of focus.
//!
//! # Smoothing
//!
//! Every control writes a *target*, and the live values ease toward it each frame. Blueprint
//! 2.6.9 asks for "smooth inertia", and on a machine that may be running at 30 fps a camera
//! that snaps makes the whole game feel worse than the frame rate alone would.
//!
//! The easing is frame-rate independent: `1 - exp(-rate * dt)` rather than a fixed per-frame
//! fraction, so the camera feels identical at 30 and 60 fps. A fixed fraction would make the
//! camera twice as sluggish at half the frame rate, which is exactly backwards.
//!
//! # Altitude, not distance
//!
//! Decision log A12 caps the view at about 5 km, and blueprint 2.6.9 states the limit as an
//! altitude range of 20 m to 5,000 m. Altitude is therefore the quantity that gets clamped,
//! and the orbit distance is derived from it — so tilting towards the horizon at maximum zoom
//! pushes the camera further out rather than letting it climb past the ceiling.
//!
//! # Panning scales with altitude
//!
//! A fixed pan speed is unusable at both ends: crawling at 5 km, uncontrollable at 20 m. Pan
//! speed is proportional to altitude, so a drag moves the view by roughly the same fraction of
//! the screen whatever the zoom.

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;
use world_model::plot;

/// Lowest camera altitude, in metres (blueprint 2.6.9).
pub const MIN_ALTITUDE_M: f32 = plot::CAMERA_MIN_ALTITUDE_M;

/// Highest camera altitude, in metres (decision log A12).
pub const MAX_ALTITUDE_M: f32 = plot::CAMERA_MAX_ALTITUDE_M;

/// Steepest downward pitch, in radians. Just short of straight down, which would make the
/// yaw control meaningless and the up vector ambiguous.
pub const MAX_PITCH: f32 = -1.48; // about -85 degrees

/// Shallowest pitch, in radians. Kept below the horizon so the camera cannot look up into
/// empty space from ground level.
pub const MIN_PITCH: f32 = -0.07; // about -4 degrees

/// How fast the camera eases towards its target, per second.
///
/// Higher is snappier. 12 settles visibly within about a quarter of a second.
pub const SMOOTHING_RATE: f32 = 12.0;

/// Pan speed as a fraction of altitude per second, for keyboard panning.
pub const PAN_RATE_PER_ALTITUDE: f32 = 1.1;

/// Rotation speed in radians per second, for keyboard rotation.
pub const KEY_ROTATE_RATE: f32 = 1.6;

/// Tilt speed in radians per second, for keyboard tilt.
pub const KEY_TILT_RATE: f32 = 1.2;

/// Fraction of altitude changed per unit of scroll.
pub const ZOOM_PER_SCROLL: f32 = 0.16;

/// Mouse drag sensitivity for rotation and tilt, in radians per pixel.
pub const DRAG_SENSITIVITY: f32 = 0.006;

/// Mouse drag sensitivity for panning, as a fraction of altitude per pixel.
pub const DRAG_PAN_SENSITIVITY: f32 = 0.0016;

/// Width of the screen-edge band that triggers edge scrolling, in pixels.
pub const EDGE_SCROLL_MARGIN_PX: f32 = 6.0;

/// The city camera's state.
///
/// Live values are what the camera renders at; target values are what the controls set. The
/// gap between them is the inertia.
#[derive(Resource, Debug, Clone, Copy)]
pub struct CityCamera {
    /// Ground point the camera orbits, in world space.
    pub focus: Vec3,
    /// Rotation about the vertical axis, in radians.
    pub yaw: f32,
    /// Downward tilt, in radians. Negative looks down.
    pub pitch: f32,
    /// Height of the camera above its focus point, in metres.
    pub altitude: f32,

    target_focus: Vec3,
    target_yaw: f32,
    target_pitch: f32,
    target_altitude: f32,
}

impl Default for CityCamera {
    fn default() -> Self {
        Self::new(Vec3::ZERO, 0.6, -0.55, 900.0)
    }
}

impl CityCamera {
    /// Creates a camera with live and target state both set to the given values.
    #[must_use]
    pub fn new(focus: Vec3, yaw: f32, pitch: f32, altitude: f32) -> Self {
        let pitch = pitch.clamp(MAX_PITCH, MIN_PITCH);
        let altitude = altitude.clamp(MIN_ALTITUDE_M, MAX_ALTITUDE_M);
        Self {
            focus,
            yaw,
            pitch,
            altitude,
            target_focus: focus,
            target_yaw: yaw,
            target_pitch: pitch,
            target_altitude: altitude,
        }
    }

    /// Derives camera state from an eye position and a look-at target.
    ///
    /// Lets bookmarks stay written as "stand here, look there", which is how a human thinks
    /// about a viewpoint, while the camera itself works in orbit terms.
    #[must_use]
    pub fn from_eye_and_target(eye: Vec3, target: Vec3) -> Self {
        let to_target = target - eye;
        let horizontal = Vec2::new(to_target.x, to_target.z);
        let horizontal_len = horizontal.length().max(0.001);

        // Bevy's forward is -Z, so a yaw of 0 looks towards -Z.
        let yaw = (-to_target.x).atan2(-to_target.z);
        let pitch = (to_target.y / horizontal_len)
            .atan()
            .clamp(MAX_PITCH, MIN_PITCH);
        let altitude = (eye.y - target.y).max(MIN_ALTITUDE_M);

        Self::new(target, yaw, pitch, altitude)
    }

    /// The orbit distance implied by the current altitude and pitch.
    #[must_use]
    pub fn orbit_distance(&self) -> f32 {
        // altitude = distance * sin(-pitch), so distance = altitude / sin(-pitch).
        let sin_pitch = (-self.pitch).sin().max(0.05);
        self.altitude / sin_pitch
    }

    /// The camera's world-space position.
    #[must_use]
    pub fn eye(&self) -> Vec3 {
        let distance = self.orbit_distance();
        let horizontal = distance * (-self.pitch).cos();
        Vec3::new(
            self.focus.x + horizontal * self.yaw.sin(),
            self.focus.y + self.altitude,
            self.focus.z + horizontal * self.yaw.cos(),
        )
    }

    /// The transform to apply to the camera entity.
    #[must_use]
    pub fn transform(&self) -> Transform {
        Transform::from_translation(self.eye()).looking_at(self.focus, Vec3::Y)
    }

    /// Moves the focus point, in world-space XZ.
    pub fn pan(&mut self, delta: Vec2) {
        self.target_focus.x += delta.x;
        self.target_focus.z += delta.y;
        let (x, z) = plot::clamp_to_plot(self.target_focus.x, self.target_focus.z);
        self.target_focus.x = x;
        self.target_focus.z = z;
    }

    /// Pans relative to the camera's own facing, so "forward" means into the screen.
    pub fn pan_relative(&mut self, forward: f32, right: f32, metres: f32) {
        let (sin, cos) = self.target_yaw.sin_cos();
        // Forward on the ground is -Z rotated by yaw.
        let forward_vec = Vec2::new(-sin, -cos);
        let right_vec = Vec2::new(cos, -sin);
        self.pan((forward_vec * forward + right_vec * right) * metres);
    }

    /// Rotates about the focus point.
    pub fn rotate(&mut self, radians: f32) {
        self.target_yaw += radians;
    }

    /// Tilts, clamped to the usable range.
    pub fn tilt(&mut self, radians: f32) {
        self.target_pitch = (self.target_pitch + radians).clamp(MAX_PITCH, MIN_PITCH);
    }

    /// Zooms by a multiplicative factor of altitude, clamped to the altitude limits.
    ///
    /// Multiplicative rather than additive so that one wheel click feels the same at 50 m and
    /// at 4 km.
    pub fn zoom(&mut self, factor: f32) {
        self.target_altitude =
            (self.target_altitude * factor).clamp(MIN_ALTITUDE_M, MAX_ALTITUDE_M);
    }

    /// Eases the live values towards the targets.
    pub fn advance(&mut self, dt: f32) {
        // Frame-rate independent easing: the same fraction of the remaining gap per second,
        // not per frame.
        let t = 1.0 - (-SMOOTHING_RATE * dt).exp();
        let t = t.clamp(0.0, 1.0);

        self.focus = self.focus.lerp(self.target_focus, t);
        self.yaw += (self.target_yaw - self.yaw) * t;
        self.pitch += (self.target_pitch - self.pitch) * t;
        self.altitude += (self.target_altitude - self.altitude) * t;
    }

    /// Keeps the focus point on the ground as the terrain under it changes.
    pub fn set_ground_height(&mut self, height: f32) {
        self.target_focus.y = height;
    }
}

/// Adds the city camera and its controls.
pub struct CityCameraPlugin;

impl Plugin for CityCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CityCamera>()
            .add_systems(Update, (read_camera_input, apply_camera).chain());
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "input handling genuinely needs keyboard, mouse buttons, motion, scroll and the window"
)]
fn read_camera_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    terrain: Option<Res<render::TerrainWorld>>,
    mut camera: ResMut<CityCamera>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    // --- Pan: WASD ---
    let mut forward = 0.0;
    let mut right = 0.0;
    if keys.pressed(KeyCode::KeyW) {
        forward += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        forward -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        right += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        right -= 1.0;
    }

    // --- Pan: screen-edge scroll ---
    if let Some(window) = windows.iter().next()
        && let Some(cursor) = window.cursor_position()
    {
        let width = window.resolution.width();
        let height = window.resolution.height();
        if cursor.x <= EDGE_SCROLL_MARGIN_PX {
            right -= 1.0;
        } else if cursor.x >= width - EDGE_SCROLL_MARGIN_PX {
            right += 1.0;
        }
        // Screen Y grows downward, so the top edge pans forward.
        if cursor.y <= EDGE_SCROLL_MARGIN_PX {
            forward += 1.0;
        } else if cursor.y >= height - EDGE_SCROLL_MARGIN_PX {
            forward -= 1.0;
        }
    }

    if forward != 0.0 || right != 0.0 {
        let speed = camera.altitude * PAN_RATE_PER_ALTITUDE * dt;
        camera.pan_relative(forward, right, speed);
    }

    // --- Pan: middle-drag ---
    if buttons.pressed(MouseButton::Middle) {
        let delta = motion.delta;
        let metres = camera.altitude * DRAG_PAN_SENSITIVITY;
        camera.pan_relative(delta.y * metres, -delta.x * metres, 1.0);
    }

    // --- Rotate and tilt: Q/E and R/F ---
    if keys.pressed(KeyCode::KeyQ) {
        camera.rotate(-KEY_ROTATE_RATE * dt);
    }
    if keys.pressed(KeyCode::KeyE) {
        camera.rotate(KEY_ROTATE_RATE * dt);
    }
    if keys.pressed(KeyCode::KeyR) {
        camera.tilt(KEY_TILT_RATE * dt);
    }
    if keys.pressed(KeyCode::KeyF) {
        camera.tilt(-KEY_TILT_RATE * dt);
    }

    // --- Rotate and tilt: right-drag, or Alt+drag for tilt alone ---
    if buttons.pressed(MouseButton::Right) {
        let delta = motion.delta;
        let alt_held = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
        if !alt_held {
            camera.rotate(delta.x * DRAG_SENSITIVITY);
        }
        camera.tilt(-delta.y * DRAG_SENSITIVITY);
    }

    // --- Zoom: wheel ---
    if scroll.delta.y != 0.0 {
        camera.zoom((-scroll.delta.y * ZOOM_PER_SCROLL).exp());
    }

    // --- Keep the focus on the ground ---
    if let Some(world) = terrain.as_deref() {
        let height = world.field.height_at_world(camera.focus.x, camera.focus.z);
        camera.set_ground_height(height);
    }
}

fn apply_camera(
    time: Res<Time>,
    mut camera: ResMut<CityCamera>,
    mut transforms: Query<&mut Transform, With<Camera3d>>,
) {
    camera.advance(time.delta_secs());
    let transform = camera.transform();
    for mut target in &mut transforms {
        *target = transform;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs the easing to convergence, as a real session would over a second or so.
    fn settle(camera: &mut CityCamera) {
        for _ in 0..240 {
            camera.advance(1.0 / 60.0);
        }
    }

    #[test]
    fn altitude_is_clamped_to_the_documented_range() {
        // Decision log A12 and blueprint 2.6.9.
        let mut camera = CityCamera::new(Vec3::ZERO, 0.0, -0.6, 900.0);
        for _ in 0..200 {
            camera.zoom(0.5);
        }
        settle(&mut camera);
        assert!(
            camera.altitude >= MIN_ALTITUDE_M - 0.01,
            "fell to {}",
            camera.altitude
        );

        for _ in 0..200 {
            camera.zoom(2.0);
        }
        settle(&mut camera);
        assert!(
            camera.altitude <= MAX_ALTITUDE_M + 0.01,
            "rose to {}",
            camera.altitude
        );
        assert_eq!(MIN_ALTITUDE_M, 20.0);
        assert_eq!(MAX_ALTITUDE_M, 5_000.0);
    }

    #[test]
    fn pitch_is_clamped_short_of_straight_down_and_below_the_horizon() {
        let mut camera = CityCamera::default();
        for _ in 0..500 {
            camera.tilt(-0.1);
        }
        settle(&mut camera);
        assert!(
            camera.pitch >= MAX_PITCH - 0.001,
            "pitched to {}",
            camera.pitch
        );

        for _ in 0..500 {
            camera.tilt(0.1);
        }
        settle(&mut camera);
        assert!(
            camera.pitch <= MIN_PITCH + 0.001,
            "pitched to {}",
            camera.pitch
        );
        // Never looking up: a city camera above the horizon shows empty sky.
        const { assert!(MIN_PITCH < 0.0) };
    }

    #[test]
    fn the_focus_point_cannot_leave_the_plot() {
        let mut camera = CityCamera::default();
        for _ in 0..1_000 {
            camera.pan(Vec2::new(1_000.0, 1_000.0));
        }
        settle(&mut camera);
        assert!(
            plot::contains(camera.focus.x, camera.focus.z),
            "{:?}",
            camera.focus
        );

        for _ in 0..2_000 {
            camera.pan(Vec2::new(-1_000.0, -1_000.0));
        }
        settle(&mut camera);
        assert!(
            plot::contains(camera.focus.x, camera.focus.z),
            "{:?}",
            camera.focus
        );
    }

    #[test]
    fn the_camera_sits_at_its_altitude_above_the_focus() {
        for altitude in [20.0_f32, 200.0, 1_000.0, 5_000.0] {
            for pitch in [-0.2_f32, -0.6, -1.4] {
                let camera = CityCamera::new(Vec3::new(100.0, 30.0, -50.0), 0.9, pitch, altitude);
                let eye = camera.eye();
                assert!(
                    (eye.y - (camera.focus.y + altitude)).abs() < 0.01,
                    "altitude {altitude}, pitch {pitch}: eye at {}",
                    eye.y
                );
            }
        }
    }

    #[test]
    fn the_camera_always_looks_at_its_focus() {
        let camera = CityCamera::new(Vec3::new(-300.0, 12.0, 700.0), 2.1, -0.8, 450.0);
        let transform = camera.transform();
        let to_focus = (camera.focus - transform.translation).normalize();
        let alignment = to_focus.dot(transform.forward().as_vec3());
        assert!(alignment > 0.999, "alignment {alignment}");
    }

    #[test]
    fn eye_and_target_round_trips_through_camera_state() {
        // Bookmarks are written as eye plus target; the camera works in orbit terms. The
        // conversion has to preserve the view or every bookmark would be framed differently.
        let eye = Vec3::new(-3_750.0, 620.0, 1_450.0);
        let target = Vec3::new(2_600.0, 180.0, -1_000.0);
        let camera = CityCamera::from_eye_and_target(eye, target);

        assert!(
            (camera.focus - target).length() < 0.01,
            "focus {:?}",
            camera.focus
        );

        let transform = camera.transform();
        let original_direction = (target - eye).normalize();
        let alignment = original_direction.dot(transform.forward().as_vec3());
        assert!(
            alignment > 0.999,
            "view direction changed, alignment {alignment}"
        );
    }

    #[test]
    fn panning_is_relative_to_facing() {
        // With yaw 0 the camera looks towards -Z, so panning forward must decrease Z.
        let mut camera = CityCamera::new(Vec3::ZERO, 0.0, -0.6, 500.0);
        camera.pan_relative(1.0, 0.0, 100.0);
        settle(&mut camera);
        assert!(
            camera.focus.z < -50.0,
            "forward pan gave {:?}",
            camera.focus
        );
        assert!(
            camera.focus.x.abs() < 1.0,
            "forward pan drifted in x: {:?}",
            camera.focus
        );

        // Turned a quarter turn, forward must now move along +X or -X instead.
        let mut camera = CityCamera::new(Vec3::ZERO, std::f32::consts::FRAC_PI_2, -0.6, 500.0);
        camera.pan_relative(1.0, 0.0, 100.0);
        settle(&mut camera);
        assert!(
            camera.focus.x.abs() > 50.0,
            "turned pan gave {:?}",
            camera.focus
        );
        assert!(
            camera.focus.z.abs() < 1.0,
            "turned pan drifted in z: {:?}",
            camera.focus
        );
    }

    #[test]
    fn pan_speed_scales_with_altitude() {
        // A fixed speed would crawl at 5 km and be uncontrollable at 20 m.
        let distance_at = |altitude: f32| {
            let mut camera = CityCamera::new(Vec3::ZERO, 0.0, -0.6, altitude);
            let speed = camera.altitude * PAN_RATE_PER_ALTITUDE * (1.0 / 60.0);
            camera.pan_relative(1.0, 0.0, speed);
            settle(&mut camera);
            camera.focus.length()
        };
        let low = distance_at(50.0);
        let high = distance_at(2_000.0);
        assert!(high > low * 10.0, "low {low} m, high {high} m");
    }

    #[test]
    fn zooming_is_multiplicative_so_it_feels_the_same_at_every_altitude() {
        let ratio_at = |start: f32| {
            let mut camera = CityCamera::new(Vec3::ZERO, 0.0, -0.6, start);
            camera.zoom(0.8);
            settle(&mut camera);
            camera.altitude / start
        };
        // Same proportional change from very different starting altitudes.
        assert!((ratio_at(100.0) - ratio_at(2_000.0)).abs() < 0.01);
    }

    #[test]
    fn smoothing_is_frame_rate_independent() {
        // The same elapsed time must produce the same result at 30 and at 60 fps, or the
        // camera feels sluggish exactly when the frame rate is already poor.
        let mut at_60 = CityCamera::new(Vec3::ZERO, 0.0, -0.6, 1_000.0);
        at_60.pan(Vec2::new(500.0, 0.0));
        for _ in 0..60 {
            at_60.advance(1.0 / 60.0);
        }

        let mut at_30 = CityCamera::new(Vec3::ZERO, 0.0, -0.6, 1_000.0);
        at_30.pan(Vec2::new(500.0, 0.0));
        for _ in 0..30 {
            at_30.advance(1.0 / 30.0);
        }

        assert!(
            (at_60.focus.x - at_30.focus.x).abs() < 1.0,
            "60 fps reached {}, 30 fps reached {}",
            at_60.focus.x,
            at_30.focus.x
        );
    }

    #[test]
    fn the_camera_eases_rather_than_snapping() {
        let mut camera = CityCamera::new(Vec3::ZERO, 0.0, -0.6, 1_000.0);
        camera.pan(Vec2::new(1_000.0, 0.0));
        // One frame must move part of the way, not all of it.
        camera.advance(1.0 / 60.0);
        assert!(camera.focus.x > 0.0, "did not move at all");
        assert!(camera.focus.x < 900.0, "snapped straight to the target");
        settle(&mut camera);
        assert!(
            (camera.focus.x - 1_000.0).abs() < 1.0,
            "never arrived: {}",
            camera.focus.x
        );
    }

    #[test]
    fn a_freshly_constructed_camera_does_not_glide() {
        // Bookmarks replace the camera outright, and must not then ease towards anything:
        // a screenshot taken mid-glide would be framed differently every run.
        let mut camera =
            CityCamera::from_eye_and_target(Vec3::new(-900.0, 400.0, 800.0), Vec3::ZERO);
        let before = camera.transform().translation;
        for _ in 0..10 {
            camera.advance(1.0 / 60.0);
        }
        assert!(
            (camera.transform().translation - before).length() < 0.01,
            "camera drifted from {before:?} to {:?}",
            camera.transform().translation
        );
    }

    #[test]
    fn orbit_distance_grows_as_the_camera_tilts_towards_the_horizon() {
        // Altitude is the clamped quantity, so a shallow pitch must push the camera out
        // rather than let it climb past the ceiling.
        let steep = CityCamera::new(Vec3::ZERO, 0.0, -1.4, 1_000.0);
        let shallow = CityCamera::new(Vec3::ZERO, 0.0, -0.2, 1_000.0);
        assert!(
            shallow.orbit_distance() > steep.orbit_distance() * 3.0,
            "steep {} m, shallow {} m",
            steep.orbit_distance(),
            shallow.orbit_distance()
        );
        // Both still obey the altitude.
        assert!((steep.eye().y - 1_000.0).abs() < 0.01);
        assert!((shallow.eye().y - 1_000.0).abs() < 0.01);
    }

    #[test]
    fn rotation_sweeps_the_camera_around_the_focus() {
        let focus = Vec3::new(500.0, 0.0, 500.0);
        let mut camera = CityCamera::new(focus, 0.0, -0.6, 800.0);
        let start = camera.eye();
        camera.rotate(std::f32::consts::PI);
        settle(&mut camera);
        let half_turn = camera.eye();

        // Half a turn puts the camera on the opposite side, at the same distance and height.
        assert!((start.y - half_turn.y).abs() < 1.0);
        let start_offset = Vec2::new(start.x - focus.x, start.z - focus.z);
        let end_offset = Vec2::new(half_turn.x - focus.x, half_turn.z - focus.z);
        assert!((start_offset.length() - end_offset.length()).abs() < 1.0);
        assert!(
            start_offset.dot(end_offset) < 0.0,
            "did not end up opposite"
        );
    }

    #[test]
    fn the_focus_follows_the_ground() {
        let mut camera = CityCamera::default();
        camera.set_ground_height(250.0);
        settle(&mut camera);
        assert!((camera.focus.y - 250.0).abs() < 0.5);
        // And the camera rises with it, staying at its altitude above the ground.
        assert!((camera.eye().y - (250.0 + camera.altitude)).abs() < 1.0);
    }
}
