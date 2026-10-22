//! Integer storage and explicit rounding.

use core::cmp::Ordering;
use core::fmt;
use core::hash::Hash;
use core::ops;

use crate::Float;

/// Lossy conversion of a primitive number to `f64` or `f32`, like `as`.
///
/// Implemented for the primitive integer and float types.
pub trait ToFloat: Copy {
    /// Converts to the nearest `f64`.
    fn to_f64(self) -> f64;
    /// Converts to the nearest `f32`.
    fn to_f32(self) -> f32;
}

macro_rules! impl_to_float {
    ($($t:ty),*) => {$(
        impl ToFloat for $t {
            #[inline(always)]
            fn to_f64(self) -> f64 {
                self as f64
            }
            #[inline(always)]
            fn to_f32(self) -> f32 {
                self as f32
            }
        }
    )*};
}
impl_to_float!(f32, f64, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

mod sealed {
    pub trait Sealed {}
}

/// The primitive integer types, usable as the storage of base-unit
/// quantities such as `Length<i64>`, and as the count of a `Fixed` quantity
/// with the `fixed` feature.
///
/// Sealed. Implemented for `i8` to `i128`, `u8` to `u128`, `isize` and
/// `usize`.
pub trait Integer:
    ToFloat
    + Ord
    + Hash
    + Default
    + fmt::Debug
    + fmt::Display
    + ops::Add<Output = Self>
    + ops::Sub<Output = Self>
    + ops::Mul<Output = Self>
    + ops::Div<Output = Self>
    + ops::Rem<Output = Self>
    + sealed::Sealed
{
    #[doc(hidden)]
    fn to_i128(self) -> Option<i128>;
    #[doc(hidden)]
    fn from_i128(value: i128) -> Option<Self>;
    #[doc(hidden)]
    fn checked_add(self, rhs: Self) -> Option<Self>;
    #[doc(hidden)]
    fn checked_sub(self, rhs: Self) -> Option<Self>;
    #[doc(hidden)]
    fn checked_mul(self, rhs: Self) -> Option<Self>;
    #[doc(hidden)]
    fn saturating_add(self, rhs: Self) -> Self;
    #[doc(hidden)]
    fn saturating_sub(self, rhs: Self) -> Self;
    /// `(is_negative, |self|)`. Unlike `abs`, it can't overflow.
    #[doc(hidden)]
    fn sign_magnitude(self) -> (bool, u128);
}

macro_rules! impl_integer {
    ($($t:ty),*) => {$(
        impl sealed::Sealed for $t {}
        impl Integer for $t {
            #[inline(always)]
            fn to_i128(self) -> Option<i128> {
                i128::try_from(self).ok()
            }
            #[inline(always)]
            fn from_i128(value: i128) -> Option<Self> {
                <$t>::try_from(value).ok()
            }
            #[inline(always)]
            fn checked_add(self, rhs: Self) -> Option<Self> {
                <$t>::checked_add(self, rhs)
            }
            #[inline(always)]
            fn checked_sub(self, rhs: Self) -> Option<Self> {
                <$t>::checked_sub(self, rhs)
            }
            #[inline(always)]
            fn checked_mul(self, rhs: Self) -> Option<Self> {
                <$t>::checked_mul(self, rhs)
            }
            #[inline(always)]
            fn saturating_add(self, rhs: Self) -> Self {
                <$t>::saturating_add(self, rhs)
            }
            #[inline(always)]
            fn saturating_sub(self, rhs: Self) -> Self {
                <$t>::saturating_sub(self, rhs)
            }
            #[inline(always)]
            #[allow(unused_comparisons, clippy::cast_lossless)]
            fn sign_magnitude(self) -> (bool, u128) {
                // `unsigned_abs` exists only on signed types; widen instead.
                if self < 0 {
                    (true, (self as i128).unsigned_abs())
                } else {
                    (false, self as u128)
                }
            }
        }
    )*};
}
impl_integer!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

/// How to round when a result isn't a whole number.
///
/// Lossy integer operations take one; lossless ones don't. A constant mode
/// is resolved at compile time.
///
/// | Mode | 2.5 | -2.5 | 2.4 | -2.6 |
/// |---|---|---|---|---|
/// | `Floor` | 2 | -3 | 2 | -3 |
/// | `Ceil` | 3 | -2 | 3 | -2 |
/// | `TowardZero` | 2 | -2 | 2 | -2 |
/// | `AwayFromZero` | 3 | -3 | 3 | -3 |
/// | `NearestTiesEven` | 2 | -2 | 2 | -3 |
/// | `NearestTiesAway` | 3 | -3 | 2 | -3 |
/// | `Exact` | error | error | error | error |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rounding {
    /// Toward negative infinity.
    Floor,
    /// Toward positive infinity.
    Ceil,
    /// Toward zero (truncation, like integer division).
    TowardZero,
    /// Away from zero.
    AwayFromZero,
    /// To the nearest integer, ties to even. This is the IEEE 754 default,
    /// and it doesn't bias sums, so it suits statistics and running totals.
    NearestTiesEven,
    /// To the nearest integer, ties away from zero, like `f64::round`.
    NearestTiesAway,
    /// Don't round. Fails with [`RoundError::Inexact`] when the result
    /// isn't a whole number.
    Exact,
}

