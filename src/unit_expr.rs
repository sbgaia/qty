//! Compound unit expressions such as `kg·m/s²`, `W/(m²·K)` or `kWh`.

use core::f64::consts::PI;
use core::str::FromStr;

use crate::{Dimensions, ParseError, Quantity};

/// A unit written as an expression of SI and common units, like
/// `"kg*m/s^2"`, `"N·m"`, `"W/(m²·K)"`, `"kWh"` or `"rev/min"`.
///
/// When text has a unit that a quantity doesn't list, parsing falls back to
/// reading it as an expression, so any equivalent spelling works:
///
/// ```rust
/// use qty::*;
///
/// let f: Force = "3 kg*m/s^2".parse()?;
/// assert_eq!(f, Force::from_newtons(3.0));
/// let p: Pressure = "2 kN/m²".parse()?;
/// assert_eq!(p, Pressure::from_pascals(2000.0));
/// let f: Frequency = "120 min⁻¹".parse()?;
/// assert_eq!(f, Frequency::from_hertz(2.0));
/// // A pressure is not a force.
/// assert_eq!("3 kg*m/s^2".parse::<Pressure>(), Err(ParseError::UnknownUnit));
/// # Ok::<(), ParseError>(())
/// ```
///
/// # Syntax
///
/// Multiply with `*`, `·`, `⋅`, `×` or a space, and divide with `/`. `a/b/c`
/// means `a/(b·c)`, and parentheses group. Exponents are written `^2`, `**-1`
/// or as superscripts (`m²`, `s⁻¹`), and `1/s` works.
///
/// # Units
///
/// These take any SI prefix: the SI base and derived units (`m`, `g`, `s`,
/// `A`, `K`, `mol`, `cd`, `N`, `J`, `W`, `Pa`, `Hz`, `C`, `V`, `Ω` or `ohm`,
/// `S`, `F`, `H`, `Wb`, `T`, `lm`, `lx`, `Bq`, `Gy`, `Sv`, `kat`, `rad`), and
/// `L` or `l`, `t`, `eV`, `Wh`, `Ah`, `bar` and `cal`.
///
/// These don't: `min`, `h`, `d`, `ha`, `°` or `deg`, `rev`, `sr`, `Å`, `in`,
/// `ft`, `yd`, `mi`, `nmi`, `lb`, `oz`, `lbf`, `psi`, `atm`, and `°C`, `°F`
/// and `°R` as temperature differences.
///
/// A whole name beats a prefixed one, so `min` is a minute and `cd` a
/// candela.
///
/// Angles count as a dimension of their own (1 for the radian, 2 for the
/// steradian), so `Hz` isn't mistaken for `rad/s`, or `N·m` for `N·m/rad`.
/// Absolute temperatures and dB levels don't parse from expressions,
/// because their units have offsets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitExpr {
    /// The physical dimensions.
    pub dimensions: Dimensions,
    /// The exponent of the radian (2 for the steradian).
    pub angle: i8,
    /// The size of the unit in coherent SI units, with radians for angles.
    pub factor: f64,
}

impl UnitExpr {
    const ONE: Self = Self {
        dimensions: Dimensions::DIMENSIONLESS,
        angle: 0,
        factor: 1.0,
    };

    /// Parses an expression. Fails with [`ParseError::UnknownUnit`] if it's
    /// malformed or names a unit it doesn't know.
    ///
    /// It's a `const fn`, so expressions can be checked at compile time:
    ///
    /// ```rust
    /// use qty::*;
    ///
    /// const KWH: UnitExpr = match UnitExpr::parse("kW·h") {
    ///     Ok(e) => e,
    ///     Err(_) => panic!("not a unit"),
    /// };
    /// assert_eq!(KWH.factor, 3.6e6);
    /// ```
    pub const fn parse(s: &str) -> Result<Self, ParseError> {
        let b = s.as_bytes();
        let pos = skip_spaces(b, 0).1;
        let (expr, pos) = match expr(b, pos) {
            Ok(step) => step,
            Err(e) => return Err(e),
        };
        if skip_spaces(b, pos).1 == b.len() {
            Ok(expr)
        } else {
            Err(ParseError::UnknownUnit)
        }
    }

    pub(crate) const fn mul(self, other: Self) -> Self {
        Self {
            dimensions: self.dimensions.mul(other.dimensions),
            angle: self.angle + other.angle,
            factor: self.factor * other.factor,
        }
    }

