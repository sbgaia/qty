//! Exact evaluation of UnitsNet conversion formulas.
//!
//! UnitsNet expresses every conversion as a C# expression such as
//! `{x} * 2.54e-2 / 72.27` or `({x} + 459.67) * 5 / 9`. Emitting those
//! expressions verbatim costs several floating-point operations per conversion
//! (LLVM may not re-associate float arithmetic) and accumulates a rounding
//! error at every step.
//!
//! This module parses each formula and reduces it **symbolically, with exact
//! rational arithmetic**, to one of three canonical forms:
//!
//! | Form | Meaning | Example |
//! |---|---|---|
//! | [`Form::Affine`] with `b = 0` | `y = a·x` | metre ↔ inch |
//! | [`Form::Affine`] | `y = a·x + b` | kelvin ↔ °F |
//! | [`Form::Reciprocal`] | `y = k / x` | km/L ↔ L/100 km |
//!
//! The constants are then rounded **once** to the nearest `f64`, so the
//! generated code performs a single operation per conversion with a single
//! rounding of its constant.

use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

use num_bigint::BigInt;
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

// ── Constants ─────────────────────────────────────────────────────────────────

/// π to 64 significant digits — far beyond what an `f64` can distinguish.
const PI_DIGITS: &str = "3.141592653589793238462643383279502884197169399375105820974944592";

/// An exact constant of the form `r · π^pi`.
///
/// Keeping the power of π separate lets every formula except the ones that
/// genuinely involve π stay exact, so we can tell whether a factor is exactly
/// representable as an `f64`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Const {
    r: BigRational,
    pi: i32,
}

impl Const {
    pub fn int(v: i64) -> Self {
        Self {
            r: BigRational::from_integer(BigInt::from(v)),
            pi: 0,
        }
    }

    pub fn zero() -> Self {
        Self::int(0)
    }

    pub fn one() -> Self {
        Self::int(1)
    }

    fn pi() -> Self {
        Self {
            r: BigRational::one(),
            pi: 1,
        }
    }

    /// Parses a decimal literal such as `1.4959787070e11` exactly.
    pub fn parse_decimal(s: &str) -> Self {
        let (mantissa, exp) = match s.find(['e', 'E']) {
            Some(i) => (&s[..i], s[i + 1..].parse::<i32>().expect("exponent")),
            None => (s, 0),
        };
        let (int_part, frac_part) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        let digits: BigInt = format!("{int_part}{frac_part}")
            .parse()
            .unwrap_or_else(|_| panic!("invalid number literal {s:?}"));
        let scale = exp - frac_part.len() as i32;
        let ten = BigInt::from(10);
        let r = if scale >= 0 {
            BigRational::from_integer(digits * num_traits::pow(ten, scale as usize))
        } else {
            BigRational::new(digits, num_traits::pow(ten, (-scale) as usize))
        };
        Self { r, pi: 0 }
    }

    /// `(numerator, denominator, power of π)` when the constant is positive
    /// and its reduced fraction fits in `u128`.
    pub fn exact_parts(&self) -> Option<(u128, u128, i32)> {
        if !self.r.is_positive() {
            return None;
        }
        Some((
            self.r.numer().to_u128()?,
            self.r.denom().to_u128()?,
            self.pi,
        ))
    }