/// Why a rounding conversion or division failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoundError {
    /// The result does not fit in the integer type.
    Overflow,
    /// The input is NaN or infinite.
    NotFinite,
    /// [`Rounding::Exact`] was asked for, but the result isn't a whole
    /// number.
    Inexact,
    /// Division by zero.
    DivisionByZero,
}

impl fmt::Display for RoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            RoundError::Overflow => "result out of range for the integer type",
            RoundError::NotFinite => "value is NaN or infinite",
            RoundError::Inexact => "result is not a whole number",
            RoundError::DivisionByZero => "division by zero",
        })
    }
}

impl core::error::Error for RoundError {}

/// Chooses between the truncated result `t` and its neighbour away from zero,
/// given how the discarded fraction compares to one half.
#[inline(always)]
fn pick(t: i128, positive: bool, half: Ordering, mode: Rounding) -> Result<i128, RoundError> {
    let away = if positive { t + 1 } else { t - 1 };
    Ok(match mode {
        Rounding::TowardZero => t,
        Rounding::AwayFromZero => away,
        Rounding::Floor => {
            if positive {
                t
            } else {
                away
            }
        }
        Rounding::Ceil => {
            if positive {
                away
            } else {
                t
            }
        }
        Rounding::NearestTiesEven | Rounding::NearestTiesAway => match half {
            Ordering::Less => t,
            Ordering::Greater => away,
            Ordering::Equal if mode == Rounding::NearestTiesAway || t % 2 != 0 => away,
            Ordering::Equal => t,
        },
        Rounding::Exact => return Err(RoundError::Inexact),
    })
}

/// Rounds `n / d` to an integer.
#[inline]
#[cfg_attr(not(feature = "fixed"), allow(dead_code))]
pub(crate) fn div_round(n: i128, d: i128, mode: Rounding) -> Result<i128, RoundError> {
    if d == 0 {
        return Err(RoundError::DivisionByZero);
    }
    let t = n.checked_div(d).ok_or(RoundError::Overflow)?;
    let r = n % d;
    if r == 0 {
        return Ok(t);
    }
    // |r| against |d| - |r| rather than 2|r| against |d|, which could
    // overflow.
    let (ar, ad) = (r.unsigned_abs(), d.unsigned_abs());
    let positive = (r > 0) == (d > 0);
    pick(t, positive, ar.cmp(&(ad - ar)), mode)
}

/// Rounds a float to an integer, with no intermediate rounding.
#[inline]
pub(crate) fn round_f64(x: f64, mode: Rounding) -> Result<i128, RoundError> {
    if !x.is_finite() {
        return Err(RoundError::NotFinite);
    }
    // Beyond 2^127 it can't fit in an i128.
    if !(-1.7014118346046923e38..1.7014118346046923e38).contains(&x) {
        return Err(RoundError::Overflow);
    }
    // Truncate toward zero. Go through i64 when it fits: that's one
    // instruction each way, while i128 needs library calls.
    let (t, back) = if (-9.223372036854775e18..9.223372036854775e18).contains(&x) {
        let t = x as i64;
        (t as i128, t as f64)
    } else {
        let t = x as i128;
        (t, t as f64)
    };
    // Exact by Sterbenz's lemma below 2^52. Above that, `x` is already an
    // integer and the difference is zero.
    let frac = x - back;
    if frac == 0.0 {
        return Ok(t);
    }
    let magnitude = Float::abs(frac);
    let half = if magnitude > 0.5 {
        Ordering::Greater
    } else if magnitude < 0.5 {
        Ordering::Less
    } else {
        Ordering::Equal
    };
    pick(t, frac > 0.0, half, mode)
}

