//! The exact size of a unit, for `qty!` and unit expressions.
//!
//! Compiled into the code generator too, with `UnitExpr`, so it takes `conv`
//! from its parent module.

use super::conv;

/// A unit's size in coherent SI units: an exact fraction times a power of
/// π where it has one, so that a literal is rounded like the matching
/// `from_` method.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Factor {
    /// The size rounded to the nearest `f64`.
    value: f64,
    numerator: u128,
    /// Zero when the fraction is too large to keep; `value` is then
    /// computed from the factors the size is made of.
    denominator: u128,
    pi: i8,
}

/// How the generated `from_` method converts a value with a [`Factor`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Conversion {
    /// `x · factor`, where the factor is exact in binary, or has π in it,
    /// or its fraction is too large for `mul_ratio`.
    Mul(f64),
    /// `x / divisor`, where the inverse of the factor is exact in binary
    /// but too large for `div_ratio`.
    Div(f64),
    /// `Number::mul_ratio(x, numerator, denominator)`.
    MulRatio(f64, f64),
    /// `Number::div_ratio(x, numerator, denominator)`, where the inverse of
    /// the factor, `numerator / denominator`, is exact in binary.
    DivRatio(f64, f64),
}

impl Conversion {
    /// Which conversion: 0 to 3, in the order above.
    pub const fn kind(self) -> u8 {
        match self {
            Self::Mul(_) => 0,
            Self::Div(_) => 1,
            Self::MulRatio(..) => 2,
            Self::DivRatio(..) => 3,
        }
    }

    /// The constants, with 0 for the one `Mul` and `Div` don't have.
    pub const fn constants(self) -> (f64, f64) {
        match self {
            Self::Mul(a) | Self::Div(a) => (a, 0.0),
            Self::MulRatio(a, b) | Self::DivRatio(a, b) => (a, b),
        }
    }

    /// The constants with the kind in the second one's sign: `(factor, 0)`,
    /// `(divisor, -0)`, `(numerator, denominator)` and `(numerator,
    /// -denominator)`. Every constant is positive.
    pub const fn encoded(self) -> (f64, f64) {
        match self {
            Self::Mul(a) => (a, 0.0),
            Self::Div(a) => (a, -0.0),
            Self::MulRatio(a, b) => (a, b),
            Self::DivRatio(a, b) => (a, -b),
        }
    }
}

impl Factor {
    /// 1.
    pub const ONE: Self = Self::exact(1.0, 1, 1, 0);

    /// Fractions are kept below this, which leaves `conv::ratio` a bit.
    const LIMIT: u128 = 1 << 127;

    /// A size that isn't kept as a fraction.
    pub const fn approx(value: f64) -> Self {
        Self {
            value,
            numerator: 0,
            denominator: 0,
            pi: 0,
        }
    }

    /// `numerator / denominator · π^pi`, already reduced, with `value` its
    /// nearest `f64`.
    pub const fn exact(value: f64, numerator: u128, denominator: u128, pi: i8) -> Self {
        Self {
            value,
            numerator,
            denominator,
            pi,
        }
    }

    /// `numerator / denominator · π^pi`, with a fraction below 2^127.
    pub(crate) const fn new(numerator: u128, denominator: u128, pi: i8) -> Self {
        let gcd = gcd(numerator, denominator);
        Self::reduced(numerator / gcd, denominator / gcd, pi)
    }

    /// `numerator / denominator · π^pi`, with a reduced fraction below
    /// 2^127.
    const fn reduced(numerator: u128, denominator: u128, pi: i8) -> Self {
        let value = conv::ratio(numerator, denominator);
        let value = if pi == 0 {
            value
        } else if numerator <= 1 << 53 && denominator <= 1 << 53 {
            // The fraction and π as pairs of floats, 106 bits each, so that
            // their product rounds like the exact value.
            let (n, d) = (numerator as f64, denominator as f64);
            let rest = conv::fma(-value, d, n) / d;
            times_pi((value, rest), pi)
        } else {
            times_pi((value, 0.0), pi)
        };
        Self::exact(value, numerator, denominator, pi)
    }

    /// The size rounded to the nearest `f64`, or, without a fraction, as
    /// computed from the factors it is made of.
    pub const fn value(self) -> f64 {
        self.value
    }

