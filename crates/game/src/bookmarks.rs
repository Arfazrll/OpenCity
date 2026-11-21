//! Fixed camera bookmarks for reproducible screenshots.
//!
//! # Design
//!
//! Blueprint 2.19 step 10 asks for six specific screenshots — dawn, noon, dusk, night, 5 km
//! altitude and street level — and blueprint 3.4 asks for a `screenshot-review` skill that
//! "launches the build at fixed camera bookmarks, captures images, and reviews them visually".
//!
//! Both need the same thing: a named, committed camera position and time of day that produces
//! the same framing every run. Hand-flying the camera and pressing F12 would give six images
//! that cannot be compared between builds, which makes visual regression review impossible.
//!
//! So the bookmarks live here, in version control, and the binary can be told to jump to one,
//! wait for terrain to finish streaming, capture and exit.

use bevy::prelude::*;
use world_model::GameTime;

/// A named camera position and time of day.
#[derive(Debug, Clone, Copy)]
pub struct Bookmark {
    /// Identifier passed on the command line and used in the screenshot filename.
    pub name: &'static str,
    /// Camera position in world space. `y` is an altitude above ground when
    /// [`Self::above_ground`] is set, and an absolute height otherwise.
    pub eye: Vec3,
    /// Point the camera looks at.
    pub target: Vec3,
    /// Hour of the game day, 0.0 to 24.0.
    pub hour: f64,
    /// Whether `eye.y` and `target.y` are offsets above the terrain rather than absolute.
    ///
    /// Needed for the street-level shot: the Coastal preset's shoreline wanders by up to
    /// 1.8 km, so a hardcoded altitude would sometimes bury the camera in a hillside and
    /// sometimes leave it hanging in the air.
    pub above_ground: bool,
}

/// The six shots blueprint 2.19 step 10 requires, plus the default preview view.
///
/// Positions are tuned for the Coastal preview terrain, whose shoreline sits near
/// `x = -2,600 m` with sea to the west and rising land to the east.
pub const BOOKMARKS: &[Bookmark] = &[
    Bookmark {
        name: "dawn",
        eye: Vec3::new(1_200.0, 430.0, 1_050.0),
        target: Vec3::new(-1_800.0, 40.0, -250.0),
        hour: 6.1,
        above_ground: false,
    },
    Bookmark {
        // From over the sea looking inland: water in the foreground, the beach in the middle
        // and the rising ground behind it. Looking the other way shows only the coastal
        // plain, which is flat by design and makes the terrain look like a painted map.
        name: "noon",
        eye: Vec3::new(-3_750.0, 620.0, 1_450.0),
        target: Vec3::new(2_600.0, 180.0, -1_000.0),
        hour: 12.0,
        above_ground: false,
    },
    Bookmark {
        name: "dusk",
        eye: Vec3::new(-400.0, 520.0, 1_850.0),
        target: Vec3::new(-3_100.0, 0.0, 100.0),
        hour: 18.4,
        above_ground: false,
    },
    Bookmark {
        name: "night",
        eye: Vec3::new(700.0, 620.0, 1_250.0),
        target: Vec3::new(-1_900.0, 0.0, -150.0),
        hour: 22.5,
        above_ground: false,
    },
    Bookmark {
        // Decision log A12 caps the city camera at about 5 km; this is that view.
        name: "altitude",
        eye: Vec3::new(600.0, 5_000.0, 3_200.0),
        target: Vec3::new(-800.0, 0.0, -600.0),
        hour: 11.0,
        above_ground: false,
    },
    Bookmark {
        // Street camera range is 2 m to 30 m (blueprint 2.6.9).
        name: "street",
        eye: Vec3::new(-1_500.0, 22.0, 600.0),
        target: Vec3::new(-2_600.0, 10.0, -200.0),
        hour: 9.0,
        above_ground: true,
    },
];

/// Looks up a bookmark by name.
#[must_use]
pub fn find(name: &str) -> Option<&'static Bookmark> {
    BOOKMARKS.iter().find(|b| b.name.eq_ignore_ascii_case(name))
}

/// Every bookmark name, for help text and error messages.
#[must_use]
pub fn names() -> Vec<&'static str> {
    BOOKMARKS.iter().map(|b| b.name).collect()
}

impl Bookmark {
    /// The game time this bookmark captures at.
    #[must_use]
    pub fn game_time(&self) -> GameTime {
        GameTime::from_day_and_hour(0, self.hour)
    }