/// Rounds a float to an integer type, for `round_from` on integer quantities.
#[inline]
pub(crate) fn round_to<F: Float, I: Integer>(x: F, mode: Rounding) -> Result<I, RoundError> {
    round_f64(x.to_f64(), mode).and_then(narrow)
}

#[inline(always)]
pub(crate) fn narrow<I: Integer>(v: i128) -> Result<I, RoundError> {
    I::from_i128(v).ok_or(RoundError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODES: [Rounding; 6] = [
        Rounding::Floor,
        Rounding::Ceil,
        Rounding::TowardZero,
        Rounding::AwayFromZero,
        Rounding::NearestTiesEven,
        Rounding::NearestTiesAway,
    ];

    /// Reference rounding of the exact rational `n / d` (d > 0), by brute force.
    fn reference(n: i128, d: i128, mode: Rounding) -> i128 {
        let floor = n.div_euclid(d);
        let ceil = if n.rem_euclid(d) == 0 {
            floor
        } else {
            floor + 1
        };
        let toward_zero = if n >= 0 { floor } else { ceil };
        let away = if n >= 0 { ceil } else { floor };
        let twice = 2 * n.rem_euclid(d);
        match mode {
            Rounding::Floor => floor,
            Rounding::Ceil => ceil,
            Rounding::TowardZero => toward_zero,
            Rounding::AwayFromZero => away,
            Rounding::NearestTiesEven | Rounding::NearestTiesAway => {
                if twice < d {
                    floor
                } else if twice > d {
                    ceil
                } else if mode == Rounding::NearestTiesAway {
                    away
                } else if floor % 2 == 0 {
                    floor
                } else {
                    ceil
                }
            }
            Rounding::Exact => unreachable!(),
        }
    }

    #[test]
    fn div_round_matches_reference() {
        for n in -50..=50 {
            for d in [1, 2, 3, 4, 7, 10] {
                for mode in MODES {
                    assert_eq!(
                        div_round(n, d, mode),
                        Ok(reference(n, d, mode)),
                        "{n}/{d} {mode:?}"
                    );
                    // Negative divisor: n / -d == -n / d.
                    assert_eq!(
                        div_round(n, -d, mode),
                        Ok(reference(-n, d, mode)),
                        "{n}/-{d} {mode:?}"
                    );
                }
                let exact = div_round(n, d, Rounding::Exact);
                if n % d == 0 {
                    assert_eq!(exact, Ok(n / d));
                } else {
                    assert_eq!(exact, Err(RoundError::Inexact));
                }
            }
        }
        assert_eq!(
            div_round(1, 0, Rounding::Floor),
            Err(RoundError::DivisionByZero)
        );
        assert_eq!(
            div_round(i128::MIN, -1, Rounding::Floor),
            Err(RoundError::Overflow)
        );
        assert_eq!(
            div_round(i128::MAX, i128::MAX - 1, Rounding::NearestTiesEven),
            Ok(1)
        );
    }

    #[test]
    fn round_f64_matches_reference() {
        // Quarter steps hit exact ties; compare against the rational n/4.
        for n in -40..=40 {
            let x = n as f64 / 4.0;
            for mode in MODES {
                assert_eq!(
                    round_f64(x, mode),
                    Ok(reference(n, 4, mode)),
                    "{x} {mode:?}"
                );
            }
        }
        assert_eq!(
            round_f64(2.5000000000000004, Rounding::NearestTiesEven),
            Ok(3)
        );
        assert_eq!(
            round_f64(4503599627370497.0, Rounding::Floor),
            Ok(4503599627370497)
        );
        assert_eq!(round_f64(1e300, Rounding::Floor), Err(RoundError::Overflow));
        assert_eq!(
            round_f64(f64::NAN, Rounding::Floor),
            Err(RoundError::NotFinite)
        );
        assert_eq!(round_f64(0.5, Rounding::Exact), Err(RoundError::Inexact));
        assert_eq!(round_f64(-3.0, Rounding::Exact), Ok(-3));
    }
}
