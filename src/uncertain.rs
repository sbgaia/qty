//! Values with a standard uncertainty, propagated through arithmetic,
//! conversions and math functions.

use core::cmp::Ordering;
use core::fmt;
use core::ops;

use crate::{Float, FloatMath, ToFloat};

/// A value with a standard uncertainty (one standard deviation), `value ± σ`.
///
/// `Uncertain` is a storage type like `f64`, so every quantity can hold one:
/// `Length<Uncertain>` gets all conversions, cross-quantity relations and
/// math functions, and the uncertainty follows each operation.
///
/// ```rust
/// use unitsnet_rs::*;
///
/// let distance = Length::from_meters(Uncertain::new(100.0, 0.5));
/// let time = Duration::from_seconds(Uncertain::new(9.58, 0.01));
/// let speed: Speed<Uncertain> = distance / time;
///
/// let kmh = speed.as_kilometers_per_hour();
/// assert!((kmh.value() - 37.578).abs() < 1e-3);
/// assert!((kmh.sigma() - 0.1919).abs() < 1e-4);
/// assert_eq!(format!("{:.1}", speed.in_unit(SpeedUnit::KilometerPerHour)), "37.6 ± 0.2 km/h");
///
/// // Unit factors and other constants (`Uncertain::from(2.0)`) are exact.
/// let width = Length::from_meters(Uncertain::new(20.0, 0.1));
/// let area: Area<Uncertain> = distance * width;
/// assert!((area.as_base().sigma() - 200f64.sqrt()).abs() < 1e-9); // hypot(20·0.5, 100·0.1)
/// ```
///
/// Propagation is to first order (linear), the usual GUM rule, and assumes
/// that the operands of each operation are **independent**. A value combined
/// with itself is not: `x - x` gets an uncertainty of `σ·√2` instead of zero,
/// and `x * x` gets `√2·|x|·σ` instead of `2·|x|·σ`. Write such expressions
/// in terms of independent inputs (`x * Uncertain::from(2.0)` rather than
/// `x + x`). First order is accurate when `σ` is small compared to the scale
/// on which the function curves.
///
/// Available with the `std` (default) or `libm` feature.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Uncertain<F = f64> {
    value: F,
    sigma: F,
}

impl<F: Float> Uncertain<F> {
    /// `value ± sigma`; the sign of `sigma` is ignored.
    #[inline]
    pub fn new(value: F, sigma: F) -> Self {
        Self {
            value,
            sigma: sigma.abs(),
        }
    }

    /// An exactly known value (`σ = 0`), e.g. a defined constant.
    #[inline]
    pub fn exact(value: F) -> Self {
        Self {
            value,
            sigma: F::from_f64(0.0),
        }
    }

    /// The best estimate.
    #[inline]
    pub fn value(self) -> F {
        self.value
    }

    /// The standard uncertainty (one standard deviation, `≥ 0`).
    #[inline]
    pub fn sigma(self) -> F {
        self.sigma
    }

    /// The relative uncertainty `σ / |value|` (infinite or NaN for zero).
    #[inline]
    pub fn relative(self) -> F {
        self.sigma / self.value.abs()
    }

    /// `value ± sigma·k`, e.g. `k = 2` for a 95 % interval of a normal
    /// distribution, as `(low, high)`.
    #[inline]
    pub fn interval(self, k: F) -> (F, F) {
        let half = self.sigma * k;
        (self.value - half, self.value + half)
    }
}

impl<F: Float> From<F> for Uncertain<F> {
    /// An exact value.
    #[inline]
    fn from(value: F) -> Self {
        Self::exact(value)
    }
}

/// Orders by value. Two values that are equal but have different
/// uncertainties are unordered (consistent with `==`, which compares both).
impl<F: PartialOrd> PartialOrd for Uncertain<F> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.value.partial_cmp(&other.value)? {
            Ordering::Equal if self.sigma != other.sigma => None,
            ordering => Some(ordering),
        }
    }
}

// ── Arithmetic ────────────────────────────────────────────────────────────────

impl<F: FloatMath> ops::Add for Uncertain<F> {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            value: self.value + rhs.value,
            sigma: self.sigma.hypot(rhs.sigma),
        }
    }
}

impl<F: FloatMath> ops::Sub for Uncertain<F> {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            value: self.value - rhs.value,
            sigma: self.sigma.hypot(rhs.sigma),
        }
    }
}

impl<F: FloatMath> ops::Mul for Uncertain<F> {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self {
            value: self.value * rhs.value,
            sigma: (rhs.value * self.sigma).hypot(self.value * rhs.sigma),
        }
    }
}

impl<F: FloatMath> ops::Div for Uncertain<F> {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self {
        let q = self.value / rhs.value;
        Self {
            value: q,
            sigma: (self.sigma / rhs.value).hypot(q * rhs.sigma / rhs.value),
        }
    }
}

impl<F: Float> ops::Neg for Uncertain<F> {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self {
            value: -self.value,
            sigma: self.sigma,
        }
    }
}

