//! Integer quantities in a unit of your choice, explicit rounding, and total
//! ordering for float quantities.
//!
//! See the crate-level documentation for an overview.

use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::ops;

use crate::{Float, ParseError, Quantity};

// ── Numeric traits ────────────────────────────────────────────────────────────

/// Lossy conversion of a primitive number to `f64` / `f32` (Rust's `as`).
///
/// Implemented for every primitive integer and float type.
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

/// The primitive integer types, usable as the count of a [`Fixed`] quantity
/// and as the storage of base-unit quantities such as `Length<i64>`.
///
/// Sealed: implemented for `i8`–`i128`, `u8`–`u128`, `isize` and `usize`.
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
    /// `(is_negative, |self|)`, defined for every value of every type.
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

// ── Rounding ──────────────────────────────────────────────────────────────────

/// How to round when a result cannot be represented exactly as an integer.
///
/// Every lossy integer operation takes one explicitly; lossless ones take
/// none. Pass a constant and the choice is resolved at compile time.
///
/// | Mode | 2.5 | −2.5 | 2.4 | −2.6 |
/// |---|---|---|---|---|
/// | `Floor` | 2 | −3 | 2 | −3 |
/// | `Ceil` | 3 | −2 | 3 | −2 |
/// | `TowardZero` | 2 | −2 | 2 | −2 |
/// | `AwayFromZero` | 3 | −3 | 3 | −3 |
/// | `NearestTiesEven` | 2 | −2 | 2 | −3 |
/// | `NearestTiesAway` | 3 | −3 | 2 | −3 |
/// | `Exact` | error | error | error | error |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rounding {
    /// Toward −∞.
    Floor,
    /// Toward +∞.
    Ceil,
    /// Toward zero (truncation, like integer division).
    TowardZero,
    /// Away from zero.
    AwayFromZero,
    /// To the nearest integer; ties to the even one ("banker's rounding",
    /// the IEEE 754 default). Unbiased, so the best choice for statistics
    /// and accumulated values.
    NearestTiesEven,
    /// To the nearest integer; ties away from zero (school rounding, like
    /// `f64::round`).
    NearestTiesAway,
    /// Refuse to round: fail with [`RoundError::Inexact`] unless the result
    /// is exactly representable.
    Exact,
}

/// Why a rounding conversion or division failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoundError {
    /// The result does not fit in the integer type.
    Overflow,
    /// The input is NaN or infinite.
    NotFinite,
    /// [`Rounding::Exact`] was requested but the result is not an integer.
    Inexact,
    /// Division by zero.
    DivisionByZero,
}

impl fmt::Display for RoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            RoundError::Overflow => "result out of range for the integer type",
            RoundError::NotFinite => "value is NaN or infinite",
            RoundError::Inexact => "result is not exactly representable",
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
pub(crate) fn div_round(n: i128, d: i128, mode: Rounding) -> Result<i128, RoundError> {
    if d == 0 {
        return Err(RoundError::DivisionByZero);
    }
    let t = n.checked_div(d).ok_or(RoundError::Overflow)?;
    let r = n % d;
    if r == 0 {
        return Ok(t);
    }
    // Compare |r| with |d| − |r| instead of 2|r| with |d|: cannot overflow.
    let (ar, ad) = (r.unsigned_abs(), d.unsigned_abs());
    let positive = (r > 0) == (d > 0);
    pick(t, positive, ar.cmp(&(ad - ar)), mode)
}

/// Rounds a float to an integer. Exact: no intermediate rounding.
#[inline]
pub(crate) fn round_f64(x: f64, mode: Rounding) -> Result<i128, RoundError> {
    if !x.is_finite() {
        return Err(RoundError::NotFinite);
    }
    // 2^127: beyond it the value cannot fit an i128.
    if !(-1.7014118346046923e38..1.7014118346046923e38).contains(&x) {
        return Err(RoundError::Overflow);
    }
    // Truncate toward zero. Through i64 when possible: one instruction each
    // way, where i128 needs library calls.
    let (t, back) = if (-9.223372036854775e18..9.223372036854775e18).contains(&x) {
        let t = x as i64;
        (t as i128, t as f64)
    } else {
        let t = x as i128;
        (t, t as f64)
    };
    // Exact: for |x| < 2^52, `t` and `x` are close floats (Sterbenz); above
    // it, `x` is already an integer and the difference is zero.
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

#[inline(always)]
fn narrow<I: Integer>(v: i128) -> Result<I, RoundError> {
    I::from_i128(v).ok_or(RoundError::Overflow)
}

#[inline(always)]
fn widen<I: Integer>(v: I) -> Result<i128, RoundError> {
    v.to_i128().ok_or(RoundError::Overflow)
}

// ── Units for fixed quantities ────────────────────────────────────────────────

/// A unit that [`Fixed`] integer quantities can be counted in.
///
/// Implemented by the zero-sized marker types in [`crate::unit`], one per
/// linear unit (affine units such as °C have no marker: adding two counts of
/// °C is not adding two temperatures).
pub trait FixedUnit: Copy + 'static {
    /// The quantity with float storage `F`, e.g. `Length<F>`.
    type Quantity<F: Float>: Quantity<Value = F, Unit = Self::Unit>
        + PartialEq
        + PartialOrd
        + fmt::Debug;

    /// The quantity's unit enum, e.g. [`LengthUnit`](crate::LengthUnit).
    /// Units of the same quantity share it.
    type Unit: Copy + Eq + fmt::Debug + fmt::Display + 'static;

    /// The marker of the quantity's base unit.
    type Base: FixedUnit<Unit = Self::Unit>;

    /// This unit as an enum value.
    const UNIT: Self::Unit;

    /// Primary abbreviation, used by `Display`.
    const ABBREVIATION: &'static str;

    /// Every abbreviation accepted by `FromStr`.
    const ABBREVIATIONS: &'static [&'static str];

    /// One of this unit is `NUM / DEN · π^PI_POW` base units, exactly
    /// (a reduced fraction).
    const NUM: u128;
    /// See [`NUM`](Self::NUM).
    const DEN: u128;
    /// See [`NUM`](Self::NUM).
    const PI_POW: i32;

    /// For affine units (°C, °F, dBµV, …): zero of this unit is
    /// `OFFSET_NUM / OFFSET_DEN` base units, exactly. Zero for linear units.
    const OFFSET_NUM: i128 = 0;
    /// See [`OFFSET_NUM`](Self::OFFSET_NUM).
    const OFFSET_DEN: u128 = 1;

    /// A float count of this unit as a quantity, e.g. `Length::from_millimeters`.
    fn to_quantity<F: Float>(count: F) -> Self::Quantity<F>;

    /// A quantity as a float count of this unit, e.g. `Length::as_millimeters`.
    fn from_quantity<F: Float>(quantity: Self::Quantity<F>) -> F;
}

