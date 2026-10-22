//! Multiplying by a conversion constant `numerator / denominator` with one
//! rounding, for `f32` and `f64` storage and for `const_new`.
//!
//! Most constants have no exact binary value (1/0.3048 = 1250/381), so a
//! multiplication by the rounded constant rounds twice. With a fused
//! multiply-add the constant is used as two parts, `hi + lo`, about 106 bits
//! together, and `x·hi + x·lo` rounds once: the result is the nearest float
//! to the exact product. That takes a multiplication and an FMA instead of a
//! division, which is several times slower.
//!
//! The FMA must be an instruction: as a library call it is slower than the
//! division. So the two-part form is used where the target guarantees one
//! (every aarch64 target, x86-64 with `-C target-cpu=haswell`,
//! `x86-64-v3`, `native` and so on) and `std` provides
//! [`f64::mul_add`]. Other builds multiply or divide by the rounded
//! constant, as before.
//!
//! `const_new` can't call `mul_add`, so it uses [`fma`], which computes the
//! same correctly rounded value with integer arithmetic.

/// Whether conversions use the two-part constants.
pub(crate) const TWO_PART: bool = cfg!(all(
    feature = "std",
    any(target_feature = "fma", target_arch = "aarch64")
));

/// `x · numerator / denominator` for `f64`: what `f64::mul_ratio` gives,
/// usable in `const fn`s.
#[inline]
pub(crate) const fn mul_ratio(x: f64, numerator: f64, denominator: f64) -> f64 {
    if TWO_PART {
        let hi = numerator / denominator;
        let lo = fma(-hi, denominator, numerator) / denominator;
        fma(x, hi, x * lo)
    } else {
        x * (numerator / denominator)
    }
}

/// `x / (numerator / denominator)` for `f64`: what `f64::div_ratio` gives,
/// usable in `const fn`s.
#[inline]
pub(crate) const fn div_ratio(x: f64, numerator: f64, denominator: f64) -> f64 {
    if TWO_PART {
        mul_ratio(x, denominator, numerator)
    } else {
        x / (numerator / denominator)
    }
}

/// `a · b + c` with a single rounding, like [`f64::mul_add`], for finite
/// arguments.
///
/// `f64::to_bits` isn't `const` before Rust 1.83, so the arguments are
/// taken apart by scaling with powers of two, which is exact.
pub(crate) const fn fma(a: f64, b: f64, c: f64) -> f64 {
    let product = a * b;
    // A zero term, or an infinite or NaN result: one rounding either way.
    if a == 0.0 || b == 0.0 || c == 0.0 || !is_finite(product) || !is_finite(c) {
        return product + c;
    }
    let (a_negative, a_mantissa, a_exponent) = parts(a);
    let (b_negative, b_mantissa, b_exponent) = parts(b);
    let (c_negative, c_mantissa, c_exponent) = parts(c);

    // The exact product has at most 106 bits. Both terms are shifted so
    // that their top bit is bit 125, which leaves room for a carry and
    // keeps every bit of either term.
    let (p, p_exponent) = align(
        a_mantissa as u128 * b_mantissa as u128,
        a_exponent + b_exponent,
    );
    let p_negative = a_negative != b_negative;
    let (q, q_exponent) = align(c_mantissa as u128, c_exponent);

    let (big, exponent, negative, small, small_exponent, small_negative) =
        if p_exponent > q_exponent || (p_exponent == q_exponent && p >= q) {
            (p, p_exponent, p_negative, q, q_exponent, c_negative)
        } else {
            (q, q_exponent, c_negative, p, p_exponent, p_negative)
        };
    // Bits of the smaller term shifted out are kept as a sticky bit. They
    // are only lost when the terms are at least 20 bits apart, and then the
    // sum keeps its top bit at 124 or above, far from bit 0.
    let shift = (exponent - small_exponent) as u32;
    let small = if shift >= 128 {
        (small != 0) as u128
    } else if shift == 0 {
        small
    } else {
        (small >> shift) | ((small & ((1 << shift) - 1) != 0) as u128)
    };
    let sum = if negative == small_negative {
        big + small
    } else {
        big - small
    };
    if sum == 0 {
        return 0.0;
    }
    round(sum, exponent, negative)
}

/// `f64::is_finite`, which isn't `const` before Rust 1.83. False for NaN.
const fn is_finite(x: f64) -> bool {
    x >= -f64::MAX && x <= f64::MAX
}

