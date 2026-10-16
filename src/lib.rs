//! # qty
//!
//! Strongly-typed physical quantities and unit conversions, generated from
//! [UnitsNet](https://github.com/angularsen/UnitsNet) definition files, with
//! **zero runtime overhead** over the bare numbers they wrap.
//!
//! ## Quick start
//!
//! ```rust
//! use qty::*;
//!
//! let dist = Length::from_kilometers(10.0);
//! let time = Duration::from_hours(0.5);
//! let speed: Speed = dist / time;
//!
//! // Exact comparison operators come from the derived PartialEq/PartialOrd.
//! // For floating-point results, prefer the approximate comparisons from the
//! // ApproxEq trait, which every float-valued quantity implements.
//! assert!(speed.approx_eq_rel(Speed::from_kilometers_per_hour(20.0), 1e-12));
//!
//! let temp = Temperature::from_degrees_celsius(100.0);
//! assert!(temp.approx_eq_rel(Temperature::from_degrees_fahrenheit(212.0), 1e-12));
//! ```
//!
//! ## Design
//!
//! Every quantity is a `#[repr(transparent)]` newtype over its value in the
//! quantity's base unit (UnitsNet's choice — usually the coherent SI unit,
//! see [`Quantity::BASE_UNIT`]), generic over the numeric storage type:
//!
//! ```rust
//! # use qty::*;
//! assert_eq!(core::mem::size_of::<Length>(), 8);        // Length<f64>
//! assert_eq!(core::mem::size_of::<Length<f32>>(), 4);
//! assert_eq!(core::mem::size_of::<Length<i32>>(), 4);
//! ```
//!
//! * **Arithmetic is free.** `a + b`, `a * 2.0`, `Length / Duration` and every
//!   other operator compiles to the single machine instruction that the bare
//!   numbers would use — the units exist only in the type system.
//! * **Conversions cost one operation.** At code-generation time each
//!   UnitsNet formula (e.g. `{x} * 2.54e-2 / 72.27`) is reduced with exact
//!   rational arithmetic to a single factor, rounded once to the nearest
//!   `f64`. Linear units convert with one multiplication or division,
//!   affine units (°C, °F) with two operations. Whenever the factor or its
//!   inverse is exactly representable (km ↔ m, h ↔ s, mm ↔ m, …) the
//!   conversion is correctly rounded.
//!   Every conversion stays within about one ulp of the exact result, and
//!   both directions share one rounded constant, so `from_x(v).as_x()`
//!   returns `v` exactly in the large majority of cases.
//! * **Any numeric type.** Storage type `T` defaults to `f64`; `f32` works
//!   everywhere, as does any type implementing [`Float`]. Integers (and any
//!   other `T`) get arithmetic, comparison, hashing and the cross-quantity
//!   operators whose base units are coherent, all working in base units.
//! * **Points vs. amounts.** A [`Temperature`] is a point on a scale: two of
//!   them cannot be added (20 °C + 20 °C is not 40 °C). Their difference is a
//!   [`TemperatureDelta`], which adds, scales and sums like any amount, and
//!   moves a temperature: `t + delta`.
//! * **Bulk data without copies.** Because of `#[repr(transparent)]`, a
//!   `&[f64]` of metres can be viewed as `&[Length]` and back for free, so
//!   typed code can feed SIMD kernels or I/O buffers directly.
//! * **`no_std`, no required dependencies, no allocation** (see the features below).
//!
//! ### Different numeric types
//!
//! ```rust
//! use qty::*;
//!
//! // f32 storage: half the memory, ~7 significant digits.
//! let l: Length<f32> = Length::from_feet(3.0);
//! assert!(l.approx_eq_rel(Length::from_inches(36.0), 1e-6));
//!
//! // Integer storage: exact base-unit arithmetic, plus Eq/Ord/Hash.
//! let a = Mass::<i64>::from_base(1_500); // kilograms
//! let b = Mass::from_base(500_i64);
//! assert_eq!(a + b, Mass::from_base(2_000));
//! assert!(a > b);
//! let force: Force<i64> = a * Acceleration::from_base(2); // kg · m/s² = N
//! assert_eq!(force.as_base(), 3_000);
//!
//! // To float storage (like `as f64`), without unwrapping.
//! let wide: Mass<f64> = a.to_f64();
//! assert_eq!(wide.as_grams(), 1_500_000.0);
//! ```
//!
//! [`Vector2`] and [`Vector3`] hold quantities: a `Vector3<Length>` divided
//! by a `Duration` is a `Vector3<Speed>`, and dot and cross products follow
//! the relations between quantities.
//!
//! `Uncertain` (value ± standard uncertainty; with `std` or `libm`) is a storage type too: a
//! `Length<Uncertain>` propagates its uncertainty through conversions,
//! relations and math functions.
//!
//! ### Integers in a unit of your choice, with explicit rounding
//!
//! [`Fixed<U, I>`](Fixed) is a whole number of the unit `U` — for example
//! `Fixed<Millimeter>` counts millimetres in an `i64`. The unit is part of
//! the type, so millimetres and metres cannot be mixed by accident. Values
//! are exact, `Eq + Ord + Hash`, and arithmetic is plain integer arithmetic.
//!
//! Rounding happens only where information is lost — float → integer, a
//! fine unit → a coarser one, integer division — and there the [`Rounding`]
//! mode is always explicit. Lossless operations take no mode: `mm + m`
//! (the result is in the left operand's unit) and `m.to_unit::<Millimeter>()`
//! just work, and comparisons work between any two units. The lossy forms
//! (`m + mm`, `mm.to_unit::<Meter>()`) are rejected when the code is
//! compiled (by `cargo build`/`cargo test`; `cargo check` does not evaluate
//! the unit ratio).
//!
//! ```rust
//! use std::collections::HashMap;
//! use qty::*;
//! use qty::unit::length::{Meter, Millimeter};
//!
//! // Type aliases keep literals short.
//! type Millimeters = Fixed<Millimeter>;
//! type Meters = Fixed<Meter>;
//!
//! // Quantise a float measurement: 12.5 mm, ties to even → 12 mm.
//! let a = Millimeters::round_from(Length::from_meters(0.0125), Rounding::NearestTiesEven)?;
//! assert_eq!(a, Millimeters::new(12));
//!
//! // Exact integer arithmetic; integer keys.
//! let mut stock: HashMap<Millimeters, u32> = HashMap::new();
//! *stock.entry(a + Millimeters::new(588)).or_default() += 1;
//! assert_eq!(stock[&Millimeters::new(600)], 1);
//!
//! // Mixing units where it is lossless: the result is in the left unit.
//! assert_eq!(a + Meters::new(1), Millimeters::new(1_012));
//! assert!(Meters::new(1) > a); // comparisons work across any units, exactly
//!
//! // Converting: lossless needs no rounding mode; lossy needs one.
//! assert_eq!(Meters::new(2).to_unit::<Millimeter>()?, Millimeters::new(2_000));
//! assert_eq!(Millimeters::new(1_500).to_unit_rounded::<Meter>(Rounding::Ceil)?, Meters::new(2));
//! assert_eq!(
//!     Fixed::<Millimeter>::new(1_500).to_unit_rounded::<Meter>(Rounding::Exact),
//!     Err(RoundError::Inexact)
//! );
//!
//! // Back to floats for everything else.
//! let l: Length = a.to_f64();
//! assert_eq!(l.as_millimeters(), 12.0);
//! # Ok::<(), RoundError>(())
//! ```
//!
//! The relations between quantities work on counts too, exactly:
//! `mm.mul_to(mm, SquareMillimeter)` is a `Fixed<SquareMillimeter>`, and
//! [`div_to`](Fixed::div_to) divides with an explicit rounding mode (see
//! [`mul_to`](Fixed::mul_to)).
//!
//! Units are in [`unit`](mod@unit), one module per quantity (`unit::duration::Nanosecond`,
//! `unit::mass::Gram`, …). Every linear unit has a marker, and so do the
//! affine units of point quantities: a temperature sensor reporting
//! millidegrees Celsius is a `Fixed<unit::temperature::MillidegreeCelsius, i32>`.
//! Point markers ([`PointUnit`]) convert and compare exactly across scales
//! (20 °C = 20 000 m°C; 300 K = 26 850 m°C), and their differences are
//! counts of the matching delta unit — see [`PointUnit`]. `Fixed<BaseUnit, I>` and `Length<I>` (integers in the base
//! unit, which also get the cross-quantity operators) convert into each other
//! with `From`.
//!
//! ### Comparing, sorting and map keys
//!
//! Quantities of the same kind compare with `<`, `==`, `max`, … regardless
//! of the unit they were created in, since all are stored in the base unit.
//! Floats are not totally ordered (NaN), so for sorting use
//! [`total_cmp`](Length::total_cmp), and for `HashMap`/`BTreeMap` keys wrap
//! them in [`Ordered`]:
//!
//! ```rust
//! use std::collections::BTreeMap;
//! use qty::*;
//!
//! let mut v = vec![Length::from_meters(2.0), Length::from_inches(1.0)];
//! v.sort_by(Length::total_cmp);
//! assert_eq!(v[0], Length::from_inches(1.0));
//!
//! let mut names = BTreeMap::new();
//! names.insert(Ordered(Length::from_kilometers(1.0)), "km");
//! names.insert(Ordered(Length::from_meters(1.0)), "m");
//! assert_eq!(names.values().copied().collect::<Vec<_>>(), ["m", "km"]);
//! ```
//!
//! ### Zero-copy slices and `const` values
//!
//! ```rust
//! use qty::*;
//!
//! // `const_new` works in any unit, with the same result as `new`.
//! const MARATHON: Length = Length::const_new(42.195, LengthUnit::Kilometer);
//! const BOILING: Temperature = Temperature::const_new(100.0, TemperatureUnit::DegreeCelsius);
//! assert_eq!(MARATHON, Length::from_meters(42_195.0));
//! assert_eq!(BOILING, Temperature::from_degrees_celsius(100.0));
//!
//! let raw = [1.0, 2.0, 3.0];
//! let typed: &[Length] = Length::from_base_slice(&raw);
//! let total: Length = typed.iter().sum();
//! assert_eq!(total, Length::from_meters(6.0));
//! assert!(MARATHON > total);
//! ```
//!
//! With `f32` storage, write `quantity * 2.0` rather than `2.0 * quantity`:
//! the reversed form is implemented for `f64` and the integer types only, so
//! that `2.0 * Length::from_meters(1.0)` never needs a type annotation.
//!
//! ## Cargo features
//!
//! * `full` (default) — every quantity. The core quantities (length, area,
//!   volume, mass, duration, speed, acceleration, force, energy, power,
//!   pressure, temperature, angle, frequency, density, ratio) are always
//!   compiled; the others come in groups — `mechanics`, `fluids`,
//!   `electrical`, `thermal`, `chemistry`, `light`, `radiation`,
//!   `information`, `misc` — and a relation between quantities exists when
//!   all of their groups are enabled. Compiling the core alone takes about a
//!   fifth of the time of `full`:
//!
//!   ```toml
//!   qty = { version = "…", default-features = false, features = ["std", "electrical"] }
//!   ```
//! * `std` (default) — math functions: `Area::sqrt() → Length`,
//!   `Volume::cbrt() → Length`, `hypot`, and trigonometry on [`Angle`]
//!   (`sin`, `cos`, `Angle::atan2(y, x)`, …). The rest of the crate is
//!   `no_std`: disable default features for embedded targets.
//! * `libm` — the same math functions without `std`, through the `libm` crate.
//! * `serde` — `Serialize`/`Deserialize` for every quantity (as its bare
//!   base-unit number) and every unit enum; the `serde_str` module reads and writes
//!   text with units instead (`"30 s"`, `"2 minutes"`).
//! * `approx` — `AbsDiffEq`/`RelativeEq`/`UlpsEq`, so `assert_relative_eq!`
//!   works on quantities.
//! * `bytemuck` — `Pod`, `Zeroable` and `TransparentWrapper`, for zero-copy
//!   casts between quantity buffers and bytes (GPU uploads, file I/O).
//! * `defmt` — `defmt::Format`, for logging on embedded targets.
//! * `chrono`, `time` — conversions between [`Duration`] (and
//!   `Fixed<Nanosecond>`, exactly) and their durations.
//! * `schemars` — JSON schemas for quantities and unit enums, and
//!   `serde_str::schema` for text fields.
//! * `dynamic` — the `any` module: quantities whose kind is known only at runtime
//!   (`AnyQuantity`, `AnyUnit`, `QuantityKind`), parsed from text like
//!   `"1.5 km"`, with runtime-checked arithmetic.