    pub fn from_f64_exact(v: f64) -> Self {
        Self {
            r: BigRational::from_float(v).expect("finite"),
            pi: 0,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.r.is_zero()
    }

    pub fn is_one(&self) -> bool {
        self.pi == 0 && self.r.is_one()
    }

    pub fn recip(&self) -> Self {
        assert!(!self.is_zero(), "division by zero in formula");
        Self {
            r: self.r.recip(),
            pi: -self.pi,
        }
    }

    /// The best rational approximation of this constant (π expanded).
    fn approx_rational(&self) -> BigRational {
        if self.pi == 0 {
            return self.r.clone();
        }
        let pi = Self::parse_decimal(PI_DIGITS).r;
        let pow = num_traits::pow(pi, self.pi.unsigned_abs() as usize);
        if self.pi > 0 {
            &self.r * pow
        } else {
            &self.r / pow
        }
    }

    /// The nearest `f64` (round-half-to-even), computed from the exact value.
    pub fn to_f64(&self) -> f64 {
        rational_to_f64(&self.approx_rational())
    }

    /// `true` when this constant is exactly representable as an `f64`, i.e.
    /// multiplying or dividing by it introduces no error beyond the final
    /// rounding of the operation itself.
    pub fn is_exact_f64(&self) -> bool {
        self.pi == 0 && Self::from_f64_exact(self.to_f64()).r == self.r
    }
}

impl Add for Const {
    type Output = Const;
    fn add(self, rhs: Const) -> Const {
        if self.is_zero() {
            return rhs;
        }
        if rhs.is_zero() {
            return self;
        }
        assert_eq!(
            self.pi, rhs.pi,
            "cannot add constants with different powers of π"
        );
        Const {
            r: self.r + rhs.r,
            pi: self.pi,
        }
    }
}

impl Neg for Const {
    type Output = Const;
    fn neg(self) -> Const {
        Const {
            r: -self.r,
            pi: self.pi,
        }
    }
}

impl Sub for Const {
    type Output = Const;
    fn sub(self, rhs: Const) -> Const {
        self + (-rhs)
    }
}

impl Mul for Const {
    type Output = Const;
    fn mul(self, rhs: Const) -> Const {
        Const {
            r: self.r * rhs.r,
            pi: self.pi + rhs.pi,
        }
    }
}

impl Div for Const {
    type Output = Const;
    // Division is multiplication by the exact reciprocal.
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Const) -> Const {
        self * rhs.recip()
    }
}

/// Correctly rounded (half-to-even) conversion of an exact rational to `f64`.
fn rational_to_f64(r: &BigRational) -> f64 {
    if r.is_zero() {
        return 0.0;
    }
    let negative = r.is_negative();
    let r = r.abs();
    let (p, q) = (r.numer(), r.denom());

    // Scale so that the integer quotient has at least 55 significant bits:
    // 53 for the mantissa plus guard bits for rounding.
    let mut shift: i64 = 56 - (p.bits() as i64 - q.bits() as i64);
    let (num, den) = if shift >= 0 {
        (p << shift as usize, q.clone())
    } else {
        (p.clone(), q << (-shift) as usize)
    };
    let (mut m, rem) = num.div_rem(&den);
    let sticky = !rem.is_zero();

    let extra = m.bits() - 53;
    let low = &m & ((BigInt::one() << extra as usize) - 1);
    m >>= extra as usize;
    shift -= extra as i64;
    let half = BigInt::one() << (extra as usize - 1);
    let round_up = low > half || (low == half && (sticky || m.is_odd()));
    if round_up {
        m += 1;
    }

    // `m` ≤ 2^53, exactly representable; scaling by a power of two is exact.
    let m = m.to_u64().expect("mantissa fits") as f64;
    let value = m * pow2(-shift);
    if negative {
        -value
    } else {
        value
    }
}

fn pow2(e: i64) -> f64 {
    assert!((-1022..=1023).contains(&e), "constant out of f64 range");
    f64::from_bits(((e + 1023) as u64) << 52)
}

/// Formats an `f64` as the shortest Rust literal that round-trips exactly.
pub fn f64_literal(v: f64) -> String {
    // Both forms print the shortest digit string that round-trips exactly;
    // scientific notation keeps very large and small factors readable.
    let s = if v != 0.0 && !(1e-4..1e7).contains(&v.abs()) {
        format!("{v:e}")
    } else {
        format!("{v:?}")
    };
    if s.contains(['.', 'e', 'E']) || s.contains("inf") || s.contains("NaN") {
        s
    } else {
        format!("{s}.0")
    }
}

impl fmt::Display for Const {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&f64_literal(self.to_f64()))
    }
}

// ── Canonical forms ───────────────────────────────────────────────────────────

/// A conversion formula reduced to canonical form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Form {
    /// `y = a·x + b`. A pure constant has `a = 0`.
    Affine { a: Const, b: Const },
    /// `y = k / x`.
    Reciprocal { k: Const },
}

impl Form {
    fn x() -> Self {
        Form::Affine {
            a: Const::one(),
            b: Const::zero(),
        }
    }

    fn constant(c: Const) -> Self {
        Form::Affine {
            a: Const::zero(),
            b: c,
        }
    }

    fn as_constant(&self) -> Option<&Const> {
        match self {
            Form::Affine { a, b } if a.is_zero() => Some(b),
            _ => None,
        }
    }

    fn scale(self, c: &Const) -> Form {
        match self {
            Form::Affine { a, b } => Form::Affine {
                a: a * c.clone(),
                b: b * c.clone(),
            },
            Form::Reciprocal { k } => Form::Reciprocal { k: k * c.clone() },
        }
    }

