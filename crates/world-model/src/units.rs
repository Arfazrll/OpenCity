//! Strong types for physical units and the game clock.
//!
//! Blueprint 3.9 asks for strong types "where it prevents bugs". The bugs these prevent are
//! the ones that cost hours: passing a heightfield index where metres were wanted, or mixing
//! real seconds with game seconds when the day-night cycle runs 72x faster than wall time.

use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// A distance in metres. One world unit is exactly one metre (blueprint 2.4).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Meters(pub f32);

impl Meters {
    /// Zero distance.
    pub const ZERO: Self = Self(0.0);

    /// The raw value in metres.
    #[inline]
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }

    /// Clamps the distance to an inclusive range.
    #[inline]
    #[must_use]
    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self(self.0.clamp(min.0, max.0))
    }

    /// Absolute value.
    #[inline]
    #[must_use]
    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }
}

impl Add for Meters {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for Meters {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sub for Meters {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for Meters {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl Mul<f32> for Meters {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self(self.0 * rhs)
    }
}

impl Div<f32> for Meters {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f32) -> Self {
        Self(self.0 / rhs)
    }
}

impl Neg for Meters {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

/// A duration in seconds of **real** time.
///
/// Kept distinct from game time on purpose: the game clock runs 72x faster than this at 1x
/// speed, and conflating the two is how day-night cycles end up 72 times too fast.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Seconds(pub f32);

impl Seconds {
    /// Zero duration.
    pub const ZERO: Self = Self(0.0);

    /// The raw value in seconds.
    #[inline]
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// Simulation speed selected by the player (decision log A30).
///
/// The tick size never changes; higher speeds run more ticks per real second, so results are
/// identical at any speed (blueprint 2.5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SimSpeed {
    /// Ticking stopped. The day-night cycle also freezes.
    Paused,
    /// Real time: 10 ticks per real second.
    #[default]
    X1,
    /// Double speed: 20 ticks per real second.
    X2,
    /// Triple speed: 30 ticks per real second.
    X3,
}

impl SimSpeed {
    /// The multiplier applied to real elapsed time. `Paused` yields zero.
    #[inline]
    #[must_use]
    pub const fn multiplier(self) -> f32 {
        match self {
            Self::Paused => 0.0,
            Self::X1 => 1.0,
            Self::X2 => 2.0,
            Self::X3 => 3.0,
        }
    }

    /// Cycles to the next speed, skipping `Paused`. Used by the HUD speed control.
    #[inline]
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Paused | Self::X3 => Self::X1,
            Self::X1 => Self::X2,
            Self::X2 => Self::X3,
        }
    }
}

/// Real seconds in one in-game day at 1x speed (blueprint 2.5.1: 20 real minutes).
pub const REAL_SECONDS_PER_GAME_DAY: f64 = 20.0 * 60.0;

/// Seconds in a day, by definition.
pub const GAME_SECONDS_PER_GAME_DAY: f64 = 24.0 * 60.0 * 60.0;

/// How much faster the game clock runs than wall time at 1x speed.
///
/// `86_400 / 1_200 = 72`.
pub const GAME_SECONDS_PER_REAL_SECOND: f64 = GAME_SECONDS_PER_GAME_DAY / REAL_SECONDS_PER_GAME_DAY;

/// The in-game clock.
///
/// Stored as total elapsed game seconds in `f64`. `f32` would lose sub-second resolution after
/// roughly a fortnight of game time, which a long sandbox session reaches easily.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct GameTime {
    game_seconds: f64,
}

impl GameTime {
    /// The clock at day 0, midnight.
    pub const START: Self = Self { game_seconds: 0.0 };

    /// Creates a clock at a given number of elapsed game seconds.
    #[inline]
    #[must_use]
    pub const fn from_game_seconds(game_seconds: f64) -> Self {
        Self { game_seconds }
    }

    /// Creates a clock at a given day and hour, for tests and camera bookmarks.
    #[must_use]
    pub fn from_day_and_hour(day: u32, hour: f64) -> Self {
        Self {
            game_seconds: f64::from(day) * GAME_SECONDS_PER_GAME_DAY + hour * 3600.0,
        }
    }

    /// Total elapsed game seconds since the world started.
    #[inline]
    #[must_use]
    pub const fn game_seconds(self) -> f64 {
        self.game_seconds
    }

    /// Whole days elapsed since the world started.
    #[inline]
    #[must_use]
    pub fn day(self) -> u32 {
        (self.game_seconds / GAME_SECONDS_PER_GAME_DAY) as u32
    }