#![no_std]
#![warn(missing_debug_implementations)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "std")]
extern crate std;

use core::fmt;
use core::ops::{Add, Div, Mul, Neg, Sub};

// ── Numeric storage ───────────────────────────────────────────────────────────

/// Floating-point storage types that support unit conversions.
///
/// Implemented for `f32` and `f64`. Implement it for your own type
/// (half-precision floats, dual numbers for automatic differentiation, …)
/// to unlock `from_<unit>` / `as_<unit>` conversions on it.
///
/// Arithmetic operators do **not** require this trait: they are available for
/// any storage type that supports the corresponding operator, including
/// integers.
pub trait Float:
    ToFloat
    + fmt::Debug
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Neg<Output = Self>
{
    /// Converts an `f64` conversion constant into `Self`, rounding to nearest.
    ///
    /// Always called with a literal, so it is folded away at compile time.
    fn from_f64(value: f64) -> Self;

    /// Absolute value.
    fn abs(self) -> Self;

    /// The larger of two values (ignoring NaN, like [`f64::max`]).
    fn max(self, other: Self) -> Self;

    /// The smaller of two values (ignoring NaN, like [`f64::min`]).
    fn min(self, other: Self) -> Self;

    /// IEEE 754 total order, like [`f64::total_cmp`].
    fn total_cmp(&self, other: &Self) -> core::cmp::Ordering;

    /// `true` unless the value is NaN or infinite.
    fn is_finite(self) -> bool;

    /// `true` for NaN.
    #[inline]
    fn is_nan(self) -> bool {
        #[allow(clippy::eq_op)]
        let nan = self != self;
        nan
    }

    /// `true` for negative values, `-0.0` included (like
    /// [`f64::is_sign_negative`]).
    #[inline]
    fn is_sign_negative(self) -> bool {
        let zero = Self::from_f64(0.0);
        self < zero || (self == zero && Self::from_f64(1.0) / self < zero)
    }
}

macro_rules! impl_float {
    ($($t:ident: $sign:expr),*) => {$(
        impl Float for $t {
            #[inline(always)]
            fn from_f64(value: f64) -> Self {
                value as $t
            }
            #[inline(always)]
            fn abs(self) -> Self {
                // Clear the sign bit. (`abs` itself is only in `core` since
                // Rust 1.85; this compiles to the same single instruction.)
                $t::from_bits(self.to_bits() & !$sign)
            }
            #[inline(always)]
            fn max(self, other: Self) -> Self {
                $t::max(self, other)
            }
            #[inline(always)]
            fn min(self, other: Self) -> Self {
                $t::min(self, other)
            }
            #[inline(always)]
            fn total_cmp(&self, other: &Self) -> core::cmp::Ordering {
                $t::total_cmp(self, other)
            }
            #[inline(always)]
            fn is_finite(self) -> bool {
                $t::is_finite(self)
            }
            #[inline(always)]
            fn is_nan(self) -> bool {
                $t::is_nan(self)
            }
            #[inline(always)]
            fn is_sign_negative(self) -> bool {
                $t::is_sign_negative(self)
            }
        }
    )*};
}
impl_float!(f32: 1 << 31, f64: 1 << 63);

/// Math functions on float storage, for `sqrt`, trigonometry and `hypot` on
/// quantities. Available with the `std` feature (default) or, on `no_std`
/// targets, the `libm` feature.
#[cfg(any(feature = "std", feature = "libm"))]
pub trait FloatMath: Float {
    /// Square root.
    fn sqrt(self) -> Self;
    /// Cube root.
    fn cbrt(self) -> Self;
    /// Sine (radians).
    fn sin(self) -> Self;
    /// Cosine (radians).
    fn cos(self) -> Self;
    /// Tangent (radians).
    fn tan(self) -> Self;
    /// Arcsine, in radians.
    fn asin(self) -> Self;
    /// Arccosine, in radians.
    fn acos(self) -> Self;
    /// Arctangent, in radians.
    fn atan(self) -> Self;
    /// Four-quadrant arctangent of `self / other`, in radians.
    fn atan2(self, other: Self) -> Self;
    /// `sqrt(self² + other²)` without intermediate overflow.
    fn hypot(self, other: Self) -> Self;
    /// Nearest integer, ties away from zero.
    fn round(self) -> Self;
    /// Largest integer not above.
    fn floor(self) -> Self;
    /// Smallest integer not below.
    fn ceil(self) -> Self;
}

