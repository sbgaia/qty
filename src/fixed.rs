//! Integer quantities in a unit of your choice.

use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::ops;

use crate::rounding::{div_round, narrow, round_f64};
use crate::{Float, Integer, ParseError, Quantity, RoundError, Rounding, Unit};

#[inline(always)]
fn widen<I: Integer>(v: I) -> Result<i128, RoundError> {
    v.to_i128().ok_or(RoundError::Overflow)
}

/// A unit that [`Fixed`] integer quantities can be counted in.
///
/// Implemented by the zero-sized markers in [`crate::unit`].
pub trait FixedUnit: Copy + 'static {
    /// The quantity with float storage `F`, such as `Length<F>`.
    type Quantity<F: Float>: Quantity<Value = F, Unit = Self::Unit>
        + PartialEq
        + PartialOrd
        + fmt::Debug;

    /// The quantity's unit enum, such as [`LengthUnit`](crate::LengthUnit),
    /// shared by all its units.
    type Unit: crate::Unit;

    /// The marker of the quantity's base unit.
    type Base: FixedUnit<Unit = Self::Unit>;

    /// This unit as an enum value.
    const UNIT: Self::Unit;

    /// One of this unit is exactly `NUM / DEN × π^PI_POW` base units, with
    /// the fraction reduced.
    const NUM: u128;
    /// See [`NUM`](Self::NUM).
    const DEN: u128;
    /// See [`NUM`](Self::NUM).
    const PI_POW: i32;

    /// For affine units such as °C: zero of this unit is exactly
    /// `OFFSET_NUM / OFFSET_DEN` base units. Zero for linear units.
    const OFFSET_NUM: i128 = 0;
    /// See [`OFFSET_NUM`](Self::OFFSET_NUM).
    const OFFSET_DEN: u128 = 1;

    /// A float count of this unit as a quantity, like `Length::from_millimeters`.
    // Provided rather than generated: 1,700 markers × two one-line bodies
    // were a few percent of the crate's build time. With `UNIT` constant,
    // `new` folds to the same single operation as `from_millimeters`.
    #[inline(always)]
    fn to_quantity<F: Float>(count: F) -> Self::Quantity<F> {
        Quantity::new(count, Self::UNIT)
    }

    /// A quantity as a float count of this unit, like `Length::as_millimeters`.
    #[inline(always)]
    fn from_quantity<F: Float>(quantity: Self::Quantity<F>) -> F {
        quantity.convert_to(Self::UNIT)
    }
}

/// A [`FixedUnit`] whose counts can be added, subtracted, negated and
/// scaled. That's every unit except those of point quantities: 20 °C + 20 °C
/// is not 40 °C.
pub trait AdditiveUnit: FixedUnit {}

/// A [`FixedUnit`] of a point quantity, such as an absolute temperature or a
/// decibel level. Counts can't be added, but their difference is a count of
/// [`Delta`](Self::Delta), which can be added back.
///
/// ```rust
/// use qty::*;
/// use qty::unit::temperature::{DegreeCelsius, Kelvin, MillidegreeCelsius};
/// use qty::unit::temperature_delta::MillidegreeCelsius as DeltaMilli;
///
/// let morning = Fixed::<MillidegreeCelsius>::new(12_500);
/// let noon = Fixed::<MillidegreeCelsius>::new(20_250);
/// let rise: Fixed<DeltaMilli> = noon - morning;
/// assert_eq!(rise.count(), 7_750);
/// assert_eq!(morning + rise, noon);
///
/// // °C and K convert to m°C without rounding.
/// assert_eq!(Fixed::<DegreeCelsius>::new(20).to_unit::<MillidegreeCelsius>()?.count(), 20_000);
/// assert_eq!(Fixed::<Kelvin>::new(300).to_unit::<MillidegreeCelsius>()?.count(), 26_850);
/// // The other way they don't: 0 °C is 273.15 K.
/// assert_eq!(
///     Fixed::<MillidegreeCelsius>::new(0).to_unit_rounded::<Kelvin>(Rounding::NearestTiesEven)?.count(),
///     273
/// );
/// // Comparisons across scales are exact.
/// assert!(Fixed::<DegreeCelsius>::new(20) > Fixed::<Kelvin>::new(293));
/// # Ok::<(), RoundError>(())
/// ```
pub trait PointUnit: FixedUnit {
    /// The unit of a difference between two points, on the same scale: Δ°C
    /// for °C, Δm°C for m°C, dB for dBµV.
    type Delta: AdditiveUnit;
}

