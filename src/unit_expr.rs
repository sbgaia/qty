//! Compound unit expressions such as `kg·m/s²`, `W/(m²·K)` or `kWh`.

use core::f64::consts::PI;
use core::str::FromStr;

use crate::{Dimensions, ParseError, Quantity};

/// A unit written as an expression of SI and common units: `"kg*m/s^2"`,
/// `"N·m"`, `"W/(m²·K)"`, `"kWh"`, `"rev/min"`.
///
/// Quantities fall back to it when parsing text whose unit is not one of
/// their listed abbreviations or names, so any equivalent spelling works:
///
/// ```rust
/// use unitsnet_rs::*;
///
/// let f: Force = "3 kg*m/s^2".parse()?;
/// assert_eq!(f, Force::from_newtons(3.0));
/// let p: Pressure = "2 kN/m²".parse()?;
/// assert_eq!(p, Pressure::from_pascals(2000.0));
/// let f: Frequency = "120 min⁻¹".parse()?;
/// assert_eq!(f, Frequency::from_hertz(2.0));
/// // Dimensions must match: a pressure is not a force.
/// assert_eq!("3 kg*m/s^2".parse::<Pressure>(), Err(ParseError::UnknownUnit));
/// # Ok::<(), ParseError>(())
/// ```
///
/// **Syntax.** Factors are multiplied with `*`, `·`, `⋅`, `×` or a space and
/// divided with `/` (`a/b/c` is `a/(b·c)`); parentheses group; exponents are
/// `^2`, `**-1` or superscripts (`m²`, `s⁻¹`); `1/s` is allowed.
///
/// **Units.** The SI base and derived units with any SI prefix (`m`, `g`,
/// `s`, `A`, `K`, `mol`, `cd`, `N`, `J`, `W`, `Pa`, `Hz`, `C`, `V`, `Ω`/`ohm`,
/// `S`, `F`, `H`, `Wb`, `T`, `lm`, `lx`, `Bq`, `Gy`, `Sv`, `kat`, `rad`), and
/// `L`/`l`, `t`, `eV`, `Wh`, `Ah`, `bar` and `cal` (also prefixed); `min`,
/// `h`, `d`, `ha`, `°`/`deg`, `rev`, `sr`, `Å`, `in`, `ft`, `yd`, `mi`,
/// `nmi`, `lb`, `oz`, `lbf`, `psi`, `atm`, and `°C`, `°F`, `°R` as temperature
/// *differences*. An exact name wins over a prefixed one: `min` is a minute,
/// `cd` a candela.
///
/// **Angles** count as a dimension of their own here (radian 1, steradian 2),
/// so that `Hz` and `rad/s`, or `N·m` and `N·m/rad`, are not confused.
/// Point quantities (absolute temperatures, dB levels) do not use
/// expressions, since their units have offsets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitExpr {
    /// The physical dimensions.
    pub dimensions: Dimensions,
    /// The exponent of the radian (2 for the steradian).
    pub angle: i8,
    /// One of this unit in coherent SI units (with radians for angles).
    pub factor: f64,
}

impl UnitExpr {
    const ONE: Self = Self {
        dimensions: Dimensions::DIMENSIONLESS,
        angle: 0,
        factor: 1.0,
    };

    /// Parses an expression; [`ParseError::UnknownUnit`] if it is malformed
    /// or names an unknown unit.
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        let mut parser = Parser { s, pos: 0 };
        parser.skip_spaces();
        let expr = parser.expr()?;
        parser.skip_spaces();
        if parser.pos != s.len() {
            return Err(ParseError::UnknownUnit);
        }
        Ok(expr)
    }

    fn mul(self, other: Self) -> Self {
        Self {
            dimensions: self.dimensions.mul(other.dimensions),
            angle: self.angle + other.angle,
            factor: self.factor * other.factor,
        }
    }

    fn powi(self, n: i8) -> Self {
        let mut factor = 1.0;
        for _ in 0..n.unsigned_abs() {
            factor *= self.factor;
        }
        Self {
            dimensions: self.dimensions.powi(n),
            angle: self.angle * n,
            factor: if n < 0 { 1.0 / factor } else { factor },
        }
    }

    /// The factor from this unit to `Q`'s base unit, if `Q` can be written
    /// with expressions and the dimensions (and angles) agree.
    #[doc(hidden)]
    pub fn factor_to_base<Q: Quantity>(self) -> Option<f64> {
        let base = Self::parse(Q::INFO.base_unit.abbreviation).ok()?;
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

// ── Units ─────────────────────────────────────────────────────────────────────

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
    // SI base units (the kilogram through the gram, with prefixes).
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

fn lookup(token: &str) -> Option<UnitExpr> {
    let from_atom = |a: &Atom, prefix: f64| UnitExpr {
        dimensions: Dimensions::new(
            a.dims[0], a.dims[1], a.dims[2], a.dims[3], a.dims[4], a.dims[5], a.dims[6],
        ),
        angle: a.angle,
        factor: a.factor * prefix,
    };
    if let Some(a) = ATOMS.iter().find(|a| a.symbol == token) {
        return Some(from_atom(a, 1.0));
    }
    PREFIXES.iter().find_map(|&(prefix, scale)| {
        let rest = token.strip_prefix(prefix)?;
        ATOMS
            .iter()
            .find(|a| a.prefixed && a.symbol == rest)
            .map(|a| from_atom(a, scale))
    })
}

// ── Parser ────────────────────────────────────────────────────────────────────

struct Parser<'a> {
    s: &'a str,
    pos: usize,
}