    pub(crate) const fn powi(self, n: i8) -> Self {
        let mut factor = 1.0;
        let mut i = 0;
        while i < n.unsigned_abs() {
            factor *= self.factor;
            i += 1;
        }
        Self {
            dimensions: self.dimensions.powi(n),
            angle: self.angle * n,
            factor: if n < 0 { 1.0 / factor } else { factor },
        }
    }

    /// The factor from this unit to `Q`'s base unit, if `Q` has a base unit
    /// expression and the dimensions and angles agree. Plain numbers don't
    /// take expressions, so `"5 m/m"` isn't a ratio.
    #[doc(hidden)]
    pub fn factor_to_base<Q: Quantity>(self) -> Option<f64> {
        let base = Q::BASE_UNIT_EXPR?;
        if base.dimensions.is_dimensionless() && base.angle == 0 {
            return None;
        }
        (base.dimensions == self.dimensions
            && base.angle == self.angle
            && base.dimensions == Q::INFO.dimensions)
            .then(|| self.factor / base.factor)
    }
}

impl FromStr for UnitExpr {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, ParseError> {
        Self::parse(s)
    }
}

struct Atom {
    symbol: &'static str,
    /// L, M, T, I, Θ, N, J.
    dims: [i8; 7],
    angle: i8,
    factor: f64,
    /// Accepts SI prefixes.
    prefixed: bool,
}

const fn atom(symbol: &'static str, dims: [i8; 7], angle: i8, factor: f64, prefixed: bool) -> Atom {
    Atom {
        symbol,
        dims,
        angle,
        factor,
        prefixed,
    }
}

const L: [i8; 7] = [1, 0, 0, 0, 0, 0, 0];
const M: [i8; 7] = [0, 1, 0, 0, 0, 0, 0];
const T: [i8; 7] = [0, 0, 1, 0, 0, 0, 0];
const ONE: [i8; 7] = [0; 7];
const FORCE: [i8; 7] = [1, 1, -2, 0, 0, 0, 0];
const ENERGY: [i8; 7] = [2, 1, -2, 0, 0, 0, 0];
const PRESSURE: [i8; 7] = [-1, 1, -2, 0, 0, 0, 0];
const TEMPERATURE: [i8; 7] = [0, 0, 0, 0, 1, 0, 0];