/// A [`FixedUnit`] whose counts can be added, subtracted, negated and
/// scaled: every unit except those of point-like quantities (absolute
/// temperatures, where 20 °C + 20 °C is not 40 °C).
pub trait AdditiveUnit: FixedUnit {}

/// A [`FixedUnit`] of a point quantity (an absolute temperature, a decibel
/// level): counts cannot be added, but their differences are counts of
/// [`Delta`](Self::Delta), which can be added back.
///
/// ```rust
/// use unitsnet_rs::*;
/// use unitsnet_rs::unit::temperature::{DegreeCelsius, Kelvin, MillidegreeCelsius};
/// use unitsnet_rs::unit::temperature_delta::MillidegreeCelsius as DeltaMilli;
///
/// let morning = Fixed::<MillidegreeCelsius>::new(12_500);
/// let noon = Fixed::<MillidegreeCelsius>::new(20_250);
/// let rise: Fixed<DeltaMilli> = noon - morning;
/// assert_eq!(rise.count(), 7_750);
/// assert_eq!(morning + rise, noon);
///
/// // Exact across scales: °C → m°C and K → m°C need no rounding.
/// assert_eq!(Fixed::<DegreeCelsius>::new(20).to_unit::<MillidegreeCelsius>()?.count(), 20_000);
/// assert_eq!(Fixed::<Kelvin>::new(300).to_unit::<MillidegreeCelsius>()?.count(), 26_850);
/// // m°C → K is not: 273.15 K is not a whole number of kelvins.
/// assert_eq!(
///     Fixed::<MillidegreeCelsius>::new(0).to_unit_rounded::<Kelvin>(Rounding::NearestTiesEven)?.count(),
///     273
/// );
/// // Comparisons across scales are exact.
/// assert!(Fixed::<DegreeCelsius>::new(20) > Fixed::<Kelvin>::new(293));
/// # Ok::<(), RoundError>(())
/// ```
pub trait PointUnit: FixedUnit {
    /// The unit of the difference between two points, with the same scale
    /// (°C → Δ°C, m°C → Δm°C, dBµV → dB).
    type Delta: AdditiveUnit;
}

/// A relation `Self × Rhs = Output` between the unit enums of three
/// quantities (`LengthUnit × LengthUnit = AreaUnit`), with the exact factor
/// between their base units: `output = K · lhs · rhs`, `K = num / den · π^pi`.
///
/// Generated for every multiplication relation between quantities; used by
/// [`Fixed::mul_to`].
pub trait UnitMul<Rhs> {
    /// The unit enum of the product's quantity.
    type Output;
    /// `(num, den, pi)`: see the trait documentation.
    const K: (u128, u128, i32);
}

/// A relation `Self ÷ Rhs = Output` between the unit enums of three
/// quantities (`LengthUnit ÷ DurationUnit = SpeedUnit`), with the exact
/// factor between their base units: `output = K · lhs / rhs`.
///
/// Generated for every division relation between quantities; used by
/// [`Fixed::div_to`].
pub trait UnitDiv<Rhs> {
    /// The unit enum of the quotient's quantity.
    type Output;
    /// `(num, den, pi)`: see [`UnitMul`].
    const K: (u128, u128, i32);
}

/// `a/b · c/d`, reduced, at compile time.
const fn frac_mul((a, b): (u128, u128), (c, d): (u128, u128)) -> (u128, u128) {
    let (g1, g2) = (gcd(a, d), gcd(c, b));
    match ((a / g1).checked_mul(c / g2), (b / g2).checked_mul(d / g1)) {
        (Some(num), Some(den)) => (num, den),
        _ => panic!("the ratio between these units is too large for integer arithmetic"),
    }
}

/// The exact factor `R` in `count_V = R · count_U ⊙ count_W` for
/// `U ⊙ W → V`, where `⊙` is `×` (`DIV = false`) or `÷` (`DIV = true`).
struct RelationRatio<U, W, V, const DIV: bool>(PhantomData<(U, W, V)>);

impl<U: FixedUnit, W: FixedUnit, V: FixedUnit, const DIV: bool> RelationRatio<U, W, V, DIV> {
    const fn parts(k: (u128, u128, i32)) -> (u128, u128) {
        // U · W^±1 · K / V, in base units.
        let w_pi = if DIV { -W::PI_POW } else { W::PI_POW };
        assert!(
            U::PI_POW + w_pi + k.2 == V::PI_POW,
            "the ratio between these units is irrational (involves π): \
             compute through float quantities instead"
        );
        let w = if DIV {
            (W::DEN, W::NUM)
        } else {
            (W::NUM, W::DEN)
        };
        let r = frac_mul((U::NUM, U::DEN), w);
        let r = frac_mul(r, (k.0, k.1));
        frac_mul(r, (V::DEN, V::NUM))
    }
}

impl<U: FixedUnit, W: FixedUnit, V: FixedUnit> RelationRatio<U, W, V, false>
where
    U::Unit: UnitMul<W::Unit>,
{
    const PARTS: (u128, u128) = Self::parts(<U::Unit as UnitMul<W::Unit>>::K);
    const EXACT: () = assert!(
        Self::PARTS.1 == 1,
        "this product is not a whole number of the target unit (mm · mm is \
         not a whole number of m²): use `mul_to_rounded`, or a finer target unit"
    );
}

impl<U: FixedUnit, W: FixedUnit, V: FixedUnit> RelationRatio<U, W, V, true>
where
    U::Unit: UnitDiv<W::Unit>,
{
    const PARTS: (u128, u128) = Self::parts(<U::Unit as UnitDiv<W::Unit>>::K);
}

/// `count · k` with the overflow behaviour of integer `*`.
#[inline(always)]
fn scale_count<I: Integer>(count: I, k: u128) -> I {
    if k == 1 {
        return count;
    }
    match i128::try_from(k).ok().and_then(I::from_i128) {
        Some(k) => count * k,
        // The factor alone exceeds `I`: only zero is representable.
        None if count == I::default() => count,
        None => panic!("attempt to multiply with overflow"),
    }
}

const fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// The exact ratio `From / To` between two units, computed at compile time.
struct Ratio<From, To>(PhantomData<(From, To)>);

