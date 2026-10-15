//! Parses UnitsNet JSON definition files into an internal representation,
//! including SI prefix expansion.

use serde::Deserialize;

use crate::expr::{self, Const, Form};
use crate::formula;

// ── SI prefix table ──────────────────────────────────────────────────────────

/// Metadata for a single SI prefix.
pub struct PrefixInfo {
    /// PascalCase name as it appears in the JSON `Prefixes` array (e.g. `"Kilo"`).
    pub name: &'static str,
    /// Symbol prepended to the base-unit abbreviation (e.g. `"k"`).
    pub symbol: &'static str,
    /// Exact multiplier relative to the base unit, as a decimal literal (e.g. `"1e3"`).
    pub factor: &'static str,
}

/// All SI and IEC binary prefixes recognised by UnitsNet definition files, in ascending order.
pub const PREFIX_TABLE: &[PrefixInfo] = &[
    // SI decimal prefixes.
    PrefixInfo {
        name: "Femto",
        symbol: "f",
        factor: "1e-15",
    },
    PrefixInfo {
        name: "Pico",
        symbol: "p",
        factor: "1e-12",
    },
    PrefixInfo {
        name: "Nano",
        symbol: "n",
        factor: "1e-9",
    },
    PrefixInfo {
        name: "Micro",
        symbol: "µ",
        factor: "1e-6",
    },
    PrefixInfo {
        name: "Milli",
        symbol: "m",
        factor: "1e-3",
    },
    PrefixInfo {
        name: "Centi",
        symbol: "c",
        factor: "1e-2",
    },
    PrefixInfo {
        name: "Deci",
        symbol: "d",
        factor: "1e-1",
    },
    PrefixInfo {
        name: "Deca",
        symbol: "da",
        factor: "1e1",
    },
    PrefixInfo {
        name: "Hecto",
        symbol: "h",
        factor: "1e2",
    },
    PrefixInfo {
        name: "Kilo",
        symbol: "k",
        factor: "1e3",
    },
    PrefixInfo {
        name: "Mega",
        symbol: "M",
        factor: "1e6",
    },
    PrefixInfo {
        name: "Giga",
        symbol: "G",
        factor: "1e9",
    },
    PrefixInfo {
        name: "Tera",
        symbol: "T",
        factor: "1e12",
    },
    PrefixInfo {
        name: "Peta",
        symbol: "P",
        factor: "1e15",
    },
    PrefixInfo {
        name: "Exa",
        symbol: "E",
        factor: "1e18",
    },
    // IEC binary prefixes (2^10, 2^20, …, 2^60) — used by BitRate / Information.
    PrefixInfo {
        name: "Kibi",
        symbol: "Ki",
        factor: "1024",
    }, // 2^10
    PrefixInfo {
        name: "Mebi",
        symbol: "Mi",
        factor: "1048576",
    }, // 2^20
    PrefixInfo {
        name: "Gibi",
        symbol: "Gi",
        factor: "1073741824",
    }, // 2^30
    PrefixInfo {
        name: "Tebi",
        symbol: "Ti",
        factor: "1099511627776",
    }, // 2^40
    PrefixInfo {
        name: "Pebi",
        symbol: "Pi",
        factor: "1125899906842624",
    }, // 2^50
    PrefixInfo {
        name: "Exbi",
        symbol: "Ei",
        factor: "1152921504606846976",
    }, // 2^60
];