    /// Builds the camera transform, resolving ground-relative heights against the terrain.
    #[must_use]
    pub fn transform(&self, ground_height_at: impl Fn(f32, f32) -> f32) -> Transform {
        let (eye, target) = if self.above_ground {
            let eye_ground = ground_height_at(self.eye.x, self.eye.z);
            let target_ground = ground_height_at(self.target.x, self.target.z);
            (
                Vec3::new(self.eye.x, eye_ground + self.eye.y, self.eye.z),
                Vec3::new(self.target.x, target_ground + self.target.y, self.target.z),
            )
        } else {
            (self.eye, self.target)
        };

        Transform::from_translation(eye).looking_at(target, Vec3::Y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_six_required_shots_all_exist() {
        // Blueprint 2.19 step 10 names these exactly.
        for required in ["dawn", "noon", "dusk", "night", "altitude", "street"] {
            assert!(find(required).is_some(), "missing bookmark {required}");
        }
        assert_eq!(BOOKMARKS.len(), 6);
    }

    #[test]
    fn names_are_unique_so_screenshots_do_not_overwrite_each_other() {
        let mut names = names();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count, "duplicate bookmark name");
    }

    #[test]
    fn lookup_is_case_insensitive() {
        assert!(find("DAWN").is_some());
        assert!(find("Noon").is_some());
        assert!(find("nope").is_none());
    }

    #[test]
    fn the_four_time_of_day_shots_are_actually_at_different_times() {
        let hour = |n: &str| find(n).expect("exists").hour;
        let dawn = hour("dawn");
        let noon = hour("noon");
        let dusk = hour("dusk");
        let night = hour("night");
        assert!(
            dawn < noon && noon < dusk && dusk < night,
            "{dawn} {noon} {dusk} {night}"
        );
        // And they land in the right parts of the day.
        assert!((5.0..8.0).contains(&dawn), "dawn at {dawn}");
        assert!((11.0..13.0).contains(&noon), "noon at {noon}");
        assert!((17.0..20.0).contains(&dusk), "dusk at {dusk}");
        assert!(night >= 21.0 || night <= 3.0, "night at {night}");
    }

    #[test]
    fn the_altitude_shot_sits_at_the_documented_zoom_limit() {
        // Decision log A12: maximum altitude about 5 km.
        let b = find("altitude").expect("exists");
        assert!(
            (4_500.0..=5_000.0).contains(&b.eye.y),
            "altitude {}",
            b.eye.y
        );
        assert!(
            !b.above_ground,
            "the 5 km shot should be an absolute altitude"
        );
    }

    #[test]
    fn the_street_shot_is_ground_relative_and_inside_the_street_camera_range() {
        let b = find("street").expect("exists");
        assert!(b.above_ground, "street level must follow the terrain");
        // Blueprint 2.6.9: street camera is 2 m to 30 m.
        assert!((2.0..=30.0).contains(&b.eye.y), "street height {}", b.eye.y);
    }

    #[test]
    fn every_bookmark_is_inside_the_plot_and_looks_somewhere_else() {
        let half = world_model::plot::PLOT_HALF_EXTENT_M;
        for b in BOOKMARKS {
            assert!(
                b.eye.x.abs() <= half && b.eye.z.abs() <= half,
                "{} eye outside",
                b.name
            );
            assert!(
                b.target.x.abs() <= half && b.target.z.abs() <= half,
                "{} target outside",
                b.name
            );
            let horizontal =
                ((b.eye.x - b.target.x).powi(2) + (b.eye.z - b.target.z).powi(2)).sqrt();
            assert!(horizontal > 100.0, "{} looks almost straight down", b.name);
            assert!((0.0..24.0).contains(&b.hour), "{} hour {}", b.name, b.hour);
        }
    }

    #[test]
    fn absolute_bookmarks_ignore_the_ground_and_relative_ones_follow_it() {
        let noon = find("noon").expect("exists");
        let absolute = noon.transform(|_, _| 500.0);
        assert!((absolute.translation.y - noon.eye.y).abs() < 0.01);

        let street = find("street").expect("exists");
        let on_low_ground = street.transform(|_, _| 0.0);
        let on_high_ground = street.transform(|_, _| 300.0);
        assert!((on_low_ground.translation.y - street.eye.y).abs() < 0.01);
        assert!((on_high_ground.translation.y - (300.0 + street.eye.y)).abs() < 0.01);
    }

    #[test]
    fn transforms_face_their_target() {
        for b in BOOKMARKS {
            let t = b.transform(|_, _| 0.0);
            let target = if b.above_ground {
                Vec3::new(b.target.x, b.target.y, b.target.z)
            } else {
                b.target
            };
            let to_target = (target - t.translation).normalize();
            let forward = t.forward().as_vec3();
            let alignment = to_target.dot(forward);
            assert!(
                alignment > 0.99,
                "{} faces {alignment} away from its target",
                b.name
            );
        }
    }

    #[test]
    fn game_time_matches_the_declared_hour() {
        for b in BOOKMARKS {
            let t = b.game_time();
            assert_eq!(t.day(), 0);
            assert!(
                (f64::from(t.hour_of_day()) - b.hour).abs() < 0.01,
                "{}",
                b.name
            );
        }
    }
}
