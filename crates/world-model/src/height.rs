//! Terrain height encoding.
//!
//! Heights are stored as `u16` in fixed point rather than `f32` (blueprint 2.4). At 4 m
//! sample spacing a plot holds 16.7 million samples, so halving the per-sample cost from 4
//! bytes to 2 saves 33 MB per city — which matters on a machine where the integrated GPU and
//! the simulation share one memory channel.
//!
//! # Encoding
//!
//! 0.02 m steps covering −100 m … +1,210 m, giving 65,500 distinct heights out of the 65,536
//! a `u16` provides. The 2 cm quantisation is far below anything visible on terrain sampled
//! every 4 m.

/// Lowest representable terrain height, in metres.
pub const HEIGHT_MIN_M: f32 = -100.0;

/// Highest representable terrain height, in metres.
pub const HEIGHT_MAX_M: f32 = 1_210.0;

/// Quantisation step, in metres.
pub const HEIGHT_STEP_M: f32 = 0.02;

/// Steps per metre. Exactly 50, so encoding is a multiplication rather than a division by
/// 0.02 — which is not exactly representable in binary floating point and would accumulate
/// rounding error.
pub const HEIGHT_STEPS_PER_M: f32 = 50.0;

/// The largest raw value the encoding produces: `(1210 − (−100)) × 50 = 65,500`.
pub const HEIGHT_MAX_RAW: u16 = 65_500;

const _: () = assert!(
    HEIGHT_MAX_RAW as u32 <= u16::MAX as u32,
    "the height range must fit in a u16"
);

/// Encodes a height in metres to its fixed-point representation.
///
/// Heights outside the representable range clamp to it. Clamping rather than wrapping matters:
/// a terraforming brush that overshoots should flatten against the ceiling, not teleport the
/// ground to the floor.
#[inline]
#[must_use]
pub fn encode(metres: f32) -> u16 {
    // NaN would survive `clamp` as NaN and then cast to 0, silently producing a −100 m pit.
    // Treat it as the minimum explicitly instead.
    if metres.is_nan() {
        return 0;
    }
    let clamped = metres.clamp(HEIGHT_MIN_M, HEIGHT_MAX_M);
    let steps = (clamped - HEIGHT_MIN_M) * HEIGHT_STEPS_PER_M;
    // `steps` is in 0.0..=65_500.0 here, so the cast cannot overflow.
    steps.round() as u16
}

/// Decodes a fixed-point height back to metres.
#[inline]
#[must_use]
pub fn decode(raw: u16) -> f32 {
    HEIGHT_MIN_M + f32::from(raw) / HEIGHT_STEPS_PER_M
}

/// Encodes a sea level in metres, for terrain presets that place water.
///
/// Identical to [`encode`]; named separately because sea level is a distinct concept in the
/// terrain generator and the call sites read better.
#[inline]
#[must_use]
pub fn encode_sea_level(metres: f32) -> u16 {
    encode(metres)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_endpoints_encode_to_the_expected_extremes() {
        assert_eq!(encode(HEIGHT_MIN_M), 0);
        assert_eq!(encode(HEIGHT_MAX_M), HEIGHT_MAX_RAW);
        assert_eq!(decode(0), HEIGHT_MIN_M);
        assert!((decode(HEIGHT_MAX_RAW) - HEIGHT_MAX_M).abs() < 1e-3);
    }

    #[test]
    fn sea_level_zero_is_representable_exactly() {
        // Sea level is the single most important height in the game; it must not drift.
        let raw = encode(0.0);
        assert_eq!(raw, 5_000);
        assert_eq!(decode(raw), 0.0);
    }

    #[test]
    fn round_trip_error_stays_under_one_centimetre() {
        // Half a 2 cm step is 1 cm, which is the worst case by construction.
        let mut worst = 0.0_f32;
        let mut at = 0.0_f32;
        let mut m = HEIGHT_MIN_M;
        while m <= HEIGHT_MAX_M {
            let err = (decode(encode(m)) - m).abs();
            if err > worst {
                worst = err;
                at = m;
            }
            m += 0.37; // A deliberately non-aligned step, to land between quantisation points.
        }
        assert!(
            worst <= 0.0101,
            "worst round-trip error {worst} m at {at} m"
        );
    }

    #[test]
    fn every_raw_value_decodes_inside_the_declared_range() {
        for raw in [0_u16, 1, 5_000, 32_768, HEIGHT_MAX_RAW] {
            let m = decode(raw);
            assert!(
                (HEIGHT_MIN_M..=HEIGHT_MAX_M).contains(&m),
                "raw {raw} decoded to {m}"
            );
        }
    }

    #[test]
    fn decode_is_monotonic() {
        let mut prev = decode(0);
        for raw in 1..=HEIGHT_MAX_RAW {
            let cur = decode(raw);
            assert!(cur > prev, "raw {raw} decoded to {cur}, not above {prev}");
            prev = cur;
        }
    }

    #[test]
    fn out_of_range_heights_clamp_rather_than_wrap() {
        assert_eq!(encode(-9_999.0), 0);
        assert_eq!(encode(99_999.0), HEIGHT_MAX_RAW);
        assert_eq!(encode(f32::NEG_INFINITY), 0);
        assert_eq!(encode(f32::INFINITY), HEIGHT_MAX_RAW);
    }

    #[test]
    fn nan_encodes_to_the_floor_instead_of_garbage() {
        assert_eq!(encode(f32::NAN), 0);
    }

    #[test]
    fn one_step_apart_encodes_one_raw_apart() {
        let a = encode(100.0);
        let b = encode(100.0 + HEIGHT_STEP_M);
        assert_eq!(b - a, 1);
    }
}