const ATOMS: &[Atom] = &[
    // SI base units. The kilogram is the gram with a prefix.
    atom("m", L, 0, 1.0, true),
    atom("g", M, 0, 1e-3, true),
    atom("s", T, 0, 1.0, true),
    atom("A", [0, 0, 0, 1, 0, 0, 0], 0, 1.0, true),
    atom("K", TEMPERATURE, 0, 1.0, true),
    atom("mol", [0, 0, 0, 0, 0, 1, 0], 0, 1.0, true),
    atom("cd", [0, 0, 0, 0, 0, 0, 1], 0, 1.0, true),
    // SI derived units.
    atom("rad", ONE, 1, 1.0, true),
    atom("sr", ONE, 2, 1.0, false),
    atom("Hz", [0, 0, -1, 0, 0, 0, 0], 0, 1.0, true),
    atom("N", FORCE, 0, 1.0, true),
    atom("Pa", PRESSURE, 0, 1.0, true),
    atom("J", ENERGY, 0, 1.0, true),
    atom("W", [2, 1, -3, 0, 0, 0, 0], 0, 1.0, true),
    atom("C", [0, 0, 1, 1, 0, 0, 0], 0, 1.0, true),
    atom("V", [2, 1, -3, -1, 0, 0, 0], 0, 1.0, true),
    atom("Ω", [2, 1, -3, -2, 0, 0, 0], 0, 1.0, true),
    atom("ohm", [2, 1, -3, -2, 0, 0, 0], 0, 1.0, true),
    atom("S", [-2, -1, 3, 2, 0, 0, 0], 0, 1.0, true),
    atom("F", [-2, -1, 4, 2, 0, 0, 0], 0, 1.0, true),
    atom("H", [2, 1, -2, -2, 0, 0, 0], 0, 1.0, true),
    atom("Wb", [2, 1, -2, -1, 0, 0, 0], 0, 1.0, true),
    atom("T", [0, 1, -2, -1, 0, 0, 0], 0, 1.0, true),
    atom("lm", [0, 0, 0, 0, 0, 0, 1], 2, 1.0, true),
    atom("lx", [-2, 0, 0, 0, 0, 0, 1], 2, 1.0, true),
    atom("Bq", [0, 0, -1, 0, 0, 0, 0], 0, 1.0, true),
    atom("Gy", [2, 0, -2, 0, 0, 0, 0], 0, 1.0, true),
    atom("Sv", [2, 0, -2, 0, 0, 0, 0], 0, 1.0, true),
    atom("kat", [0, 0, -1, 0, 0, 1, 0], 0, 1.0, true),
    // Units accepted with SI prefixes.
    atom("L", [3, 0, 0, 0, 0, 0, 0], 0, 1e-3, true),
    atom("l", [3, 0, 0, 0, 0, 0, 0], 0, 1e-3, true),
    atom("t", M, 0, 1e3, true),
    atom("eV", ENERGY, 0, 1.602_176_634e-19, true),
    atom("Wh", ENERGY, 0, 3600.0, true),
    atom("Ah", [0, 0, 1, 1, 0, 0, 0], 0, 3600.0, true),
    atom("bar", PRESSURE, 0, 1e5, true),
    atom("cal", ENERGY, 0, 4.184, true),
    // Other units.
    atom("min", T, 0, 60.0, false),
    atom("h", T, 0, 3600.0, false),
    atom("d", T, 0, 86_400.0, false),
    atom("ha", [2, 0, 0, 0, 0, 0, 0], 0, 1e4, false),
    atom("°", ONE, 1, PI / 180.0, false),
    atom("deg", ONE, 1, PI / 180.0, false),
    atom("rev", ONE, 1, 2.0 * PI, false),
    atom("Å", L, 0, 1e-10, false),
    atom("in", L, 0, 0.0254, false),
    atom("ft", L, 0, 0.3048, false),
    atom("yd", L, 0, 0.9144, false),
    atom("mi", L, 0, 1609.344, false),
    atom("nmi", L, 0, 1852.0, false),
    atom("lb", M, 0, 0.453_592_37, false),
    atom("oz", M, 0, 0.028_349_523_125, false),
    atom("lbf", FORCE, 0, 4.448_221_615_260_5, false),
    atom("psi", PRESSURE, 0, 6_894.757_293_168_361, false),
    atom("atm", PRESSURE, 0, 101_325.0, false),
    // Temperature differences.
    atom("°C", TEMPERATURE, 0, 1.0, false),
    atom("°F", TEMPERATURE, 0, 5.0 / 9.0, false),
    atom("°R", TEMPERATURE, 0, 5.0 / 9.0, false),
];

const PREFIXES: &[(&str, f64)] = &[
    ("da", 1e1),
    ("Q", 1e30),
    ("R", 1e27),
    ("Y", 1e24),
    ("Z", 1e21),
    ("E", 1e18),
    ("P", 1e15),
    ("T", 1e12),
    ("G", 1e9),
    ("M", 1e6),
    ("k", 1e3),
    ("h", 1e2),
    ("d", 1e-1),
    ("c", 1e-2),
    ("m", 1e-3),
    ("µ", 1e-6),
    ("μ", 1e-6),
    ("u", 1e-6),
    ("n", 1e-9),
    ("p", 1e-12),
    ("f", 1e-15),
    ("a", 1e-18),
    ("z", 1e-21),
    ("y", 1e-24),
    ("r", 1e-27),
    ("q", 1e-30),
];

// The parser is `const` so that expressions can be checked at compile time.
// `const fn` can't take `&mut` before Rust 1.83, so each step takes the
// input and a byte position and returns the new position.

type Step<T> = Result<(T, usize), ParseError>;

const fn bytes_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Whether `pat` follows at `pos`, and the position after it if so.
const fn eat(b: &[u8], pos: usize, pat: &str) -> (bool, usize) {
    let pat = pat.as_bytes();
    if pos + pat.len() > b.len() {
        return (false, pos);
    }
    let mut i = 0;
    while i < pat.len() {
        if b[pos + i] != pat[i] {
            return (false, pos);
        }
        i += 1;
    }
    (true, pos + pat.len())
}

/// The character at `pos` and its length in bytes. `b` is valid UTF-8 and
/// `pos` a character boundary.
const fn peek(b: &[u8], pos: usize) -> Option<(char, usize)> {
    if pos >= b.len() {
        return None;
    }
    let first = b[pos] as u32;
    let (len, mut c) = if first < 0x80 {
        (1, first)
    } else if first < 0xE0 {
        (2, first & 0x1F)
    } else if first < 0xF0 {
        (3, first & 0x0F)
    } else {
        (4, first & 0x07)
    };
    let mut i = 1;
    while i < len {
        c = (c << 6) | (b[pos + i] as u32 & 0x3F);
        i += 1;
    }
    match char::from_u32(c) {
        Some(c) => Some((c, len)),
        None => None,
    }
}