    fn add(self, rhs: Form) -> Form {
        match (self, rhs) {
            (Form::Affine { a: a1, b: b1 }, Form::Affine { a: a2, b: b2 }) => Form::Affine {
                a: a1 + a2,
                b: b1 + b2,
            },
            (l, r) => panic!("unsupported formula: {l:?} + {r:?}"),
        }
    }

    fn mul(self, rhs: Form) -> Form {
        if let Some(c) = rhs.as_constant() {
            let c = c.clone();
            return self.scale(&c);
        }
        if let Some(c) = self.as_constant() {
            let c = c.clone();
            return rhs.scale(&c);
        }
        panic!("unsupported non-linear formula: {self:?} * {rhs:?}")
    }

    fn div(self, rhs: Form) -> Form {
        if let Some(c) = rhs.as_constant() {
            let c = c.recip();
            return self.scale(&c);
        }
        if let Some(c) = self.as_constant() {
            let c = c.clone();
            return match rhs {
                Form::Affine { a, b } if b.is_zero() => Form::Reciprocal { k: c / a },
                Form::Reciprocal { k } => Form::Affine {
                    a: c / k,
                    b: Const::zero(),
                },
                other => panic!("unsupported formula: constant / {other:?}"),
            };
        }
        panic!("unsupported non-linear formula: {self:?} / {rhs:?}")
    }

    /// Substitutes `x → x·p`, i.e. applies an SI prefix to the input unit.
    pub fn with_input_factor(self, p: &Const) -> Form {
        match self {
            Form::Affine { a, b } => Form::Affine {
                a: a * p.clone(),
                b,
            },
            Form::Reciprocal { k } => Form::Reciprocal { k: k / p.clone() },
        }
    }

    /// The inverse function (`y = f(x)` → `x = f⁻¹(y)`).
    pub fn inverse(&self) -> Form {
        match self {
            Form::Affine { a, b } => {
                let inv = a.recip();
                Form::Affine {
                    b: -(b.clone() * inv.clone()),
                    a: inv,
                }
            }
            Form::Reciprocal { k } => Form::Reciprocal { k: k.clone() },
        }
    }

    /// `true` for `y = a·x` (no offset): the conversion commutes with
    /// addition, so it is safe for sums, differences and SI prefixes.
    pub fn is_linear(&self) -> bool {
        matches!(self, Form::Affine { b, .. } if b.is_zero())
    }

    /// Relative difference between two forms, used to cross-check the
    /// upstream to-base and from-base formulas against each other.
    pub fn rel_diff(&self, other: &Form) -> f64 {
        fn rel(x: &Const, y: &Const) -> f64 {
            let (x, y) = (x.to_f64(), y.to_f64());
            let scale = x.abs().max(y.abs());
            if scale == 0.0 {
                0.0
            } else {
                (x - y).abs() / scale
            }
        }
        match (self, other) {
            (Form::Affine { a: a1, b: b1 }, Form::Affine { a: a2, b: b2 }) => {
                rel(a1, a2).max(rel(b1, b2))
            }
            (Form::Reciprocal { k: k1 }, Form::Reciprocal { k: k2 }) => rel(k1, k2),
            _ => f64::INFINITY,
        }
    }
}

// ── Rust emission ─────────────────────────────────────────────────────────────

/// Which way a conversion goes; decides how inexact factors are emitted.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// From the named unit into the quantity's base unit.
    ToBase,
    /// From the base unit into the named unit.
    FromBase,
}

/// Wraps a constant for use in generic code over `T: Float`.
fn lit(c: &Const) -> String {
    format!("T::from_f64({c})")
}

/// Emits `x · f` using a single operation.
///
/// Prefers whichever of `f` or `1/f` is exactly representable, so that, e.g.,
/// kilometres → metres is `x * 1000` and metres → kilometres is `x / 1000`
/// (both correctly rounded). When neither is exact, both directions share the
/// constant rounded in the to-base direction, so that
/// `from_unit(v).as_unit()` round-trips `v` in practically every case.
fn emit_scale(x: &str, f: &Const, dir: Direction) -> String {
    if f.is_one() {
        return x.to_owned();
    }
    let inv = f.recip();
    if f.is_exact_f64() {
        format!("{x} * {}", lit(f))
    } else if inv.is_exact_f64() {
        format!("{x} / {}", lit(&inv))
    } else {
        match dir {
            Direction::ToBase => format!("{x} * {}", lit(f)),
            Direction::FromBase => format!("{x} / {}", lit(&inv)),
        }
    }
}

