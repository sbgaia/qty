//! Translates C# arithmetic expressions (UnitsNet format) verbatim into Rust `f64`
//! expressions. Used only to emit independent oracles in the generated tests; the
//! library code itself uses the exact, folded forms from [`crate::expr`].

/// Rewrites the number literals in `s` as `f64` literals: `40d` becomes
/// `40.0` and `365` becomes `365.0`, while `1.5` and `1e-3` stay as they
/// are. Digits inside identifiers are left alone.
fn float_literals(s: &str) -> String {
    let bytes = s.as_bytes();
    let word = |i: usize| {
        bytes
            .get(i)
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
    };
    let digits = |mut i: usize| {
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        i
    };
    let mut out = String::with_capacity(s.len() + 8);
    let mut i = 0;
    while i < bytes.len() {
        let starts_number = bytes[i].is_ascii_digit() && (i == 0 || !word(i - 1));
        if !starts_number {
            let c = s[i..].chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
            continue;
        }
        let start = i;
        i = digits(i);
        let mut integer = true;
        if bytes.get(i) == Some(&b'.') {
            integer = false;
            i = digits(i + 1);
        }
        if matches!(bytes.get(i), Some(b'e' | b'E')) {
            let sign = usize::from(matches!(bytes.get(i + 1), Some(b'+' | b'-')));
            if bytes.get(i + 1 + sign).is_some_and(u8::is_ascii_digit) {
                integer = false;
                i = digits(i + 1 + sign);
            }
        }
        out.push_str(&s[start..i]);
        let suffix = bytes.get(i) == Some(&b'd') && !word(i + 1);
        if suffix {
            i += 1;
        }
        if integer && (suffix || !word(i)) {
            out.push_str(".0");
        }
    }
    out
}

/// Translates a C# expression from a UnitsNet file into a Rust `f64`
/// expression of `var`. `Math.PI` becomes `std::f64::consts::PI`, and every
/// number becomes an `f64` literal.
pub fn translate(raw: &str, var: &str) -> String {
    let s = raw.replace("{x}", var);
    let s = s.replace("Math.PI", "std::f64::consts::PI");
    float_literals(&s)
}

/// Translates a formula for a prefixed unit, substituting `x * factor` for
/// `{x}` (or `x / factor` from the base unit). That's right for linear
/// formulas, the only ones UnitsNet prefixes.
pub fn translate_with_factor(raw: &str, factor: &str, to_base: bool) -> String {
    // Scientific notation is always a valid `f64` literal.
    let factor: f64 = factor.parse().expect("prefix factor");
    let factor_str = format!("{factor:e}");
    let substituted = if to_base {
        raw.replace("{x}", &format!("(x * {factor_str})"))
    } else {
        raw.replace("{x}", &format!("(x / {factor_str})"))
    };
    let substituted = substituted.replace("Math.PI", "std::f64::consts::PI");
    float_literals(&substituted)
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
        let result = translate_with_factor("{x}", "1e3", true);
        assert_eq!(result, "(x * 1e3)");
    }

    #[test]
    fn prefix_from_base() {
        let result = translate_with_factor("{x}", "1e3", false);
        assert_eq!(result, "(x / 1e3)");
    }
}