/// Same as `char::is_whitespace`, which isn't `const`.
const fn is_whitespace(c: char) -> bool {
    matches!(
        c,
        '\t'..='\r'
            | ' '
            | '\u{85}'
            | '\u{A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
    )
}

/// Characters that can appear in unit symbols: those used by `ATOMS` and
/// `PREFIXES`. A token with any other letter couldn't match a unit anyway.
const fn is_symbol_char(c: char) -> bool {
    c.is_ascii_alphabetic() || matches!(c, '°' | 'Å' | 'µ' | 'μ' | 'Ω')
}

/// Whether any whitespace was skipped, and the position after it.
const fn skip_spaces(b: &[u8], mut pos: usize) -> (bool, usize) {
    let start = pos;
    while let Some((c, len)) = peek(b, pos) {
        if !is_whitespace(c) {
            break;
        }
        pos += len;
    }
    (pos != start, pos)
}

const fn lookup(token: &[u8]) -> Option<UnitExpr> {
    const fn from_atom(a: &Atom, prefix: f64) -> UnitExpr {
        UnitExpr {
            dimensions: Dimensions::new(
                a.dims[0], a.dims[1], a.dims[2], a.dims[3], a.dims[4], a.dims[5], a.dims[6],
            ),
            angle: a.angle,
            factor: a.factor * prefix,
        }
    }
    let mut i = 0;
    while i < ATOMS.len() {
        if bytes_eq(ATOMS[i].symbol.as_bytes(), token) {
            return Some(from_atom(&ATOMS[i], 1.0));
        }
        i += 1;
    }
    let mut p = 0;
    while p < PREFIXES.len() {
        let (prefix, scale) = PREFIXES[p];
        let (matched, rest) = eat(token, 0, prefix);
        if matched {
            let rest = token.split_at(rest).1;
            let mut i = 0;
            while i < ATOMS.len() {
                if ATOMS[i].prefixed && bytes_eq(ATOMS[i].symbol.as_bytes(), rest) {
                    return Some(from_atom(&ATOMS[i], scale));
                }
                i += 1;
            }
        }
        p += 1;
    }
    None
}

/// `factor (op factor)*`, where `op` is a multiplication sign, a space or `/`.
const fn expr(b: &[u8], pos: usize) -> Step<UnitExpr> {
    let (mut acc, mut pos) = match factor(b, pos) {
        Ok(step) => step,
        Err(e) => return Err(e),
    };
    loop {
        let (spaced, after_spaces) = skip_spaces(b, pos);
        pos = after_spaces;
        let divide = if let (true, p) = eat(b, pos, "/") {
            pos = p;
            true
        } else if eat(b, pos, "**").0 {
            // `**` right after a factor was consumed as an exponent.
            return Err(ParseError::UnknownUnit);
        } else if let (true, p) = eat_multiplication_sign(b, pos) {
            pos = p;
            false
        } else if spaced && !matches!(peek(b, pos), None | Some((')', _))) {
            // A space between factors.
            false
        } else {
            return Ok((acc, pos));
        };
        pos = skip_spaces(b, pos).1;
        let rhs = match factor(b, pos) {
            Ok((rhs, p)) => {
                pos = p;
                rhs
            }
            Err(e) => return Err(e),
        };
        acc = acc.mul(if divide { rhs.powi(-1) } else { rhs });
    }
}

const fn eat_multiplication_sign(b: &[u8], pos: usize) -> (bool, usize) {
    let signs = ["*", "·", "⋅", "×"];
    let mut i = 0;
    while i < signs.len() {
        if let (true, p) = eat(b, pos, signs[i]) {
            return (true, p);
        }
        i += 1;
    }
    (false, pos)
}

/// `primary exponent?`.
const fn factor(b: &[u8], pos: usize) -> Step<UnitExpr> {
    let (base, pos) = match primary(b, pos) {
        Ok(step) => step,
        Err(e) => return Err(e),
    };
    match exponent(b, pos) {
        Ok((Some(n), pos)) => Ok((base.powi(n), pos)),
        Ok((None, pos)) => Ok((base, pos)),
        Err(e) => Err(e),
    }
}

