//! Trigonometry on [`Angle`], which is stored in radians.

use crate::{Angle, FloatMath, Quantity};

/// Trigonometric functions. Available with the `std` (default) or `libm`
/// feature.
///
/// ```rust
/// use unitsnet_rs::*;
///
/// let a = Angle::from_degrees(30.0);
/// assert!((a.sin() - 0.5).abs() < 1e-15);
///
/// // Inverse functions return an Angle.
/// let b = Angle::asin(0.5);
/// assert!(b.approx_eq_rel(a, 1e-15));
///
/// // The direction of a 2D displacement, from any pair of same-kind quantities.
/// let heading = Angle::atan2(Length::from_meters(1.0), Length::from_meters(1.0));
/// assert!((heading.as_degrees() - 45.0).abs() < 1e-12);
/// ```
impl<T: FloatMath> Angle<T> {
    /// Sine.
    #[inline]
    pub fn sin(self) -> T {
        self.into_base().sin()
    }

    /// Cosine.
    #[inline]
    pub fn cos(self) -> T {
        self.into_base().cos()
    }

    /// Tangent.
    #[inline]
    pub fn tan(self) -> T {
        self.into_base().tan()
    }

    /// Sine and cosine.
    #[inline]
    pub fn sin_cos(self) -> (T, T) {
        let r = self.into_base();
        (r.sin(), r.cos())
    }

    /// The angle whose sine is `value` (in −90°…90°).
    #[inline]
    pub fn asin(value: T) -> Self {
        Self::from_base(value.asin())
    }

    /// The angle whose cosine is `value` (in 0°…180°).
    #[inline]
    pub fn acos(value: T) -> Self {
        Self::from_base(value.acos())
    }

    /// The angle whose tangent is `value` (in −90°…90°).
    #[inline]
    pub fn atan(value: T) -> Self {
        Self::from_base(value.atan())
    }

    /// The angle of the vector `(x, y)` from the x axis (in −180°…180°),
    /// for any two values of the same quantity: lengths, speeds, forces, …
    #[inline]
    pub fn atan2<Q: Quantity<Value = T>>(y: Q, x: Q) -> Self {
        Self::from_base(y.as_base().atan2(x.as_base()))
    }
}
