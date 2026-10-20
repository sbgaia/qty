//! Formatting with significant digits, and lists of customary units for
//! `auto_unit_in`.

use core::fmt::{self, Write};

use crate::{InUnit, LengthUnit, MassUnit, Unit, VolumeUnit};

impl LengthUnit {
    /// Inch, foot, yard and mile, for [`auto_unit_in`](crate::Length::auto_unit_in).
    pub const IMPERIAL: &'static [Self] = &[Self::Inch, Self::Foot, Self::Yard, Self::Mile];
}

impl MassUnit {
    /// Ounce, pound and short ton (US customary).
    pub const US: &'static [Self] = &[Self::Ounce, Self::Pound, Self::ShortTon];
    /// Ounce, pound, stone and long ton (British imperial).
    pub const IMPERIAL: &'static [Self] = &[Self::Ounce, Self::Pound, Self::Stone, Self::LongTon];
}

impl VolumeUnit {
    /// US teaspoon, tablespoon, fluid ounce, cup, pint, quart and gallon.
    pub const US_LIQUID: &'static [Self] = &[
        Self::UsTeaspoon,
        Self::UsTablespoon,
        Self::UsOunce,
        Self::UsCustomaryCup,
        Self::UsPint,
        Self::UsQuart,
        Self::UsGallon,
    ];
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for f64 {}
    impl Sealed for f32 {}
}

/// `f32` and `f64`, which [`InUnit::significant`] can format.
pub trait SignificantFloat: sealed::Sealed + Copy {
    #[doc(hidden)]
    fn as_f64(self) -> f64;
    #[doc(hidden)]
    const MAX_DIGITS: usize;
}

impl SignificantFloat for f64 {
    fn as_f64(self) -> f64 {
        self
    }
    const MAX_DIGITS: usize = 17;
}

impl SignificantFloat for f32 {
    fn as_f64(self) -> f64 {
        f64::from(self)
    }
    const MAX_DIGITS: usize = 9;
}

impl<T: SignificantFloat, U: Unit> InUnit<T, U> {
    /// Shows the value rounded to `digits` significant digits, 1 to 17 for
    /// `f64` and 1 to 9 for `f32`. Together with
    /// [`auto_unit`](crate::Length::auto_unit) this gives engineering
    /// notation with SI prefixes.
    ///
    /// ```rust
    /// use qty::*;
    ///
    /// let f = Force::from_newtons(1234.5);
    /// assert_eq!(f.auto_unit().significant(3).to_string(), "1.23 kN");
    /// assert_eq!(f.in_unit(ForceUnit::Newton).significant(2).to_string(), "1200 N");
    /// assert_eq!(Length::from_meters(0.000_123_4).in_unit(LengthUnit::Meter).significant(2).to_string(), "0.00012 m");
    /// assert_eq!(format!("{:>9}", Length::from_meters(9.996).auto_unit().significant(3)), "   10.0 m");
    /// ```
    ///
    /// Width, fill, alignment and `+` apply to the whole text. Values under
    /// 10⁻⁶ or from 10²¹ up are written with an exponent, as in `1.2e-7 m`.
    pub fn significant(self, digits: usize) -> Significant<U> {
        Significant {
            value: self.value.as_f64(),
            digits: digits.clamp(1, T::MAX_DIGITS),
            unit: self.unit,
        }
    }
}

/// A value shown with a number of significant digits, from
/// [`InUnit::significant`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Significant<U> {
    value: f64,
    digits: usize,
    unit: U,
}

/// A stack buffer, so formatting doesn't allocate.
struct Buffer {
    bytes: [u8; 96],
    len: usize,
}

impl Buffer {
    fn new() -> Self {
        Self {
            bytes: [0; 96],
            len: 0,
        }
    }
    fn as_str(&self) -> &str {
        // Only whole `str`s are ever written, so this is valid UTF-8.
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

impl Write for Buffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len + s.len();
        if end > self.bytes.len() {
            return Err(fmt::Error);
        }
        self.bytes[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

impl<U: Unit> fmt::Display for Significant<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut text = Buffer::new();
        let v = self.value;
        if !v.is_finite() {
            write!(text, "{v}")?;
        } else {
            if v.is_sign_negative() && v != 0.0 {
                text.write_str("-")?;
            } else if f.sign_plus() {
                text.write_str("+")?;
            }
            // Let the standard library do the rounding: it is correct, and
            // `{:e}` gives the digits and the exponent.
            let mut scientific = Buffer::new();
            write!(scientific, "{:.*e}", self.digits - 1, v.abs())?;
            let (mantissa, exponent) = scientific.as_str().split_once('e').ok_or(fmt::Error)?;
            let exponent: i32 = exponent.parse().map_err(|_| fmt::Error)?;
            let mut digits = Buffer::new();
            for c in mantissa.chars().filter(char::is_ascii_digit) {
                digits.write_char(c)?;
            }
            let digits = digits.as_str();
            let n = digits.len() as i32;
            if !(-6..21).contains(&exponent) {
                write!(text, "{mantissa}e{exponent}")?;
            } else if exponent >= n - 1 {
                text.write_str(digits)?;
                for _ in 0..exponent - (n - 1) {
                    text.write_str("0")?;
                }
            } else if exponent >= 0 {
                let split = (exponent + 1) as usize;
                write!(text, "{}.{}", &digits[..split], &digits[split..])?;
            } else {
                text.write_str("0.")?;
                for _ in 0..-exponent - 1 {
                    text.write_str("0")?;
                }
                text.write_str(digits)?;
            }
        }
        let abbreviation = self.unit.abbreviation();
        let len = text.as_str().chars().count()
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
        for _ in 0..before {
            f.write_char(f.fill())?;
        }
        f.write_str(text.as_str())?;
        if !abbreviation.is_empty() {
            write!(f, " {abbreviation}")?;
        }
        for _ in 0..after {
            f.write_char(f.fill())?;
        }
        Ok(())
    }
}