const fn primary(b: &[u8], pos: usize) -> Step<UnitExpr> {
    if let (true, pos) = eat(b, pos, "(") {
        let pos = skip_spaces(b, pos).1;
        let (inner, pos) = match expr(b, pos) {
            Ok(step) => step,
            Err(e) => return Err(e),
        };
        let pos = skip_spaces(b, pos).1;
        return match eat(b, pos, ")") {
            (true, pos) => Ok((inner, pos)),
            (false, _) => Err(ParseError::UnknownUnit),
        };
    }
    if let (true, pos) = eat(b, pos, "1") {
        return Ok((UnitExpr::ONE, pos));
    }
    let start = pos;
    let mut pos = pos;
    while let Some((c, len)) = peek(b, pos) {
        if !is_symbol_char(c) {
            break;
        }
        pos += len;
    }
    match lookup(b.split_at(pos).0.split_at(start).1) {
        Some(unit) => Ok((unit, pos)),
        None => Err(ParseError::UnknownUnit),
    }
}

/// The value of a superscript digit.
const fn superscript_digit(c: char) -> Option<i8> {
    match c {
        '⁰' => Some(0),
        '¹' => Some(1),
        '²' => Some(2),
        '³' => Some(3),
        '⁴'..='⁹' => Some((c as u32 - '⁴' as u32) as i8 + 4),
        _ => None,
    }
}