impl<From: FixedUnit, To: FixedUnit> Ratio<From, To> {
    /// `(n1/d1) / (n2/d2)` as a reduced fraction.
    const PARTS: (u128, u128) = {
        assert!(
            From::PI_POW == To::PI_POW,
            "the ratio between these units is irrational (involves π): \
             convert through a float quantity instead"
        );
        let g1 = gcd(From::NUM, To::NUM);
        let g2 = gcd(From::DEN, To::DEN);
        match (
            (From::NUM / g1).checked_mul(To::DEN / g2),
            (From::DEN / g2).checked_mul(To::NUM / g1),
        ) {
            (Some(num), Some(den)) => (num, den),
            _ => panic!("the ratio between these units is too large for integer conversion"),
        }
    };
    const NUM: u128 = Self::PARTS.0;
    const DEN: u128 = Self::PARTS.1;
    /// `(b_From − b_To) / a_To`: the offset, in units of `To`, added after
    /// scaling. Zero unless an affine unit is involved.
    const OFFSET: (i128, u128) = {
        if From::OFFSET_NUM == 0 && To::OFFSET_NUM == 0 {
            (0, 1)
        } else {
            assert!(
                From::PI_POW == 0 && To::PI_POW == 0,
                "affine units with a factor of π are not supported"
            );
            let diff = sub_i(
                mul_i(From::OFFSET_NUM, to_i(To::OFFSET_DEN)),
                mul_i(To::OFFSET_NUM, to_i(From::OFFSET_DEN)),
            );
            let num = mul_i(diff, to_i(To::DEN));
            let den = mul_i(
                mul_i(to_i(From::OFFSET_DEN), to_i(To::OFFSET_DEN)),
                to_i(To::NUM),
            );
            let g = gcd(num.unsigned_abs(), den as u128) as i128;
            (num / g, (den / g) as u128)
        }
    };
    const EXACT: () = assert!(
        Self::DEN == 1 && Self::OFFSET.1 == 1,
        "this unit conversion is not exact (the target unit does not divide the \
         source unit): for `+`/`-` put the finer unit on the left (`mm + m`), \
         for conversions use `to_unit_rounded` with a `Rounding` mode"
    );
}

const fn to_i(x: u128) -> i128 {
    assert!(
        x <= i128::MAX as u128,
        "unit definition too large for integer conversion"
    );
    x as i128
}

const fn mul_i(a: i128, b: i128) -> i128 {
    match a.checked_mul(b) {
        Some(v) => v,
        None => panic!("unit definitions too large for integer conversion"),
    }
}

const fn sub_i(a: i128, b: i128) -> i128 {
    match a.checked_sub(b) {
        Some(v) => v,
        None => panic!("unit definitions too large for integer conversion"),
    }
}

/// `count · num / den + offset`, rounded once.
#[inline]
fn rescale(
    count: i128,
    num: u128,
    den: u128,
    (off_num, off_den): (i128, u128),
    mode: Rounding,
) -> Result<i128, RoundError> {
    let overflow = |_| RoundError::Overflow;
    let num = i128::try_from(num).map_err(overflow)?;
    let den = i128::try_from(den).map_err(overflow)?;
    let n = count.checked_mul(num).ok_or(RoundError::Overflow)?;
    if off_num == 0 {
        return if den == 1 {
            Ok(n)
        } else {
            div_round(n, den, mode)
        };
    }
    // (count·num·off_den + off_num·den) / (den·off_den)
    let off_den = i128::try_from(off_den).map_err(overflow)?;
    let n = n
        .checked_mul(off_den)
        .and_then(|n| n.checked_add(off_num.checked_mul(den)?))
        .ok_or(RoundError::Overflow)?;
    let d = den.checked_mul(off_den).ok_or(RoundError::Overflow)?;
    if d == 1 {
        Ok(n)
    } else {
        div_round(n, d, mode)
    }
}

// ── Fixed ─────────────────────────────────────────────────────────────────────

/// An integer number of a fixed unit `U`, e.g. `Fixed<Millimeter>` is a
/// whole number of millimetres stored as an `i64`.
///
/// * **The unit is part of the type.** Mixing units is allowed exactly where
///   it is lossless: `millimetres + metres` gives millimetres, while
///   `metres + millimetres` does not compile (the result would need
///   rounding: swap the operands, or use
///   [`to_unit_rounded`](Self::to_unit_rounded) with a [`Rounding`] mode).
///   Comparisons (`==`, `<`, …) work between any two units, exactly.
/// * **Exact and free**: arithmetic in the same unit is plain integer
///   arithmetic (`#[repr(transparent)]`, same machine code as the bare
///   integer), and values are `Eq + Ord + Hash` — ideal as map keys.
/// * **Overflow** behaves like Rust integers (panics in debug builds, wraps
///   in release); `checked_*` / `saturating_*` variants are provided.
///
/// ```rust
/// use unitsnet_rs::*;
/// use unitsnet_rs::unit::length::{Meter, Millimeter};
///
/// // Type aliases keep literals short.
/// type Millimeters = Fixed<Millimeter>;
/// type Meters = Fixed<Meter>;
///
/// let a = Millimeters::new(1_250);
/// let b = Millimeters::new(300);
/// assert_eq!(a + b, Millimeters::new(1_550));
/// assert_eq!(format!("{}", a + b), "1550 mm");
///
/// // Mixed units: exact, in the left operand's unit.
/// assert_eq!(a + Meters::new(2), Millimeters::new(3_250));
/// assert!(Meters::new(1) < a);
/// assert_eq!(Meters::new(2), Millimeters::new(2_000));
///
/// // To floats and back.
/// assert_eq!(a.to_f64(), Length::from_meters(1.25));
/// let c = Fixed::<Millimeter>::round_from(Length::from_meters(0.0125), Rounding::NearestTiesEven);
/// assert_eq!(c, Ok(Fixed::new(12))); // 12.5 mm, tie to even
///
/// // Between units: exact one way, explicitly rounded the other.
/// let m = Fixed::<Meter>::new(2);
/// assert_eq!(m.to_unit::<Millimeter>(), Ok(Fixed::new(2_000)));
/// assert_eq!(a.to_unit_rounded::<Meter>(Rounding::Floor), Ok(Fixed::new(1)));
/// assert_eq!(a.to_unit_rounded::<Meter>(Rounding::Exact), Err(RoundError::Inexact));
/// ```
#[repr(transparent)]
#[must_use]
pub struct Fixed<U, I = i64> {
    count: I,
    unit: PhantomData<U>,
}

impl<U, I> Fixed<U, I> {
    /// A count of `U`. Free and `const`.
    #[inline(always)]
    pub const fn new(count: I) -> Self {
        Self {
            count,
            unit: PhantomData,
        }
    }

    /// Views a slice of counts as quantities, without copying.
    #[inline(always)]
    pub const fn from_count_slice(counts: &[I]) -> &[Self] {
        // SAFETY: `Self` is `#[repr(transparent)]` over `I` (`PhantomData` is a ZST).
        unsafe { &*(counts as *const [I] as *const [Self]) }
    }

    /// Views a slice of quantities as their counts, without copying.
    #[inline(always)]
    pub const fn as_count_slice(values: &[Self]) -> &[I] {
        // SAFETY: as above.
        unsafe { &*(values as *const [Self] as *const [I]) }
    }