/// The relation `Self × Rhs = Output` between three quantities' unit enums,
/// such as `LengthUnit × LengthUnit = AreaUnit`, with the exact factor between
/// their base units: `output = K × lhs × rhs`, where `K = num / den × π^pi`.
///
/// Generated for every product of quantities, for [`Fixed::mul_to`].
pub trait UnitMul<Rhs> {
    /// The unit enum of the product's quantity.
    type Output;
    /// `(num, den, pi)`, as described on the trait.
    const K: (u128, u128, i32);
}

/// The relation `Self ÷ Rhs = Output` between three quantities' unit enums,
/// such as `LengthUnit ÷ DurationUnit = SpeedUnit`, with the exact factor
/// between their base units: `output = K × lhs / rhs`.
///
/// Generated for every quotient of quantities, for [`Fixed::div_to`].
pub trait UnitDiv<Rhs> {
    /// The unit enum of the quotient's quantity.
    type Output;
    /// `(num, den, pi)`, as for [`UnitMul`].
    const K: (u128, u128, i32);
}

/// `a/b × c/d`, reduced.
const fn frac_mul((a, b): (u128, u128), (c, d): (u128, u128)) -> (u128, u128) {
    let (g1, g2) = (gcd(a, d), gcd(c, b));
    match ((a / g1).checked_mul(c / g2), (b / g2).checked_mul(d / g1)) {
        (Some(num), Some(den)) => (num, den),
        _ => panic!("ratio between these units is too large for integer arithmetic"),
    }
}

/// The exact factor `R` in `count_V = R × count_U × count_W`, or
/// `÷ count_W` when `DIV`.
struct RelationRatio<U, W, V, const DIV: bool>(PhantomData<(U, W, V)>);

impl<U: FixedUnit, W: FixedUnit, V: FixedUnit, const DIV: bool> RelationRatio<U, W, V, DIV> {
    const fn parts(k: (u128, u128, i32)) -> (u128, u128) {
        // U × W^±1 × K / V, in base units.
        let w_pi = if DIV { -W::PI_POW } else { W::PI_POW };
        assert!(
            U::PI_POW + w_pi + k.2 == V::PI_POW,
            "ratio between these units involves π: use float quantities instead"
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
        "product is not always a whole number of the target unit (like \
         mm × mm in m²): use `mul_to_rounded` or a finer unit"
    );
}

impl<U: FixedUnit, W: FixedUnit, V: FixedUnit> RelationRatio<U, W, V, true>
where
    U::Unit: UnitDiv<W::Unit>,
{
    const PARTS: (u128, u128) = Self::parts(<U::Unit as UnitDiv<W::Unit>>::K);
}

/// `count × k`, overflowing like integer `*`.
#[inline(always)]
fn scale_count<I: Integer>(count: I, k: u128) -> I {
    if k == 1 {
        return count;
    }
    match i128::try_from(k).ok().and_then(I::from_i128) {
        Some(k) => count * k,
        // The factor alone doesn't fit in `I`, so only zero survives.
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
            "ratio between these units involves π: convert through a float quantity instead"
        );
        let g1 = gcd(From::NUM, To::NUM);
        let g2 = gcd(From::DEN, To::DEN);
        match (
            (From::NUM / g1).checked_mul(To::DEN / g2),
            (From::DEN / g2).checked_mul(To::NUM / g1),
        ) {
            (Some(num), Some(den)) => (num, den),
            _ => panic!("ratio between these units is too large for integer conversion"),
        }
    };
    const NUM: u128 = Self::PARTS.0;
    const DEN: u128 = Self::PARTS.1;
    /// The offset added after scaling, in units of `To`:
    /// `(b_From - b_To) / a_To`. Zero unless a unit is affine.
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
        "unit conversion is not exact: for `+` and `-` put the finer unit on \
         the left (`mm + m`), otherwise use `to_unit_rounded`"
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

/// `count × num / den + offset`, rounded once.
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

/// An integer number of a fixed unit `U`: `Fixed<Millimeter>` is a whole
/// number of millimetres, stored as an `i64`.
///
/// The unit is part of the type. Units can be mixed where that's lossless:
/// millimetres + metres gives millimetres, while metres + millimetres doesn't
/// compile because the result would need rounding. Swap the operands, or use
/// [`to_unit_rounded`](Self::to_unit_rounded). Comparisons work exactly
/// between any two units of a quantity.
///
/// Arithmetic in one unit is plain integer arithmetic, with the same machine
/// code as the bare integer, and values are `Eq + Ord + Hash`, so they make
/// good map keys. Overflow behaves like Rust's integers: a panic in debug
/// builds, wrapping in release. There are `checked_*` and `saturating_*`
/// variants.
///
/// ```rust
/// use qty::*;
/// use qty::unit::length::{Meter, Millimeter};
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
/// // Mixed units give the left operand's unit.
/// assert_eq!(a + Meters::new(2), Millimeters::new(3_250));
/// assert!(Meters::new(1) < a);
/// assert_eq!(Meters::new(2), Millimeters::new(2_000));
///
/// // To floats and back.
/// assert_eq!(a.to_f64(), Length::from_meters(1.25));
/// let c = Fixed::<Millimeter>::round_from(Length::from_meters(0.0125), Rounding::NearestTiesEven);
/// assert_eq!(c, Ok(Fixed::new(12))); // 12.5 mm, tie to even
///
/// // Converting down is exact; converting up needs a rounding mode.
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
    /// A count of `U`.
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
    /// The number of `U`.
    #[inline(always)]
    pub const fn count(self) -> I {
        self.count
    }
}