/// `^n`, `**n` or superscript digits, with an optional minus sign.
const fn exponent(b: &[u8], pos: usize) -> Step<Option<i8>> {
    // Spaces may surround `^` (`s ^ 2`), but a space alone separates factors.
    let after_spaces = skip_spaces(b, pos).1;
    let (caret, mut p) = eat(b, after_spaces, "^");
    let (stars, p2) = eat(b, after_spaces, "**");
    if caret || stars {
        if stars {
            p = p2;
        }
        p = skip_spaces(b, p).1;
        let (minus, p3) = eat(b, p, "-");
        let (minus_sign, p4) = eat(b, p, "−");
        let negative = minus || minus_sign;
        p = if minus {
            p3
        } else if minus_sign {
            p4
        } else {
            eat(b, p, "+").1
        };
        // Stop at i8::MAX, as `str::parse::<i8>` would.
        let start = p;
        let mut n: i32 = 0;
        while p < b.len() && b[p].is_ascii_digit() {
            n = n * 10 + (b[p] - b'0') as i32;
            if n > i8::MAX as i32 {
                return Err(ParseError::UnknownUnit);
            }
            p += 1;
        }
        if p == start {
            return Err(ParseError::UnknownUnit);
        }
        let n = n as i8;
        return Ok((Some(if negative { -n } else { n }), p));
    }
    let (negative, mut p) = eat(b, pos, "⁻");
    let mut n: Option<i8> = None;
    while let Some((c, len)) = peek(b, p) {
        let Some(digit) = superscript_digit(c) else {
            break;
        };
        p += len;
        let value = match n {
            Some(n) => n,
            None => 0,
        };
        n = match value.checked_mul(10) {
            Some(v) => match v.checked_add(digit) {
                Some(v) => Some(v),
                None => return Err(ParseError::UnknownUnit),
            },
            None => return Err(ParseError::UnknownUnit),
        };
    }
    match (negative, n) {
        (true, None) => Err(ParseError::UnknownUnit),
        (true, Some(n)) => Ok((Some(-n), p)),
        (false, n) => Ok((n, p)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> UnitExpr {
        UnitExpr::parse(s).unwrap_or_else(|_| panic!("{s:?} should parse"))
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-12 * a.abs().max(b.abs())
    }

    #[test]
    fn syntax() {
        let newton = parse("N");
        for s in [
            "kg*m/s^2",
            "kg·m/s²",
            "kg m s⁻²",
            "kg*m*s**-2",
            "(kg*m)/(s*s)",
            "kg ⋅ m / s ^ 2",
            "kg×m/s/s",
        ] {
            let e = parse(s);
            assert_eq!(e.dimensions, newton.dimensions, "{s}");
            assert!(close(e.factor, 1.0), "{s}: {}", e.factor);
        }
        assert_eq!(parse("1/s").dimensions, parse("Hz").dimensions);
        assert_eq!(
            parse("W/(m²·K)").dimensions,
            Dimensions::new(0, 1, -3, 0, -1, 0, 0)
        );
        for bad in [
            "", "kg*", "/s", "m^", "m^x", "(m", "m)", "foo", "m**", "m⁻", "kg//s", "m^999",
        ] {
            assert!(UnitExpr::parse(bad).is_err(), "{bad:?} should not parse");
        }
    }

    #[test]
    fn prefixes_and_exact_names() {
        assert!(close(parse("km").factor, 1e3));
        assert!(close(parse("mm").factor, 1e-3));
        assert!(close(parse("Mm").factor, 1e6));
        assert!(close(parse("µs").factor, 1e-6));
        assert!(close(parse("us").factor, 1e-6));
        assert!(close(parse("dam").factor, 10.0));
        assert!(close(parse("hPa").factor, 100.0));
        assert!(close(parse("kWh").factor, 3.6e6));
        assert!(close(parse("mAh").factor, 3.6));
        // An exact name wins over a prefix split.
        assert!(close(parse("min").factor, 60.0)); // not milli-inch
        assert_eq!(parse("cd").dimensions.luminous_intensity, 1); // not centi-day
        assert!(close(parse("h").factor, 3600.0));
        assert!(close(parse("hm").factor, 100.0)); // hectometre
        assert!(close(parse("Gy").factor, 1.0));
        // Units without prefixes refuse them.
        assert!(UnitExpr::parse("kmin").is_err());
        assert!(UnitExpr::parse("kft").is_err());
    }

    /// `src/literal/sym.rs`: a `literal::Unit` constant for every unit
    /// symbol that is a Rust identifier, with the value `UnitExpr::parse`
    /// gives it. `QTY_BLESS=1 cargo test` rewrites the file.
    #[cfg(feature = "std")]
    #[test]
    fn literal_symbols_are_up_to_date() {
        extern crate std;
        use std::{format, string::String, vec::Vec};

        const KEYWORDS: &[&str] = &["as", "in"];
        // Not the micro sign `µ`, which rustc warns about in identifiers;
        // the Greek `μ` and `u` work.
        let is_ident = |s: &str| {
            s.chars()
                .all(|c| c.is_ascii_alphabetic() || matches!(c, 'Å' | 'μ' | 'Ω'))
        };
        let mut names: Vec<String> = ATOMS.iter().map(|a| a.symbol.into()).collect();
        for (prefix, _) in PREFIXES {
            for a in ATOMS.iter().filter(|a| a.prefixed) {
                names.push(format!("{prefix}{}", a.symbol));
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        names.retain(|name| is_ident(name) && seen.insert(name.clone()));

        let mut text = String::from(
            "// Generated by `QTY_BLESS=1 cargo test literal_symbols` — DO NOT EDIT\n\
             //! The unit symbols of `qty!`, one constant per symbol that is a Rust\n\
             //! identifier, with SI prefixes.\n\n\
             use super::Unit;\n\
             use crate::formula::{Dims, E};\n",
        );
        for name in &names {
            let e = parse(name);
            let mut dims = e.dimensions.to_array().to_vec();
            while dims.last() == Some(&0) {
                dims.pop();
            }
            let dims: Vec<String> = dims.iter().map(|d| format!("{d}")).collect();
            let dims = if dims.is_empty() {
                String::from("Dims")
            } else {
                format!("Dims<{}>", dims.join(", "))
            };
            let ident = if KEYWORDS.contains(&name.as_str()) {
                format!("r#{name}")
            } else {
                assert!(!matches!(
                    name.as_str(),
                    "self" | "Self" | "super" | "crate"
                ));
                name.clone()
            };
            text += &format!(
                "pub const {ident}: Unit<{dims}, E<{}>> = Unit::new({:?});\n",
                e.angle, e.factor,
            );
        }
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/literal/sym.rs");
        if std::env::var_os("QTY_BLESS").is_some() {
            std::fs::write(path, &text).unwrap();
        }
        let current = std::fs::read_to_string(path).unwrap_or_default();
        assert!(
            current == text,
            "src/literal/sym.rs is out of date: run `QTY_BLESS=1 cargo test`"
        );
    }

    #[test]
    fn angles_are_tracked() {
        assert_eq!(parse("Hz").angle, 0);
        assert_eq!(parse("rad/s").angle, 1);
        assert_eq!(parse("rev/min").angle, 1);
        assert!(close(parse("rev/min").factor, 2.0 * PI / 60.0));
        assert_eq!(parse("lx").angle, 2);
        assert_eq!(parse("cd/m²").angle, 0);
        assert!(close(parse("°").factor, PI / 180.0));
    }
}
