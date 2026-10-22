//! [`Number`](crate::Number) for storage types from other crates, each behind
//! a feature of the same name: complex numbers (`num-complex`), decimals
//! (`rust_decimal`) and rationals (`num-rational`).
//!
//! Decimals and rationals get conversion constants as the shortest decimal
//! that round-trips through `f64`, which is how the UnitsNet definitions
//! write them. A foot is 0.3048 m exactly, as `Decimal` and as `381/1250`.

/// An `f64` as `mantissa × 10^exponent`, with the shortest mantissa that
/// gives back the same `f64` (what `{:e}` prints). Conversion constants are
/// always finite.
#[cfg(any(feature = "rust_decimal", feature = "num-rational"))]
fn decimal_parts(value: f64) -> (i64, i32) {
    use core::fmt::Write;

    struct Buffer {
        bytes: [u8; 32],
        len: usize,
    }
    impl Write for Buffer {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let end = self.len + s.len();
            self.bytes
                .get_mut(self.len..end)
                .ok_or(core::fmt::Error)?
                .copy_from_slice(s.as_bytes());
            self.len = end;
            Ok(())
        }
    }

    assert!(
        value.is_finite(),
        "conversion constant {value} is not finite"
    );
    let mut buffer = Buffer {
        bytes: [0; 32],
        len: 0,
    };
    // At most a sign, 17 digits, a point and `e-324`.
    write!(buffer, "{value:e}").expect("an f64 fits in 32 bytes");
    let text = &buffer.bytes[..buffer.len];
    let e = text
        .iter()
        .position(|&b| b == b'e')
        .expect("`{:e}` writes an exponent");
    let (mut mantissa, mut exponent, mut negative) = (0i64, 0i32, false);
    for &b in &text[..e] {
        match b {
            b'-' => negative = true,
            b'.' => {}
            digit => {
                mantissa = mantissa * 10 + i64::from(digit - b'0');
                exponent -= 1;
            }
        }
    }
    // The first digit is before the point.
    exponent += 1;
    let (mut power, mut power_negative) = (0i32, false);
    for &b in &text[e + 1..] {
        match b {
            b'-' => power_negative = true,
            digit => power = power * 10 + i32::from(digit - b'0'),
        }
    }
    exponent += if power_negative { -power } else { power };
    (if negative { -mantissa } else { mantissa }, exponent)
}

#[cfg(any(feature = "rust_decimal", feature = "num-rational"))]
#[cold]
fn overflow(value: f64, storage: &str) -> ! {
    panic!("the conversion constant {value} doesn't fit in {storage}")
}

#[cfg(feature = "num-complex")]
mod complex {
    use num_complex::Complex;

    macro_rules! impl_complex {
        ($($t:ty),*) => {$(
            /// The constant as a real number.
            impl crate::Number for Complex<$t> {
                #[inline]
                fn from_f64(value: f64) -> Self {
                    Complex::new(value as $t, 0.0)
                }

                // Scaling both parts by the real factor gives the same
                // values as with real storage.
                #[inline]
                fn from_ratio(numerator: f64, denominator: f64) -> Self {
                    Complex::new(<$t as crate::Number>::from_ratio(numerator, denominator), 0.0)
                }

                #[inline]
                fn mul_ratio(self, numerator: f64, denominator: f64) -> Self {
                    Complex::new(
                        <$t as crate::Number>::mul_ratio(self.re, numerator, denominator),
                        <$t as crate::Number>::mul_ratio(self.im, numerator, denominator),
                    )
                }

                #[inline]
                fn div_ratio(self, numerator: f64, denominator: f64) -> Self {
                    Complex::new(
                        <$t as crate::Number>::div_ratio(self.re, numerator, denominator),
                        <$t as crate::Number>::div_ratio(self.im, numerator, denominator),
                    )
                }
            }
        )*};
    }
    impl_complex!(f32, f64);
}

#[cfg(feature = "rust_decimal")]
mod decimal {
    use rust_decimal::Decimal;

    /// The constant exactly as it is written in decimal, except that digits
    /// past the 28th decimal place are rounded off (one atomic mass unit,
    /// 1.66 × 10⁻²⁷ kg, keeps two significant digits).
    ///
    /// # Panics
    ///
    /// For constants past `Decimal::MAX`, about 7.9 × 10²⁸, such as the
    /// solar mass in kilograms.
    impl crate::Number for Decimal {
        fn from_f64(value: f64) -> Self {
            const MAX_SCALE: u32 = 28;
            let (mantissa, exponent) = super::decimal_parts(value);
            if exponent >= 0 {
                return (0..exponent)
                    .try_fold(Decimal::from(mantissa), |d, _| d.checked_mul(Decimal::TEN))
                    .unwrap_or_else(|| super::overflow(value, "Decimal"));
            }
            let scale = exponent.unsigned_abs();
            if scale <= MAX_SCALE {
                return Decimal::new(mantissa, scale);
            }
            // Round to 28 decimal places, half away from zero.
            let Some(divisor) = 10i64.checked_pow(scale - MAX_SCALE) else {
                return Decimal::ZERO;
            };
            let (quotient, remainder) = (mantissa / divisor, mantissa % divisor);
            let rounded = if remainder.unsigned_abs() >= divisor.unsigned_abs().div_ceil(2) {
                quotient + mantissa.signum()
            } else {
                quotient
            };
            Decimal::new(rounded, MAX_SCALE)
        }
    }
}

#[cfg(feature = "num-rational")]
mod rational {
    use num_rational::Ratio;

    macro_rules! impl_ratio {
        ($($t:ty),*) => {$(
            /// The constant exactly as it is written in decimal, in lowest
            /// terms: 0.3048 becomes 381/1250.
            ///
            /// # Panics
            ///
            #[doc = concat!("When the numerator or denominator doesn't fit in `", stringify!($t), "`.")]
            /// For `i64`, constants from about 10⁻¹⁸ to 10¹⁸ with up to 18
            /// digits are fine; `Ratio<i128>` covers every built-in unit.
            impl crate::Number for Ratio<$t> {
                fn from_f64(value: f64) -> Self {
                    let (mantissa, exponent) = super::decimal_parts(value);
                    let too_large = || super::overflow(value, concat!("Ratio<", stringify!($t), ">"));
                    let mantissa = <$t>::try_from(mantissa).unwrap_or_else(|_| too_large());
                    let power = (0..exponent.unsigned_abs())
                        .try_fold(1 as $t, |p, _| p.checked_mul(10))
                        .unwrap_or_else(|| too_large());
                    if exponent >= 0 {
                        Ratio::from_integer(mantissa.checked_mul(power).unwrap_or_else(|| too_large()))
                    } else {
                        Ratio::new(mantissa, power)
                    }
                }
            }
        )*};
    }
    impl_ratio!(i32, i64, i128);
}

#[cfg(test)]
mod tests {
    #[cfg(any(feature = "rust_decimal", feature = "num-rational"))]
    #[test]
    fn decimal_parts() {
        use super::decimal_parts;
        assert_eq!(decimal_parts(0.3048), (3048, -4));
        assert_eq!(decimal_parts(1.0), (1, 0));
        assert_eq!(decimal_parts(0.0), (0, 0));
        assert_eq!(decimal_parts(-273.15), (-27315, -2));
        assert_eq!(decimal_parts(1.98947e30), (198947, 25));
        assert_eq!(decimal_parts(1.6605390666e-27), (16605390666, -37));
        assert_eq!(decimal_parts(1e-300), (1, -300));
    }
}