    /// Views a mutable slice of counts as quantities, without copying.
    #[inline(always)]
    pub fn from_count_slice_mut(counts: &mut [I]) -> &mut [Self] {
        // SAFETY: as above.
        unsafe { &mut *(counts as *mut [I] as *mut [Self]) }
    }

    /// Views a mutable slice of quantities as their counts, without copying.
    #[inline(always)]
    pub fn as_count_slice_mut(values: &mut [Self]) -> &mut [I] {
        // SAFETY: as above.
        unsafe { &mut *(values as *mut [Self] as *mut [I]) }
    }
}

impl<U, I: Copy> Fixed<U, I> {
    /// The number of `U`. Free and `const`.
    #[inline(always)]
    pub const fn count(self) -> I {
        self.count
    }
}

impl<U: FixedUnit, I: Integer> Fixed<U, I> {
    /// The quantity as `f64` storage, e.g. 1 250 mm → 1.25 m.
    #[inline]
    pub fn to_f64(self) -> U::Quantity<f64> {
        U::to_quantity(self.count.to_f64())
    }

    /// The quantity as `f32` storage.
    #[inline]
    pub fn to_f32(self) -> U::Quantity<f32> {
        U::to_quantity(self.count.to_f32())
    }

    /// Rounds a float quantity to a whole number of `U`.
    #[inline]
    pub fn round_from<F: Float>(
        quantity: U::Quantity<F>,
        mode: Rounding,
    ) -> Result<Self, RoundError> {
        Self::round_from_count(U::from_quantity(quantity), mode)
    }

    /// Rounds a float count of `U` (e.g. a sensor reading in millimetres)
    /// to a whole number of `U`. Unlike going through
    /// [`round_from`](Self::round_from), this involves no unit conversion,
    /// so ties such as 2.5 are seen exactly.
    #[inline]
    pub fn round_from_count<F: Float>(count: F, mode: Rounding) -> Result<Self, RoundError> {
        round_f64(count.to_f64(), mode)
            .and_then(narrow)
            .map(Self::new)
    }

    /// Converts to a smaller (or equal) unit, exactly: metres → millimetres.
    ///
    /// Fails only on overflow. Converting to a unit that does not divide
    /// this one evenly (millimetres → metres) is a **compile-time error**;
    /// use [`to_unit_rounded`](Self::to_unit_rounded) for those.
    #[inline]
    pub fn to_unit<V: FixedUnit<Unit = U::Unit>>(self) -> Result<Fixed<V, I>, RoundError> {
        #[allow(clippy::let_unit_value)]
        let () = Ratio::<U, V>::EXACT;
        self.to_unit_rounded(Rounding::Exact)
    }

    /// Converts to any unit of the same quantity, rounding with `mode`.
    #[inline]
    pub fn to_unit_rounded<V: FixedUnit<Unit = U::Unit>>(
        self,
        mode: Rounding,
    ) -> Result<Fixed<V, I>, RoundError> {
        let n = rescale(
            widen(self.count)?,
            Ratio::<U, V>::NUM,
            Ratio::<U, V>::DEN,
            Ratio::<U, V>::OFFSET,
            mode,
        )?;
        narrow(n).map(Fixed::new)
    }
}

/// Arithmetic helpers for units whose counts can be added.
impl<U: AdditiveUnit, I: Integer> Fixed<U, I> {
    /// Ratio of two values as a rounded integer.
    #[inline]
    pub fn ratio_rounded(self, other: Self, mode: Rounding) -> Result<I, RoundError> {
        div_round(widen(self.count)?, widen(other.count)?, mode).and_then(narrow)
    }

    /// Divides by an integer, rounding with `mode` (`/` truncates toward
    /// zero, like integer division).
    #[inline]
    pub fn div_rounded(self, divisor: I, mode: Rounding) -> Result<Self, RoundError> {
        div_round(widen(self.count)?, widen(divisor)?, mode)
            .and_then(narrow)
            .map(Self::new)
    }