#[cfg(any(feature = "std", feature = "libm"))]
macro_rules! impl_float_math {
    ($t:ident, $($method:ident($($arg:ident),*) => $libm:ident),*) => {
        impl FloatMath for $t {$(
            #[inline(always)]
            fn $method(self $(, $arg: Self)*) -> Self {
                // `std` provides inherent methods; prefer them when present.
                #[cfg(feature = "std")]
                return $t::$method(self $(, $arg)*);
                #[cfg(not(feature = "std"))]
                return libm::$libm(self $(, $arg)*);
            }
        )*}
    };
}
#[cfg(any(feature = "std", feature = "libm"))]
impl_float_math!(f64,
    sqrt() => sqrt, cbrt() => cbrt, sin() => sin, cos() => cos, tan() => tan,
    asin() => asin, acos() => acos, atan() => atan,
    atan2(other) => atan2, hypot(other) => hypot,
    round() => round, floor() => floor, ceil() => ceil);
#[cfg(any(feature = "std", feature = "libm"))]
impl_float_math!(f32,
    sqrt() => sqrtf, cbrt() => cbrtf, sin() => sinf, cos() => cosf, tan() => tanf,
    asin() => asinf, acos() => acosf, atan() => atanf,
    atan2(other) => atan2f, hypot(other) => hypotf,
    round() => roundf, floor() => floorf, ceil() => ceilf);

// ── Quantity trait ────────────────────────────────────────────────────────────

/// Implemented by every quantity type, for writing code that is generic over
/// quantities.
///
/// The inherent methods of the same names are `const` and preferred in
/// non-generic code.
pub trait Quantity: Copy {
    /// The numeric storage type (`f64` unless specified).
    type Value: Copy;

    /// The enum listing every unit of this quantity.
    type Unit: UnitEnum;

    /// The unit in which values are stored.
    const BASE_UNIT: Self::Unit;

    /// Name, description, dimensions and units of the quantity.
    const INFO: QuantityInfo;

    /// Wraps a value expressed in [`Self::BASE_UNIT`].
    fn from_base(value: Self::Value) -> Self;

    /// Returns the value expressed in [`Self::BASE_UNIT`].
    fn as_base(self) -> Self::Value;

    /// A quantity from a value in `unit`, like the inherent `new`.
    ///
    /// ```rust
    /// use qty::*;
    ///
    /// fn in_kilo<Q: Quantity<Value = f64>>(value: f64, kilo: Q::Unit, base: Q::Unit) -> f64 {
    ///     Q::new(value, kilo).convert_to(base)
    /// }
    /// assert_eq!(in_kilo::<Length>(1.5, LengthUnit::Kilometer, LengthUnit::Meter), 1500.0);
    /// ```
    fn new(value: Self::Value, unit: Self::Unit) -> Self
    where
        Self::Value: Float;

    /// The value expressed in `unit`, like the inherent `convert_to`.
    fn convert_to(self, unit: Self::Unit) -> Self::Value
    where
        Self::Value: Float;
}

/// Implemented by every unit enum (`LengthUnit`, `MassUnit`, …).
pub trait UnitEnum: Unit {
    /// Every unit of the quantity.
    const ALL: &'static [Self];
}

/// A unit: its abbreviation and names. Implemented by every unit enum and,
/// with the `dynamic` feature, by `AnyUnit`.
pub trait Unit: Copy + Eq + core::hash::Hash + fmt::Debug + fmt::Display + 'static {
    /// The primary abbreviation, e.g. `"km"`.
    fn abbreviation(self) -> &'static str;

    /// Names and abbreviations.
    fn info(self) -> &'static UnitInfo;
}

/// A value expressed in a given unit, as returned by `in_unit` and
/// `auto_unit`. Formats as the number followed by the unit's abbreviation,
/// applying format flags to the number:
///
/// ```rust
/// use qty::*;
///
/// let d = Length::from_meters(1234.5);
/// assert_eq!(d.in_unit(LengthUnit::Kilometer).to_string(), "1.2345 km");
/// assert_eq!(format!("{:.1}", d.in_unit(LengthUnit::Kilometer)), "1.2 km");
/// assert_eq!(format!("{:.2}", Length::from_meters(0.0025).auto_unit()), "2.50 mm");
///
/// // The parts are available too.
/// let InUnit { value, unit } = d.auto_unit();
/// assert_eq!((value, unit), (1.2345, LengthUnit::Kilometer));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct InUnit<T, U> {
    /// The numeric value, expressed in `unit`.
    pub value: T,
    /// The unit `value` is expressed in.
    pub unit: U,
}

macro_rules! in_unit_fmt {
    ($($Trait:ident),*) => {$(
        impl<T: fmt::$Trait, U: Unit> fmt::$Trait for InUnit<T, U> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::$Trait::fmt(&self.value, f)?;
                let abbr = self.unit.abbreviation();
                if abbr.is_empty() {
                    Ok(())
                } else {
                    write!(f, " {abbr}")
                }
            }
        }
    )*};
}
in_unit_fmt!(Display, LowerExp, UpperExp);

// ── Approximate comparison ────────────────────────────────────────────────────

/// Approximate floating-point comparison for any float-valued [`Quantity`].
///
/// Every quantity type implements this trait automatically through a blanket
/// implementation; the comparison logic is written exactly once.
///
/// # Choosing a method
///
/// | Method | Formula | Use when |
/// |---|---|---|
/// | [`approx_eq_abs`][ApproxEq::approx_eq_abs] | `\|a−b\| ≤ ε` | magnitude is known in advance |
/// | [`approx_eq_rel`][ApproxEq::approx_eq_rel] | `\|a−b\| ≤ tol·max(\|a\|,\|b\|)` | magnitude varies; relative accuracy matters |
/// | [`approx_eq`][ApproxEq::approx_eq] | `\|a−b\| ≤ ε + tol·max(\|a\|,\|b\|)` | mixed: safe near zero **and** for large values |
///
/// Tolerances are expressed in base-unit terms.
///
/// # Examples
///
/// ```rust
/// use qty::*;
///
/// let a = Length::from_meters(1.0);
/// let b = Length::from_centimeters(100.0);
///
/// // Absolute: difference must be under 1 nanometre.
/// assert!(a.approx_eq_abs(b, 1e-9));
///
/// // Relative: difference must be under 1 part per billion.
/// assert!(a.approx_eq_rel(b, 1e-9));
///
/// // Combined: absolute OR relative criterion, safe near zero.
/// assert!(a.approx_eq(b, 1e-12, 1e-9));
/// ```
pub trait ApproxEq: Quantity {
    /// Returns `true` if `|self − other| ≤ epsilon`.
    fn approx_eq_abs(self, other: Self, epsilon: Self::Value) -> bool;

    /// Returns `true` if `|self − other| ≤ rel_tol × max(|self|, |other|)`.
    ///
    /// Returns `true` when both values are exactly zero. Typical tolerances:
    /// `1e-12` for `f64`, `1e-6` for `f32`.
    fn approx_eq_rel(self, other: Self, rel_tol: Self::Value) -> bool;

    /// Returns `true` if `|self − other| ≤ epsilon + rel_tol × max(|self|, |other|)`.
    ///
    /// Combines absolute and relative tolerances, matching the semantics of
    /// NumPy's [`numpy.isclose`](https://numpy.org/doc/stable/reference/generated/numpy.isclose.html).
    fn approx_eq(self, other: Self, epsilon: Self::Value, rel_tol: Self::Value) -> bool;
}