/// `x` as `(negative, mantissa, exponent)` with `|x| = mantissa · 2^exponent`
/// and `mantissa < 2^53`; `x` is finite and not zero.
const fn parts(x: f64) -> (bool, u64, i32) {
    const TWO_52: f64 = 4_503_599_627_370_496.0;
    const TWO_53: f64 = 9_007_199_254_740_992.0;
    const TWO_64: f64 = 18_446_744_073_709_551_616.0;
    let negative = x < 0.0;
    let mut y = if negative { -x } else { x };
    let mut exponent = 0;
    while y >= TWO_53 * TWO_64 {
        y /= TWO_64;
        exponent += 64;
    }
    while y >= TWO_53 {
        y /= 2.0;
        exponent += 1;
    }
    while y * TWO_64 < TWO_52 && exponent - 64 >= -1074 {
        y *= TWO_64;
        exponent -= 64;
    }
    while y < TWO_52 && exponent > -1074 {
        y *= 2.0;
        exponent -= 1;
    }
    (negative, y as u64, exponent)
}

/// `(m, e)` scaled so that the top bit of `m` is bit 125.
const fn align(m: u128, e: i32) -> (u128, i32) {
    let shift = m.leading_zeros() as i32 - 2;
    (m << shift, e - shift)
}

/// `±m · 2^e` rounded to the nearest `f64`, ties to even.
const fn round(m: u128, e: i32, negative: bool) -> f64 {
    let top = 127 - m.leading_zeros() as i32;
    let mut shift = top - 52;
    // Below the normal range, fewer bits are kept.
    if e + shift < -1074 {
        shift = -1074 - e;
    }
    let mantissa = if shift <= 0 {
        m << -shift
    } else if shift >= 128 {
        0
    } else {
        let kept = m >> shift;
        let rest = m & ((1 << shift) - 1);
        let half = 1 << (shift - 1);
        if rest > half || (rest == half && kept & 1 == 1) {
            kept + 1
        } else {
            kept
        }
    };
    // The mantissa has at most 54 bits (2^53 after rounding up), so it
    // converts exactly, and so does the scaling unless it overflows.
    let value = scale(mantissa as f64, e + shift);
    if negative {
        -value
    } else {
        value
    }
}

/// `x · 2^e`, exact when the result is a normal or subnormal `f64` that
/// holds all of `x`'s bits.
const fn scale(mut x: f64, mut e: i32) -> f64 {
    const TWO_64: f64 = 18_446_744_073_709_551_616.0;
    while e >= 64 {
        x *= TWO_64;
        e -= 64;
    }
    while e > 0 {
        x *= 2.0;
        e -= 1;
    }
    // Each step's result is larger than the final one, which holds all of
    // `x`'s bits, so no step rounds.
    while e <= -64 {
        x /= TWO_64;
        e += 64;
    }
    while e < 0 {
        x /= 2.0;
        e += 1;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small xorshift generator, so the test needs no dependency.
    #[cfg(feature = "std")]
    struct Rng(u64);
    #[cfg(feature = "std")]
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        /// Any finite `f64` with an exponent in `-range..range`.
        fn float(&mut self, range: i32) -> f64 {
            let mantissa = self.next() >> 12;
            let exponent = (self.next() % (2 * range as u64)) as i32 - range;
            let sign = (self.next() & 1) << 63;
            f64::from_bits(sign | (((exponent + 1023) as u64) << 52) | mantissa)
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn fma_matches_mul_add() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for i in 0..1_000_000 {
            // Mostly nearby exponents, where the terms cancel or overlap,
            // and some far apart.
            let range = if i % 4 == 0 { 600 } else { 30 };
            let (a, b, c) = (
                rng.float(range),
                rng.float(range),
                rng.float((2 * range).min(1000)),
            );
            let expected = a.mul_add(b, c);
            if !expected.is_finite() {
                continue;
            }
            assert_eq!(
                fma(a, b, c).to_bits(),
                expected.to_bits(),
                "{a:e} * {b:e} + {c:e}"
            );
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn fma_cancels_exactly() {
        // The remainder of a division, the way the low part is computed.
        for (n, d) in [(381.0, 1250.0), (5.0, 18.0), (1200.0, 3937.0), (1.0, 3.0)] {
            let hi: f64 = n / d;
            assert_eq!(fma(-hi, d, n), (-hi).mul_add(d, n));
        }
        // Subnormal results.
        let tiny = f64::MIN_POSITIVE;
        assert_eq!(fma(tiny, 0.5, tiny / 4.0), tiny.mul_add(0.5, tiny / 4.0));
        assert_eq!(fma(tiny, -1.0, tiny), 0.0);
    }

    #[test]
    fn ratios_are_correctly_rounded_with_two_parts() {
        if !TWO_PART {
            return;
        }
        // 12 ft is 3.6576 m exactly as decimals; a single rounded factor
        // gives 3.6576000000000004.
        assert_eq!(mul_ratio(12.0, 381.0, 1250.0), 3.6576);
        assert_eq!(div_ratio(3.6576, 381.0, 1250.0), 12.0);
    }
}