    /// `self + other`, or `None` on overflow.
    #[inline]
    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.count.checked_add(other.count).map(Self::new)
    }

    /// `self - other`, or `None` on overflow.
    #[inline]
    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.count.checked_sub(other.count).map(Self::new)
    }

    /// `self * factor`, or `None` on overflow.
    #[inline]
    pub fn checked_mul(self, factor: I) -> Option<Self> {
        self.count.checked_mul(factor).map(Self::new)
    }

    /// The product with a count of another quantity, as a count of the
    /// related quantity in `unit` — exactly: `mm × mm → mm²`,
    /// `N × m → N·m`, `km/h × h → km`.
    ///
    /// `unit` names the result's unit (the marker is a unit struct). The
    /// conversion must be exact — for example `mm × mm` into `m²` is not —
    /// which is checked when the code is compiled; for other units use
    /// [`mul_to_rounded`](Self::mul_to_rounded). Overflow behaves like
    /// integer `*`.
    ///
    /// ```rust
    /// use unitsnet_rs::*;
    /// use unitsnet_rs::unit::{area::SquareMillimeter, length::{Meter, Millimeter}};
    /// use unitsnet_rs::unit::{duration::Hour, speed::KilometerPerHour, length::Kilometer};
    ///
    /// let width = Fixed::<Millimeter>::new(1_200);
    /// let height = Fixed::<Millimeter>::new(800);
    /// assert_eq!(width.mul_to(height, SquareMillimeter).count(), 960_000);
    ///
    /// // Mixed units, as long as the result is exact: m × mm = 1000 mm².
    /// assert_eq!(Fixed::<Meter>::new(2).mul_to(height, SquareMillimeter).count(), 1_600_000);
    ///
    /// let distance = Fixed::<KilometerPerHour>::new(90).mul_to(Fixed::<Hour>::new(3), Kilometer);
    /// assert_eq!(distance.count(), 270);
    /// ```
    #[inline]
    pub fn mul_to<W, V>(self, rhs: Fixed<W, I>, unit: V) -> Fixed<V, I>
    where
        W: AdditiveUnit,
        U::Unit: UnitMul<W::Unit>,
        V: AdditiveUnit<Unit = <U::Unit as UnitMul<W::Unit>>::Output>,
    {
        let _ = unit;
        #[allow(clippy::let_unit_value)]
        let () = RelationRatio::<U, W, V, false>::EXACT;
        Fixed::new(scale_count(
            self.count * rhs.count,
            RelationRatio::<U, W, V, false>::PARTS.0,
        ))
    }

    /// The product with a count of another quantity, as a count of the
    /// related quantity in `unit`, rounding with `mode`.
    ///
    /// ```rust
    /// use unitsnet_rs::*;
    /// use unitsnet_rs::unit::{area::SquareMeter, length::Millimeter};
    ///
    /// let side = Fixed::<Millimeter>::new(1_500);
    /// assert_eq!(side.mul_to_rounded(side, SquareMeter, Rounding::NearestTiesEven), Ok(Fixed::new(2))); // 2.25 m²
    /// assert_eq!(side.mul_to_rounded(side, SquareMeter, Rounding::Exact), Err(RoundError::Inexact));
    /// ```
    #[inline]
    pub fn mul_to_rounded<W, V>(
        self,
        rhs: Fixed<W, I>,
        unit: V,
        mode: Rounding,
    ) -> Result<Fixed<V, I>, RoundError>
    where
        W: AdditiveUnit,
        U::Unit: UnitMul<W::Unit>,
        V: AdditiveUnit<Unit = <U::Unit as UnitMul<W::Unit>>::Output>,
    {
        let _ = unit;
        let (num, den) = RelationRatio::<U, W, V, false>::PARTS;
        let product = widen(self.count)?
            .checked_mul(widen(rhs.count)?)
            .ok_or(RoundError::Overflow)?;
        rescale(product, num, den, (0, 1), mode)
            .and_then(narrow)
            .map(Fixed::new)
    }

    /// The quotient by a count of another quantity, as a count of the
    /// related quantity in `unit`, rounding with `mode`:
    /// `mm ÷ ms → mm/s`, `J ÷ s → W`.
    ///
    /// ```rust
    /// use unitsnet_rs::*;
    /// use unitsnet_rs::unit::{duration::Millisecond, length::Millimeter, speed::MillimeterPerSecond};
    ///
    /// let travel = Fixed::<Millimeter>::new(250);
    /// let time = Fixed::<Millisecond>::new(400);
    /// // 250 mm / 0.4 s = 625 mm/s, exactly.
    /// assert_eq!(travel.div_to(time, MillimeterPerSecond, Rounding::Exact), Ok(Fixed::new(625)));
    /// assert_eq!(
    ///     travel.div_to(Fixed::<Millisecond>::new(0), MillimeterPerSecond, Rounding::Floor),
    ///     Err(RoundError::DivisionByZero)
    /// );
    /// ```
    #[inline]
    pub fn div_to<W, V>(
        self,
        rhs: Fixed<W, I>,
        unit: V,
        mode: Rounding,
    ) -> Result<Fixed<V, I>, RoundError>
    where
        W: AdditiveUnit,
        U::Unit: UnitDiv<W::Unit>,
        V: AdditiveUnit<Unit = <U::Unit as UnitDiv<W::Unit>>::Output>,
    {
        let _ = unit;
        let (num, den) = RelationRatio::<U, W, V, true>::PARTS;
        let overflow = |_| RoundError::Overflow;
        let n = widen(self.count)?
            .checked_mul(i128::try_from(num).map_err(overflow)?)
            .ok_or(RoundError::Overflow)?;
        let d = widen(rhs.count)?
            .checked_mul(i128::try_from(den).map_err(overflow)?)
            .ok_or(RoundError::Overflow)?;
        div_round(n, d, mode).and_then(narrow).map(Fixed::new)
    }

    /// `self + other`, clamped to the integer range.
    #[inline]
    pub fn saturating_add(self, other: Self) -> Self {
        Self::new(self.count.saturating_add(other.count))
    }

    /// `self - other`, clamped to the integer range.
    #[inline]
    pub fn saturating_sub(self, other: Self) -> Self {
        Self::new(self.count.saturating_sub(other.count))
    }
}

/// Points: subtracting a difference. (`point - delta` cannot be an operator:
/// `point - point` already is, and Rust cannot tell the two apart by the
/// kind of the right-hand side. `point + (-delta)` also works.)
impl<P: PointUnit, I: Integer> Fixed<P, I> {
    /// `self - delta`, where `delta` must convert exactly into
    /// [`P::Delta`](PointUnit::Delta) (checked at compile time).
    #[inline]
    pub fn sub_delta<D: AdditiveUnit<Unit = <P::Delta as FixedUnit>::Unit>>(
        self,
        delta: Fixed<D, I>,
    ) -> Self {
        Self::new(self.count - delta.count_in::<P::Delta>())
    }
}

// Manual impls: derives would require `U: Trait` for the zero-sized marker.

impl<U, I: Clone> Clone for Fixed<U, I> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self::new(self.count.clone())
    }
}
impl<U, I: Copy> Copy for Fixed<U, I> {}
impl<U, I: Default> Default for Fixed<U, I> {
    #[inline(always)]
    fn default() -> Self {
        Self::new(I::default())
    }
}
/// Exact comparison across units of the same quantity: `2 m > 1500 mm`.
///
/// Decided with exact integer arithmetic (no rounding, no floats, no
/// overflow), so equality and order are mathematically correct for any two
/// values. Comparing within one unit compiles to a plain integer comparison.
impl<U: FixedUnit, V: FixedUnit<Unit = U::Unit>, I: Integer> PartialEq<Fixed<V, I>>
    for Fixed<U, I>
{
    #[inline(always)]
    fn eq(&self, other: &Fixed<V, I>) -> bool {
        cmp_units::<U, V, I>(self.count, other.count) == Ordering::Equal
    }
}
impl<U: FixedUnit, I: Integer> Eq for Fixed<U, I> {}
impl<U: FixedUnit, V: FixedUnit<Unit = U::Unit>, I: Integer> PartialOrd<Fixed<V, I>>
    for Fixed<U, I>
{
    #[inline(always)]
    fn partial_cmp(&self, other: &Fixed<V, I>) -> Option<Ordering> {
        Some(cmp_units::<U, V, I>(self.count, other.count))
    }
}
impl<U: FixedUnit, I: Integer> Ord for Fixed<U, I> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.count.cmp(&other.count)
    }
}
impl<U, I: Hash> Hash for Fixed<U, I> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.count.hash(state);
    }
}

macro_rules! fixed_binop {
    ($Trait:ident $method:ident $TraitAssign:ident $method_assign:ident) => {
        impl<U: AdditiveUnit, I: ops::$Trait<Output = I>> ops::$Trait for Fixed<U, I> {
            type Output = Self;
            #[inline(always)]
            fn $method(self, rhs: Self) -> Self {
                Self::new(ops::$Trait::$method(self.count, rhs.count))
            }
        }
        impl<U: AdditiveUnit, I: ops::$TraitAssign> ops::$TraitAssign for Fixed<U, I> {
            #[inline(always)]
            fn $method_assign(&mut self, rhs: Self) {
                ops::$TraitAssign::$method_assign(&mut self.count, rhs.count);
            }
        }
    };
}
fixed_binop!(Rem rem RemAssign rem_assign);