impl<Q: Quantity> ApproxEq for Q
where
    Q::Value: Float,
{
    #[inline]
    fn approx_eq_abs(self, other: Self, epsilon: Self::Value) -> bool {
        (self.as_base() - other.as_base()).abs() <= epsilon
    }

    #[inline]
    fn approx_eq_rel(self, other: Self, rel_tol: Self::Value) -> bool {
        let (a, b) = (self.as_base(), other.as_base());
        // Exactly equal (including both zero) → trivially equal.
        a == b || (a - b).abs() <= rel_tol * a.abs().max(b.abs())
    }

    #[inline]
    fn approx_eq(self, other: Self, epsilon: Self::Value, rel_tol: Self::Value) -> bool {
        let (a, b) = (self.as_base(), other.as_base());
        (a - b).abs() <= epsilon + rel_tol * a.abs().max(b.abs())
    }
}

// ── Parsing ───────────────────────────────────────────────────────────────────

/// Error returned when parsing a quantity or a unit from a string fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ParseError {
    /// The numeric part is missing or malformed.
    InvalidNumber,
    /// The unit abbreviation is not recognised for this quantity.
    UnknownUnit,
    /// The unit belongs to several quantities (`"m"`: metre or minute), so
    /// the kind cannot be told (runtime quantities, `dynamic` feature).
    Ambiguous,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ParseError::InvalidNumber => "invalid number",
            ParseError::UnknownUnit => "unknown unit",
            ParseError::Ambiguous => "unit used by several quantities",
        })
    }
}

impl core::error::Error for ParseError {}

/// Normalises a unit *name* for matching: ASCII lowercase, without spaces,
/// `_` or `-`, with British "-re" spellings (metre, litre) turned into
/// "-er". Returns `None` for input that cannot be a name (non-ASCII, or
/// longer than the buffer).
#[doc(hidden)]
pub fn normalize_unit_name<'a>(name: &str, buf: &'a mut [u8; 64]) -> Option<&'a str> {
    let mut len = 0;
    for &b in name.trim().as_bytes() {
        if matches!(b, b' ' | b'_' | b'-') {
            continue;
        }
        if !b.is_ascii() || len == buf.len() {
            return None;
        }
        buf[len] = b.to_ascii_lowercase();
        len += 1;
    }
    let out = &mut buf[..len];
    for word in [&b"metre"[..], b"litre"] {
        let mut i = 0;
        while i + word.len() <= out.len() {
            if &out[i..i + word.len()] == word {
                out[i + word.len() - 2..i + word.len()].copy_from_slice(b"er");
                i += word.len();
            } else {
                i += 1;
            }
        }
    }
    core::str::from_utf8(out).ok()
}

/// Splits `"1.5e3 km/h"` into `("1.5e3", "km/h")`.
///
/// The number is the longest prefix matching a decimal float literal, so unit
/// abbreviations may themselves contain digits (`"2 m3"`) and the space is
/// optional (`"2m3"`).
#[doc(hidden)]
pub fn split_value_unit(s: &str) -> (&str, &str) {
    let s = s.trim();
    let b = s.as_bytes();
    let digits = |mut i: usize| {
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        i
    };
    let mut i = 0;
    if i < b.len() && matches!(b[i], b'+' | b'-') {
        i += 1;
    }
    i = digits(i);
    if i < b.len() && b[i] == b'.' {
        i = digits(i + 1);
    }
    if i < b.len() && matches!(b[i], b'e' | b'E') {
        let mut j = i + 1;
        if j < b.len() && matches!(b[j], b'+' | b'-') {
            j += 1;
        }
        if j < b.len() && b[j].is_ascii_digit() {
            i = digits(j);
        }
    }
    (&s[..i], s[i..].trim_start())
}

// ── Code shared by every generated quantity ──────────────────────────────────

/// Storage types that get `scalar * quantity` (the reverse of
/// `quantity * scalar`, which is generic). Coherence rules forbid a generic
/// impl on a foreign type parameter, so this list is explicit.
///
/// `f32` is deliberately absent: with two float impls, `2.0 * q` would be
/// ambiguous whenever `q`'s float type is not yet inferred
/// (`let q = Length::from_meters(1.0)`), forcing annotations on the far more
/// common `f64` code. With `f32` storage, write `q * 2.0`.
macro_rules! scalar_lhs_mul {
    ($Q:ident: $($t:ty),*) => {$(
        impl core::ops::Mul<$Q<$t>> for $t {
            type Output = $Q<$t>;
            #[inline(always)]
            fn mul(self, rhs: $Q<$t>) -> $Q<$t> {
                $Q::from_base(self * rhs.0)
            }
        }
    )*};
}

/// `quantity * Vector2/Vector3` for the given storage types.
macro_rules! quantity_times_vector {
    ($Q:ident: $($t:ty),*) => {$(
        quantity_times_vector!(@impl $Q, [], $t, Vector2);
        quantity_times_vector!(@impl $Q, [], $t, Vector3);
    )*};
    ($Q:ident<$F:ident>: $t:ty) => {
        quantity_times_vector!(@impl $Q, [$F], $t, Vector2);
        quantity_times_vector!(@impl $Q, [$F], $t, Vector3);
    };
    (@impl $Q:ident, [$($F:ident)?], $t:ty, $V:ident) => {
        impl<$($F,)? R> core::ops::Mul<$crate::$V<R>> for $Q<$t>
        where
            $Q<$t>: core::ops::Mul<R> + Copy,
        {
            type Output = $crate::$V<<$Q<$t> as core::ops::Mul<R>>::Output>;
            #[inline(always)]
            fn mul(self, rhs: $crate::$V<R>) -> Self::Output {
                rhs.map(|c| self * c)
            }
        }
    };
}

/// Formats the value in the base unit followed by the unit abbreviation,
/// honouring precision and other flags:
/// `format!("{:.2}", Length::from_meters(1.0))` is `"1.00 m"` and
/// `format!("{:e}", Length::from_meters(1500.0))` is `"1.5e3 m"`.
macro_rules! quantity_fmt {
    ($Q:ident, $U:ident, $base:ident: $($Trait:ident),*) => {$(
        impl<T: core::fmt::$Trait> core::fmt::$Trait for $Q<T> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::$Trait::fmt(&self.0, f)?;
                let abbr = $U::$base.abbreviation();
                if abbr.is_empty() {
                    Ok(())
                } else {
                    write!(f, " {abbr}")
                }
            }
        }
    )*};
}

