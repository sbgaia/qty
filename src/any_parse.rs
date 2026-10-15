//! Parsing, formatting and errors for runtime quantities (hand-written; the
//! rest of the module is generated).

use core::fmt;
use core::str::FromStr;

use super::{AnyQuantity, AnyUnit, QuantityKind};
use crate::{split_value_unit, ParseError, UnitExpr};

/// A quantity or unit of one kind was used where another kind was expected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KindMismatch {
    /// The kind that was expected.
    pub expected: QuantityKind,
    /// The kind that was given.
    pub found: QuantityKind,
}

impl fmt::Display for KindMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "expected a {}, found a {}", self.expected, self.found)
    }
}

impl core::error::Error for KindMismatch {}

/// The name: `Length`.
impl fmt::Display for QuantityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.info().name)
    }
}

/// From the name, ignoring case: `"length"`, `"MassFlow"`.
impl FromStr for QuantityKind {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, ParseError> {
        Self::all()
            .find(|k| k.info().name.eq_ignore_ascii_case(s))
            .ok_or(ParseError::UnknownUnit)
    }
}

/// The abbreviation: `km`.
impl fmt::Display for AnyUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.abbreviation())
    }
}

impl crate::Unit for AnyUnit {
    #[inline]
    fn abbreviation(self) -> &'static str {
        AnyUnit::abbreviation(self)
    }
    #[inline]
    fn info(self) -> &'static crate::UnitInfo {
        AnyUnit::info(self)
    }
}

impl AnyUnit {
    /// Every unit, of any kind, that `s` names (an abbreviation or a name):
    /// `"m"` is both the metre and the minute.
    pub fn candidates(s: &str) -> impl Iterator<Item = Self> + '_ {
        QuantityKind::all().filter_map(move |kind| Self::parse_in(kind, s).ok())
    }
}

impl AnyQuantity {
    /// Every reading of `s` as a quantity of some kind. Units listed for a
    /// quantity (abbreviations and names) come first: if any kind lists the
    /// unit, only those readings are returned; otherwise the readings of
    /// the unit as an expression ([`UnitExpr`]).
    ///
    /// ```rust
    /// use unitsnet_rs::*;
    ///
    /// // "m" is a metre and a minute.
    /// let kinds: Vec<QuantityKind> = AnyQuantity::candidates("5 m").map(|q| q.kind()).collect();
    /// assert_eq!(kinds, [QuantityKind::Duration, QuantityKind::Length]);
    /// ```
    pub fn candidates(s: &str) -> impl Iterator<Item = Self> + '_ {
        let (number, unit) = split_value_unit(s);
        let value: Option<f64> = number.parse().ok();
        let listed =
            value.is_some_and(|v| QuantityKind::all().any(|k| Self::listed(k, v, unit).is_some()));
        let expr = if listed {
            None
        } else {
            UnitExpr::parse(unit).ok()
        };
        QuantityKind::all().filter_map(move |kind| {
            let value = value?;
            if listed {
                Self::listed(kind, value, unit)
            } else {
                Self::expressed(kind, value, expr?)
            }
        })
    }

    /// The physical dimensions.
    pub fn dimensions(&self) -> crate::Dimensions {
        self.kind().info().dimensions
    }

    /// The unit values of this kind are stored in.
    pub fn base_unit(&self) -> AnyUnit {
        self.kind().base_unit()
    }
}

/// Parses a number and a unit when the unit determines the kind:
/// `"5 km"` is a length. A unit used by several quantities (`"5 m"`: metre
/// or minute) gives [`ParseError::Ambiguous`]; use
/// [`parse_as`](AnyQuantity::parse_as) or [`candidates`](AnyQuantity::candidates).
impl FromStr for AnyQuantity {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, ParseError> {
        let (number, _) = split_value_unit(s);
        number
            .parse::<f64>()
            .map_err(|_| ParseError::InvalidNumber)?;
        let mut readings = Self::candidates(s);
        match (readings.next(), readings.next()) {
            (Some(q), None) => Ok(q),
            (Some(_), Some(_)) => Err(ParseError::Ambiguous),
            (None, _) => Err(ParseError::UnknownUnit),
        }
    }
}