/// Adding or subtracting another unit of the same quantity: the result is in
/// the left operand's unit, so the right operand must convert into it
/// exactly. `millimetres + metres` is fine (1 m = 1000 mm);
/// `metres + millimetres` is a compile-time error — swap the operands, or
/// convert explicitly with [`Fixed::to_unit_rounded`].
macro_rules! fixed_unit_op {
    ($Trait:ident $method:ident $TraitAssign:ident $method_assign:ident $op:tt) => {
        impl<U: AdditiveUnit, V: AdditiveUnit<Unit = U::Unit>, I: Integer> ops::$Trait<Fixed<V, I>>
            for Fixed<U, I>
        {
            type Output = Self;
            #[inline(always)]
            fn $method(self, rhs: Fixed<V, I>) -> Self {
                Self::new(self.count $op rhs.count_in::<U>())
            }
        }
        impl<U: AdditiveUnit, V: AdditiveUnit<Unit = U::Unit>, I: Integer> ops::$TraitAssign<Fixed<V, I>>
            for Fixed<U, I>
        {
            #[inline(always)]
            fn $method_assign(&mut self, rhs: Fixed<V, I>) {
                self.count = self.count $op rhs.count_in::<U>();
            }
        }
    };
}
fixed_unit_op!(Add add AddAssign add_assign +);
fixed_unit_op!(Sub sub SubAssign sub_assign -);

impl<V: FixedUnit, I: Integer> Fixed<V, I> {
    /// The count expressed in `U`, which this unit must convert into
    /// exactly (checked at compile time). Overflow behaves like integer `*`.
    #[doc(hidden)]
    #[inline(always)]
    pub fn count_in<U: FixedUnit<Unit = V::Unit>>(self) -> I {
        #[allow(clippy::let_unit_value)]
        let () = Ratio::<V, U>::EXACT;
        let scaled = if Ratio::<V, U>::NUM == 1 {
            self.count
        } else {
            match i128::try_from(Ratio::<V, U>::NUM)
                .ok()
                .and_then(I::from_i128)
            {
                Some(k) => self.count * k,
                // The factor alone exceeds `I`: only zero converts.
                None if self.count == I::default() => self.count,
                None => panic!("attempt to convert units with overflow"),
            }
        };
        let offset = Ratio::<V, U>::OFFSET.0;
        if offset == 0 {
            return scaled;
        }
        match I::from_i128(offset) {
            Some(offset) => scaled + offset,
            None => panic!("attempt to convert units with overflow"),
        }
    }
}

/// Exact `a [U]` vs `b [V]`: compares `a · num` with `b · den`.
#[inline(always)]
fn cmp_units<U: FixedUnit, V: FixedUnit<Unit = U::Unit>, I: Integer>(a: I, b: I) -> Ordering {
    let (num, den) = (Ratio::<U, V>::NUM, Ratio::<U, V>::DEN);
    let (off_num, off_den) = Ratio::<U, V>::OFFSET;
    if off_num != 0 {
        return cmp_affine(a, b, num, den, off_num, off_den);
    }
    if num == 1 && den == 1 {
        return a.cmp(&b);
    }
    // Fast path: both products fit in i128 (all realistic values).
    if let (Some(a), Some(b), Ok(num), Ok(den)) = (
        a.to_i128(),
        b.to_i128(),
        i128::try_from(num),
        i128::try_from(den),
    ) {
        if let (Some(pa), Some(pb)) = (a.checked_mul(num), b.checked_mul(den)) {
            return pa.cmp(&pb);
        }
    }
    // Exact for any magnitudes: sign and 256-bit magnitude.
    let (neg_a, mag_a) = a.sign_magnitude();
    let (neg_b, mag_b) = b.sign_magnitude();
    let (pa, pb) = (mul_wide(mag_a, num), mul_wide(mag_b, den));
    // Zero has no sign: compare magnitudes when the signs agree or both are zero.
    let neg_a = neg_a && pa != (0, 0);
    let neg_b = neg_b && pb != (0, 0);
    match (neg_a, neg_b) {
        (false, false) => pa.cmp(&pb),
        (true, true) => pb.cmp(&pa),
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
    }
}

/// `a·num/den + off_num/off_den` vs `b`, i.e.
/// `a·num·off_den + off_num·den` vs `b·den·off_den`. Exact in `i128`, which
/// holds every realistic temperature or level; beyond that (counts near the
/// limits of `i128`), compared through `f64`.
#[cold]
#[inline(never)]
fn cmp_affine<I: Integer>(
    a: I,
    b: I,
    num: u128,
    den: u128,
    off_num: i128,
    off_den: u128,
) -> Ordering {
    let exact = (|| {
        let (num, den, off_den) = (to_i128(num)?, to_i128(den)?, to_i128(off_den)?);
        let lhs = a
            .to_i128()?
            .checked_mul(num)?
            .checked_mul(off_den)?
            .checked_add(off_num.checked_mul(den)?)?;
        let rhs = b.to_i128()?.checked_mul(den)?.checked_mul(off_den)?;
        Some(lhs.cmp(&rhs))
    })();
    exact.unwrap_or_else(|| {
        let lhs = a.to_f64() * num as f64 / den as f64 + off_num as f64 / off_den as f64;
        lhs.total_cmp(&b.to_f64())
    })
}

fn to_i128(x: u128) -> Option<i128> {
    i128::try_from(x).ok()
}

/// Full 256-bit product of two `u128`, as `(high, low)`.
#[inline(always)]
fn mul_wide(x: u128, y: u128) -> (u128, u128) {
    const LO: u128 = u64::MAX as u128;
    let (x1, x0) = (x >> 64, x & LO);
    let (y1, y0) = (y >> 64, y & LO);
    let (p00, p01, p10, p11) = (x0 * y0, x0 * y1, x1 * y0, x1 * y1);
    let mid = (p00 >> 64) + (p01 & LO) + (p10 & LO);
    let low = (mid << 64) | (p00 & LO);
    let high = p11 + (p01 >> 64) + (p10 >> 64) + (mid >> 64);
    (high, low)
}

macro_rules! fixed_scalar_op {
    ($Trait:ident $method:ident $TraitAssign:ident $method_assign:ident) => {
        impl<U: AdditiveUnit, I: ops::$Trait<Output = I>> ops::$Trait<I> for Fixed<U, I> {
            type Output = Self;
            #[inline(always)]
            fn $method(self, rhs: I) -> Self {
                Self::new(ops::$Trait::$method(self.count, rhs))
            }
        }
        impl<U: AdditiveUnit, I: ops::$TraitAssign> ops::$TraitAssign<I> for Fixed<U, I> {
            #[inline(always)]
            fn $method_assign(&mut self, rhs: I) {
                ops::$TraitAssign::$method_assign(&mut self.count, rhs);
            }
        }
    };
}
fixed_scalar_op!(Mul mul MulAssign mul_assign);
fixed_scalar_op!(Div div DivAssign div_assign);