/// Implements everything a quantity needs apart from its unit conversions:
/// base-unit access, zero-copy slice views, arithmetic operators,
/// [`Quantity`], `Display` and `FromStr`.
macro_rules! quantity_core {
    ($Q:ident, $U:ident, $base:ident, point: $point:literal) => {
        impl<T> $Q<T> {
            /// Wraps a value expressed in the base unit. Free and `const`.
            #[inline(always)]
            pub const fn from_base(value: T) -> Self {
                Self(value)
            }

            /// Unwraps the value expressed in the base unit. Unlike
            /// [`as_base`](Self::as_base) this does not require `T: Copy`.
            #[inline(always)]
            pub fn into_base(self) -> T {
                self.0
            }

            /// Views a slice of base-unit values as quantities, without copying.
            #[inline(always)]
            pub const fn from_base_slice(values: &[T]) -> &[Self] {
                // SAFETY: `Self` is `#[repr(transparent)]` over `T`.
                unsafe { &*(values as *const [T] as *const [Self]) }
            }

            /// Views a mutable slice of base-unit values as quantities, without copying.
            #[inline(always)]
            pub fn from_base_slice_mut(values: &mut [T]) -> &mut [Self] {
                // SAFETY: `Self` is `#[repr(transparent)]` over `T`.
                unsafe { &mut *(values as *mut [T] as *mut [Self]) }
            }

            /// Views a slice of quantities as their base-unit values, without copying.
            #[inline(always)]
            pub const fn as_base_slice(values: &[Self]) -> &[T] {
                // SAFETY: `Self` is `#[repr(transparent)]` over `T`.
                unsafe { &*(values as *const [Self] as *const [T]) }
            }

            /// Views a mutable slice of quantities as their base-unit values, without copying.
            #[inline(always)]
            pub fn as_base_slice_mut(values: &mut [Self]) -> &mut [T] {
                // SAFETY: `Self` is `#[repr(transparent)]` over `T`.
                unsafe { &mut *(values as *mut [Self] as *mut [T]) }
            }

            /// Changes the storage type losslessly, e.g. `i32` → `f64`.
            #[inline(always)]
            pub fn cast<U: From<T>>(self) -> $Q<U> {
                $Q(U::from(self.0))
            }
        }

        impl<T: Copy> $Q<T> {
            /// Returns the value expressed in the base unit. Free and `const`.
            #[inline(always)]
            pub const fn as_base(self) -> T {
                self.0
            }
        }

        impl<T: $crate::Float> $Q<T> {
            /// Absolute value.
            #[inline(always)]
            pub fn abs(self) -> Self {
                Self(self.0.abs())
            }

            /// `true` for NaN.
            #[inline(always)]
            pub fn is_nan(self) -> bool {
                self.0.is_nan()
            }

            /// `true` unless NaN or infinite.
            #[inline(always)]
            pub fn is_finite(self) -> bool {
                self.0.is_finite()
            }

            /// The larger of two quantities (ignoring NaN).
            #[inline(always)]
            pub fn max(self, other: Self) -> Self {
                Self(self.0.max(other.0))
            }

            /// The smaller of two quantities (ignoring NaN).
            #[inline(always)]
            pub fn min(self, other: Self) -> Self {
                Self(self.0.min(other.0))
            }

            /// Restricts to `[min, max]`, like [`f64::clamp`]: NaN stays NaN.
            ///
            /// # Panics
            ///
            /// If `min > max`.
            #[inline]
            pub fn clamp(self, min: Self, max: Self) -> Self {
                assert!(
                    min.0.partial_cmp(&max.0) != Some(core::cmp::Ordering::Greater),
                    "clamp: min > max"
                );
                if self.0 < min.0 {
                    min
                } else if self.0 > max.0 {
                    max
                } else {
                    self
                }
            }

            /// The mean of `values`, or `None` if there are none. Computed as
            /// the first value plus the mean difference from it, so it also
            /// works for points: the mean of 20 °C and 30 °C is 25 °C.
            ///
            /// With `Uncertain` storage the first value
            /// enters every difference, which overstates the uncertainty;
            /// sum and divide instead.
            pub fn mean<I: IntoIterator<Item = Self>>(values: I) -> Option<Self> {
                let mut values = values.into_iter();
                let first = values.next()?.0;
                let mut sum = T::from_f64(0.0);
                let mut n = 1_u32;
                for q in values {
                    sum = sum + (q.0 - first);
                    n += 1;
                }
                Some(Self(first + sum / T::from_f64(f64::from(n))))
            }

            /// Linear interpolation: `self` at `t = 0`, `other` at `t = 1`
            /// (to within rounding), extrapolating outside `[0, 1]`. Works
            /// for points too: halfway between 10 °C and 20 °C is 15 °C.
            #[inline]
            pub fn lerp(self, other: Self, t: T) -> Self {
                Self(self.0 + (other.0 - self.0) * t)
            }

            /// IEEE 754 total order (like [`f64::total_cmp`]), for sorting:
            /// `values.sort_by(Length::total_cmp)`. For map keys, see
            /// [`Ordered`](crate::Ordered).
            #[inline(always)]
            pub fn total_cmp(&self, other: &Self) -> core::cmp::Ordering {
                self.0.total_cmp(&other.0)
            }
        }

        impl<T: $crate::ToFloat> $Q<T> {
            /// Converts the storage to `f64` (like `as f64`), e.g.
            /// `Length<i64>` → `Length<f64>`.
            #[inline(always)]
            pub fn to_f64(self) -> $Q<f64> {
                $Q(self.0.to_f64())
            }

            /// Converts the storage to `f32` (like `as f32`).
            #[inline(always)]
            pub fn to_f32(self) -> $Q<f32> {
                $Q(self.0.to_f32())
            }
        }

        impl<I: $crate::Integer> $Q<I> {
            /// Rounds a float quantity to a whole number of base units.
            /// For other units, see [`Fixed`](crate::Fixed).
            #[inline]
            pub fn round_from<F: $crate::Float>(
                quantity: $Q<F>,
                mode: $crate::Rounding,
            ) -> Result<Self, $crate::RoundError> {
                $crate::Fixed::<<$Q<f64> as $crate::HasFixedBase>::Base, I>::round_from(quantity, mode)
                    .map(Self::from)
            }
        }

        impl<T: Copy> $crate::Quantity for $Q<T> {
            type Value = T;
            type Unit = $U;
            const BASE_UNIT: $U = $U::$base;
            const INFO: $crate::QuantityInfo = $U::QUANTITY;
            #[inline(always)]
            fn from_base(value: T) -> Self {
                Self(value)
            }
            #[inline(always)]
            fn as_base(self) -> T {
                self.0
            }
            #[inline]
            fn new(value: T, unit: $U) -> Self
            where
                T: $crate::Float,
            {
                $Q::new(value, unit)
            }
            #[inline]
            fn convert_to(self, unit: $U) -> T
            where
                T: $crate::Float,
            {
                $Q::convert_to(self, unit)
            }
        }

        // ── Scaling vectors: `mass * acceleration_vector` ──
        // For concrete storage types: a generic `$Q<T>` impl would overlap
        // with `$Q<T> * T` when `T` is itself a vector.
        quantity_times_vector!($Q: f64, f32);
        #[cfg(any(feature = "std", feature = "libm"))]
        quantity_times_vector!($Q<F>: $crate::Uncertain<F>);

        // ── Same-quantity ratio ──

        /// Dividing two values of the same quantity gives a plain number.
        impl<T: core::ops::Div<Output = T>> core::ops::Div for $Q<T> {
            type Output = T;
            #[inline(always)]
            fn div(self, rhs: Self) -> T {
                self.0 / rhs.0
            }
        }

        // ── Formatting & parsing ──

        quantity_fmt!($Q, $U, $base: Display, LowerExp, UpperExp);

        impl core::fmt::Display for $U {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.pad(self.abbreviation())
            }
        }

        // ── Optional integrations ──

        #[cfg(feature = "approx")]
        impl<T: approx::AbsDiffEq> approx::AbsDiffEq for $Q<T> {
            type Epsilon = T::Epsilon;
            #[inline]
            fn default_epsilon() -> T::Epsilon {
                T::default_epsilon()
            }
            #[inline]
            fn abs_diff_eq(&self, other: &Self, epsilon: T::Epsilon) -> bool {
                self.0.abs_diff_eq(&other.0, epsilon)
            }
        }
        #[cfg(feature = "approx")]
        impl<T: approx::RelativeEq> approx::RelativeEq for $Q<T> {
            #[inline]
            fn default_max_relative() -> T::Epsilon {
                T::default_max_relative()
            }
            #[inline]
            fn relative_eq(&self, other: &Self, epsilon: T::Epsilon, max_relative: T::Epsilon) -> bool {
                self.0.relative_eq(&other.0, epsilon, max_relative)
            }
        }
        #[cfg(feature = "approx")]
        impl<T: approx::UlpsEq> approx::UlpsEq for $Q<T> {
            #[inline]
            fn default_max_ulps() -> u32 {
                T::default_max_ulps()
            }
            #[inline]
            fn ulps_eq(&self, other: &Self, epsilon: T::Epsilon, max_ulps: u32) -> bool {
                self.0.ulps_eq(&other.0, epsilon, max_ulps)
            }
        }

        // SAFETY: `#[repr(transparent)]` over `T`.
        #[cfg(feature = "bytemuck")]
        unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for $Q<T> {}
        // SAFETY: as above; `Pod` requires `T: Pod`.
        #[cfg(feature = "bytemuck")]
        unsafe impl<T: bytemuck::Pod> bytemuck::Pod for $Q<T> {}
        // SAFETY: as above.
        #[cfg(feature = "bytemuck")]
        unsafe impl<T> bytemuck::TransparentWrapper<T> for $Q<T> {}

        #[cfg(feature = "defmt")]
        impl<T: defmt::Format> defmt::Format for $Q<T> {
            fn format(&self, f: defmt::Formatter<'_>) {
                defmt::write!(f, "{} {=str}", self.0, $U::$base.abbreviation());
            }
        }
        #[cfg(feature = "schemars")]
        impl<T: schemars::JsonSchema> schemars::JsonSchema for $Q<T> {
            fn inline_schema() -> bool {
                true
            }
            fn schema_name() -> std::borrow::Cow<'static, str> {
                std::format!("{}_{}", stringify!($Q), T::schema_name()).into()
            }
            /// The storage type's schema (a number for floats), described
            /// with the base unit.
            fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
                let mut schema = T::json_schema(generator);
                schema.insert(
                    "description".into(),
                    std::format!(
                        "{} in {} ({})",
                        stringify!($Q),
                        $U::$base.info().plural,
                        $U::$base.abbreviation()
                    )
                    .into(),
                );
                schema
            }
        }
        #[cfg(feature = "schemars")]
        impl schemars::JsonSchema for $U {
            fn schema_name() -> std::borrow::Cow<'static, str> {
                stringify!($U).into()
            }
            /// The variant names, as serde writes them.
            fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
                let names: std::vec::Vec<&str> = $U::ALL.iter().map(|u| u.info().singular).collect();
                schemars::json_schema!({ "type": "string", "enum": names })
            }
        }

        #[cfg(feature = "defmt")]
        impl defmt::Format for $U {
            fn format(&self, f: defmt::Formatter<'_>) {
                defmt::write!(f, "{=str}", self.abbreviation());
            }
        }

        impl $crate::UnitEnum for $U {
            const ALL: &'static [Self] = $U::ALL;
        }
        impl $crate::Unit for $U {
            #[inline(always)]
            fn abbreviation(self) -> &'static str {
                $U::abbreviation(self)
            }
            #[inline(always)]
            fn info(self) -> &'static $crate::UnitInfo {
                $U::info(self)
            }
        }

        impl<T: $crate::Float> $Q<T> {
            /// The value expressed in `unit`, for display or inspection:
            /// `length.in_unit(LengthUnit::Kilometer)` formats as `"1.5 km"`.
            #[inline]
            pub fn in_unit(self, unit: $U) -> $crate::InUnit<T, $U> {
                $crate::InUnit {
                    value: self.convert_to(unit),
                    unit,
                }
            }

            /// The value in the largest of `units` that does not exceed it
            /// (the smallest of them for smaller values), like
            /// [`auto_unit`](Self::auto_unit) over a unit list of your
            /// choice: `length.auto_unit_in(LengthUnit::IMPERIAL)`. Meant
            /// for linear units; with an empty list, the base unit.
            pub fn auto_unit_in(self, units: &[$U]) -> $crate::InUnit<T, $U> {
                let magnitude = self.0.abs();
                let one = |unit: $U| Self::new(T::from_f64(1.0), unit).0.abs();
                let mut best: Option<($U, T)> = None;
                let mut smallest: Option<($U, T)> = None;
                for &unit in units {
                    let size = one(unit);
                    if smallest.map_or(true, |(_, s)| size < s) {
                        smallest = Some((unit, size));
                    }
                    if size <= magnitude && best.map_or(true, |(_, b)| size > b) {
                        best = Some((unit, size));
                    }
                }
                let unit = best.or(smallest).map_or($U::$base, |(unit, _)| unit);
                self.in_unit(unit)
            }
        }

        #[cfg(any(feature = "std", feature = "libm"))]
        impl<T: $crate::FloatMath> $Q<T> {
            /// Rounds to a whole number of `unit`, ties away from zero:
            /// 1.26 m in centimetres is 126 cm. Works for points too
            /// (21.6 °C rounds to 22 °C).
            #[inline]
            pub fn round_in(self, unit: $U) -> Self {
                Self::new(self.convert_to(unit).round(), unit)
            }

            /// Rounds down to a whole number of `unit`.
            #[inline]
            pub fn floor_in(self, unit: $U) -> Self {
                Self::new(self.convert_to(unit).floor(), unit)
            }

            /// Rounds up to a whole number of `unit`.
            #[inline]
            pub fn ceil_in(self, unit: $U) -> Self {
                Self::new(self.convert_to(unit).ceil(), unit)
            }
        }

        /// Parses a number followed by a unit: an abbreviation (`"1.5 km"`),
        /// a name (`"1.5 kilometres"`) or, for quantities other than points,
        /// an expression with the same dimensions (`"1.5 km/s*s"`, see
        /// [`UnitExpr`](crate::UnitExpr)).
        impl<T: $crate::Float + core::str::FromStr> core::str::FromStr for $Q<T> {
            type Err = $crate::ParseError;
            fn from_str(s: &str) -> Result<Self, $crate::ParseError> {
                let (number, unit) = $crate::split_value_unit(s);
                let value: T = number
                    .parse()
                    .map_err(|_| $crate::ParseError::InvalidNumber)?;
                match unit.parse::<$U>() {
                    Ok(unit) => Ok(Self::new(value, unit)),
                    Err(e) if $point => Err(e),
                    Err(e) => {
                        let factor = $crate::UnitExpr::parse(unit)
                            .ok()
                            .and_then(|expr| expr.factor_to_base::<Self>())
                            .ok_or(e)?;
                        Ok(Self::from_base(value * T::from_f64(factor)))
                    }
                }
            }
        }
    };
}