// ── Raw JSON deserialization ──────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct RawQuantity {
    pub name: String,
    pub base_unit: String,
    #[serde(default)]
    pub xml_doc_summary: String,
    /// Exponents of the SI base dimensions, e.g. `{"L": 1, "T": -1}`;
    /// absent for dimensionless quantities.
    #[serde(default)]
    pub base_dimensions: std::collections::BTreeMap<String, i32>,
    pub units: Vec<RawUnit>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct RawUnit {
    pub singular_name: String,
    pub plural_name: String,
    pub from_unit_to_base_func: String,
    pub from_base_to_unit_func: String,
    #[serde(default)]
    pub prefixes: Vec<String>,
    #[serde(default)]
    pub xml_doc_summary: Option<String>,
    #[serde(default)]
    pub localization: Vec<RawLocalization>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct RawLocalization {
    pub culture: String,
    pub abbreviations: Vec<String>,
}

// ── Internal model ────────────────────────────────────────────────────────────

/// Fully-resolved definition for a single physical quantity (e.g. `Length`).
pub struct QuantityDef {
    /// PascalCase quantity name, used as the Rust struct name (e.g. `Length`).
    pub name: String,
    /// PascalCase name of the SI base unit variant (e.g. `Meter`).
    pub base_unit: String,
    /// Doc comment for the generated struct.
    pub doc: String,
    /// All unit variants, including prefix expansions.
    pub units: Vec<UnitDef>,
    /// Exponents of the SI base dimensions, in the order of [`DIMENSIONS`].
    pub dimensions: Dimensions,
}

/// The SI base dimensions, in the order used by [`Dimensions`]: length,
/// mass, time, electric current, temperature, amount of substance,
/// luminous intensity.
pub const DIMENSIONS: [&str; 7] = ["L", "M", "T", "I", "Θ", "N", "J"];

/// Exponents of the SI base dimensions.
pub type Dimensions = [i32; 7];

/// A single unit variant (possibly prefix-expanded).
pub struct UnitDef {
    /// PascalCase variant name used in the enum (e.g. `KilometerPerSecond`).
    pub singular: String,
    /// PascalCase plural name used to derive method names (e.g. `KilometersPerSecond`).
    pub plural: String,
    /// Exact conversion from this unit to the base unit.
    pub to_base: Form,
    /// Exact conversion from the base unit to this unit (inverse of `to_base`).
    pub from_base: Form,
    /// The upstream to-base formula translated verbatim into a Rust `f64`
    /// expression of `x`; an independent oracle for the generated tests.
    pub oracle_to_base: String,
    /// The upstream from-base formula, translated verbatim (test oracle).
    pub oracle_from_base: String,
    /// Primary display abbreviation (e.g. `"km/s"`).
    pub abbreviation: String,
    /// All recognised abbreviations for `FromStr` parsing.
    pub all_abbreviations: Vec<String>,
    /// Optional doc comment for the enum variant.
    pub doc: Option<String>,
    /// The upstream (unprefixed) unit this one was expanded from, e.g.
    /// `Gram` for `Kilogram` and `Milligram`, and `Gram` for `Gram` itself.
    pub family: String,
    /// Power of ten of the SI prefix (`3` for kilo, `0` for the unprefixed
    /// unit); `None` for binary prefixes (kibi, mebi, …).
    pub decimal_exp: Option<i32>,
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parses a UnitsNet JSON definition file into a [`QuantityDef`] with all prefixes expanded.
pub fn parse(path: &std::path::Path) -> QuantityDef {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    // Some upstream files start with a UTF-8 BOM (\u{FEFF}); serde_json rejects it.
    let content = content.strip_prefix('\u{FEFF}').unwrap_or(&content);
    // Some upstream files contain trailing commas (valid JSON5, invalid JSON).
    // Strip them before handing to serde_json.
    let content = strip_trailing_commas(content);
    let mut raw: RawQuantity = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()));
    apply_abbreviation_fixes(&mut raw);

    let mut units = Vec::new();

    for raw_unit in &raw.units {
        let to_base = expr::parse(&raw_unit.from_unit_to_base_func);
        let from_base = expr::parse(&raw_unit.from_base_to_unit_func);
        check_inverse(&raw.name, &raw_unit.singular_name, &to_base, &from_base);

        let abbreviations = en_us_abbreviations(raw_unit);
        let abbreviation = abbreviations.first().cloned().unwrap_or_default();

        // Base (non-prefixed) unit.
        units.push(UnitDef {
            singular: raw_unit.singular_name.clone(),
            plural: raw_unit.plural_name.clone(),
            to_base: to_base.clone(),
            from_base: to_base.inverse(),
            oracle_to_base: formula::translate(&raw_unit.from_unit_to_base_func, "x"),
            oracle_from_base: formula::translate(&raw_unit.from_base_to_unit_func, "x"),
            abbreviation: abbreviation.clone(),
            all_abbreviations: abbreviations,
            doc: raw_unit.xml_doc_summary.clone(),
            family: raw_unit.singular_name.clone(),
            decimal_exp: Some(0),
        });

        // Prefix-expanded variants.
        for prefix_name in &raw_unit.prefixes {
            let prefix = PREFIX_TABLE
                .iter()
                .find(|p| p.name == prefix_name)
                .unwrap_or_else(|| panic!("unknown prefix '{prefix_name}'"));

            // Construct names by lowercasing the first letter of the base name so that
            // `heck::to_snake_case` produces clean output: e.g. `Kilo` + `meter` = `Kilometer`
            // → snake_case → `kilometer`.
            // UnitsNet only applies prefixes to linear units, for which
            // prefixing the input is exact: f(x · prefix).
            assert!(
                to_base.is_linear(),
                "{}.{}: prefixes on an affine unit",
                raw.name,
                raw_unit.singular_name
            );
            let prefixed = to_base
                .clone()
                .with_input_factor(&Const::parse_decimal(prefix.factor));

            let singular = prefix_expanded_name(prefix.name, &raw_unit.singular_name);
            let plural = prefix_expanded_name(prefix.name, &raw_unit.plural_name);

            let prefixed_abbr = if abbreviation.is_empty() {
                prefix.symbol.to_string()
            } else {
                format!("{}{}", prefix.symbol, abbreviation)
            };

            units.push(UnitDef {
                singular,
                plural,
                to_base: prefixed.clone(),
                from_base: prefixed.inverse(),
                oracle_to_base: formula::translate_with_factor(
                    &raw_unit.from_unit_to_base_func,
                    prefix.factor,
                    true,
                ),
                oracle_from_base: formula::translate_with_factor(
                    &raw_unit.from_base_to_unit_func,
                    prefix.factor,
                    false,
                ),
                abbreviation: prefixed_abbr.clone(),
                all_abbreviations: vec![prefixed_abbr],
                doc: None,
                family: raw_unit.singular_name.clone(),
                decimal_exp: prefix
                    .factor
                    .strip_prefix("1e")
                    .map(|e| e.parse().expect("decimal prefix exponent")),
            });
        }
    }

    let mut dimensions = [0; 7];
    for (symbol, exponent) in &raw.base_dimensions {
        let i = DIMENSIONS
            .iter()
            .position(|d| d == symbol)
            .unwrap_or_else(|| panic!("{}: unknown base dimension {symbol:?}", raw.name));
        dimensions[i] = *exponent;
    }

    QuantityDef {
        name: raw.name,
        base_unit: raw.base_unit,
        doc: raw.xml_doc_summary,
        units,
        dimensions,
    }
}

