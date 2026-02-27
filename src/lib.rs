//! # unitsnet-rs
//!
//! Strongly-typed physical quantities and unit conversions, generated from
//! [UnitsNet](https://github.com/angularsen/UnitsNet) definition files.
//!
//! ## Quick start
//!
//! ```rust
//! use unitsnet_rs::*;
//!
//! let dist = Length::from_kilometers(10.0);
//! let time = Duration::from_hours(0.5);
//! let speed: Speed = dist / time;
//!
//! // Exact comparison operators are available via the derived PartialOrd/PartialEq.
//! // For floating-point results, prefer the approximate comparison methods from
//! // the ApproxEq trait, which every quantity type inherits automatically.
//! #[cfg(not(feature = "f32"))]
//! assert!(speed.approx_eq_rel(Speed::from_kilometers_per_hour(20.0), 1e-9));
//!
//! let temp = Temperature::from_degrees_celsius(100.0);
//! #[cfg(not(feature = "f32"))]
//! assert!(temp.approx_eq_rel(Temperature::from_degrees_fahrenheit(212.0), 1e-9));
//! ```

/// The underlying numeric type used by every quantity struct.
///
/// Defaults to `f64` (~15 significant digits). Enable the `f32` Cargo feature
/// to switch to `f32` (~7 significant digits) for lower memory usage.
#[cfg(feature = "f32")]
pub type Num = f32;
#[cfg(not(feature = "f32"))]
pub type Num = f64;

/// Core trait implemented by every generated quantity struct.
///
/// Exposes the raw base-unit value so that [`ApproxEq`] can be implemented
/// once for all quantity types via a blanket `impl<T: Quantity> ApproxEq for T`.
pub trait Quantity: Copy + Sized {
    /// Returns the raw value stored in the SI base unit.
    fn as_base(self) -> Num;

    /// Constructs a quantity from a raw base-unit value.
    fn from_base(value: Num) -> Self;
}

/// Approximate floating-point comparison for any [`Quantity`].
///
/// Every generated quantity type implements this trait automatically through the
/// blanket `impl<T: Quantity> ApproxEq for T` — the comparison logic is written
/// exactly once here, not once per type.
///
/// # Choosing a method
///
/// | Method | Formula | Use when |
/// |---|---|---|
/// | [`approx_eq_abs`][ApproxEq::approx_eq_abs] | `\|a−b\| ≤ ε` | magnitude is known in advance |
/// | [`approx_eq_rel`][ApproxEq::approx_eq_rel] | `\|a−b\| ≤ tol·max(\|a\|,\|b\|)` | magnitude varies; relative accuracy matters |
/// | [`approx_eq`][ApproxEq::approx_eq] | `\|a−b\| ≤ ε + tol·max(\|a\|,\|b\|)` | mixed: safe near zero **and** for large values |
///
/// # Examples
///
/// ```rust
/// use unitsnet_rs::*;
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
pub trait ApproxEq: Sized {
    /// Returns `true` if `|self − other| ≤ epsilon`.
    ///
    /// # Arguments
    ///
    /// * `other` - The value to compare against.
    /// * `epsilon` - Maximum allowed absolute difference (in base-unit terms).
    fn approx_eq_abs(self, other: Self, epsilon: Num) -> bool;

    /// Returns `true` if `|self − other| ≤ rel_tol × max(|self|, |other|)`.
    ///
    /// Returns `true` when both values are exactly zero.
    ///
    /// # Arguments
    ///
    /// * `other` - The value to compare against.
    /// * `rel_tol` - Maximum allowed relative difference. Typical values:
    ///   `1e-9` for `f64`, `1e-5` for `f32`.
    fn approx_eq_rel(self, other: Self, rel_tol: Num) -> bool;

    /// Returns `true` if `|self − other| ≤ epsilon + rel_tol × max(|self|, |other|)`.
    ///
    /// Combines absolute and relative tolerances, matching the semantics of
    /// NumPy's [`numpy.isclose`](https://numpy.org/doc/stable/reference/generated/numpy.isclose.html).
    /// Prefer [`approx_eq_abs`][ApproxEq::approx_eq_abs] or
    /// [`approx_eq_rel`][ApproxEq::approx_eq_rel] when only one form is needed.
    ///
    /// # Arguments
    ///
    /// * `other` - The value to compare against.
    /// * `epsilon` - Absolute tolerance floor (keeps the test meaningful near zero).
    /// * `rel_tol` - Relative tolerance for large magnitudes.
    fn approx_eq(self, other: Self, epsilon: Num, rel_tol: Num) -> bool;
}

impl<T: Quantity> ApproxEq for T {
    #[inline]
    fn approx_eq_abs(self, other: Self, epsilon: Num) -> bool {
        (self.as_base() - other.as_base()).abs() <= epsilon
    }

    #[inline]
    fn approx_eq_rel(self, other: Self, rel_tol: Num) -> bool {
        let a = self.as_base();
        let b = other.as_base();
        let max = a.abs().max(b.abs());
        // Both zero → trivially equal regardless of rel_tol.
        max == 0.0 || (a - b).abs() <= rel_tol * max
    }

    #[inline]
    fn approx_eq(self, other: Self, epsilon: Num, rel_tol: Num) -> bool {
        let diff = (self.as_base() - other.as_base()).abs();
        let max = self.as_base().abs().max(other.as_base().abs());
        diff <= epsilon + rel_tol * max
    }
}

// Generated quantity modules — re-exports every public item from each module.
pub mod units;
pub use units::*;

// Generated cross-type arithmetic (Length / Duration = Speed, etc.).
// These are pure `impl` blocks — no named items to re-export.
pub mod cross_type;

// Convenience alias: the JSON quantity is named "Duration"; `Time` is the colloquial name.
pub use units::duration::Duration as Time;