/// Arithmetic for ordinary ("vector") quantities: values that can be added,
/// negated and scaled — lengths, durations, temperature differences, …
macro_rules! quantity_vector_ops {
    ($Q:ident) => {
        impl<I: $crate::Integer> $Q<I> {
            /// `self + other`, or `None` on overflow.
            #[inline]
            pub fn checked_add(self, other: Self) -> Option<Self> {
                self.0.checked_add(other.0).map(Self)
            }

            /// `self - other`, or `None` on overflow.
            #[inline]
            pub fn checked_sub(self, other: Self) -> Option<Self> {
                self.0.checked_sub(other.0).map(Self)
            }
        }

        impl<T: $crate::Float> $Q<T> {
            /// `1` for positive values and `+0`, `-1` for negative values
            /// and `-0`, NaN for NaN (like [`f64::signum`]).
            #[inline]
            pub fn signum(self) -> T {
                if self.0.is_nan() {
                    self.0
                } else if self.0.is_sign_negative() {
                    T::from_f64(-1.0)
                } else {
                    T::from_f64(1.0)
                }
            }

            /// The magnitude of `self` with the sign of `sign` (like
            /// [`f64::copysign`]).
            #[inline]
            pub fn copysign(self, sign: Self) -> Self {
                if self.0.is_sign_negative() == sign.0.is_sign_negative() {
                    self
                } else {
                    Self(-self.0)
                }
            }
        }

        impl $Q<f64> {
            /// Zero, as a constant. For `f64` storage only, like
            /// `const_new`: with an `f32` constant too, `Length::ZERO`
            /// would be ambiguous. Other storage types have `default()`.
            pub const ZERO: Self = Self(0.0);
        }

        #[cfg(any(feature = "std", feature = "libm"))]
        impl<T: $crate::FloatMath> $Q<T> {
            /// `sqrt(self² + other²)`: the magnitude of two perpendicular
            /// components, without intermediate overflow.
            #[inline]
            pub fn hypot(self, other: Self) -> Self {
                Self(self.0.hypot(other.0))
            }
        }

        // ── Arithmetic: same quantity ──

        impl<T: core::ops::Add<Output = T>> core::ops::Add for $Q<T> {
            type Output = Self;
            #[inline(always)]
            fn add(self, rhs: Self) -> Self {
                Self(self.0 + rhs.0)
            }
        }
        impl<T: core::ops::Sub<Output = T>> core::ops::Sub for $Q<T> {
            type Output = Self;
            #[inline(always)]
            fn sub(self, rhs: Self) -> Self {
                Self(self.0 - rhs.0)
            }
        }
        impl<T: core::ops::Neg<Output = T>> core::ops::Neg for $Q<T> {
            type Output = Self;
            #[inline(always)]
            fn neg(self) -> Self {
                Self(-self.0)
            }
        }
        impl<T: core::ops::Rem<Output = T>> core::ops::Rem for $Q<T> {
            type Output = Self;
            #[inline(always)]
            fn rem(self, rhs: Self) -> Self {
                Self(self.0 % rhs.0)
            }
        }
        impl<T: core::ops::AddAssign> core::ops::AddAssign for $Q<T> {
            #[inline(always)]
            fn add_assign(&mut self, rhs: Self) {
                self.0 += rhs.0;
            }
        }
        impl<T: core::ops::SubAssign> core::ops::SubAssign for $Q<T> {
            #[inline(always)]
            fn sub_assign(&mut self, rhs: Self) {
                self.0 -= rhs.0;
            }
        }
        impl<T: core::ops::RemAssign> core::ops::RemAssign for $Q<T> {
            #[inline(always)]
            fn rem_assign(&mut self, rhs: Self) {
                self.0 %= rhs.0;
            }
        }

        // ── Arithmetic: scaling by a plain number ──

        impl<T: core::ops::Mul<Output = T>> core::ops::Mul<T> for $Q<T> {
            type Output = Self;
            #[inline(always)]
            fn mul(self, rhs: T) -> Self {
                Self(self.0 * rhs)
            }
        }
        impl<T: core::ops::Div<Output = T>> core::ops::Div<T> for $Q<T> {
            type Output = Self;
            #[inline(always)]
            fn div(self, rhs: T) -> Self {
                Self(self.0 / rhs)
            }
        }
        impl<T: core::ops::MulAssign> core::ops::MulAssign<T> for $Q<T> {
            #[inline(always)]
            fn mul_assign(&mut self, rhs: T) {
                self.0 *= rhs;
            }
        }
        impl<T: core::ops::DivAssign> core::ops::DivAssign<T> for $Q<T> {
            #[inline(always)]
            fn div_assign(&mut self, rhs: T) {
                self.0 /= rhs;
            }
        }
        scalar_lhs_mul!($Q: f64, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

        // ── Iterators ──

        // Delegates to `T`'s own `Sum`, so the semantics (and the machine
        // code) are exactly those of summing the bare numbers.
        impl<T: core::iter::Sum> core::iter::Sum for $Q<T> {
            #[inline]
            fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
                Self(iter.map(|q| q.0).sum())
            }
        }
        impl<'a, T: core::iter::Sum<&'a T> + 'a> core::iter::Sum<&'a $Q<T>> for $Q<T> {
            #[inline]
            fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
                Self(iter.map(|q| &q.0).sum())
            }
        }
    };
}

