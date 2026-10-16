//! Conversions between [`Duration`] and the standard library's
//! [`core::time::Duration`] (also `std::time::Duration`).
//!
//! ```rust
//! use qty::*;
//! use qty::unit::duration::Nanosecond;
//!
//! // From std: e.g. `Instant::elapsed()`.
//! let elapsed = std::time::Duration::from_millis(1500);
//! let t: Duration = elapsed.into();
//! assert_eq!(t, Duration::from_seconds(1.5));
//!
//! // Back to std: fails for negative, NaN or too large values.
//! let timeout: std::time::Duration = Duration::from_minutes(2.0).try_into().unwrap();
//! assert_eq!(timeout.as_secs(), 120);
//! assert!(std::time::Duration::try_from(Duration::from_seconds(-1.0)).is_err());
//!
//! // Integer nanoseconds convert exactly both ways.
//! let ns = Fixed::<Nanosecond>::try_from(elapsed)?;
//! assert_eq!(ns.count(), 1_500_000_000);
//! assert_eq!(std::time::Duration::try_from(ns)?, elapsed);
//! # Ok::<(), RoundError>(())
//! ```

use core::time::Duration as StdDuration;

use crate::unit::duration::Nanosecond;
use crate::{Duration, Fixed, Integer, RoundError};

/// Rounds to the nearest `f64` second (exact below about 104 days at
/// nanosecond resolution).
impl From<StdDuration> for Duration<f64> {
    #[inline]
    fn from(d: StdDuration) -> Self {
        Duration::from_base(d.as_secs_f64())
    }
}

/// Rounds to the nearest `f32` second.
impl From<StdDuration> for Duration<f32> {
    #[inline]
    fn from(d: StdDuration) -> Self {
        Duration::from_base(d.as_secs_f32())
    }
}

/// Fails for negative, NaN or too large values; rounds to the nearest
/// nanosecond otherwise.
impl TryFrom<Duration<f64>> for StdDuration {
    type Error = core::time::TryFromFloatSecsError;
    #[inline]
    fn try_from(d: Duration<f64>) -> Result<Self, Self::Error> {
        StdDuration::try_from_secs_f64(d.as_base())
    }
}

/// Fails for negative, NaN or too large values; rounds to the nearest
/// nanosecond otherwise.
impl TryFrom<Duration<f32>> for StdDuration {
    type Error = core::time::TryFromFloatSecsError;
    #[inline]
    fn try_from(d: Duration<f32>) -> Result<Self, Self::Error> {
        StdDuration::try_from_secs_f32(d.as_base())
    }
}

/// Exact; [`RoundError::Overflow`] if the count does not fit in `I`.
impl<I: Integer> TryFrom<StdDuration> for Fixed<Nanosecond, I> {
    type Error = RoundError;
    #[inline]
    fn try_from(d: StdDuration) -> Result<Self, RoundError> {
        i128::try_from(d.as_nanos())
            .ok()
            .and_then(I::from_i128)
            .map(Fixed::new)
            .ok_or(RoundError::Overflow)
    }
}

/// Exact; [`RoundError::Overflow`] for negative counts and for counts beyond
/// [`core::time::Duration::MAX`].
impl<I: Integer> TryFrom<Fixed<Nanosecond, I>> for StdDuration {
    type Error = RoundError;
    #[inline]
    fn try_from(ns: Fixed<Nanosecond, I>) -> Result<Self, RoundError> {
        let n = ns
            .count()
            .to_i128()
            .and_then(|n| u128::try_from(n).ok())
            .ok_or(RoundError::Overflow)?;
        let secs = u64::try_from(n / 1_000_000_000).map_err(|_| RoundError::Overflow)?;
        // The remainder is below 10⁹, so it fits in a u32.
        Ok(StdDuration::new(secs, (n % 1_000_000_000) as u32))
    }
}