impl<U: FixedUnit, I: Integer> Fixed<U, I> {
    /// The quantity with `f64` storage: 1250 mm becomes 1.25 m.
    #[inline]
    pub fn to_f64(self) -> U::Quantity<f64> {
        U::to_quantity(self.count.to_f64())
    }

    /// The quantity with `f32` storage.
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

    /// Rounds a float count of `U`, such as a sensor reading in millimetres,
    /// to a whole number. Unlike [`round_from`](Self::round_from) there is
    /// no unit conversion, so ties like 2.5 stay exact.
    #[inline]
    pub fn round_from_count<F: Float>(count: F, mode: Rounding) -> Result<Self, RoundError> {
        round_f64(count.to_f64(), mode)
            .and_then(narrow)
            .map(Self::new)
    }

    /// Converts exactly to a unit that divides this one, such as metres to
    /// millimetres.
    ///
    /// Fails only on overflow. Other conversions, like millimetres to
    /// metres, don't compile; use [`to_unit_rounded`](Self::to_unit_rounded)
    /// for those.
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

    /// The exact product with a count of another quantity, as a count of
    /// `unit`: mm × mm in mm², N × m in N·m, km/h × h in km.
    ///
    /// The result must be a whole number of `unit` for any input, which is
    /// checked at compile time, so mm × mm in m² doesn't compile. Use
    /// [`mul_to_rounded`](Self::mul_to_rounded) for those. Overflow behaves
    /// like integer `*`.
    ///
    /// ```rust
    /// use qty::*;
    /// use qty::unit::{area::SquareMillimeter, length::{Meter, Millimeter}};
    /// use qty::unit::{duration::Hour, speed::KilometerPerHour, length::Kilometer};
    ///
    /// let width = Fixed::<Millimeter>::new(1_200);
    /// let height = Fixed::<Millimeter>::new(800);
    /// assert_eq!(width.mul_to(height, SquareMillimeter).count(), 960_000);
    ///
    /// // Mixed units work when the result is exact: 1 m × 1 mm is 1000 mm².
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

    /// Like [`mul_to`](Self::mul_to), for any target unit, rounding with
    /// `mode`.
    ///
    /// ```rust
    /// use qty::*;
    /// use qty::unit::{area::SquareMeter, length::Millimeter};
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

