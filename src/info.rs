//! Names, abbreviations and physical dimensions of quantities and units.

use core::fmt;
use core::ops::{Div, Mul};

/// Exponents of the seven SI base dimensions: length L, mass M, time T,
/// electric current I, temperature Θ, amount of substance N and luminous
/// intensity J. A speed is `L·T⁻¹`. Ratios and angles have every exponent
/// at zero.
///
/// ```rust
/// use qty::*;
///
/// let speed = Speed::<f64>::INFO.dimensions;
/// assert_eq!(speed, Length::<f64>::INFO.dimensions / Duration::<f64>::INFO.dimensions);
/// assert_eq!(speed.to_string(), "L·T⁻¹");
/// assert_eq!(Force::<f64>::INFO.dimensions, Dimensions::new(1, 1, -2, 0, 0, 0, 0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Dimensions {
    /// Length (L).
    pub length: i8,
    /// Mass (M).
    pub mass: i8,
    /// Time (T).
    pub time: i8,
    /// Electric current (I).
    pub electric_current: i8,
    /// Thermodynamic temperature (Θ).
    pub temperature: i8,
    /// Amount of substance (N).
    pub amount_of_substance: i8,
    /// Luminous intensity (J).
    pub luminous_intensity: i8,
}

impl Dimensions {
    /// The dimensions of a pure number.
    pub const DIMENSIONLESS: Self = Self::new(0, 0, 0, 0, 0, 0, 0);

    /// From the exponents of L, M, T, I, Θ, N and J, in that order.
    pub const fn new(l: i8, m: i8, t: i8, i: i8, theta: i8, n: i8, j: i8) -> Self {
        Self {
            length: l,
            mass: m,
            time: t,
            electric_current: i,
            temperature: theta,
            amount_of_substance: n,
            luminous_intensity: j,
        }
    }

    /// The exponents in the order L, M, T, I, Θ, N, J.
    pub const fn to_array(self) -> [i8; 7] {
        [
            self.length,
            self.mass,
            self.time,
            self.electric_current,
            self.temperature,
            self.amount_of_substance,
            self.luminous_intensity,
        ]
    }

    const fn from_array(a: [i8; 7]) -> Self {
        Self::new(a[0], a[1], a[2], a[3], a[4], a[5], a[6])
    }

    /// `true` when every exponent is zero.
    pub const fn is_dimensionless(self) -> bool {
        let a = self.to_array();
        let mut i = 0;
        while i < 7 {
            if a[i] != 0 {
                return false;
            }
            i += 1;
        }
        true
    }

    /// The dimensions of a product. Exponents add.
    pub const fn mul(self, other: Self) -> Self {
        self.combine(other, 1)
    }

    /// The dimensions of a quotient. Exponents subtract.
    pub const fn div(self, other: Self) -> Self {
        self.combine(other, -1)
    }

    /// The dimensions of `self` raised to the power `n`.
    pub const fn powi(self, n: i8) -> Self {
        let mut a = self.to_array();
        let mut i = 0;
        while i < 7 {
            a[i] *= n;
            i += 1;
        }
        Self::from_array(a)
    }

    const fn combine(self, other: Self, sign: i8) -> Self {
        let (mut a, b) = (self.to_array(), other.to_array());
        let mut i = 0;
        while i < 7 {
            a[i] += sign * b[i];
            i += 1;
        }
        Self::from_array(a)
    }
}

impl Mul for Dimensions {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Dimensions::mul(self, rhs)
    }
}

impl Div for Dimensions {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        Dimensions::div(self, rhs)
    }
}

/// `L·M·T⁻²`, or `1` when dimensionless.
impl fmt::Display for Dimensions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const SYMBOLS: [&str; 7] = ["L", "M", "T", "I", "Θ", "N", "J"];
        const SUPERSCRIPTS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
        let mut first = true;
        for (symbol, exponent) in SYMBOLS.iter().zip(self.to_array()) {
            if exponent == 0 {
                continue;
            }
            if !first {
                f.write_str("·")?;
            }
            first = false;
            f.write_str(symbol)?;
            if exponent != 1 {
                if exponent < 0 {
                    f.write_str("⁻")?;
                }
                let e = exponent.unsigned_abs();
                if e >= 10 {
                    write!(f, "{}", SUPERSCRIPTS[usize::from(e / 10)])?;
                }
                write!(f, "{}", SUPERSCRIPTS[usize::from(e % 10)])?;
            }
        }
        if first {
            f.write_str("1")?;
        }
        Ok(())
    }
}

/// Names and abbreviations of a unit, as returned by
/// [`LengthUnit::info`](crate::LengthUnit::info) and its siblings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnitInfo {
    /// The name, spelled like the enum variant: `"KilometerPerHour"`.
    pub singular: &'static str,
    /// The plural name: `"KilometersPerHour"`.
    pub plural: &'static str,
    /// The abbreviation used for display: `"km/h"`.
    pub abbreviation: &'static str,
    /// Every abbreviation accepted when parsing.
    pub abbreviations: &'static [&'static str],
}

/// Name, description, dimensions and units of a quantity, from
/// [`Quantity::INFO`](crate::Quantity::INFO).
///
/// ```rust
/// use qty::*;
///
/// let info = Length::<f64>::INFO;
/// assert_eq!(info.name, "Length");
/// assert_eq!(info.base_unit.abbreviation, "m");
/// assert!(info.units.iter().any(|u| u.abbreviation == "km"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QuantityInfo {
    /// The name, spelled like the type: `"Length"`.
    pub name: &'static str,
    /// UnitsNet's description of the quantity.
    pub description: &'static str,
    /// The physical dimensions.
    pub dimensions: Dimensions,
    /// The unit that values are stored in.
    pub base_unit: &'static UnitInfo,
    /// Every unit, in the order of the unit enum's `ALL`.
    pub units: &'static [UnitInfo],
}