/// Emits a Rust expression of type `T` computing `form` applied to `x`.
pub fn emit(form: &Form, x: &str, dir: Direction) -> String {
    match form {
        Form::Affine { a, b } => {
            if b.is_zero() {
                return emit_scale(x, a, dir);
            }
            match dir {
                // y = a·x + b
                Direction::ToBase => {
                    let scaled = emit_scale(x, a, dir);
                    offset(&scaled, b)
                }
                // `form` is the inverse of the to-base form `a₀·x + b₀`. Undo
                // the to-base steps in reverse order, `(x − b₀) / a₀`, so both
                // directions use the same rounded constants.
                Direction::FromBase => {
                    let a0 = a.recip();
                    let b0 = -(b.clone() * a0.clone());
                    let shifted = offset(x, &(-b0));
                    if a0.is_one() {
                        shifted
                    } else {
                        emit_scale(&format!("({shifted})"), &a0.recip(), dir)
                    }
                }
            }
        }
        Form::Reciprocal { k } => format!("{} / {x}", lit(k)),
    }
}

fn offset(expr: &str, b: &Const) -> String {
    if b.r.is_negative() {
        format!("{expr} - {}", lit(&-b.clone()))
    } else {
        format!("{expr} + {}", lit(b))
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
    X,
    Pi,
    Num(String),
    Op(char),
}

fn tokenize(src: &str) -> Vec<Token> {
    let chars: Vec<char> = src.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if src[byte_index(&chars, i)..].starts_with("{x}") {
            tokens.push(Token::X);
            i += 3;
        } else if src[byte_index(&chars, i)..].starts_with("Math.PI") {
            tokens.push(Token::Pi);
            i += 7;
        } else if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            if i < chars.len() && matches!(chars[i], 'e' | 'E') {
                i += 1;
                if i < chars.len() && matches!(chars[i], '+' | '-') {
                    i += 1;
                }
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let lit: String = chars[start..i].iter().collect();
            // C# literal suffixes: `d` (double), `m` (decimal).
            if i < chars.len() && matches!(chars[i], 'd' | 'D' | 'm' | 'M') {
                i += 1;
            }
            tokens.push(Token::Num(lit));
        } else if "+-*/()".contains(c) {
            tokens.push(Token::Op(c));
            i += 1;
        } else {
            panic!("unexpected character {c:?} in formula {src:?}");
        }
    }
    tokens
}

fn byte_index(chars: &[char], i: usize) -> usize {
    chars[..i].iter().map(|c| c.len_utf8()).sum()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    // expr := term (('+' | '-') term)*
    fn expr(&mut self) -> Form {
        let mut lhs = self.term();
        while let Some(Token::Op(op @ ('+' | '-'))) = self.peek().cloned() {
            self.pos += 1;
            let rhs = self.term();
            lhs = if op == '+' {
                lhs.add(rhs)
            } else {
                lhs.add(rhs.scale(&Const::int(-1)))
            };
        }
        lhs
    }

    // term := unary (('*' | '/') unary)*
    fn term(&mut self) -> Form {
        let mut lhs = self.unary();
        while let Some(Token::Op(op @ ('*' | '/'))) = self.peek().cloned() {
            self.pos += 1;
            let rhs = self.unary();
            lhs = if op == '*' {
                lhs.mul(rhs)
            } else {
                lhs.div(rhs)
            };
        }
        lhs
    }

    // unary := '-' unary | atom
    fn unary(&mut self) -> Form {
        if self.peek() == Some(&Token::Op('-')) {
            self.pos += 1;
            return self.unary().scale(&Const::int(-1));
        }
        self.atom()
    }

    // atom := number | {x} | Math.PI | '(' expr ')'
    fn atom(&mut self) -> Form {
        match self.next() {
            Some(Token::X) => Form::x(),
            Some(Token::Pi) => Form::constant(Const::pi()),
            Some(Token::Num(n)) => Form::constant(Const::parse_decimal(&n)),
            Some(Token::Op('(')) => {
                let inner = self.expr();
                assert_eq!(self.next(), Some(Token::Op(')')), "unbalanced parentheses");
                inner
            }
            other => panic!("unexpected token {other:?}"),
        }
    }
}