    /// The quotient by a count of another quantity, as a count of `unit`,
    /// rounding with `mode`: mm ÷ ms in mm/s, J ÷ s in W.
    ///
    /// ```rust
    /// use qty::*;
    /// use qty::unit::{duration::Millisecond, length::Millimeter, speed::MillimeterPerSecond};
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

    /// The absolute value, or `None` if it doesn't fit (`i32::MIN`).
    #[inline]
    pub fn checked_abs(self) -> Option<Self> {
        let (_, magnitude) = self.count.sign_magnitude();
        i128::try_from(magnitude)
            .ok()
            .and_then(I::from_i128)
            .map(Self::new)
    }

    /// The absolute value.
    ///
    /// # Panics
    ///
    /// If it doesn't fit: `Fixed::<_, i32>::new(i32::MIN).abs()`.
    #[inline]
    #[track_caller]
    pub fn abs(self) -> Self {
        self.checked_abs().expect("Fixed::abs overflowed")
    }

    /// `-self`, or `None` if it doesn't fit: `i32::MIN`, or any unsigned
    /// count but zero.
    #[inline]
    pub fn checked_neg(self) -> Option<Self> {
        self.count
            .to_i128()
            .and_then(i128::checked_neg)
            .and_then(I::from_i128)
            .map(Self::new)
    }

    /// `-1`, `0` or `1`, by the sign of the count.
    #[inline]
    pub fn signum(self) -> I {
        let (negative, magnitude) = self.count.sign_magnitude();
        let sign = match (negative, magnitude) {
            (_, 0) => 0,
            (true, _) => -1,
            (false, _) => 1,
        };
        // -1 occurs only for signed types, where it fits.
        I::from_i128(sign).unwrap_or_default()
    }
}

/// Subtracting a difference from a point. This can't be the `-` operator,
/// which already means `point - point` here. `point + (-delta)` works too.
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

// Written by hand: derives would require the marker `U` to implement the
// trait too.

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
/// Compares across units of the same quantity: 2 m > 1500 mm.
///
/// The comparison uses exact integer arithmetic, so it doesn't round or
/// overflow. Only affine units such as °C fall back to floats, near the
/// limits of `i128`. Within one unit it's a plain integer comparison.
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

/// Adding or subtracting another unit of the same quantity. The result is in
/// the left operand's unit, so the right operand must convert into it
/// exactly: millimetres + metres works, metres + millimetres doesn't compile.
/// Swap the operands, or convert first with [`Fixed::to_unit_rounded`].
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
    /// The count in `U`, which this unit must convert into exactly (checked
    /// at compile time). Overflows like integer `*`.
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
                // The factor alone doesn't fit in `I`, so only zero converts.
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

/// Compares `a` in `U` with `b` in `V` exactly, as `a × num` against
/// `b × den`.
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
    // Fast path: both products fit in i128, as all realistic values do.
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
    // Otherwise compare signs and 256-bit magnitudes.
    let (neg_a, mag_a) = a.sign_magnitude();
    let (neg_b, mag_b) = b.sign_magnitude();
    let (pa, pb) = (mul_wide(mag_a, num), mul_wide(mag_b, den));
    // A zero product counts as positive, whatever the sign of the count.
    let neg_a = neg_a && pa != (0, 0);
    let neg_b = neg_b && pb != (0, 0);
    match (neg_a, neg_b) {
        (false, false) => pa.cmp(&pb),
        (true, true) => pb.cmp(&pa),
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
    }
}

/// Compares `a × num/den + off_num/off_den` with `b` by cross-multiplying.
/// Exact in `i128`, which holds every realistic temperature or level; past
/// that it falls back to `f64`.
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

/// Dividing two values in the same unit gives a plain integer, truncated.
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

/// Formats as `1550 mm`, with formatting flags applied to the number.
impl<U: FixedUnit, I: fmt::Display> fmt::Display for Fixed<U, I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.count, f)?;
        write!(f, " {}", U::UNIT.abbreviation())
    }
}

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

/// `count_V = count_U × 10^UP / 10^DOWN`, where one exponent is zero.
struct DecimalScale<U, V>(PhantomData<(U, V)>);