impl<U: AdditiveUnit, I: ops::Neg<Output = I>> ops::Neg for Fixed<U, I> {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Self {
        Self::new(-self.count)
    }
}

/// Dividing two values in the same unit gives a plain (truncated) integer.
impl<U: AdditiveUnit, I: ops::Div<Output = I>> ops::Div for Fixed<U, I> {
    type Output = I;
    #[inline(always)]
    fn div(self, rhs: Self) -> I {
        self.count / rhs.count
    }
}

macro_rules! fixed_scalar_lhs {
    ($($t:ty),*) => {$(
        impl<U: AdditiveUnit> ops::Mul<Fixed<U, $t>> for $t {
            type Output = Fixed<U, $t>;
            #[inline(always)]
            fn mul(self, rhs: Fixed<U, $t>) -> Fixed<U, $t> {
                Fixed::new(self * rhs.count)
            }
        }
    )*};
}
fixed_scalar_lhs!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

impl<U: AdditiveUnit, I: core::iter::Sum> core::iter::Sum for Fixed<U, I> {
    #[inline]
    fn sum<It: Iterator<Item = Self>>(iter: It) -> Self {
        Self::new(iter.map(|f| f.count).sum())
    }
}
impl<'a, U: AdditiveUnit, I: core::iter::Sum<&'a I> + 'a> core::iter::Sum<&'a Fixed<U, I>>
    for Fixed<U, I>
{
    #[inline]
    fn sum<It: Iterator<Item = &'a Self>>(iter: It) -> Self {
        Self::new(iter.map(|f| &f.count).sum())
    }
}

/// `1550 mm`; formatting flags apply to the number.
impl<U: FixedUnit, I: fmt::Display> fmt::Display for Fixed<U, I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.count, f)?;
        write!(f, " {}", U::ABBREVIATION)
    }
}

// ── Exact decimal display in another unit ─────────────────────────────────────

/// `Some(k)` when `x == 10^k`.
const fn pow10_exp(x: u128) -> Option<u32> {
    let (mut v, mut k) = (x, 0);
    while v > 1 {
        if v % 10 != 0 {
            return None;
        }
        v /= 10;
        k += 1;
    }
    if v == 1 {
        Some(k)
    } else {
        None
    }
}

/// `count_V = count_U · 10^UP / 10^DOWN` (one of the two is zero).
struct DecimalScale<U, V>(PhantomData<(U, V)>);

impl<U: FixedUnit, V: FixedUnit> DecimalScale<U, V> {
    const EXP: (u32, u32) = {
        assert!(
            Ratio::<U, V>::OFFSET.0 == 0,
            "these units have different zero points (°C and K): \
             convert with `to_unit_rounded` first"
        );
        match (pow10_exp(Ratio::<U, V>::NUM), pow10_exp(Ratio::<U, V>::DEN)) {
            (Some(up), Some(down)) => (up, down),
            _ => panic!(
                "the ratio between these units is not a power of ten (mm and in): \
                 use `to_unit_rounded`, or `to_f64().in_unit(...)` for an approximation"
            ),
        }
    };
}

impl<U: FixedUnit, I: Integer> Fixed<U, I> {
    /// Shows the count in another unit of the same quantity, as an exact
    /// decimal: 1234 mm as `1.234 m`, 21 375 m°C as `21.375 °C`, 2 km as
    /// `2000 m`. No float is involved.
    ///
    /// The ratio between the units must be a power of ten, which is checked
    /// when the code is compiled. All decimals are shown by default (so the
    /// resolution stays visible); a precision rounds to nearest, ties to
    /// even, and width, alignment and `+` apply to the whole text.
    ///
    /// ```rust
    /// use unitsnet_rs::*;
    /// use unitsnet_rs::unit::length::{Kilometer, Meter, Millimeter};
    /// use unitsnet_rs::unit::temperature::{DegreeCelsius, MillidegreeCelsius};
    ///
    /// let d = Fixed::<Millimeter>::new(1230);
    /// assert_eq!(d.display_in::<Meter>().to_string(), "1.230 m");
    /// assert_eq!(format!("{:.1}", d.display_in::<Meter>()), "1.2 m");
    /// assert_eq!(format!("{:>9}", d.display_in::<Meter>()), "  1.230 m");
    /// assert_eq!(Fixed::<Millimeter>::new(-5).display_in::<Meter>().to_string(), "-0.005 m");
    /// assert_eq!(Fixed::<Kilometer>::new(2).display_in::<Meter>().to_string(), "2000 m");
    ///
    /// let t = Fixed::<MillidegreeCelsius, i32>::new(21_375);
    /// assert_eq!(t.display_in::<DegreeCelsius>().to_string(), "21.375 °C");
    /// ```
    #[inline]
    pub fn display_in<V: FixedUnit<Unit = U::Unit>>(self) -> Decimal<V> {
        let (up, down) = DecimalScale::<U, V>::EXP;
        let (negative, magnitude) = self.count.sign_magnitude();
        Decimal {
            negative,
            magnitude,
            up,
            down,
            unit: PhantomData,
        }
    }
}

/// A [`Fixed`] count shown exactly in the unit `V`, as returned by
/// [`Fixed::display_in`].
#[derive(Clone, Copy)]
pub struct Decimal<V> {
    negative: bool,
    magnitude: u128,
    up: u32,
    down: u32,
    unit: PhantomData<V>,
}