const SUPERSCRIPT_DIGITS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.s[self.pos..].chars().next()
    }

    fn bump(&mut self) {
        if let Some(c) = self.peek() {
            self.pos += c.len_utf8();
        }
    }

    fn eat(&mut self, token: &str) -> bool {
        if self.s[self.pos..].starts_with(token) {
            self.pos += token.len();
            true
        } else {
            false
        }
    }

    fn skip_spaces(&mut self) -> bool {
        let start = self.pos;
        while self.peek().is_some_and(char::is_whitespace) {
            self.bump();
        }
        self.pos != start
    }

    /// `factor (op factor)*`, where `op` is a multiplication sign, a space or `/`.
    fn expr(&mut self) -> Result<UnitExpr, ParseError> {
        let mut acc = self.factor()?;
        loop {
            let spaced = self.skip_spaces();
            let divide = if self.eat("/") {
                true
            } else if self.eat("**") {
                // `**` right after a factor was consumed as an exponent.
                return Err(ParseError::UnknownUnit);
            } else if ["*", "·", "⋅", "×"].iter().any(|op| self.eat(op))
                || (spaced && self.peek().is_some_and(|c| c != ')'))
            {
                // An explicit multiplication sign, or a space between factors.
                false
            } else {
                return Ok(acc);
            };
            self.skip_spaces();
            let rhs = self.factor()?;
            acc = acc.mul(if divide { rhs.powi(-1) } else { rhs });
        }
    }

    /// `primary exponent?`.
    fn factor(&mut self) -> Result<UnitExpr, ParseError> {
        let base = self.primary()?;
        Ok(match self.exponent()? {
            Some(n) => base.powi(n),
            None => base,
        })
    }

    fn primary(&mut self) -> Result<UnitExpr, ParseError> {
        if self.eat("(") {
            self.skip_spaces();
            let inner = self.expr()?;
            self.skip_spaces();
            return if self.eat(")") {
                Ok(inner)
            } else {
                Err(ParseError::UnknownUnit)
            };
        }
        if self.eat("1") {
            return Ok(UnitExpr::ONE);
        }
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|c| c.is_alphabetic() || matches!(c, '°' | 'Å'))
        {
            self.bump();
        }
        lookup(&self.s[start..self.pos]).ok_or(ParseError::UnknownUnit)
    }

    /// `^n`, `**n` or superscript digits, with an optional minus sign.
    fn exponent(&mut self) -> Result<Option<i8>, ParseError> {
        // Spaces may surround `^` (`s ^ 2`), but a space alone separates factors.
        let before_spaces = self.pos;
        self.skip_spaces();
        if self.eat("^") || self.eat("**") {
            self.skip_spaces();
            let negative = self.eat("-") || self.eat("−");
            if !negative {
                self.eat("+");
            }
            let start = self.pos;
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.bump();
            }
            let n: i8 = self.s[start..self.pos]
                .parse()
                .map_err(|_| ParseError::UnknownUnit)?;
            return Ok(Some(if negative { -n } else { n }));
        }
        self.pos = before_spaces;
        let negative = self.eat("⁻");
        let mut n: Option<i8> = None;
        while let Some(d) = self
            .peek()
            .and_then(|c| SUPERSCRIPT_DIGITS.iter().position(|&s| s == c))
        {
            self.bump();
            let digit = i8::try_from(d).map_err(|_| ParseError::UnknownUnit)?;
            n = Some(
                n.unwrap_or(0)
                    .checked_mul(10)
                    .and_then(|v| v.checked_add(digit))
                    .ok_or(ParseError::UnknownUnit)?,
            );
        }
        match (negative, n) {
            (true, None) => Err(ParseError::UnknownUnit),
            (true, Some(n)) => Ok(Some(-n)),
            (false, n) => Ok(n),
        }
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
