//! Conversions with other crates, each behind a feature of the same name.

#[allow(unused_imports)]
use crate::{Duration, Fixed, Quantity, RoundError};

/// `secs + nanos·10⁻⁹` for a float number of seconds, with `0 ≤ nanos < 10⁹`
/// rounded to the nearest nanosecond; `None` if not finite or beyond `i64`
/// seconds.
#[cfg(any(feature = "chrono", feature = "time"))]
fn split_seconds(seconds: f64) -> Result<(i64, u32), RoundError> {
    if !seconds.is_finite() {
        return Err(RoundError::NotFinite);
    }
    // Floor without `std`: truncate, then step down for negative fractions.
    if !(-9.2e18..9.2e18).contains(&seconds) {
        return Err(RoundError::Overflow);
    }
    let mut whole = seconds as i64;
    if (whole as f64) > seconds {
        whole -= 1;
    }
    let fraction = seconds - whole as f64;
    let mut nanos = (fraction * 1e9 + 0.5) as u32;
    if nanos >= 1_000_000_000 {
        whole = whole.checked_add(1).ok_or(RoundError::Overflow)?;
        nanos -= 1_000_000_000;
    }
    Ok((whole, nanos))
}

/// `(seconds, nanoseconds)` of a nanosecond count, with
/// `0 ≤ nanoseconds < 10⁹`.
#[cfg(any(feature = "chrono", feature = "time"))]
fn split_nanos<I: crate::Integer>(
    n: Fixed<crate::unit::duration::Nanosecond, I>,
) -> Result<(i64, u32), RoundError> {
    let n = n.count().to_i128().ok_or(RoundError::Overflow)?;
    let secs = i64::try_from(n.div_euclid(1_000_000_000)).map_err(|_| RoundError::Overflow)?;
    // The remainder is in 0..10⁹.
    Ok((secs, n.rem_euclid(1_000_000_000) as u32))
}

// ── chrono ────────────────────────────────────────────────────────────────────

#[cfg(feature = "chrono")]
mod chrono_impls {
    use super::*;
    use crate::unit::duration::Nanosecond;
    use crate::Integer;
    use chrono::TimeDelta;

    /// Rounds to the nearest `f64` second.
    impl From<TimeDelta> for Duration<f64> {
        fn from(d: TimeDelta) -> Self {
            Duration::from_base(d.num_seconds() as f64 + f64::from(d.subsec_nanos()) * 1e-9)
        }
    }

    /// Rounds to the nearest nanosecond; fails for NaN, infinities and
    /// durations beyond `TimeDelta`'s range.
    impl TryFrom<Duration<f64>> for TimeDelta {
        type Error = RoundError;
        fn try_from(d: Duration<f64>) -> Result<Self, RoundError> {
            let (secs, nanos) = split_seconds(d.as_base())?;
            TimeDelta::new(secs, nanos).ok_or(RoundError::Overflow)
        }
    }

    /// Exact; [`RoundError::Overflow`] if the count does not fit in `I`.
    impl<I: Integer> TryFrom<TimeDelta> for Fixed<Nanosecond, I> {
        type Error = RoundError;
        fn try_from(d: TimeDelta) -> Result<Self, RoundError> {
            let n = i128::from(d.num_seconds()) * 1_000_000_000 + i128::from(d.subsec_nanos());
            I::from_i128(n).map(Fixed::new).ok_or(RoundError::Overflow)
        }
    }

    /// Exact; [`RoundError::Overflow`] beyond `TimeDelta`'s range.
    impl<I: Integer> TryFrom<Fixed<Nanosecond, I>> for TimeDelta {
        type Error = RoundError;
        fn try_from(n: Fixed<Nanosecond, I>) -> Result<Self, RoundError> {
            let (secs, nanos) = split_nanos(n)?;
            TimeDelta::new(secs, nanos).ok_or(RoundError::Overflow)
        }
    }
}

// ── time ──────────────────────────────────────────────────────────────────────

#[cfg(feature = "time")]
mod time_impls {
    use super::*;
    use crate::unit::duration::Nanosecond;
    use crate::Integer;

    /// Rounds to the nearest `f64` second.
    impl From<time::Duration> for Duration<f64> {
        fn from(d: time::Duration) -> Self {
            Duration::from_base(d.as_seconds_f64())
        }
    }

    /// Rounds to the nearest nanosecond; fails for NaN, infinities and
    /// durations beyond `time::Duration`'s range.
    impl TryFrom<Duration<f64>> for time::Duration {
        type Error = RoundError;
        fn try_from(d: Duration<f64>) -> Result<Self, RoundError> {
            let (secs, nanos) = split_seconds(d.as_base())?;
            // `nanos < 10⁹` fits in an i32 and needs no carry.
            Ok(time::Duration::new(secs, nanos as i32))
        }
    }

    /// Exact; [`RoundError::Overflow`] if the count does not fit in `I`.
    impl<I: Integer> TryFrom<time::Duration> for Fixed<Nanosecond, I> {
        type Error = RoundError;
        fn try_from(d: time::Duration) -> Result<Self, RoundError> {
            I::from_i128(d.whole_nanoseconds())
                .map(Fixed::new)
                .ok_or(RoundError::Overflow)
        }
    }

    /// Exact; [`RoundError::Overflow`] beyond `time::Duration`'s range.
    impl<I: Integer> TryFrom<Fixed<Nanosecond, I>> for time::Duration {
        type Error = RoundError;
        fn try_from(n: Fixed<Nanosecond, I>) -> Result<Self, RoundError> {
            let (secs, nanos) = split_nanos(n)?;
            Ok(time::Duration::new(secs, nanos as i32))
        }
    }
}