impl<V: FixedUnit> fmt::Display for Decimal<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // value = magnitude · 10^up / 10^down, shown with `decimals` digits
        // after the point.
        let decimals = f.precision().unwrap_or(self.down as usize);
        let (digits, fraction_digits) = if decimals < self.down as usize {
            // Round to `decimals` digits: nearest, ties to even.
            let p = 10u128.pow(self.down - decimals as u32);
            let (q, r) = (self.magnitude / p, self.magnitude % p);
            let up = r > p - r || (r == p - r && q % 2 == 1);
            (q + u128::from(up), decimals as u32)
        } else {
            (self.magnitude, self.down)
        };
        let scale = 10u128.pow(fraction_digits);
        let (int, frac) = (digits / scale, digits % scale);
        // Trailing zeros of an up-scaled whole number (2 km → 2000 m), and
        // zeros beyond the count's resolution.
        let int_zeros = if int == 0 { 0 } else { self.up as usize };
        let pad_zeros = decimals - fraction_digits as usize;
        let sign = if self.negative && digits != 0 {
            "-"
        } else if f.sign_plus() {
            "+"
        } else {
            ""
        };
        let abbreviation = V::ABBREVIATION;

        let mut int_len = 1;
        let mut rest = int;
        while rest >= 10 {
            rest /= 10;
            int_len += 1;
        }
        let len = sign.len()
            + int_len
            + int_zeros
            + if decimals > 0 { 1 + decimals } else { 0 }
            + if abbreviation.is_empty() {
                0
            } else {
                1 + abbreviation.chars().count()
            };
        let padding = f.width().map_or(0, |w| w.saturating_sub(len));
        let (before, after) = match f.align() {
            Some(fmt::Alignment::Left) => (0, padding),
            Some(fmt::Alignment::Center) => (padding / 2, padding - padding / 2),
            _ => (padding, 0),
        };
        let fill = f.fill();
        for _ in 0..before {
            f.write_fmt(format_args!("{fill}"))?;
        }
        write!(f, "{sign}{int}")?;
        for _ in 0..int_zeros {
            f.write_str("0")?;
        }
        if decimals > 0 {
            f.write_str(".")?;
            if fraction_digits > 0 {
                write!(f, "{frac:0width$}", width = fraction_digits as usize)?;
            }
            for _ in 0..pad_zeros {
                f.write_str("0")?;
            }
        }
        if !abbreviation.is_empty() {
            write!(f, " {abbreviation}")?;
        }
        for _ in 0..after {
            f.write_fmt(format_args!("{fill}"))?;
        }
        Ok(())
    }
}

impl<V: FixedUnit> fmt::Debug for Decimal<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl<U: FixedUnit, I: fmt::Debug> fmt::Debug for Fixed<U, I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.count, f)?;
        write!(f, " {}", U::ABBREVIATION)
    }
}

/// Parses an integer followed by one of `U`'s abbreviations, e.g. `"1550 mm"`.
impl<U: FixedUnit, I: core::str::FromStr> core::str::FromStr for Fixed<U, I> {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, ParseError> {
        let (number, unit) = crate::split_value_unit(s);
        let count = number.parse().map_err(|_| ParseError::InvalidNumber)?;
        if U::ABBREVIATIONS.contains(&unit) {
            Ok(Self::new(count))
        } else {
            Err(ParseError::UnknownUnit)
        }
    }
}

// SAFETY: `#[repr(transparent)]` over `I` (`PhantomData` is a ZST).
#[cfg(feature = "bytemuck")]
unsafe impl<U, I: bytemuck::Zeroable> bytemuck::Zeroable for Fixed<U, I> {}
// SAFETY: as above; `Pod` requires `I: Pod`, and `U: 'static` for `Self: 'static`.
#[cfg(feature = "bytemuck")]
unsafe impl<U: 'static, I: bytemuck::Pod> bytemuck::Pod for Fixed<U, I> {}
// SAFETY: as above.
#[cfg(feature = "bytemuck")]
unsafe impl<U, I> bytemuck::TransparentWrapper<I> for Fixed<U, I> {}

#[cfg(feature = "num-traits")]
impl<U: AdditiveUnit, I: Integer + num_traits::Zero> num_traits::Zero for Fixed<U, I> {
    #[inline]
    fn zero() -> Self {
        Self::new(I::zero())
    }
    #[inline]
    fn is_zero(&self) -> bool {
        self.count.is_zero()
    }
}

#[cfg(feature = "defmt")]
impl<U: FixedUnit, I: defmt::Format> defmt::Format for Fixed<U, I> {
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(f, "{} {=str}", self.count, U::ABBREVIATION);
    }
}

#[cfg(feature = "serde")]
impl<U, I: serde::Serialize> serde::Serialize for Fixed<U, I> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.count.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, U, I: serde::Deserialize<'de>> serde::Deserialize<'de> for Fixed<U, I> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        I::deserialize(deserializer).map(Self::new)
    }
}

// ── Total order for float quantities ─────────────────────────────────────────

/// A float quantity with a total order, equality and hash, so it can be a
/// key in `HashMap`, `HashSet`, `BTreeMap` or `BTreeSet`, or sorted with
/// `sort()`.
///
/// Values are compared by [`f64::total_cmp`] after normalising `-0.0` to
/// `0.0` and every NaN to a single NaN, so equality agrees with `==` on
/// numbers (`0.0 == -0.0`) and NaN is a valid key, greater than everything.
///
/// For keys that should merge nearly equal measurements, quantise instead:
/// [`Fixed::round_from`] to a whole number of a suitable unit.
///
/// ```rust
/// use std::collections::BTreeMap;
/// use unitsnet_rs::*;
///
/// let mut by_length = BTreeMap::new();
/// by_length.insert(Ordered(Length::from_meters(2.0)), "b");
/// by_length.insert(Ordered(Length::from_centimeters(50.0)), "a");
/// let names: Vec<_> = by_length.values().copied().collect();
/// assert_eq!(names, ["a", "b"]);
/// ```
#[derive(Debug, Clone, Copy, Default)]
#[repr(transparent)]
pub struct Ordered<Q>(pub Q);

impl<Q: Quantity> Ordered<Q>
where
    Q::Value: Float,
{
    #[inline(always)]
    fn canonical(&self) -> Q::Value {
        let v = self.0.as_base();
        let zero = <Q::Value as Float>::from_f64(0.0);
        #[allow(clippy::eq_op)]
        if v != v {
            <Q::Value as Float>::from_f64(f64::NAN)
        } else if v == zero {
            zero
        } else {
            v
        }
    }
}

impl<Q: Quantity> PartialEq for Ordered<Q>
where
    Q::Value: Float,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl<Q: Quantity> Eq for Ordered<Q> where Q::Value: Float {}
impl<Q: Quantity> PartialOrd for Ordered<Q>
where
    Q::Value: Float,
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<Q: Quantity> Ord for Ordered<Q>
where
    Q::Value: Float,
{
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.canonical().total_cmp(&other.canonical())
    }
}
impl<Q: Quantity> Hash for Ordered<Q>
where
    Q::Value: Float,
{
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.canonical().to_f64().to_bits().hash(state);
    }
}
impl<Q: fmt::Display> fmt::Display for Ordered<Q> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl<Q> From<Q> for Ordered<Q> {
    #[inline(always)]
    fn from(q: Q) -> Self {
        Ordered(q)
    }
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
    fn mul_wide_matches_u128_where_it_fits_and_is_exact_beyond() {
        for (x, y) in [
            (0, 5),
            (7, 9),
            (u64::MAX as u128, u64::MAX as u128),
            (1 << 100, 3),
        ] {
            assert_eq!(mul_wide(x, y), (0, x * y));
        }
        assert_eq!(mul_wide(u128::MAX, 2), (1, u128::MAX - 1));
        assert_eq!(mul_wide(u128::MAX, u128::MAX), (u128::MAX - 1, 1));
        assert_eq!(mul_wide(1 << 127, 1 << 127), (1 << 126, 0));
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
