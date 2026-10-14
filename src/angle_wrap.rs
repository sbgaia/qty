//! Wrapping angles into one turn.

use crate::{Angle, Float};

/// Wrapping, for float storage types with `%` (`f32`, `f64`).
impl<T: Float + core::ops::Rem<Output = T>> Angle<T> {
    /// The same direction as an angle in `[0°, 360°)`.
    ///
    /// ```rust
    /// use unitsnet_rs::*;
    ///
    /// assert!((Angle::from_degrees(-90.0).wrap_unsigned().as_degrees() - 270.0).abs() < 1e-12);
    /// assert!((Angle::from_degrees(725.0).wrap_unsigned().as_degrees() - 5.0).abs() < 1e-12);
    /// ```
    ///
    /// NaN and infinities give NaN.
    #[inline]
    pub fn wrap_unsigned(self) -> Self {
        let (zero, tau) = (T::from_f64(0.0), T::from_f64(core::f64::consts::TAU));
        let r = self.into_base() % tau;
        let r = if r < zero { r + tau } else { r };
        // A tiny negative remainder plus TAU can round up to TAU.
        Self::from_base(if r >= tau { zero } else { r })
    }

    /// The same direction as an angle in `(−180°, 180°]`, e.g. for the
    /// difference between two headings.
    ///
    /// ```rust
    /// use unitsnet_rs::*;
    ///
    /// let turn = Angle::from_degrees(10.0) - Angle::from_degrees(350.0);
    /// assert!((turn.wrap_signed().as_degrees() - 20.0).abs() < 1e-12);
    /// ```
    ///
    /// NaN and infinities give NaN.
    #[inline]
    pub fn wrap_signed(self) -> Self {
        let r = self.wrap_unsigned().into_base();
        if r > T::from_f64(core::f64::consts::PI) {
            Self::from_base(r - T::from_f64(core::f64::consts::TAU))
        } else {
            Self::from_base(r)
        }
    }
}