    /// `(numerator, denominator, power of π)`, if the size is kept as a
    /// fraction.
    pub const fn parts(self) -> Option<(u128, u128, i8)> {
        if self.denominator == 0 {
            None
        } else {
            Some((self.numerator, self.denominator, self.pi))
        }
    }

    /// The product.
    pub const fn mul(self, other: Self) -> Self {
        // Coherent units need no arithmetic, which is slow at compile time.
        if self.is_one() {
            return other;
        }
        if other.is_one() {
            return self;
        }
        if let (Some((a, b, p)), Some((c, d, q))) = (self.parts(), other.parts()) {
            // `a/b · c/d` with `a/b` and `c/d` reduced; the result is too.
            let (g, h) = (gcd(a, d), gcd(c, b));
            if let (Some(numerator), Some(denominator), Some(pi)) = (
                (a / g).checked_mul(c / h),
                (b / h).checked_mul(d / g),
                p.checked_add(q),
            ) {
                if numerator < Self::LIMIT && denominator < Self::LIMIT {
                    return Self::reduced(numerator, denominator, pi);
                }
            }
        }
        Self::approx(self.value * other.value)
    }

    /// `1 / self`.
    pub const fn recip(self) -> Self {
        if self.is_one() {
            return self;
        }
        match self.parts() {
            Some((numerator, denominator, pi)) if pi != i8::MIN => {
                Self::reduced(denominator, numerator, -pi)
            }
            _ => Self::approx(1.0 / self.value),
        }
    }

    const fn is_one(self) -> bool {
        self.numerator == 1 && self.denominator == 1 && self.pi == 0
    }

    /// `self` to the power `n`.
    pub const fn powi(self, n: i8) -> Self {
        let mut power = Self::ONE;
        let mut i = 0;
        while i < n.unsigned_abs() {
            power = power.mul(self);
            i += 1;
        }
        if n < 0 {
            power.recip()
        } else {
            power
        }
    }

    /// The conversion the generator emits for this factor: one
    /// multiplication when the factor or its inverse is exact in binary, the
    /// fraction where it fits, and the rounded value for the rest (π, large
    /// fractions).
    pub const fn conversion(self) -> Conversion {
        const TWO_53: u128 = 1 << 53;
        let Some((numerator, denominator, 0)) = self.parts() else {
            return Conversion::Mul(self.value);
        };
        let fits = numerator <= TWO_53 && denominator <= TWO_53;
        if is_exact(numerator, denominator) {
            Conversion::Mul(self.value)
        } else if is_exact(denominator, numerator) {
            if fits {
                Conversion::DivRatio(denominator as f64, numerator as f64)
            } else {
                // Exact: `denominator` is an `f64` and `numerator` a power
                // of two.
                Conversion::Div(denominator as f64 / numerator as f64)
            }
        } else if fits {
            Conversion::MulRatio(numerator as f64, denominator as f64)
        } else {
            Conversion::Mul(self.value)
        }
    }
}

/// `(hi + lo) · π^pi`, rounded, in double-float arithmetic.
const fn times_pi(mut x: (f64, f64), pi: i8) -> f64 {
    // π and 1/π, each as the nearest `f64` and the rest.
    const PI: (f64, f64) = (core::f64::consts::PI, 1.224_646_799_147_353_2e-16);
    const FRAC_1_PI: (f64, f64) = (core::f64::consts::FRAC_1_PI, -1.967_867_667_518_248_6e-17);
    let factor = if pi > 0 { PI } else { FRAC_1_PI };
    let mut i = 0;
    while i < pi.unsigned_abs() {
        let product = x.0 * factor.0;
        let error = conv::fma(x.0, factor.0, -product) + (x.0 * factor.1 + x.1 * factor.0);
        let hi = product + error;
        x = (hi, error - (hi - product));
        i += 1;
    }
    x.0 + x.1
}

/// Whether the reduced fraction `numerator / denominator` is an `f64`.
const fn is_exact(numerator: u128, denominator: u128) -> bool {
    denominator.is_power_of_two() && numerator >> numerator.trailing_zeros() <= 1 << 53
}

const fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}