macro_rules! assign_op {
    ($($Trait:ident $method:ident $op:tt),*) => {$(
        impl<F: FloatMath> ops::$Trait for Uncertain<F> {
            #[inline]
            fn $method(&mut self, rhs: Self) {
                *self = *self $op rhs;
            }
        }
    )*};
}
assign_op!(AddAssign add_assign +, SubAssign sub_assign -, MulAssign mul_assign *, DivAssign div_assign /);

impl<F: FloatMath> core::iter::Sum for Uncertain<F> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::exact(F::from_f64(0.0)), |acc, x| acc + x)
    }
}

impl<'a, F: FloatMath> core::iter::Sum<&'a Uncertain<F>> for Uncertain<F> {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.copied().sum()
    }
}

// ── Storage traits ────────────────────────────────────────────────────────────

impl<F: Float> ToFloat for Uncertain<F> {
    /// The value, without its uncertainty.
    #[inline]
    fn to_f64(self) -> f64 {
        self.value.to_f64()
    }
    /// The value, without its uncertainty.
    #[inline]
    fn to_f32(self) -> f32 {
        self.value.to_f32()
    }
}

impl<F: FloatMath> Float for Uncertain<F> {
    #[inline]
    fn from_f64(value: f64) -> Self {
        Self::exact(F::from_f64(value))
    }
    #[inline]
    fn abs(self) -> Self {
        Self {
            value: self.value.abs(),
            sigma: self.sigma,
        }
    }
    /// The operand with the larger value (ignoring NaN), with its uncertainty.
    #[inline]
    fn max(self, other: Self) -> Self {
        #[allow(clippy::eq_op)]
        let self_is_nan = self.value != self.value;
        if self_is_nan || other.value > self.value {
            other
        } else {
            self
        }
    }
    /// The operand with the smaller value (ignoring NaN), with its uncertainty.
    #[inline]
    fn min(self, other: Self) -> Self {
        #[allow(clippy::eq_op)]
        let self_is_nan = self.value != self.value;
        if self_is_nan || other.value < self.value {
            other
        } else {
            self
        }
    }
    /// By value, then by uncertainty.
    #[inline]
    fn total_cmp(&self, other: &Self) -> Ordering {
        self.value
            .total_cmp(&other.value)
            .then_with(|| self.sigma.total_cmp(&other.sigma))
    }
    #[inline]
    fn is_finite(self) -> bool {
        self.value.is_finite() && self.sigma.is_finite()
    }
}

/// `f(x ± σ) = f(x) ± |f′(x)|·σ`.
impl<F: FloatMath> FloatMath for Uncertain<F> {
    fn sqrt(self) -> Self {
        let r = self.value.sqrt();
        self.through(r, F::from_f64(0.5) / r)
    }
    fn cbrt(self) -> Self {
        let r = self.value.cbrt();
        self.through(r, F::from_f64(1.0 / 3.0) / (r * r))
    }
    fn sin(self) -> Self {
        self.through(self.value.sin(), self.value.cos())
    }
    fn cos(self) -> Self {
        self.through(self.value.cos(), self.value.sin())
    }
    fn tan(self) -> Self {
        let t = self.value.tan();
        self.through(t, F::from_f64(1.0) + t * t)
    }
    fn asin(self) -> Self {
        let d = F::from_f64(1.0) / (F::from_f64(1.0) - self.value * self.value).sqrt();
        self.through(self.value.asin(), d)
    }
    fn acos(self) -> Self {
        let d = F::from_f64(1.0) / (F::from_f64(1.0) - self.value * self.value).sqrt();
        self.through(self.value.acos(), d)
    }
    fn atan(self) -> Self {
        let d = F::from_f64(1.0) / (F::from_f64(1.0) + self.value * self.value);
        self.through(self.value.atan(), d)
    }
    /// `atan2(y = self, x = other)`.
    fn atan2(self, other: Self) -> Self {
        let (y, x) = (self.value, other.value);
        let r2 = x * x + y * y;
        Self {
            value: y.atan2(x),
            sigma: (x * self.sigma).hypot(y * other.sigma) / r2,
        }
    }
    fn hypot(self, other: Self) -> Self {
        let h = self.value.hypot(other.value);
        Self {
            value: h,
            sigma: (self.value * self.sigma).hypot(other.value * other.sigma) / h,
        }
    }
}

impl<F: Float> Uncertain<F> {
    /// The result `value` of a function whose derivative at `self` is `slope`.
    #[inline]
    fn through(self, value: F, slope: F) -> Self {
        Self {
            value,
            sigma: (slope * self.sigma).abs(),
        }
    }
}

// ── Formatting ────────────────────────────────────────────────────────────────

macro_rules! uncertain_fmt {
    ($($Trait:ident),*) => {$(
        /// `value ± sigma`, applying the format flags (such as precision) to both.
        impl<F: fmt::$Trait> fmt::$Trait for Uncertain<F> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::$Trait::fmt(&self.value, f)?;
                f.write_str(" ± ")?;
                fmt::$Trait::fmt(&self.sigma, f)
            }
        }
    )*};
}
uncertain_fmt!(Display, LowerExp, UpperExp);