impl<U: FixedUnit, V: FixedUnit> DecimalScale<U, V> {
    const EXP: (u32, u32) = {
        assert!(
            Ratio::<U, V>::OFFSET.0 == 0,
            "units have different zero points (like °C and K): \
             convert with `to_unit_rounded` first"
        );
        match (pow10_exp(Ratio::<U, V>::NUM), pow10_exp(Ratio::<U, V>::DEN)) {
            (Some(up), Some(down)) => (up, down),
            _ => panic!(
                "ratio between these units is not a power of ten (like mm and in): \
                 use `to_unit_rounded`, or `to_f64().in_unit(...)` for an approximation"
            ),
        }
    };
}

impl<U: FixedUnit, I: Integer> Fixed<U, I> {
    /// The mean of `values`, rounded once with `mode`. The sum is taken in
    /// `i128`, so it's exact. Works for points such as temperatures too.
    ///
    /// # Errors
    ///
    /// [`RoundError::DivisionByZero`] if there are no values, and
    /// [`RoundError::Overflow`] if the sum doesn't fit in `i128` or the mean
    /// doesn't fit in `I`.
    ///
    /// ```rust
    /// use qty::*;
    /// use qty::unit::temperature::MillidegreeCelsius;
    ///
    /// let readings = [21_375, 21_500, 22_125].map(Fixed::<MillidegreeCelsius, i32>::new);
    /// let mean = Fixed::mean(readings, Rounding::NearestTiesEven)?;
    /// assert_eq!(mean.count(), 21_667); // 21 666.67, rounded
    /// # Ok::<(), RoundError>(())
    /// ```
    pub fn mean<It: IntoIterator<Item = Self>>(
        values: It,
        mode: Rounding,
    ) -> Result<Self, RoundError> {
        let (mut sum, mut n) = (0_i128, 0_i128);
        for v in values {
            sum = sum
                .checked_add(widen(v.count)?)
                .ok_or(RoundError::Overflow)?;
            n += 1;
        }
        if n == 0 {
            return Err(RoundError::DivisionByZero);
        }
        div_round(sum, n, mode).and_then(narrow).map(Self::new)
    }
}

impl<U: FixedUnit, I: Integer> Fixed<U, I> {
    /// Shows the count in another unit of the same quantity as an exact
    /// decimal, without floats: 1234 mm as `1.234 m`, 21375 m°C as
    /// `21.375 °C`, 2 km as `2000 m`.
    ///
    /// The ratio between the units must be a power of ten, which is checked
    /// at compile time. By default all decimals are shown, so the resolution
    /// stays visible. A precision rounds to nearest, ties to even; width,
    /// alignment and `+` apply to the whole text.
    ///
    /// ```rust
    /// use qty::*;
    /// use qty::unit::length::{Kilometer, Meter, Millimeter};
    /// use qty::unit::temperature::{DegreeCelsius, MillidegreeCelsius};
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
        // value = magnitude × 10^up / 10^down, shown with `decimals` digits
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
        // Zeros after a scaled-up whole number (2 km is 2000 m), and zeros
        // past the count's resolution.
        let int_zeros = if int == 0 { 0 } else { self.up as usize };
        let pad_zeros = decimals - fraction_digits as usize;
        let sign = if self.negative && digits != 0 {
            "-"
        } else if f.sign_plus() {
            "+"
        } else {
            ""
        };
        let abbreviation = V::UNIT.abbreviation();

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
        write!(f, " {}", U::UNIT.abbreviation())
    }
}

/// Parses an integer followed by one of `U`'s abbreviations, such as
/// `"1550 mm"`.
impl<U: FixedUnit, I: core::str::FromStr> core::str::FromStr for Fixed<U, I> {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, ParseError> {
        let (number, unit) = crate::split_value_unit(s);
        let count = number.parse().map_err(|_| ParseError::InvalidNumber)?;
        if U::UNIT.info().abbreviations.contains(&unit) {
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

#[cfg(feature = "defmt")]
impl<U: FixedUnit, I: defmt::Format> defmt::Format for Fixed<U, I> {
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(f, "{} {=str}", self.count, U::UNIT.abbreviation());
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
