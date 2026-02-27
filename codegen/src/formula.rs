//! Translates C# arithmetic expressions (UnitsNet format) into valid Rust `f64` expressions.

use std::sync::OnceLock;

use regex::Regex;

/// Returns the compiled regex for stripping the C# `d` double literal suffix.
fn double_suffix_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(\d)d\b").unwrap())
}

/// Returns the compiled regex for promoting bare integer literals to `f64`.
///
/// Alternation is left-to-right: float and scientific-notation patterns are tried first,
/// so they are preserved as-is. Only bare integers (group 1) are promoted.
fn int_promote_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\d+\.\d*(?:[eE][+-]?\d+)?|\d+[eE][+-]?\d+|\b(\d+)\b").unwrap())
}

/// Promotes any bare integer literal in `s` to an `f64` literal (appends `.0`).
fn promote_integers(s: &str) -> String {
    int_promote_re()
        .replace_all(s, |caps: &regex::Captures| match caps.get(1) {
            Some(m) => format!("{}.0", m.as_str()),
            None => caps[0].to_string(),
        })
        .into_owned()
}

/// Translates a raw C# expression from a UnitsNet definition file into a valid Rust `f64`
/// expression, with `{x}` replaced by `var`.
///
/// Steps applied in order:
/// 1. `{x}` → `var`
/// 2. `Math.PI` → `std::f64::consts::PI`
/// 3. C# `double` literal suffix stripped: `40d` → `40.0`
/// 4. Bare integer literals promoted to `f64`: `365` → `365.0`
pub fn translate(raw: &str, var: &str) -> String {
    let s = raw.replace("{x}", var);
    let s = s.replace("Math.PI", "std::f64::consts::PI");
    let s = double_suffix_re().replace_all(&s, "${1}.0");
    promote_integers(&s)
}

/// Translates the to-base or from-base formula for a **prefixed** variant of a unit.
///
/// For `to_base = true`:  substitutes `{x}` with `(x * factor)` before translating.
/// For `to_base = false`: substitutes `{x}` with `(x / factor)` before translating.
///
/// This correctly handles all linear (non-affine) formulas, which is the only kind that
/// UnitsNet applies SI prefixes to.
pub fn translate_with_factor(raw: &str, factor: f64, to_base: bool) -> String {
    // Format factor in scientific notation so it is always a valid Rust `f64` literal.
    let factor_str = format!("{factor:e}");
    let substituted = if to_base {
        raw.replace("{x}", &format!("(x * {factor_str})"))
    } else {
        raw.replace("{x}", &format!("(x / {factor_str})"))
    };
    let substituted = substituted.replace("Math.PI", "std::f64::consts::PI");
    let s = double_suffix_re().replace_all(&substituted, "${1}.0");
    promote_integers(&s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_formula() {
        assert_eq!(translate("{x}", "x"), "x");
    }

    #[test]
    fn integer_literals_promoted() {
        assert_eq!(
            translate("{x} * 365 * 24 * 3600", "x"),
            "x * 365.0 * 24.0 * 3600.0"
        );
    }

    #[test]
    fn float_literals_preserved() {
        assert_eq!(translate("{x} * 273.15", "x"), "x * 273.15");
    }

    #[test]
    fn scientific_notation_preserved() {
        assert_eq!(
            translate("{x} * 1.602176634e-19", "x"),
            "x * 1.602176634e-19"
        );
    }

    #[test]
    fn csharp_double_suffix_stripped() {
        assert_eq!(translate("{x} * 3600d", "x"), "x * 3600.0");
    }

    #[test]
    fn mixed_formula() {
        // Fahrenheit to Kelvin: integers promoted, floats left alone
        let result = translate("{x} * 5 / 9 + 459.67 * 5 / 9", "x");
        assert_eq!(result, "x * 5.0 / 9.0 + 459.67 * 5.0 / 9.0");
    }

    #[test]
    fn prefix_to_base() {
        let result = translate_with_factor("{x}", 1e3_f64, true);
        assert_eq!(result, "(x * 1e3)");
    }

    #[test]
    fn prefix_from_base() {
        let result = translate_with_factor("{x}", 1e3_f64, false);
        assert_eq!(result, "(x / 1e3)");
    }
}