/// Parses a UnitsNet formula (with `{x}` as the variable) into canonical form.
pub fn parse(src: &str) -> Form {
    let mut p = Parser {
        tokens: tokenize(src),
        pos: 0,
    };
    let form = p.expr();
    assert!(
        p.pos == p.tokens.len(),
        "trailing tokens in formula {src:?}"
    );
    form
}

#[cfg(test)]
mod tests {
    use super::*;

    fn affine(a: f64, b: f64) -> Form {
        Form::Affine {
            a: Const::from_f64_exact(a),
            b: Const::from_f64_exact(b),
        }
    }

    #[test]
    fn folds_linear_chain() {
        // 2.54e-2 / 72.27 folded exactly, then rounded once.
        let f = parse("{x} * 2.54e-2 / 72.27");
        let Form::Affine { a, b } = &f else { panic!() };
        assert!(b.is_zero());
        assert_eq!(a.to_f64(), 0.0254 / 72.27);
    }

    #[test]
    fn folds_affine() {
        let f = parse("({x} + 459.67) * 5 / 9");
        let Form::Affine { a, b } = &f else { panic!() };
        assert_eq!(a, &(Const::int(5) / Const::int(9)));
        assert_eq!(
            b,
            &(Const::parse_decimal("459.67") * Const::int(5) / Const::int(9))
        );
    }

    #[test]
    fn reciprocal() {
        assert_eq!(parse("100 / {x}"), Form::Reciprocal { k: Const::int(100) });
        assert_eq!(parse("100 / {x}").inverse(), parse("100 / {x}"));
    }

    #[test]
    fn csharp_suffixes_and_unary_minus() {
        assert_eq!(parse("-{x} * 3600d"), affine(-3600.0, 0.0));
    }

    #[test]
    fn inverse_round_trip() {
        let f = parse("{x} * 9 / 5 - 459.67");
        assert_eq!(f.inverse().inverse(), f);
    }

    #[test]
    fn pi_is_never_exact() {
        let f = parse("{x} * Math.PI / 180");
        let Form::Affine { a, .. } = &f else { panic!() };
        assert!(!a.is_exact_f64());
        assert_eq!(a.to_f64(), std::f64::consts::PI / 180.0);
    }

    #[test]
    fn exactness() {
        assert!(Const::int(1000).is_exact_f64());
        assert!(!Const::parse_decimal("1e-3").is_exact_f64());
        assert!(Const::parse_decimal("0.5").is_exact_f64());
        assert!(!Const::parse_decimal("0.0254").is_exact_f64());
    }

    #[test]
    fn correct_rounding() {
        for s in [
            "0.1",
            "1609.344",
            "3.08567758128e16",
            "1e-15",
            "1.602176634e-19",
            "6.02214076e23",
        ] {
            assert_eq!(
                Const::parse_decimal(s).to_f64(),
                s.parse::<f64>().unwrap(),
                "{s}"
            );
        }
        let third = Const::int(1) / Const::int(3);
        assert_eq!(third.to_f64(), 1.0 / 3.0);
    }

    #[test]
    fn emission_prefers_exact_constants() {
        let km = parse("{x} * 1e3");
        assert_eq!(emit(&km, "x", Direction::ToBase), "x * T::from_f64(1000.0)");
        assert_eq!(
            emit(&km.inverse(), "x", Direction::FromBase),
            "x / T::from_f64(1000.0)"
        );
        let mm = parse("{x} / 1e3");
        assert_eq!(emit(&mm, "x", Direction::ToBase), "x / T::from_f64(1000.0)");
        let fm = parse("{x} * 1e-15");
        assert_eq!(emit(&fm, "x", Direction::ToBase), "x / T::from_f64(1e15)");
        assert_eq!(
            emit(&mm.inverse(), "x", Direction::FromBase),
            "x * T::from_f64(1000.0)"
        );
        // Neither 0.0254 nor 1/0.0254 is exact: both directions share 0.0254.
        let inch = parse("{x} * 2.54e-2");
        assert_eq!(
            emit(&inch, "x", Direction::ToBase),
            "x * T::from_f64(0.0254)"
        );
        assert_eq!(
            emit(&inch.inverse(), "x", Direction::FromBase),
            "x / T::from_f64(0.0254)"
        );
    }

    #[test]
    fn emission_affine() {
        let c = parse("{x} + 273.15");
        assert_eq!(emit(&c, "x", Direction::ToBase), "x + T::from_f64(273.15)");
        assert_eq!(
            emit(&c.inverse(), "x", Direction::FromBase),
            "x - T::from_f64(273.15)"
        );
    }
}