    /// Position within the current day, in `0.0..1.0` where `0.0` is midnight.
    ///
    /// This is what drives the sun: the renderer needs a cyclic value, not a day count.
    #[inline]
    #[must_use]
    pub fn fraction_of_day(self) -> f32 {
        let frac = (self.game_seconds / GAME_SECONDS_PER_GAME_DAY).fract();
        // `fract` keeps the sign, so a negative clock would produce a negative fraction.
        (if frac < 0.0 { frac + 1.0 } else { frac }) as f32
    }

    /// Hour of the current day in `0.0..24.0`.
    #[inline]
    #[must_use]
    pub fn hour_of_day(self) -> f32 {
        self.fraction_of_day() * 24.0
    }

    /// Advances the clock by real elapsed time at the given speed.
    ///
    /// `Paused` advances nothing, which is what stops the day-night cycle.
    #[inline]
    pub fn advance(&mut self, real_delta: Seconds, speed: SimSpeed) {
        let real = f64::from(real_delta.get() * speed.multiplier());
        self.game_seconds += real * GAME_SECONDS_PER_REAL_SECOND;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meters_arithmetic_is_plain_f32() {
        assert_eq!((Meters(3.0) + Meters(4.0)).get(), 7.0);
        assert_eq!((Meters(3.0) - Meters(4.0)).get(), -1.0);
        assert_eq!((Meters(3.0) * 2.0).get(), 6.0);
        assert_eq!((Meters(6.0) / 2.0).get(), 3.0);
        assert_eq!((-Meters(3.0)).get(), -3.0);
        assert_eq!(Meters(-3.0).abs().get(), 3.0);
        assert_eq!(Meters(10.0).clamp(Meters(0.0), Meters(5.0)).get(), 5.0);
    }

    #[test]
    fn game_clock_runs_72_times_real_time() {
        assert!((GAME_SECONDS_PER_REAL_SECOND - 72.0).abs() < 1e-12);
    }

    #[test]
    fn twenty_real_minutes_is_exactly_one_game_day() {
        let mut t = GameTime::START;
        // 1200 real seconds in one-second steps.
        for _ in 0..1200 {
            t.advance(Seconds(1.0), SimSpeed::X1);
        }
        assert_eq!(t.day(), 1);
        // Back to midnight, within f32 rounding of the accumulated f64 total.
        assert!(t.fraction_of_day() < 1e-4 || t.fraction_of_day() > 1.0 - 1e-4);
    }

    #[test]
    fn speed_scales_elapsed_time_linearly() {
        let step = Seconds(10.0);
        let mut a = GameTime::START;
        let mut b = GameTime::START;
        let mut c = GameTime::START;
        a.advance(step, SimSpeed::X1);
        b.advance(step, SimSpeed::X2);
        c.advance(step, SimSpeed::X3);
        assert!((b.game_seconds() - a.game_seconds() * 2.0).abs() < 1e-9);
        assert!((c.game_seconds() - a.game_seconds() * 3.0).abs() < 1e-9);
    }

    #[test]
    fn paused_freezes_the_clock() {
        let mut t = GameTime::from_day_and_hour(3, 9.0);
        let before = t.game_seconds();
        t.advance(Seconds(100.0), SimSpeed::Paused);
        assert_eq!(t.game_seconds(), before);
    }

    #[test]
    fn day_and_hour_round_trip() {
        let t = GameTime::from_day_and_hour(5, 13.5);
        assert_eq!(t.day(), 5);
        assert!((t.hour_of_day() - 13.5).abs() < 1e-3);
    }

    #[test]
    fn fraction_of_day_is_cyclic_and_in_range() {
        for hour in 0..24 {
            let t = GameTime::from_day_and_hour(2, f64::from(hour));
            let f = t.fraction_of_day();
            assert!((0.0..1.0).contains(&f), "hour {hour} gave {f}");
        }
        // Midnight of day 0 and midnight of day 7 share the same phase.
        let a = GameTime::from_day_and_hour(0, 0.0).fraction_of_day();
        let b = GameTime::from_day_and_hour(7, 0.0).fraction_of_day();
        assert!((a - b).abs() < 1e-6);
    }

    #[test]
    fn speed_cycle_skips_paused() {
        assert_eq!(SimSpeed::X1.next(), SimSpeed::X2);
        assert_eq!(SimSpeed::X2.next(), SimSpeed::X3);
        assert_eq!(SimSpeed::X3.next(), SimSpeed::X1);
        assert_eq!(SimSpeed::Paused.next(), SimSpeed::X1);
    }
}