/// Implements everything an ordinary quantity needs apart from its unit
/// conversions: base-unit access, zero-copy slice views, arithmetic
/// operators, [`Quantity`], `Display` and `FromStr`.
macro_rules! quantity {
    ($Q:ident, $U:ident, $base:ident) => {
        quantity_core!($Q, $U, $base, point: false);
        quantity_vector_ops!($Q);
    };
}

/// A quantity that is a *point on a scale* rather than an amount — an
/// absolute temperature. Points cannot be added, negated or scaled
/// (20 °C + 20 °C is not 40 °C); their differences are amounts of `$D`:
///
/// * `point - point = $D`
/// * `point ± $D = point`, `$D + point = point`, `point ±= $D`
///
/// `point / point` remains: it is a ratio of absolute values.
macro_rules! point_quantity {
    ($Q:ident, $U:ident, $base:ident, $D:ident) => {
        quantity_core!($Q, $U, $base, point: true);

        impl<T: core::ops::Sub<Output = T>> core::ops::Sub for $Q<T> {
            type Output = $D<T>;
            #[inline(always)]
            fn sub(self, rhs: Self) -> $D<T> {
                $D::from_base(self.0 - rhs.0)
            }
        }
        impl<T: core::ops::Add<Output = T>> core::ops::Add<$D<T>> for $Q<T> {
            type Output = Self;
            #[inline(always)]
            fn add(self, rhs: $D<T>) -> Self {
                Self(self.0 + rhs.into_base())
            }
        }
        impl<T: core::ops::Add<Output = T>> core::ops::Add<$Q<T>> for $D<T> {
            type Output = $Q<T>;
            #[inline(always)]
            fn add(self, rhs: $Q<T>) -> $Q<T> {
                $Q::from_base(self.into_base() + rhs.0)
            }
        }
        impl<T: core::ops::Sub<Output = T>> core::ops::Sub<$D<T>> for $Q<T> {
            type Output = Self;
            #[inline(always)]
            fn sub(self, rhs: $D<T>) -> Self {
                Self(self.0 - rhs.into_base())
            }
        }
        impl<T: core::ops::AddAssign> core::ops::AddAssign<$D<T>> for $Q<T> {
            #[inline(always)]
            fn add_assign(&mut self, rhs: $D<T>) {
                self.0 += rhs.into_base();
            }
        }
        impl<T: core::ops::SubAssign> core::ops::SubAssign<$D<T>> for $Q<T> {
            #[inline(always)]
            fn sub_assign(&mut self, rhs: $D<T>) {
                self.0 -= rhs.into_base();
            }
        }
    };
}

/// `A * B = C` (and `B * A = C` when emitted separately), computed in base
/// units. `$k` is the factor relating the base units, `1` for coherent SI.
macro_rules! cross_mul {
    ($A:ident * $B:ident = $C:ident) => {
        impl<T: core::ops::Mul<Output = T>> core::ops::Mul<$B<T>> for $A<T> {
            type Output = $C<T>;
            #[inline(always)]
            fn mul(self, rhs: $B<T>) -> $C<T> {
                $C::from_base(self.into_base() * rhs.into_base())
            }
        }
    };
    ($A:ident * $B:ident = $C:ident; $k:literal) => {
        impl<T: $crate::Float> core::ops::Mul<$B<T>> for $A<T> {
            type Output = $C<T>;
            #[inline(always)]
            fn mul(self, rhs: $B<T>) -> $C<T> {
                $C::from_base(self.into_base() * rhs.into_base() * T::from_f64($k))
            }
        }
    };
}

/// `A / B = C`, computed in base units.
macro_rules! cross_div {
    ($A:ident / $B:ident = $C:ident) => {
        impl<T: core::ops::Div<Output = T>> core::ops::Div<$B<T>> for $A<T> {
            type Output = $C<T>;
            #[inline(always)]
            fn div(self, rhs: $B<T>) -> $C<T> {
                $C::from_base(self.into_base() / rhs.into_base())
            }
        }
    };
    ($A:ident / $B:ident = $C:ident; $k:literal) => {
        impl<T: $crate::Float> core::ops::Div<$B<T>> for $A<T> {
            type Output = $C<T>;
            #[inline(always)]
            #[allow(clippy::suspicious_arithmetic_impl)]
            fn div(self, rhs: $B<T>) -> $C<T> {
                $C::from_base(self.into_base() / rhs.into_base() * T::from_f64($k))
            }
        }
    };
}

/// `A * B` is dimensionless: the result is a plain number.
macro_rules! cross_mul_scalar {
    ($A:ident * $B:ident) => {
        impl<T: core::ops::Mul<Output = T>> core::ops::Mul<$B<T>> for $A<T> {
            type Output = T;
            #[inline(always)]
            fn mul(self, rhs: $B<T>) -> T {
                self.into_base() * rhs.into_base()
            }
        }
    };
}

/// `C.sqrt() = A` where `C = A * A` (e.g. `Area` → `Length`); `$k` relates
/// the base units when they are not coherent.
macro_rules! cross_sqrt {
    ($C:ident => $A:ident) => {
        cross_sqrt!(@impl $C => $A, sqrt, "square", |v| v);
        cross_power!($A => $C, squared, "square", |a| a * a);
    };
    ($C:ident => $A:ident; $k:literal) => {
        cross_sqrt!(@impl $C => $A, sqrt, "square", |v| v / T::from_f64($k));
        cross_power!($A => $C, squared, "square", |a| a * a * T::from_f64($k));
    };
    (@impl $C:ident => $A:ident, $method:ident, $what:literal, |$v:ident| $scaled:expr) => {
        #[cfg(any(feature = "std", feature = "libm"))]
        impl<T: $crate::FloatMath> $C<T> {
            #[doc = concat!(
                "The [`", stringify!($A), "`] whose ", $what, " is this [`",
                stringify!($C), "`]."
            )]
            #[inline]
            pub fn $method(self) -> $A<T> {
                let $v = self.into_base();
                $A::from_base(($scaled).$method())
            }
        }
    };
}

/// `A.squared() = C` and `A.cubed() = C`: the inverses of `cross_sqrt!` and
/// `cross_cbrt!`; `$k` as there.
macro_rules! cross_power {
    ($A:ident => $C:ident, $method:ident, $what:literal, |$v:ident| $value:expr) => {
        impl<T: $crate::Float> $A<T> {
            #[doc = concat!("The ", $what, " of this quantity: a [`", stringify!($C), "`].")]
            #[inline]
            pub fn $method(self) -> $C<T> {
                let $v = self.into_base();
                $C::from_base($value)
            }
        }
    };
}