/// Warns when the upstream from-base formula is not the exact inverse of the
/// to-base formula. The to-base formula is authoritative: the generated
/// from-base conversion is always its exact inverse.
fn check_inverse(quantity: &str, unit: &str, to_base: &Form, from_base: &Form) {
    let diff = to_base.inverse().rel_diff(from_base);
    if diff > 1e-15 {
        eprintln!(
            "codegen: note: {quantity}.{unit}: upstream from-base formula differs from the exact \
             inverse of the to-base formula by {diff:.1e} (relative); using the exact inverse"
        );
    }
    assert!(
        diff < 1e-6,
        "{quantity}.{unit}: to-base and from-base formulas disagree"
    );
}

/// Returns the en-US abbreviations for `unit`, falling back to the first available culture.
fn en_us_abbreviations(unit: &RawUnit) -> Vec<String> {
    unit.localization
        .iter()
        .find(|l| l.culture == "en-US")
        .or_else(|| unit.localization.first())
        .map(|l| l.abbreviations.clone())
        .unwrap_or_default()
}

/// Corrections to upstream abbreviations: `(quantity, unit, wrong prefix of
/// each abbreviation, correct prefix)`. Each was found by the generated test
/// that parses every abbreviation as a unit expression and compares it with
/// the unit's conversion.
///
/// - HeatTransferCoefficient `CaloriePerHourSquareMeterDegreeCelsius`
///   converts as calories (`× 4.184 / 3600`) but is abbreviated `kcal/…`, so
///   `"1 kcal/(h·m²·°C)"` parsed 1000× too small and its kilo variant got
///   `kkcal/…`.
const ABBREVIATION_FIXES: &[(&str, &str, &str, &str)] = &[(
    "HeatTransferCoefficient",
    "CaloriePerHourSquareMeterDegreeCelsius",
    "kcal/",
    "cal/",
)];

/// Applies [`ABBREVIATION_FIXES`]; panics when one no longer matches, so
/// that it is removed once upstream is fixed.
fn apply_abbreviation_fixes(raw: &mut RawQuantity) {
    for &(quantity, unit, wrong, right) in ABBREVIATION_FIXES {
        if raw.name != quantity {
            continue;
        }
        let abbreviations = raw
            .units
            .iter_mut()
            .filter(|u| u.singular_name == unit)
            .flat_map(|u| u.localization.iter_mut())
            .filter(|l| l.culture == "en-US")
            .flat_map(|l| l.abbreviations.iter_mut());
        let mut fixed = 0;
        for abbreviation in abbreviations {
            if let Some(rest) = abbreviation.strip_prefix(wrong) {
                *abbreviation = format!("{right}{rest}");
                fixed += 1;
            }
        }
        assert!(
            fixed > 0,
            "abbreviation fix for {quantity}.{unit} no longer applies: remove it"
        );
    }
}

/// Removes trailing commas before `}` or `]` to accept JSON5-style upstream files.
///
/// Some UnitsNet definition files contain trailing commas which are valid JSON5
/// but rejected by `serde_json`. This performs a lightweight fix without a full
/// JSON5 parser, handling the only pattern present in the UnitsNet sources.
fn strip_trailing_commas(src: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r",(\s*[}\]])").unwrap());
    re.replace_all(src, "$1").into_owned()
}

/// Constructs a prefix-expanded name by lowercasing the first character of `base_name`
/// and prepending `prefix_name`, producing e.g. `Kilometer` instead of `KiloMeter`.
fn prefix_expanded_name(prefix_name: &str, base_name: &str) -> String {
    let mut chars = base_name.chars();
    match chars.next() {
        None => prefix_name.to_owned(),
        Some(first) => {
            format!("{}{}{}", prefix_name, first.to_lowercase(), chars.as_str())
        }
    }
}
