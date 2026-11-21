//! Vsync toggle (blueprint 2.19 step 4).
//!
//! # Why this is not optional
//!
//! The dev laptop's panel runs at 60 Hz. With vsync on, every graphics preset reports exactly
//! 60 fps and the FPS table in the milestone report becomes meaningless. Measuring requires
//! turning vsync off, so the toggle has to exist before the first measurement, not after.
//!
//! Default is **on**: tearing is unpleasant and an uncapped frame rate wastes battery and
//! generates heat on a 15 W laptop, which then thermally throttles everything else.

use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow};

/// Whether vsync is currently enabled.
///
/// Mirrors the window's present mode so the HUD can show it without querying the window.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct VsyncEnabled(pub bool);

impl Default for VsyncEnabled {
    fn default() -> Self {
        Self(true)
    }
}

impl VsyncEnabled {
    /// The present mode this setting corresponds to.
    ///
    /// Both are the `Auto*` variants, which fall back gracefully; `Immediate` and `Mailbox`
    /// panic on platforms that lack them, which is not a trade worth making for a toggle.
    #[must_use]
    pub const fn present_mode(self) -> PresentMode {
        if self.0 {
            PresentMode::AutoVsync
        } else {
            PresentMode::AutoNoVsync
        }
    }
}

/// Adds the vsync toggle, bound to `V`.
pub struct VsyncPlugin;

impl Plugin for VsyncPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VsyncEnabled>()
            .add_systems(Update, toggle_vsync);
    }
}

fn toggle_vsync(
    keys: Res<ButtonInput<KeyCode>>,
    mut vsync: ResMut<VsyncEnabled>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !keys.just_pressed(KeyCode::KeyV) {
        return;
    }
    vsync.0 = !vsync.0;
    let mode = vsync.present_mode();
    for mut window in &mut windows {
        window.present_mode = mode;
    }
    info!(
        "vsync {} (present mode {:?})",
        if vsync.0 { "on" } else { "off" },
        mode
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_vsync_on() {
        assert_eq!(VsyncEnabled::default(), VsyncEnabled(true));
    }

    #[test]
    fn present_mode_maps_to_the_fallback_safe_variants() {
        assert_eq!(VsyncEnabled(true).present_mode(), PresentMode::AutoVsync);
        assert_eq!(VsyncEnabled(false).present_mode(), PresentMode::AutoNoVsync);
    }
}