/// `C.cbrt() = A` where `C = A * A * A` (e.g. `Volume` → `Length`).
macro_rules! cross_cbrt {
    ($C:ident => $A:ident) => {
        cross_sqrt!(@impl $C => $A, cbrt, "cube", |v| v);
        cross_power!($A => $C, cubed, "cube", |a| a * a * a);
    };
    ($C:ident => $A:ident; $k:literal) => {
        cross_sqrt!(@impl $C => $A, cbrt, "cube", |v| v / T::from_f64($k));
        cross_power!($A => $C, cubed, "cube", |a| a * a * a * T::from_f64($k));
    };
}

/// `1 / A = B` for `f64` storage, e.g. `1.0 / duration` is a `Frequency`.
/// (`f32` is omitted for the inference reason given on `scalar_lhs_mul!`.)
macro_rules! cross_recip {
    ($A:ident => $B:ident) => {
        impl core::ops::Div<$A<f64>> for f64 {
            type Output = $B<f64>;
            #[inline(always)]
            fn div(self, rhs: $A<f64>) -> $B<f64> {
                $B::from_base(self / rhs.into_base())
            }
        }
    };
}

/// Declares the [`FixedUnit`] marker for one unit.
macro_rules! fixed_unit {
    // Units of ordinary quantities: counts can be added.
    ($M:ident: $Q:ident, $U:ident :: $V:ident, base $B:ident,
     $num:literal / $den:literal, pi^$pi:literal, $from:ident, $as:ident, additive) => {
        fixed_unit!($M: $Q, $U::$V, base $B, $num / $den, pi^$pi, $from, $as);
        impl $crate::AdditiveUnit for $M {}
    };
    // Units of point quantities: differences are counts of `$delta`.
    ($M:ident: $Q:ident, $U:ident :: $V:ident, base $B:ident,
     $num:literal / $den:literal, pi^$pi:literal, $from:ident, $as:ident
     $(, offset($on:expr, $od:expr))?, point $delta:path) => {
        fixed_unit!($M: $Q, $U::$V, base $B, $num / $den, pi^$pi, $from, $as $(, offset($on, $od))?);

        impl $crate::PointUnit for $M {
            type Delta = $delta;
        }
        /// `point - point` is a difference, in the delta unit of the same scale.
        impl<V: $crate::PointUnit<Unit = super::$U>, I: $crate::Integer>
            core::ops::Sub<$crate::Fixed<V, I>> for $crate::Fixed<$M, I>
        {
            type Output = $crate::Fixed<$delta, I>;
            #[inline(always)]
            fn sub(self, rhs: $crate::Fixed<V, I>) -> Self::Output {
                $crate::Fixed::new(self.count() - rhs.count_in::<$M>())
            }
        }
        impl<D: $crate::AdditiveUnit<Unit = <$delta as $crate::FixedUnit>::Unit>, I: $crate::Integer>
            core::ops::Add<$crate::Fixed<D, I>> for $crate::Fixed<$M, I>
        {
            type Output = Self;
            #[inline(always)]
            fn add(self, rhs: $crate::Fixed<D, I>) -> Self {
                $crate::Fixed::new(self.count() + rhs.count_in::<$delta>())
            }
        }
        impl<D: $crate::AdditiveUnit<Unit = <$delta as $crate::FixedUnit>::Unit>, I: $crate::Integer>
            core::ops::AddAssign<$crate::Fixed<D, I>> for $crate::Fixed<$M, I>
        {
            #[inline(always)]
            fn add_assign(&mut self, rhs: $crate::Fixed<D, I>) {
                *self = *self + rhs;
            }
        }
        impl<D: $crate::AdditiveUnit<Unit = <$delta as $crate::FixedUnit>::Unit>, I: $crate::Integer>
            core::ops::SubAssign<$crate::Fixed<D, I>> for $crate::Fixed<$M, I>
        {
            #[inline(always)]
            fn sub_assign(&mut self, rhs: $crate::Fixed<D, I>) {
                *self = self.sub_delta(rhs);
            }
        }
    };
    ($M:ident: $Q:ident, $U:ident :: $V:ident, base $B:ident,
     $num:literal / $den:literal, pi^$pi:literal, $from:ident, $as:ident
     $(, offset($on:expr, $od:expr))?) => {
        #[doc = concat!(
            "Marker for counting [`", stringify!($Q), "`](super::", stringify!($Q),
            ") in [`", stringify!($V), "`](super::", stringify!($U), "::", stringify!($V),
            ") with [`Fixed`](crate::Fixed)."
        )]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        pub struct $M;

        impl $crate::FixedUnit for $M {
            type Quantity<F: $crate::Float> = super::$Q<F>;
            type Unit = super::$U;
            type Base = $B;
            const UNIT: super::$U = super::$U::$V;
            const ABBREVIATION: &'static str = super::$U::$V.abbreviation();
            const ABBREVIATIONS: &'static [&'static str] = super::$U::$V.all_abbreviations();
            const NUM: u128 = $num;
            const DEN: u128 = $den;
            const PI_POW: i32 = $pi;
            $(
                const OFFSET_NUM: i128 = $on;
                const OFFSET_DEN: u128 = $od;
            )?
            #[inline(always)]
            fn to_quantity<F: $crate::Float>(count: F) -> super::$Q<F> {
                super::$Q::$from(count)
            }
            #[inline(always)]
            fn from_quantity<F: $crate::Float>(quantity: super::$Q<F>) -> F {
                quantity.$as()
            }
        }
    };
}

/// The relation between three quantities' unit enums, for `Fixed::mul_to`
/// and `Fixed::div_to`.
macro_rules! fixed_relation {
    ($Tr:ident: $A:ident, $B:ident => $C:ident; $num:literal / $den:literal, pi^$pi:literal) => {
        impl $crate::$Tr<$B> for $A {
            type Output = $C;
            const K: (u128, u128, i32) = ($num, $den, $pi);
        }
    };
}

/// Links a quantity with its base-unit marker: `Length<I>` and
/// `Fixed<Meter, I>` are the same thing and convert freely.
macro_rules! fixed_base {
    ($Q:ident, $B:ident) => {
        impl<T> $crate::HasFixedBase for $Q<T> {
            type Base = unit::$B;
        }
        impl<I> From<$Q<I>> for $crate::Fixed<unit::$B, I> {
            #[inline(always)]
            fn from(q: $Q<I>) -> Self {
                $crate::Fixed::new(q.into_base())
            }
        }
        impl<I: Copy> From<$crate::Fixed<unit::$B, I>> for $Q<I> {
            #[inline(always)]
            fn from(f: $crate::Fixed<unit::$B, I>) -> Self {
                $Q::from_base(f.count())
            }
        }
    };
}

/// Maps a quantity type to the [`FixedUnit`] marker of its base unit.
#[doc(hidden)]
pub trait HasFixedBase {
    type Base: FixedUnit;
}

mod fixed;
pub use fixed::{
    AdditiveUnit, Decimal, Fixed, FixedUnit, Integer, Ordered, PointUnit, RoundError, Rounding,
    ToFloat, UnitDiv, UnitMul,
};

// Generated quantity modules — re-exports each quantity and its unit enum.
pub mod units;
pub use units::*;

// Generated: `unit::<quantity>::<Unit>` markers for `Fixed`.
pub mod unit;

// Trigonometry on `Angle`.
#[cfg(any(feature = "std", feature = "libm"))]
mod math;

mod info;
pub use info::{Dimensions, QuantityInfo, UnitInfo};

mod unit_expr;
pub use unit_expr::UnitExpr;

#[cfg(feature = "dynamic")]
#[cfg_attr(docsrs, doc(cfg(feature = "dynamic")))]
pub mod any;
#[cfg(feature = "dynamic")]
pub use any::{AnyQuantity, AnyUnit, KindMismatch, QuantityKind};

mod vector;
pub use vector::{Vector2, Vector3};

mod time;

mod interop;

mod format;
pub use format::{Significant, SignificantFloat};

mod angle_wrap;

#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
pub mod serde_str;

#[cfg(any(feature = "std", feature = "libm"))]
mod uncertain;
#[cfg(any(feature = "std", feature = "libm"))]
pub use uncertain::Uncertain;

// Generated cross-type arithmetic (Length / Duration = Speed, etc.).
// These are pure `impl` blocks — no named items to re-export.
mod cross_type;

// Compile and run the README's examples as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

/// Convenience alias: the UnitsNet quantity is named `Duration`; `Time` is the
/// colloquial name.
pub type Time<T = f64> = Duration<T>;
