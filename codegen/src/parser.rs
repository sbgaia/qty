//! Parses UnitsNet JSON definition files into an internal representation,
//! including SI prefix expansion.

use serde::Deserialize;

use crate::expr::{self, Const, Form};
use crate::formula;

/// Metadata for a single SI prefix.
pub struct PrefixInfo {
    /// PascalCase name as it appears in the JSON `Prefixes` array (such as `"Kilo"`).
    pub name: &'static str,
    /// Symbol prepended to the base-unit abbreviation (such as `"k"`).
    pub symbol: &'static str,
    /// Exact multiplier relative to the base unit, as a decimal literal (such as `"1e3"`).
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
    // IEC binary prefixes, 2^10 to 2^60, used by BitRate and Information.
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
    /// The SI-system units the unit is made of, by dimension: `{"L": "Meter"}`.
    #[serde(default)]
    pub base_units: Option<std::collections::BTreeMap<String, String>>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct RawLocalization {
    pub culture: String,
    pub abbreviations: Vec<String>,
}

/// One quantity, with its prefixed units expanded.
pub struct QuantityDef {
    /// The name, which is also the type's: `Length`.
    pub name: String,
    /// The base unit's variant: `Meter`.
    pub base_unit: String,
    /// UnitsNet's description, for the struct's docs.
    pub doc: String,
    /// Every unit, prefixed ones included.
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

/// One unit, possibly a prefixed form of an upstream one.
pub struct UnitDef {
    /// The variant name: `KilometerPerSecond`.
    pub singular: String,
    /// The plural, for method names: `KilometersPerSecond`.
    pub plural: String,
    /// Exact conversion from this unit to the base unit.
    pub to_base: Form,
    /// Exact conversion from the base unit to this unit (inverse of `to_base`).
    pub from_base: Form,
    /// UnitsNet's to-base formula, translated verbatim into a Rust `f64`
    /// expression of `x`. The generated tests use it as an oracle.
    pub oracle_to_base: String,
    /// The upstream from-base formula, translated verbatim (test oracle).
    pub oracle_from_base: String,
    /// The abbreviation used for display: `"km/s"`.
    pub abbreviation: String,
    /// Every abbreviation `FromStr` accepts.
    pub all_abbreviations: Vec<String>,
    /// Docs for the variant, if UnitsNet has any.
    pub doc: Option<String>,
    /// The unprefixed upstream unit: `Gram` for `Kilogram`, `Milligram`
    /// and `Gram` itself.
    pub family: String,
    /// Power of ten of the SI prefix (`3` for kilo, `0` for the unprefixed
    /// unit); `None` for binary prefixes (kibi, mebi, …).
    pub decimal_exp: Option<i32>,
    /// UnitsNet's `BaseUnits`, by dimension symbol. Only upstream units
    /// have them, not their prefixed forms.
    pub base_units: Option<std::collections::BTreeMap<String, String>>,
}

/// Parses a UnitsNet definition file, expanding the prefixes.
pub fn parse(path: &std::path::Path) -> QuantityDef {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    // Some upstream files start with a UTF-8 BOM (\u{FEFF}); serde_json rejects it.
    let content = content.strip_prefix('\u{FEFF}').unwrap_or(&content);
    // Some upstream files have trailing commas, which serde_json rejects.
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
            base_units: raw_unit.base_units.clone(),
        });

        // Prefix-expanded variants.
        for prefix_name in &raw_unit.prefixes {
            let prefix = PREFIX_TABLE
                .iter()
                .find(|p| p.name == prefix_name)
                .unwrap_or_else(|| panic!("unknown prefix '{prefix_name}'"));

            // UnitsNet only prefixes linear units, for which prefixing the
            // input is exact: f(x · prefix).
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
                base_units: None,
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

/// The en-US abbreviations of `unit`, or the first culture's if it has none.
fn en_us_abbreviations(unit: &RawUnit) -> Vec<String> {
    unit.localization
        .iter()
        .find(|l| l.culture == "en-US")
        .or_else(|| unit.localization.first())
        .map(|l| l.abbreviations.clone())
        .unwrap_or_default()
}

/// Corrections to upstream abbreviations, as `(quantity, unit, wrong prefix
/// of each abbreviation, correct prefix)`. The generated test that reads
/// every abbreviation as a unit expression found them.
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

/// Removes trailing commas before `}` or `]`. Some UnitsNet files have them,
/// and they're the only thing in those files that `serde_json` rejects.
fn strip_trailing_commas(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for (i, c) in src.char_indices() {
        let closes = || src[i + 1..].trim_start().starts_with(['}', ']']);
        if c != ',' || !closes() {
            out.push(c);
        }
    }
    out
}

/// The prefixed name: `Kilometer`, not `KiloMeter`.
fn prefix_expanded_name(prefix_name: &str, base_name: &str) -> String {
    let mut chars = base_name.chars();
    match chars.next() {
        None => prefix_name.to_owned(),
        Some(first) => {
            format!("{}{}{}", prefix_name, first.to_lowercase(), chars.as_str())
        }
    }
}
