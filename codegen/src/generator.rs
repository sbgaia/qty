//! Converts a [`QuantityDef`] into a complete Rust source file as a `String`,
//! and emits the cross-type arithmetic and the generated tests.
//!
//! Everything that is identical across quantities (operators, slice views,
//! `Display`, `FromStr`, …) lives in the `quantity!` macro in `src/lib.rs`;
//! the generated files only contain what differs: the unit enum and the
//! conversion methods.

use crate::case::ToSnakeCase;

use crate::expr::{self, Direction, Form};
use crate::parser::{QuantityDef, UnitDef};
use crate::si;

// Appends a line to `out`. It always goes through `format!`, so `{{` and
// `}}` in a literal come out as `{` and `}`.
macro_rules! w {
    ($out:expr) => { $out.push('\n'); };
    ($out:expr, $($arg:tt)+) => {
        $out.push_str(&format!($($arg)+));
        $out.push('\n');
    };
}

/// Quantities grouped into Cargo features, so that users who need a few of
/// them don't compile all 128. Quantities in no group are always compiled,
/// and `full` (on by default) enables every group.
pub const GROUPS: &[(&str, &[&str])] = &[
    (
        "mechanics",
        &[
            "AreaDensity",
            "AreaMomentOfInertia",
            "Compressibility",
            "EnergyDensity",
            "ForceChangeRate",
            "ForcePerLength",
            "Impulse",
            "Jerk",
            "LinearDensity",
            "LinearPowerDensity",
            "MassMomentOfInertia",
            "PowerDensity",
            "PressureChangeRate",
            "RatioChangeRate",
            "ReciprocalArea",
            "ReciprocalLength",
            "RotationalAcceleration",
            "RotationalSpeed",
            "RotationalStiffness",
            "RotationalStiffnessPerLength",
            "SpecificEnergy",
            "SpecificVolume",
            "SpecificWeight",
            "Torque",
            "VolumePerLength",
            "WarpingMomentOfInertia",
        ],
    ),
    (
        "fluids",
        &[
            "DynamicViscosity",
            "FluidResistance",
            "KinematicViscosity",
            "LeakRate",
            "MassFlow",
            "MassFlux",
            "PorousMediumPermeability",
            "StandardVolumeFlow",
            "VolumeFlow",
            "VolumeFlowPerArea",
        ],
    ),
    (
        "electrical",
        &[
            "AmplitudeRatio",
            "ElectricAdmittance",
            "ElectricApparentEnergy",
            "ElectricApparentPower",
            "ElectricCapacitance",
            "ElectricCharge",
            "ElectricChargeDensity",
            "ElectricConductance",
            "ElectricConductivity",
            "ElectricCurrent",
            "ElectricCurrentDensity",
            "ElectricCurrentGradient",
            "ElectricField",
            "ElectricImpedance",
            "ElectricInductance",
            "ElectricPotential",
            "ElectricPotentialChangeRate",
            "ElectricReactance",
            "ElectricReactiveEnergy",
            "ElectricReactivePower",
            "ElectricResistance",
            "ElectricResistivity",
            "ElectricSurfaceChargeDensity",
            "ElectricSusceptance",
            "Level",
            "MagneticField",
            "MagneticFlux",
            "Magnetization",
            "Permeability",
            "Permittivity",
            "PowerRatio",
        ],
    ),
    (
        "thermal",
        &[
            "CoefficientOfThermalExpansion",
            "Entropy",
            "HeatFlux",
            "HeatTransferCoefficient",
            "SpecificEntropy",
            "TemperatureChangeRate",
            "TemperatureGradient",
            "ThermalConductivity",
            "ThermalInsulance",
            "ThermalResistance",
            "VolumetricHeatCapacity",
        ],
    ),
    (
        "chemistry",
        &[
            "AmountOfSubstance",
            "MassConcentration",
            "MassFraction",
            "Molality",
            "MolarEnergy",
            "MolarEntropy",
            "MolarFlow",
            "MolarMass",
            "Molarity",
            "VolumeConcentration",
        ],
    ),
    (
        "light",
        &[
            "Illuminance",
            "Irradiance",
            "Irradiation",
            "Luminance",
            "Luminosity",
            "LuminousFlux",
            "LuminousIntensity",
            "SolidAngle",
        ],
    ),
    (
        "radiation",
        &[
            "AbsorbedDoseOfIonizingRadiation",
            "DoseAreaProduct",
            "RadiationEquivalentDose",
            "RadiationEquivalentDoseRate",
            "RadiationExposure",
            "Radioactivity",
        ],
    ),
    ("information", &["BitRate", "Information"]),
    (
        "misc",
        &[
            "BrakeSpecificFuelConsumption",
            "FuelEfficiency",
            "RelativeHumidity",
            "SpecificFuelConsumption",
            "Turbidity",
            "VitaminA",
        ],
    ),
];

/// Quantities compiled whatever the features.
pub const CORE: &[&str] = &[
    "Acceleration",
    "Angle",
    "Area",
    "Density",
    "Duration",
    "Energy",
    "Force",
    "Frequency",
    "Length",
    "Mass",
    "Power",
    "Pressure",
    "Ratio",
    "Scalar",
    "Speed",
    "Temperature",
    "TemperatureDelta",
    "Volume",
];

/// The feature group of a quantity; `None` for core quantities. Quantities
/// added upstream and not yet classified land in `misc`.
pub fn group(name: &str) -> Option<&'static str> {
    if CORE.contains(&name) {
        return None;
    }
    Some(
        GROUPS
            .iter()
            .find(|(_, members)| members.contains(&name))
            .map_or_else(
                || {
                    eprintln!("codegen: note: {name} is in no feature group; using `misc`");
                    "misc"
                },
                |(g, _)| *g,
            ),
    )
}

/// `#[cfg(...)]` requiring the groups of all `quantities`, or `""` when they
/// are all core.
pub fn cfg_for(quantities: &[&str]) -> String {
    let mut groups: Vec<&str> = quantities.iter().filter_map(|q| group(q)).collect();
    groups.sort_unstable();
    groups.dedup();
    match groups.as_slice() {
        [] => String::new(),
        [g] => format!("#[cfg(feature = \"{g}\")]"),
        gs => format!(
            "#[cfg(all({}))]",
            gs.iter()
                .map(|g| format!("feature = \"{g}\""))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// A quantity that is a point on a scale rather than an amount, and the
/// quantity for the difference between two points. UnitsNet doesn't mark
/// these, so the list lives here.
pub struct PointQuantity {
    pub point: &'static str,
    pub delta: &'static str,
    /// Why two points cannot be added, for the docs.
    pub example: &'static str,
    /// Units used in the generated doc example (plural, snake case).
    pub point_unit: &'static str,
    pub delta_unit: &'static str,
}

pub const POINT_QUANTITIES: &[PointQuantity] = &[
    PointQuantity {
        point: "Temperature",
        delta: "TemperatureDelta",
        example: "20 °C + 20 °C is not 40 °C",
        point_unit: "degrees_celsius",
        delta_unit: "kelvins",
    },
    // Decibel levels against a fixed reference, such as dBV and dBm. Adding
    // two levels would multiply the amplitudes or powers underneath. The
    // difference between two levels is a gain, a `Level` in dB.
    PointQuantity {
        point: "AmplitudeRatio",
        delta: "Level",
        example: "a 20 dBV signal plus a 20 dBV signal is not 40 dBV",
        point_unit: "decibel_volts",
        delta_unit: "decibels",
    },
    PointQuantity {
        point: "PowerRatio",
        delta: "Level",
        example: "a 20 dBm signal plus a 20 dBm signal is not 40 dBm",
        point_unit: "decibel_milliwatts",
        delta_unit: "decibels",
    },
];

/// The point-quantity entry for `name`, if it is one.
pub fn point_quantity(name: &str) -> Option<&'static PointQuantity> {
    POINT_QUANTITIES.iter().find(|p| p.point == name)
}

/// The difference quantity of a point quantity (`TemperatureDelta` for
/// `Temperature`), or `None` for ordinary quantities.
pub fn point_delta(name: &str) -> Option<&'static str> {
    point_quantity(name).map(|p| p.delta)
}

/// The source file of one quantity.
pub fn generate(qty: &QuantityDef, all: &[QuantityDef]) -> String {
    let mut out = String::with_capacity(64 * 1024);

    emit_header(qty, all, &mut out);
    emit_enum(qty, &mut out);
    emit_enum_impl(qty, &mut out);
    emit_struct_impl(qty, &mut out);
    emit_fixed_units(qty, all, &mut out);
    out
}

/// The base unit in SI from UnitsNet's data, as an `Option<UnitExpr>`
/// expression for the generated code.
fn si_unit_expr(qty: &QuantityDef, all: &[QuantityDef]) -> String {
    match si::base_unit_size(qty, all) {
        Some(size) => format!(
            "Some(crate::__private::si_unit({:?}, {size}))",
            qty.dimensions
        ),
        None => "None".to_string(),
    }
}

/// The exact definition of a `Fixed` marker: one unit is
/// `num / den · π^pi` base units, plus `offset` base units for affine units.
pub struct FixedParts {
    pub num: u128,
    pub den: u128,
    pub pi: i32,
    pub offset: Option<(i128, u128)>,
}

/// The `Fixed` marker definition for `unit`, if it has one. Linear units
/// qualify when their factor is an exact fraction, times a power of π, that
/// fits in `u128`. Point quantities also get affine units with an exact
/// positive factor and offset, such as °C, m°C, °F and dBµV.
pub fn fixed_parts(qty: &QuantityDef, unit: &UnitDef) -> Option<FixedParts> {
    let Form::Affine { a, b } = &unit.to_base else {
        return None;
    };
    let (num, den, pi) = a.exact_parts()?;
    let offset = if b.is_zero() {
        None
    } else if point_delta(&qty.name).is_some() && pi == 0 {
        Some(b.exact_signed_parts()?)
    } else {
        return None;
    };
    Some(FixedParts {
        num,
        den,
        pi,
        offset,
    })
}

/// For a unit of a point quantity, the delta unit with the same scale
/// (Δ°C for °C, Δm°C for m°C), preferring the same name.
fn delta_unit<'a>(unit: &UnitDef, delta: &'a QuantityDef) -> Option<&'a UnitDef> {
    let Form::Affine { a, .. } = &unit.to_base else {
        return None;
    };
    let same_scale =
        |d: &&UnitDef| matches!(&d.to_base, Form::Affine { a: da, b } if b.is_zero() && da == a);
    let candidates: Vec<&UnitDef> = delta.units.iter().filter(same_scale).collect();
    candidates
        .iter()
        .find(|d| d.singular == unit.singular)
        .or_else(|| candidates.first())
        .copied()
        .filter(|d| fixed_parts(delta, d).is_some())
}

fn emit_fixed_units(qty: &QuantityDef, all: &[QuantityDef], out: &mut String) {
    let name = &qty.name;
    w!(out);
    w!(out, "fixed_base!({}, {});", name, qty.base_unit);
    w!(out);
    w!(
        out,
        "/// Units for counting [`{}`] in integers with [`Fixed`](crate::Fixed).",
        name
    );
    w!(out, "///");
    w!(
        out,
        "/// Also at [`unit::{}`](crate::unit::{}).",
        name.to_snake_case(),
        name.to_snake_case()
    );
    w!(out, "pub mod unit {{");
    let delta_qty = point_delta(name).map(|d| {
        all.iter()
            .find(|q| q.name == d)
            .unwrap_or_else(|| panic!("delta quantity {d} not generated"))
    });
    for unit in &qty.units {
        let Some(FixedParts {
            num,
            den,
            pi,
            offset,
        }) = fixed_parts(qty, unit)
        else {
            continue;
        };
        let snake = unit.plural.to_snake_case();
        let offset = offset
            .map(|(n, d)| format!(", offset({n}, {d})"))
            .unwrap_or_default();
        // Amounts can be added; points take the matching delta unit.
        let kind = match delta_qty {
            None => ", additive".to_owned(),
            Some(dq) => match delta_unit(unit, dq) {
                Some(du) => format!(
                    ", point crate::units::{}::unit::{}",
                    dq.name.to_snake_case(),
                    du.singular
                ),
                None => String::new(),
            },
        };
        w!(
            out,
            "    fixed_unit!({s}: {name}, {name}Unit::{s}, base {base}, {num} / {den}, pi^{pi}, from_{snake}, as_{snake}{offset}{kind});",
            s = unit.singular,
            base = qty.base_unit,
        );
    }
    w!(out, "}}");
}

fn emit_header(qty: &QuantityDef, all: &[QuantityDef], out: &mut String) {
    let name = &qty.name;
    w!(
        out,
        "// Code generated by `cargo run -p codegen` \u{2014} DO NOT EDIT"
    );
    w!(out, "// Source: data/quantities/{}.json", name);
    w!(out);
    // Folded factors such as 2π are exact constants, not approximations.
    w!(
        out,
        "#![allow(clippy::approx_constant, clippy::excessive_precision, clippy::unreadable_literal)]"
    );
    w!(out);
    w!(out, "use crate::Float;");
    if let Some(delta) = point_delta(name) {
        w!(out, "use crate::{};", delta);
    }
    w!(out);

    let doc = escape_doc(&qty.doc);
    if !doc.is_empty() {
        w!(out, "/// {}", doc);
        w!(out, "///");
    }
    w!(
        out,
        "/// Stored as a `T` (`f64` by default) in [`{}Unit::{}`], with the same layout as `T`.",
        name,
        qty.base_unit
    );
    if let Some(pq) = point_quantity(name) {
        let (delta, snake, delta_unit) = (pq.delta, pq.point_unit, pq.delta_unit);
        for line in [
            "///".to_owned(),
            format!("/// A {name} is a point on a scale, not an amount, so two of them can't"),
            format!(
                "/// be added ({}), negated or scaled. Subtracting",
                pq.example
            ),
            format!("/// one from another gives a [`{delta}`], which can be added back:"),
            "///".to_owned(),
            "/// ```rust".to_owned(),
            "/// use qty::*;".to_owned(),
            "///".to_owned(),
            format!("/// let morning = {name}::from_{snake}(12.0);"),
            format!("/// let noon = {name}::from_{snake}(20.0);"),
            format!("/// let rise: {delta} = noon - morning;"),
            format!("/// assert!((rise.as_{delta_unit}() - 8.0).abs() < 1e-12);"),
            "/// assert!((morning + rise).approx_eq_rel(noon, 1e-12));".to_owned(),
            "/// ```".to_owned(),
            "///".to_owned(),
            "/// ```compile_fail".to_owned(),
            "/// use qty::*;".to_owned(),
            format!("/// let t = {name}::from_{snake}(20.0);"),
            format!("/// let _ = t + t; // error: add a {delta} instead"),
            "/// ```".to_owned(),
        ] {
            w!(out, "{}", line);
        }
    }
    w!(
        out,
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]"
    );
    w!(
        out,
        "#[cfg_attr(feature = \"serde\", derive(serde::Serialize, serde::Deserialize), serde(transparent))]"
    );
    w!(out, "#[repr(transparent)]");
    w!(out, "#[must_use]");
    w!(out, "pub struct {}<T = f64>(T);", name);
    w!(out);
    let si = si_unit_expr(qty, all);
    let invocation = match point_delta(name) {
        Some(delta) => format!(
            "point_quantity!({name}, {name}Unit, {}, {delta}, si: {si});",
            qty.base_unit
        ),
        None => format!(
            "quantity!({name}, {name}Unit, {}, si: {si});",
            qty.base_unit
        ),
    };
    w!(out, "{invocation}");
    if in_formulas(qty, all) {
        w!(out, "crate::__qty_formula!({name}, {name}Unit);");
    }
    w!(out);
}

/// Whether `formula!` accepts the quantity. It needs the size of the base
/// unit in SI, and points (temperatures, levels) aren't amounts to multiply.
/// Plain numbers, fractions and angles are dimensionless with a base unit
/// of size 1.
pub fn in_formulas(qty: &QuantityDef, all: &[QuantityDef]) -> bool {
    const PLAIN: &[&str] = &["Amount", "DecimalFraction", "Radian", "Steradian"];
    point_delta(&qty.name).is_none()
        && (si::base_unit_size(qty, all).is_some()
            || (qty.dimensions == [0; 7] && PLAIN.contains(&qty.base_unit.as_str())))
}

/// The source of `src/formula/exponents.rs`: sums, differences, products,
/// halves and thirds of exponents as types. The range is twice the largest exponent of
/// any quantity, so any product or quotient of two quantities fits.
pub fn generate_formula_exponents(quantities: &[QuantityDef]) -> String {
    let max = 2 * quantities
        .iter()
        .flat_map(|q| q.dimensions.iter())
        .map(|e| e.unsigned_abs())
        .max()
        .unwrap_or(0) as i32;
    let mut out = String::new();
    w!(
        out,
        "// Code generated by `cargo run -p codegen` — DO NOT EDIT"
    );
    w!(
        out,
        "//! Exponent arithmetic for formulas, from -{max} to {max}."
    );
    w!(out);
    w!(out, "use super::{{AddE, HalfE, MulE, SubE, ThirdE, E}};");
    w!(out);
    w!(
        out,
        "/// The largest exponent in a formula, twice the largest of any quantity."
    );
    w!(out, "pub const MAX_EXPONENT: i8 = {max};");
    w!(out);
    let range = -max..=max;
    for a in range.clone() {
        for b in range.clone() {
            if range.contains(&(a + b)) {
                w!(
                    out,
                    "impl AddE<E<{b}>> for E<{a}> {{ type Out = E<{}>; }}",
                    a + b
                );
            }
            if range.contains(&(a - b)) {
                w!(
                    out,
                    "impl SubE<E<{b}>> for E<{a}> {{ type Out = E<{}>; }}",
                    a - b
                );
            }
        }
    }
    for a in range.clone() {
        for n in range.clone() {
            if range.contains(&(a * n)) {
                w!(
                    out,
                    "impl MulE<{n}> for E<{a}> {{ type Out = E<{}>; }}",
                    a * n
                );
            }
        }
    }
    for a in range.clone() {
        if a % 2 == 0 {
            w!(out, "impl HalfE for E<{a}> {{ type Out = E<{}>; }}", a / 2);
        }
        if a % 3 == 0 {
            w!(out, "impl ThirdE for E<{a}> {{ type Out = E<{}>; }}", a / 3);
        }
    }
    out
}

fn emit_enum(qty: &QuantityDef, out: &mut String) {
    w!(out, "/// The units of [`{}`].", qty.name);
    w!(out, "///");
    w!(
        out,
        "/// Non-exhaustive, since UnitsNet keeps adding units."
    );
    w!(out, "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]");
    w!(
        out,
        "#[cfg_attr(feature = \"serde\", derive(serde::Serialize, serde::Deserialize))]"
    );
    w!(out, "#[non_exhaustive]");
    w!(out, "pub enum {}Unit {{", qty.name);
    for unit in &qty.units {
        if let Some(doc) = &unit.doc {
            let doc = escape_doc(doc);
            w!(out, "    /// {}", doc);
        }
        w!(out, "    {},", unit.singular);
    }
    w!(out, "}}");
    w!(out);
}

fn emit_enum_impl(qty: &QuantityDef, out: &mut String) {
    let enum_name = format!("{}Unit", qty.name);

    w!(out, "impl {} {{", enum_name);

    w!(out, "    /// Every unit.");
    w!(out, "    pub const ALL: &'static [Self] = &[");
    for unit in &qty.units {
        w!(out, "        Self::{},", unit.singular);
    }
    w!(out, "    ];");
    w!(out);

    // Metadata: one `UnitInfo` per unit, in the order of `ALL` (and of the
    // enum's discriminants, so `info()` is an index).
    w!(out, "    const INFOS: &'static [crate::UnitInfo] = &[");
    for unit in &qty.units {
        let abbrs: Vec<String> = unit
            .all_abbreviations
            .iter()
            .map(|a| rust_str_literal(a))
            .collect();
        w!(
            out,
            "        crate::UnitInfo {{ singular: {}, plural: {}, abbreviation: {}, abbreviations: &[{}] }},",
            rust_str_literal(&unit.singular),
            rust_str_literal(&unit.plural),
            rust_str_literal(&unit.abbreviation),
            abbrs.join(", ")
        );
    }
    w!(out, "    ];");
    w!(out);
    w!(out, "    /// Names and abbreviations.");
    w!(out, "    #[inline]");
    w!(
        out,
        "    pub const fn info(self) -> &'static crate::UnitInfo {{"
    );
    w!(out, "        &Self::INFOS[self as usize]");
    w!(out, "    }}");
    w!(out);
    let base_index = qty
        .units
        .iter()
        .position(|u| u.singular == qty.base_unit)
        .expect("base unit is listed");
    let d = qty.dimensions;
    w!(
        out,
        "    /// Name, description, dimensions and units of [`{}`].",
        qty.name
    );
    w!(
        out,
        "    pub const QUANTITY: crate::QuantityInfo = crate::QuantityInfo {{"
    );
    w!(out, "        name: {},", rust_str_literal(&qty.name));
    w!(out, "        description: {},", rust_str_literal(&qty.doc));
    w!(
        out,
        "        dimensions: crate::Dimensions::new({}, {}, {}, {}, {}, {}, {}),",
        d[0],
        d[1],
        d[2],
        d[3],
        d[4],
        d[5],
        d[6]
    );
    w!(out, "        base_unit: &Self::INFOS[{base_index}],");
    w!(out, "        units: Self::INFOS,");
    w!(out, "    }};");
    w!(out);

    w!(out, "    /// The abbreviation used for display.");
    w!(out, "    #[inline]");
    w!(
        out,
        "    pub const fn abbreviation(self) -> &'static str {{"
    );
    w!(out, "        match self {{");
    for unit in &qty.units {
        let abbr = rust_str_literal(&unit.abbreviation);
        w!(out, "            Self::{} => {},", unit.singular, abbr);
    }
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out);

    w!(out, "    /// Every abbreviation accepted when parsing.");
    w!(out, "    #[inline]");
    w!(
        out,
        "    pub const fn all_abbreviations(self) -> &'static [&'static str] {{"
    );
    w!(out, "        match self {{");
    for unit in &qty.units {
        let abbrs: Vec<String> = unit
            .all_abbreviations
            .iter()
            .map(|a| rust_str_literal(a))
            .collect();
        let list = abbrs.join(", ");
        w!(out, "            Self::{} => &[{}],", unit.singular, list);
    }
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out);

    w!(
        out,
        "    /// Parses a unit name, singular or plural. Case, spaces, `_` and `-` don't"
    );
    w!(
        out,
        "    /// matter, and British spellings work: `\"kilometres\"`,"
    );
    w!(out, "    /// `\"Square Meter\"`, `\"square_meters\"`.");
    w!(out, "    pub fn from_name(name: &str) -> Option<Self> {{");
    w!(out, "        let mut buf = [0; 64];");
    w!(
        out,
        "        match crate::normalize_unit_name(name, &mut buf)? {{"
    );
    for (key, variant) in name_map(qty) {
        w!(
            out,
            "            {} => Some(Self::{}),",
            rust_str_literal(&key),
            variant
        );
    }
    w!(out, "            _ => None,");
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out, "}}");
    w!(out);

    // Abbreviations are case-sensitive (`mm` is not `Mm`); names are not.
    w!(out, "impl core::str::FromStr for {} {{", enum_name);
    w!(out, "    type Err = crate::ParseError;");
    w!(out);
    w!(
        out,
        "    /// Parses an abbreviation such as `\"km\"`, which must match exactly, or a"
    );
    w!(
        out,
        "    /// name such as `\"kilometers\"` (see [`Self::from_name`])."
    );
    w!(
        out,
        "    fn from_str(s: &str) -> Result<Self, crate::ParseError> {{"
    );
    let abbreviations = parse_map(qty);
    if abbreviations.is_empty() {
        w!(
            out,
            "        Self::from_name(s).ok_or(crate::ParseError::UnknownUnit)"
        );
        w!(out, "    }}");
        w!(out, "}}");
        w!(out);
        return;
    }
    w!(out, "        match s {{");
    for (abbr, variant) in abbreviations {
        w!(
            out,
            "            {} => Ok(Self::{}),",
            rust_str_literal(abbr),
            variant
        );
    }
    w!(
        out,
        "            _ => Self::from_name(s).ok_or(crate::ParseError::UnknownUnit),"
    );
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out, "}}");
    w!(out);
}

/// Every normalised unit name, singular and plural, with its variant. Names
/// shared by two units are ambiguous and left out.
pub fn name_map(qty: &QuantityDef) -> Vec<(String, &str)> {
    let mut map: Vec<(String, &str)> = Vec::new();
    let mut ambiguous = std::collections::HashSet::new();
    for unit in &qty.units {
        for name in [&unit.singular, &unit.plural] {
            let key = name.to_lowercase();
            match map.iter().find(|(k, _)| *k == key) {
                Some((_, v)) if *v != unit.singular => {
                    ambiguous.insert(key);
                }
                Some(_) => {}
                None => map.push((key, &unit.singular)),
            }
        }
    }
    map.retain(|(k, _)| !ambiguous.contains(k));
    map
}

/// Every `(abbreviation, variant)` pair accepted by `FromStr`. When several
/// units share an abbreviation, the first unit in definition order wins.
fn parse_map(qty: &QuantityDef) -> Vec<(&str, &str)> {
    let mut seen = std::collections::HashSet::new();
    qty.units
        .iter()
        .flat_map(|u| {
            u.all_abbreviations
                .iter()
                .map(move |a| (a.as_str(), u.singular.as_str()))
        })
        .filter(|(a, _)| !a.is_empty() && seen.insert(*a))
        .collect()
}

fn emit_struct_impl(qty: &QuantityDef, out: &mut String) {
    let name = &qty.name;
    let enum_name = format!("{name}Unit");

    w!(
        out,
        "/// Each conversion is one multiplication or division by a constant, or two"
    );
    w!(out, "/// operations for units with an offset.");
    w!(
        out,
        "impl<T: Float> crate::UnitOf<{}<T>> for {} {{",
        name,
        enum_name
    );
    w!(out, "    #[inline]");
    w!(out, "    fn value_to_base(self, value: T) -> T {{");
    w!(out, "        match self {{");
    for unit in &qty.units {
        w!(
            out,
            "            {}::{} => {},",
            enum_name,
            unit.singular,
            expr::emit(&unit.to_base, "value", Direction::ToBase)
        );
    }
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out);
    w!(out, "    #[inline]");
    w!(out, "    fn value_from_base(self, value: T) -> T {{");
    w!(out, "        match self {{");
    for unit in &qty.units {
        w!(
            out,
            "            {}::{} => {},",
            enum_name,
            unit.singular,
            expr::emit(&unit.from_base, "value", Direction::FromBase)
        );
    }
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out, "}}");
    w!(out);
    w!(out, "impl<T: Float> {}<T> {{", name);

    emit_auto_unit(qty, out);

    // `from_<unit>` and `as_<unit>`.
    for unit in &qty.units {
        let snake_plural = unit.plural.to_snake_case();
        // The method name already spells the unit out, so the docs give the
        // abbreviation, or the enum variant for units without one.
        let in_unit = if unit.abbreviation.is_empty() {
            format!("[`{name}Unit::{}`]", unit.singular)
        } else {
            format!("`{}`", unit.abbreviation)
        };

        w!(out, "    /// A [`{}`] from a value in {}.", name, in_unit);
        w!(out, "    #[inline]");
        w!(out, "    pub fn from_{}(value: T) -> Self {{", snake_plural);
        w!(
            out,
            "        Self({})",
            expr::emit(&unit.to_base, "value", Direction::ToBase)
        );
        w!(out, "    }}");
        w!(out);

        w!(out, "    /// The value in {}.", in_unit);
        w!(out, "    #[inline]");
        w!(out, "    pub fn as_{}(self) -> T {{", snake_plural);
        w!(
            out,
            "        {}",
            expr::emit(&unit.from_base, "self.0", Direction::FromBase)
        );
        w!(out, "    }}");

        w!(out);
    }

    w!(out, "}}");
    w!(out);
    emit_const_new(qty, out);
}

/// `const_new()` for `f64` storage. It uses the same expressions as `new()`
/// with the constants as plain literals, so the results are identical.
fn emit_const_new(qty: &QuantityDef, out: &mut String) {
    let name = &qty.name;
    let enum_name = format!("{name}Unit");
    w!(out, "impl {name}<f64> {{");
    w!(
        out,
        "    /// Like [`new`](Self::new), but usable in `const` items. The result is"
    );
    w!(out, "    /// the same.");
    w!(out, "    #[inline]");
    w!(
        out,
        "    pub const fn const_new(value: f64, unit: {enum_name}) -> Self {{"
    );
    w!(out, "        Self(match unit {{");
    for unit in &qty.units {
        w!(
            out,
            "            {}::{} => {},",
            enum_name,
            unit.singular,
            strip_from_f64(&expr::emit(&unit.to_base, "value", Direction::ToBase))
        );
    }
    w!(out, "        }})");
    w!(out, "    }}");
    w!(out, "}}");
}

/// Turns `T::from_f64(LIT)` into `LIT`. The literals never contain parentheses.
fn strip_from_f64(expr: &str) -> String {
    const OPEN: &str = "T::from_f64(";
    let mut out = String::with_capacity(expr.len());
    let mut rest = expr;
    while let Some(i) = rest.find(OPEN) {
        out.push_str(&rest[..i]);
        rest = &rest[i + OPEN.len()..];
        let close = rest.find(')').expect("from_f64 literal is closed");
        out.push_str(&rest[..close]);
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    out
}

/// `auto_unit()`: picks, among the SI-prefixed forms of the base unit in
/// steps of 1000 (m, km, mm, µm, …), the largest one not exceeding the value.
fn emit_auto_unit(qty: &QuantityDef, out: &mut String) {
    let enum_name = format!("{}Unit", qty.name);
    let base = qty
        .units
        .iter()
        .find(|u| u.singular == qty.base_unit)
        .expect("base unit is listed");
    let mut scale: Vec<(f64, &str)> = qty
        .units
        .iter()
        .filter(|u| u.family == base.family && u.decimal_exp.is_some_and(|e| e % 3 == 0))
        .filter_map(|u| match &u.to_base {
            Form::Affine { a, b } if b.is_zero() => Some((a.to_f64(), u.singular.as_str())),
            _ => None,
        })
        .collect();
    scale.sort_by(|x, y| y.0.total_cmp(&x.0));
    let names: Vec<String> = scale
        .iter()
        .map(|(_, u)| format!("{enum_name}::{u}"))
        .collect();

    w!(
        out,
        "    /// The value in the largest SI-prefixed form of the base unit that doesn't"
    );
    w!(
        out,
        "    /// exceed it, in steps of 1000 ({}).",
        scale
            .iter()
            .rev()
            .map(|(_, u)| {
                let unit = qty.units.iter().find(|x| x.singular == *u).unwrap();
                format!("`{}`", unit.abbreviation)
            })
            .collect::<Vec<_>>()
            .join(", ")
    );
    w!(
        out,
        "    /// Zero, NaN and infinities stay in the base unit."
    );
    w!(out, "    #[inline]");
    w!(
        out,
        "    pub fn auto_unit(self) -> crate::InUnit<T, {enum_name}> {{"
    );
    if scale.len() <= 1 {
        w!(out, "        self.in_unit({enum_name}::{})", qty.base_unit);
        w!(out, "    }}");
        w!(out);
        return;
    }
    w!(
        out,
        "        const SCALE: [(f64, {enum_name}); {}] = [{}];",
        scale.len(),
        scale
            .iter()
            .zip(&names)
            .map(|((f, _), n)| format!("({}, {n})", crate::expr::f64_literal(*f)))
            .collect::<Vec<_>>()
            .join(", ")
    );
    w!(out, "        let magnitude = self.0.abs();");
    w!(
        out,
        "        if magnitude == T::from_f64(0.0) || !magnitude.is_finite() {{"
    );
    w!(
        out,
        "            return self.in_unit({enum_name}::{});",
        qty.base_unit
    );
    w!(out, "        }}");
    w!(out, "        let mut unit = SCALE[SCALE.len() - 1].1;");
    w!(out, "        for (factor, candidate) in SCALE {{");
    w!(out, "            if magnitude >= T::from_f64(factor) {{");
    w!(out, "                unit = candidate;");
    w!(out, "                break;");
    w!(out, "            }}");
    w!(out, "        }}");
    w!(out, "        self.in_unit(unit)");
    w!(out, "    }}");
    w!(out);
}

/// The kind of operator a [`CrossOp`] implements.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CrossKind {
    /// `lhs * rhs = output`.
    Mul,
    /// `lhs / rhs = output`.
    Div,
    /// `lhs * rhs` is dimensionless: a plain number.
    MulScalar,
    /// `1 / rhs = output`, with a plain number on the left.
    Recip,
    /// `lhs.sqrt() = output`, where `lhs = output * output`.
    Sqrt,
    /// `lhs.cbrt() = output`, where `lhs = output * output * output`.
    Cbrt,
}

impl CrossKind {
    /// The operator symbol; two ops with the same symbol and operand types
    /// would be conflicting impls.
    pub fn symbol(self) -> &'static str {
        match self {
            CrossKind::Mul | CrossKind::MulScalar => "*",
            CrossKind::Div | CrossKind::Recip => "/",
            CrossKind::Sqrt => "sqrt",
            CrossKind::Cbrt => "cbrt",
        }
    }
}

/// One product or quotient between quantities.
///
/// For example, `Length / Duration = Speed` is represented as:
/// ```text
/// CrossOp { kind: Div, lhs: "Length", rhs: "Duration", output: "Speed", factor: None }
/// ```
#[derive(Clone, Debug)]
pub struct CrossOp {
    pub kind: CrossKind,
    /// Left-hand type, such as `"Length"`. Empty for [`CrossKind::Recip`].
    pub lhs: String,
    /// Right-hand type, such as `"Duration"`.
    pub rhs: String,
    /// Result type, such as `"Speed"`. Empty for [`CrossKind::MulScalar`].
    pub output: String,
    /// Factor relating the base units, when they are not coherent. `None`
    /// means exactly 1, so the operator works for any storage type.
    pub factor: Option<String>,
    /// The same factor, exactly: `output_base = k · lhs_base ⊙ rhs_base`.
    pub k: crate::expr::Const,
}

/// The source of `src/cross_type.rs`. Every impl works on base-unit values,
/// so the unit a value was created in doesn't matter.
pub fn generate_cross_type(ops: &[CrossOp]) -> String {
    let mut out = String::with_capacity(16 * 1024);

    w!(
        out,
        "// Code generated by `cargo run -p codegen` \u{2014} DO NOT EDIT"
    );
    w!(out, "//! Products and quotients between quantities.");
    w!(out);
    w!(out, "use crate::units::*;");
    w!(out);

    for op in ops {
        let factor = op
            .factor
            .as_ref()
            .map(|f| format!("; {f}"))
            .unwrap_or_default();
        let line = match op.kind {
            CrossKind::Mul => format!(
                "cross_mul!({} * {} = {}{factor});",
                op.lhs, op.rhs, op.output
            ),
            CrossKind::Div => format!(
                "cross_div!({} / {} = {}{factor});",
                op.lhs, op.rhs, op.output
            ),
            CrossKind::MulScalar => format!("cross_mul_scalar!({} * {});", op.lhs, op.rhs),
            CrossKind::Recip => format!("cross_recip!({} => {});", op.rhs, op.output),
            CrossKind::Sqrt => format!("cross_sqrt!({} => {}{factor});", op.lhs, op.output),
            CrossKind::Cbrt => format!("cross_cbrt!({} => {}{factor});", op.lhs, op.output),
        };
        let mut involved = vec![op.lhs.as_str(), op.rhs.as_str(), op.output.as_str()];
        involved.retain(|q| !q.is_empty());
        let cfg = cfg_for(&involved);
        if !cfg.is_empty() {
            w!(out, "{cfg}");
        }
        w!(out, "{line}");
        // The same relation between unit enums, with its exact factor, for
        // `Fixed::mul_to` / `div_to`.
        let relation = match op.kind {
            CrossKind::Mul => Some("UnitMul"),
            CrossKind::Div => Some("UnitDiv"),
            _ => None,
        };
        if let (Some(relation), Some((num, den, pi))) = (relation, op.k.exact_parts()) {
            if !cfg.is_empty() {
                w!(out, "{cfg}");
            }
            w!(
                out,
                "fixed_relation!({relation}: {}Unit, {}Unit => {}Unit; {num} / {den}, pi^{pi});",
                op.lhs,
                op.rhs,
                op.output
            );
        }
    }

    out
}

/// The tests of one quantity, `tests/units/<quantity>.rs`. They check the
/// folded conversions against UnitsNet's formulas, translated verbatim and
/// compiled by rustc as an independent oracle.
pub fn generate_quantity_tests(qty: &QuantityDef, all: &[QuantityDef]) -> String {
    let mut out = String::with_capacity(16 * 1024);
    let name = &qty.name;
    let enum_name = format!("{}Unit", name);

    w!(
        out,
        "// Code generated by `cargo run -p codegen` \u{2014} DO NOT EDIT"
    );
    w!(out, "// Quantity: {}", name);
    w!(out);
    let mut involved = vec![name.as_str()];
    if let Some(delta) = point_delta(name) {
        involved.push(delta);
    }
    if let Some(cfg) = cfg_for(&involved).strip_prefix("#[") {
        w!(out, "#![{}", cfg);
    }
    w!(
        out,
        "#![allow(clippy::excessive_precision, clippy::unreadable_literal, clippy::double_parens, unused_parens)]"
    );
    w!(out);
    w!(
        out,
        "use super::{{assert_close, check_fixed, check_fixed_rescale, F32_TOL, F64_TOL, ROUND_TRIP_TOL}};"
    );
    w!(out, "use qty::*;");
    w!(out);

    for unit in &qty.units {
        emit_unit_test(name, unit, &mut out);
    }
    emit_base_unit_test(qty, all, &mut out);

    // Fixed markers: float bridge and integer rescaling agree with floats.
    let markers: Vec<&UnitDef> = qty
        .units
        .iter()
        .filter(|u| fixed_parts(qty, u).is_some())
        .collect();
    if !markers.is_empty() {
        w!(out, "#[test]");
        w!(out, "fn fixed_units() {{");
        for unit in markers {
            let path = format!("unit::{}::{}", name.to_snake_case(), unit.singular);
            w!(out, "    check_fixed::<{path}>();");
            if fixed_parts(qty, unit).is_some_and(|p| p.pi == 0) {
                w!(out, "    check_fixed_rescale::<{path}>();");
            }
        }
        w!(out, "}}");
        w!(out);
    }

    // Unit expressions agree with the conversions.
    w!(out, "#[test]");
    w!(out, "fn unit_expressions_agree() {{");
    w!(out, "    super::check_unit_exprs::<{name}>({name}::new);");
    w!(out, "}}");
    w!(out);

    // Metadata matches the unit enum.
    w!(out, "#[test]");
    w!(out, "fn unit_metadata() {{");
    w!(out, "    let info = {name}::<f64>::INFO;");
    w!(
        out,
        "    assert_eq!(info.name, {});",
        rust_str_literal(name)
    );
    w!(
        out,
        "    assert_eq!(info.units.len(), {enum_name}::ALL.len());"
    );
    w!(
        out,
        "    for (&unit, unit_info) in {enum_name}::ALL.iter().zip(info.units) {{"
    );
    w!(out, "        assert_eq!(unit.info(), unit_info);");
    w!(
        out,
        "        assert_eq!(format!(\"{{unit:?}}\"), unit_info.singular);"
    );
    w!(
        out,
        "        assert_eq!(unit.abbreviation(), unit_info.abbreviation);"
    );
    w!(
        out,
        "        assert_eq!(unit.all_abbreviations(), unit_info.abbreviations);"
    );
    w!(out, "    }}");
    w!(
        out,
        "    assert_eq!(info.base_unit, {enum_name}::{}.info());",
        qty.base_unit
    );
    w!(out, "}}");
    w!(out);

    // `new` / `convert_to` agree with the per-unit methods.
    w!(out, "#[test]");
    w!(out, "fn new_and_convert_to_match_named_methods() {{");
    w!(out, "    let v = 1234.5;");
    for unit in &qty.units {
        let snake = unit.plural.to_snake_case();
        w!(
            out,
            "    assert_eq!({name}::new(v, {enum_name}::{}), {name}::from_{snake}(v));",
            unit.singular
        );
        w!(
            out,
            "    assert_eq!({name}::from_base(v).convert_to({enum_name}::{}), {name}::from_base(v).as_{snake}());",
            unit.singular
        );
    }
    w!(out, "}}");
    w!(out);

    // const_new: evaluated at compile time, identical to `new`.
    w!(out, "#[test]");
    w!(out, "fn const_new_matches_new() {{");
    w!(
        out,
        "    const AT_COMPILE_TIME: [{name}; {}] = [",
        qty.units.len()
    );
    for unit in &qty.units {
        w!(
            out,
            "        {name}::const_new(1234.5, {enum_name}::{}),",
            unit.singular
        );
    }
    w!(out, "    ];");
    w!(
        out,
        "    for (q, &unit) in AT_COMPILE_TIME.iter().zip({enum_name}::ALL) {{"
    );
    w!(
        out,
        "        assert_eq!(q.as_base().to_bits(), {name}::new(1234.5_f64, unit).as_base().to_bits(), \"{{unit:?}}\");"
    );
    w!(out, "    }}");
    w!(
        out,
        "    for v in [0.0, -0.0, 1e-300, -2.5, 3.0e8, f64::MAX] {{"
    );
    w!(out, "        for &unit in {enum_name}::ALL {{");
    w!(
        out,
        "            assert_eq!({name}::const_new(v, unit).as_base().to_bits(), {name}::<f64>::new(v, unit).as_base().to_bits());"
    );
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out, "}}");
    w!(out);

    // in_unit / auto_unit: the value and unit describe the same quantity.
    w!(out, "#[test]");
    w!(out, "fn in_unit_and_auto_unit_round_trip() {{");
    w!(out, "    for v in [1e-9, 0.5, 1.0, 1234.5, 1e7, -3e4] {{");
    w!(out, "        let q = {name}::from_base(v);");
    w!(out, "        let auto = q.auto_unit();");
    w!(
        out,
        "        assert_close({name}::new(auto.value, auto.unit).as_base(), v, F64_TOL);"
    );
    w!(out, "        for &unit in {enum_name}::ALL {{");
    w!(out, "            let shown = q.in_unit(unit);");
    w!(
        out,
        "            assert_eq!((shown.value, shown.unit), (q.convert_to(unit), unit));"
    );
    w!(out, "        }}");
    w!(out, "    }}");
    w!(
        out,
        "    assert_eq!({name}::from_base(0.0).auto_unit().unit, {enum_name}::{});",
        qty.base_unit
    );
    w!(out, "}}");
    w!(out);

    // FromStr: every abbreviation parses to its unit.
    w!(out, "#[test]");
    w!(out, "fn parse_every_abbreviation() {{");
    for (abbr, variant) in parse_map(qty) {
        let lit = rust_str_literal(abbr);
        w!(
            out,
            "    assert_eq!({lit}.parse::<{enum_name}>(), Ok({enum_name}::{variant}));"
        );
        w!(
            out,
            "    assert_eq!(format!(\"2.5 {{}}\", {lit}).parse::<{name}>(), Ok({name}::new(2.5, {enum_name}::{variant})));"
        );
    }
    w!(
        out,
        "    assert_eq!(\"1 not-a-unit\".parse::<{name}>(), Err(ParseError::UnknownUnit));"
    );
    w!(out, "}}");
    w!(out);

    // Names: every singular and plural name, in several spellings.
    w!(out, "#[test]");
    w!(out, "fn parse_every_name() {{");
    for unit in &qty.units {
        for written in [&unit.singular, &unit.plural] {
            if !name_map(qty)
                .iter()
                .any(|(k, v)| *k == written.to_lowercase() && *v == unit.singular)
            {
                continue;
            }
            let spaced = written.to_snake_case().replace('_', " ");
            for form in [
                written.to_lowercase(),
                spaced.clone(),
                spaced.to_uppercase(),
            ] {
                w!(
                    out,
                    "    assert_eq!({enum_name}::from_name({}), Some({enum_name}::{}));",
                    rust_str_literal(&form),
                    unit.singular
                );
            }
        }
    }
    w!(out, "}}");
    w!(out);

    if point_delta(name).is_some() {
        // Point arithmetic: differences are deltas; deltas move points.
        let delta = point_delta(name).unwrap_or_default();
        w!(out, "#[test]");
        w!(out, "fn point_arithmetic() {{");
        w!(out, "    let a = {name}::from_base(6.0);");
        w!(out, "    let b = {name}::from_base(2.0);");
        w!(out, "    let d: {delta} = a - b;");
        w!(out, "    assert_eq!(d.as_base(), 4.0);");
        w!(out, "    assert_eq!(b + d, a);");
        w!(out, "    assert_eq!(d + b, a);");
        w!(out, "    assert_eq!(a - d, b);");
        w!(out, "    assert_eq!(a / b, 3.0);");
        w!(out, "    let mut c = b;");
        w!(out, "    c += d;");
        w!(out, "    assert_eq!(c, a);");
        w!(out, "    c -= d;");
        w!(out, "    assert_eq!(c, b);");
        w!(
            out,
            "    let i: {delta}<i64> = {name}::<i64>::from_base(7) - {name}::from_base(2);"
        );
        w!(out, "    assert_eq!(i.as_base(), 5);");
        w!(out, "}}");
        w!(out);
    } else {
        // Arithmetic on exact base-unit values, with float and integer storage.
        w!(out, "#[test]");
        w!(out, "fn arithmetic_f64() {{");
        w!(out, "    let a = {name}::from_base(6.0);");
        w!(out, "    let b = {name}::from_base(2.0);");
        w!(out, "    assert_eq!((a + b).as_base(), 8.0);");
        w!(out, "    assert_eq!((a - b).as_base(), 4.0);");
        w!(out, "    assert_eq!((-a).as_base(), -6.0);");
        w!(out, "    assert_eq!((a * 2.0).as_base(), 12.0);");
        w!(out, "    assert_eq!((2.0 * a).as_base(), 12.0);");
        w!(out, "    assert_eq!((a / 2.0).as_base(), 3.0);");
        w!(out, "    assert_eq!(a / b, 3.0);");
        w!(out, "    let mut c = a;");
        w!(out, "    c += b;");
        w!(out, "    c -= b;");
        w!(out, "    c *= 2.0;");
        w!(out, "    c /= 4.0;");
        w!(out, "    assert_eq!(c.as_base(), 3.0);");
        w!(out, "}}");
        w!(out);

        w!(out, "#[test]");
        w!(out, "fn arithmetic_integer() {{");
        w!(out, "    let a = {name}::<i64>::from_base(7);");
        w!(out, "    let b = {name}::<i64>::from_base(2);");
        w!(out, "    assert_eq!(a + b, {name}::from_base(9));");
        w!(out, "    assert_eq!(a % b, {name}::from_base(1));");
        w!(out, "    assert_eq!(a / b, 3);");
        w!(out, "    assert!(a > b);");
        w!(
            out,
            "    assert_eq!([a, b].iter().sum::<{name}<i64>>(), {name}::from_base(9));"
        );
        w!(out, "}}");
    }

    out
}

/// One test per unit: f64 against the upstream formula, f32 against the
/// same, and the f64 round trip.
/// Checks `BASE_UNIT_EXPR`. Where the base unit's abbreviation parses with
/// the right dimensions, its size must match the one from UnitsNet's data,
/// which tests the data-derived fallback against the parser.
fn emit_base_unit_test(qty: &QuantityDef, all: &[QuantityDef], out: &mut String) {
    let name = &qty.name;
    let base = qty
        .units
        .iter()
        .find(|u| u.singular == qty.base_unit)
        .expect("base unit");
    // What qty falls back to when the abbreviation doesn't parse: a plain
    // number for an empty abbreviation without dimensions, otherwise the
    // size from UnitsNet's data.
    let from_data = match si::base_unit_size(qty, all) {
        _ if base.abbreviation.is_empty() && qty.dimensions == [0; 7] => "Some(1.0)".to_string(),
        Some(size) => format!("Some({size})"),
        None => "None".to_string(),
    };
    w!(out, "#[test]");
    w!(out, "fn base_unit_expression() {{");
    w!(out, "    let info = <{name}<f64> as Quantity>::INFO;");
    w!(
        out,
        "    let expr = <{name}<f64> as Quantity>::BASE_UNIT_EXPR;"
    );
    w!(
        out,
        "    let parsed = UnitExpr::parse({}).ok().filter(|e| e.dimensions == info.dimensions);",
        rust_str_literal(&base.abbreviation)
    );
    w!(out, "    let from_data: Option<f64> = {from_data};");
    w!(
        out,
        "    if let (Some(parsed), Some(size)) = (parsed, from_data) {{"
    );
    w!(out, "        assert_close(parsed.factor, size, F64_TOL);");
    w!(out, "    }}");
    w!(out, "    match parsed {{");
    w!(
        out,
        "        Some(parsed) => assert_eq!(expr, Some(parsed)),"
    );
    w!(
        out,
        "        None => assert_eq!(expr.map(|e| e.factor), from_data),"
    );
    w!(out, "    }}");
    w!(out, "    if let Some(expr) = expr {{");
    w!(out, "        assert_eq!(expr.dimensions, info.dimensions);");
    w!(out, "    }}");
    w!(out, "}}");
    w!(out);
}

fn emit_unit_test(name: &str, unit: &UnitDef, out: &mut String) {
    let snake = unit.plural.to_snake_case();

    // Affine conversions lose relative accuracy near their offset, so test
    // well away from it; linear ones are tested at awkward values too.
    let values: &[&str] = match &unit.to_base {
        Form::Affine { .. } if !unit.to_base.is_linear() => &["1e6", "-2.5e6"],
        _ => &["1.0", "0.3", "-42.75", "1e6", "123456.789"],
    };

    w!(out, "#[test]");
    w!(out, "fn {}() {{", snake);
    w!(out, "    for v in [{}] {{", values.join(", "));
    w!(
        out,
        "        // Generated conversion vs. the upstream formula, verbatim."
    );
    w!(out, "        let x: f64 = v;");
    w!(out, "        let base = {};", unit.oracle_to_base);
    w!(
        out,
        "        assert_close({name}::from_{snake}(v).as_base(), base, F64_TOL);"
    );
    w!(out, "        let x: f64 = base;");
    w!(out, "        let back = {};", unit.oracle_from_base);
    w!(
        out,
        "        assert_close({name}::from_base(base).as_{snake}(), back, F64_TOL);"
    );
    w!(
        out,
        "        assert_close({name}::from_{snake}(v).as_{snake}(), v, ROUND_TRIP_TOL);"
    );
    w!(out, "        // f32 storage.");
    w!(
        out,
        "        assert_close({name}::from_{snake}(v as f32).as_base() as f64, base, F32_TOL);"
    );
    w!(
        out,
        "        assert_close({name}::from_base(base as f32).as_{snake}() as f64, back, F32_TOL);"
    );
    w!(out, "    }}");
    w!(out, "}}");
    w!(out);
}

/// The source of `tests/cross_type.rs`, one test per product or quotient.
///
/// Base-unit operands are small integers, so the expected results are exact.
pub fn generate_cross_type_tests(ops: &[CrossOp], quantities: &[QuantityDef]) -> String {
    let mut out = String::with_capacity(32 * 1024);

    w!(
        out,
        "// Code generated by `cargo run -p codegen` \u{2014} DO NOT EDIT"
    );
    w!(out);
    w!(out, "use qty::*;");
    w!(out);
    w!(
        out,
        "/// Checks `Fixed::mul_to_rounded` and `div_to` against the float operators:"
    );
    w!(
        out,
        "/// within half a unit, or an overflow when the result is huge."
    );
    w!(out, "#[allow(dead_code)]");
    w!(
        out,
        "fn check_fixed_relation(fixed: Result<i64, RoundError>, float: f64) {{"
    );
    w!(out, "    match fixed {{");
    w!(
        out,
        "        Ok(n) => assert!((n as f64 - float).abs() <= 0.5 + float.abs() * 1e-12, \"{{n}} vs {{float}}\"),"
    );
    w!(
        out,
        "        Err(RoundError::Overflow) => assert!(float.abs() >= 9.2e18, \"spurious overflow for {{float}}\"),"
    );
    w!(out, "        Err(e) => panic!(\"{{e:?}} for {{float}}\"),");
    w!(out, "    }}");
    w!(out, "}}");
    w!(out);

    // The marker of a quantity's base unit, for additive quantities.
    let base_marker = |name: &str| -> Option<String> {
        let qty = quantities.iter().find(|q| q.name == name)?;
        if point_delta(name).is_some() {
            return None;
        }
        let base = qty.units.iter().find(|u| u.singular == qty.base_unit)?;
        let parts = fixed_parts(qty, base)?;
        (parts.pi == 0).then(|| format!("unit::{}::{}", name.to_snake_case(), base.singular))
    };

    for op in ops {
        let test_name = format!(
            "{}_{}_{}",
            format!("{:?}", op.kind).to_snake_case(),
            if op.lhs.is_empty() {
                "one".to_owned()
            } else {
                op.lhs.to_snake_case()
            },
            op.rhs.to_snake_case(),
        );
        let factor = op.factor.as_deref().unwrap_or("1.0");
        if matches!(op.kind, CrossKind::Sqrt | CrossKind::Cbrt) {
            w!(out, "#[cfg(any(feature = \"std\", feature = \"libm\"))]");
        }
        let mut involved = vec![op.lhs.as_str(), op.rhs.as_str(), op.output.as_str()];
        involved.retain(|q| !q.is_empty());
        let cfg = cfg_for(&involved);
        if !cfg.is_empty() {
            w!(out, "{cfg}");
        }
        w!(out, "#[test]");
        w!(out, "fn {}() {{", test_name);
        match op.kind {
            CrossKind::Mul | CrossKind::Div => {
                let (sym, expected) = if op.kind == CrossKind::Mul {
                    ("*", "12.0")
                } else {
                    ("/", "3.0")
                };
                w!(out, "    let a = {}::from_base(6.0);", op.lhs);
                w!(out, "    let b = {}::from_base(2.0);", op.rhs);
                w!(out, "    let c: {} = a {sym} b;", op.output);
                w!(out, "    assert_eq!(c.as_base(), {expected} * {factor});");
                if op.factor.is_none() {
                    w!(
                        out,
                        "    // Coherent base units: works for integer storage too."
                    );
                    w!(out, "    let a = {}::<i32>::from_base(6);", op.lhs);
                    w!(out, "    let b = {}::<i32>::from_base(2);", op.rhs);
                    w!(out, "    let c: {}<i32> = a {sym} b;", op.output);
                    w!(
                        out,
                        "    assert_eq!(c.as_base(), {});",
                        expected.trim_end_matches(".0")
                    );
                }
                // The same relation on `Fixed` counts of the base units.
                let markers = (
                    base_marker(&op.lhs),
                    base_marker(&op.rhs),
                    base_marker(&op.output),
                );
                let rational = op.k.exact_parts().is_some_and(|(_, _, pi)| pi == 0);
                if let ((Some(ma), Some(mb), Some(mc)), true) = (markers, rational) {
                    let method = if op.kind == CrossKind::Mul {
                        "mul_to_rounded"
                    } else {
                        "div_to"
                    };
                    w!(
                        out,
                        "    for (x, y) in [(6_i64, 2_i64), (-1234, 567), (1, 1), (999_999, -3), (7, 1_000_000)] {{"
                    );
                    w!(
                        out,
                        "        let (a, b) = (Fixed::<{ma}>::new(x), Fixed::<{mb}>::new(y));"
                    );
                    w!(
                        out,
                        "        let fixed = a.{method}(b, {mc}, Rounding::NearestTiesEven).map(|c| c.count());"
                    );
                    w!(
                        out,
                        "        let float: {} = a.to_f64() {sym} b.to_f64();",
                        op.output
                    );
                    w!(out, "        check_fixed_relation(fixed, float.as_base());");
                    w!(out, "    }}");
                }
            }
            CrossKind::MulScalar => {
                w!(out, "    let a = {}::from_base(6.0);", op.lhs);
                w!(out, "    let b = {}::from_base(2.0);", op.rhs);
                w!(out, "    let c: f64 = a * b;");
                w!(out, "    assert_eq!(c, 12.0);");
            }
            CrossKind::Sqrt | CrossKind::Cbrt => {
                let (method, value, root) = if op.kind == CrossKind::Sqrt {
                    ("sqrt", "16.0", "4.0")
                } else {
                    ("cbrt", "64.0", "4.0")
                };
                w!(
                    out,
                    "    let c = {}::from_base({value} * {factor});",
                    op.lhs
                );
                w!(out, "    let r: {} = c.{method}();", op.output);
                w!(out, "    assert!((r.as_base() - {root}).abs() < 1e-12);");
            }
            CrossKind::Recip => {
                w!(out, "    let b = {}::from_base(4.0);", op.rhs);
                w!(out, "    let c: {} = 1.0 / b;", op.output);
                w!(out, "    assert_eq!(c.as_base(), 0.25);");
            }
        }
        w!(out, "}}");
        w!(out);
    }

    out
}

/// Wraps `s` in a Rust string literal, escaping backslashes and double-quotes.
fn rust_str_literal(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

/// The first line of upstream text, made safe for a `///` doc comment.
fn escape_doc(s: &str) -> String {
    let first = s.split(['\n', '\r']).next().unwrap_or(s);
    // Escape brackets so upstream citation markers like `[18]` are not
    // parsed as intra-doc links, and turn bare URLs into links.
    let escaped = first
        .replace("///", "//")
        .replace("/*", "/ *")
        .replace('[', "\\[")
        .replace(']', "\\]");
    link_urls(escaped.trim())
}

/// Wraps bare `http://` and `https://` URLs in `<>` so rustdoc links them.
fn link_urls(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    let mut rest = s;
    while let Some(start) = ["http://", "https://"]
        .iter()
        .filter_map(|scheme| rest.find(scheme))
        .min()
    {
        let len = rest[start..]
            .find(|c: char| c.is_whitespace() || c == '<' || c == '>')
            .unwrap_or(rest.len() - start);
        out.push_str(&rest[..start]);
        out.push('<');
        out.push_str(&rest[start..start + len]);
        out.push('>');
        rest = &rest[start + len..];
    }
    out.push_str(rest);
    out
}

/// The source of `src/any.rs`: `QuantityKind`, `AnyUnit` and `AnyQuantity`.
pub fn generate_any(quantities: &[QuantityDef], ops: &[CrossOp]) -> String {
    let mut out = String::with_capacity(256 * 1024);
    w!(
        out,
        "// Code generated by `cargo run -p codegen` \u{2014} DO NOT EDIT"
    );
    w!(out, "//! Quantities whose kind is known only at runtime.");
    w!(out, "//!");
    w!(out, "#![doc = include_str!(\"any.md\")]");
    w!(out);
    w!(out, "#![allow(clippy::match_single_binding)]");
    w!(out);
    w!(out, "use crate::units::*;");
    w!(
        out,
        "use crate::{{InUnit, ParseError, QuantityInfo, UnitExpr, UnitInfo}};"
    );
    w!(out);
    w!(out, "#[path = \"any_parse.rs\"]");
    w!(out, "mod parse;");
    w!(out, "pub use parse::KindMismatch;");
    w!(out);

    let gate = |name: &str| cfg_for(&[name]);
    let arm_gate = |out: &mut String, names: &[&str], indent: &str| {
        let cfg = cfg_for(names);
        if !cfg.is_empty() {
            w!(out, "{indent}{cfg}");
        }
    };

    // QuantityKind
    w!(out, "/// The kind of a quantity: `Length`, `Mass`, …");
    w!(
        out,
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]"
    );
    w!(
        out,
        "#[cfg_attr(feature = \"serde\", derive(serde::Serialize, serde::Deserialize))]"
    );
    w!(out, "#[non_exhaustive]");
    w!(out, "pub enum QuantityKind {{");
    for q in quantities {
        let g = gate(&q.name);
        if !g.is_empty() {
            w!(out, "    {g}");
        }
        w!(out, "    #[allow(missing_docs)]");
        w!(out, "    {},", q.name);
    }
    w!(out, "}}");
    w!(out);

    // Kinds per feature group, chained by `all()` (attributes are not
    // allowed on array elements).
    let mut groups: Vec<Option<&str>> = quantities.iter().map(|q| group(&q.name)).collect();
    groups.sort_unstable();
    groups.dedup();
    for g in &groups {
        let members: Vec<&str> = quantities
            .iter()
            .filter(|q| group(&q.name) == *g)
            .map(|q| q.name.as_str())
            .collect();
        let ident = g.unwrap_or("core").to_uppercase();
        let list = members
            .iter()
            .map(|m| format!("QuantityKind::{m}"))
            .collect::<Vec<_>>()
            .join(", ");
        match g {
            None => {
                w!(out, "const KINDS_{ident}: &[QuantityKind] = &[{list}];");
            }
            Some(g) => {
                w!(out, "#[cfg(feature = \"{g}\")]");
                w!(out, "const KINDS_{ident}: &[QuantityKind] = &[{list}];");
                w!(out, "#[cfg(not(feature = \"{g}\"))]");
                w!(out, "const KINDS_{ident}: &[QuantityKind] = &[];");
            }
        }
    }
    w!(out);

    w!(out, "impl QuantityKind {{");
    w!(
        out,
        "    /// Every kind compiled in (see the feature groups)."
    );
    w!(
        out,
        "    pub fn all() -> impl Iterator<Item = Self> + Clone {{"
    );
    let chain = groups
        .iter()
        .map(|g| format!("KINDS_{}", g.unwrap_or("core").to_uppercase()))
        .collect::<Vec<_>>();
    w!(out, "        {}", chain[0]);
    w!(out, "            .iter()");
    for c in &chain[1..] {
        w!(out, "            .chain({c})");
    }
    w!(out, "            .copied()");
    w!(out, "    }}");
    w!(out);
    w!(out, "    /// Name, description, dimensions and units.");
    w!(
        out,
        "    pub const fn info(self) -> &'static QuantityInfo {{"
    );
    w!(out, "        match self {{");
    for q in quantities {
        arm_gate(&mut out, &[&q.name], "            ");
        w!(
            out,
            "            Self::{n} => &{n}Unit::QUANTITY,",
            n = q.name
        );
    }
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out);
    w!(out, "    /// The unit values of this kind are stored in.");
    w!(out, "    pub const fn base_unit(self) -> AnyUnit {{");
    w!(out, "        match self {{");
    for q in quantities {
        arm_gate(&mut out, &[&q.name], "            ");
        w!(
            out,
            "            Self::{n} => AnyUnit::{n}({n}Unit::{b}),",
            n = q.name,
            b = q.base_unit
        );
    }
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out, "}}");
    w!(out);

    // AnyUnit
    w!(
        out,
        "/// A unit of any quantity: `AnyUnit::Length(LengthUnit::Meter)`."
    );
    w!(out, "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]");
    w!(
        out,
        "#[cfg_attr(feature = \"serde\", derive(serde::Serialize, serde::Deserialize))]"
    );
    w!(out, "#[non_exhaustive]");
    w!(out, "pub enum AnyUnit {{");
    for q in quantities {
        let g = gate(&q.name);
        if !g.is_empty() {
            w!(out, "    {g}");
        }
        w!(out, "    #[allow(missing_docs)]");
        w!(out, "    {n}({n}Unit),", n = q.name);
    }
    w!(out, "}}");
    w!(out);
    w!(out, "impl AnyUnit {{");
    w!(out, "    /// The kind of quantity this unit measures.");
    w!(out, "    pub const fn kind(self) -> QuantityKind {{");
    w!(out, "        match self {{");
    for q in quantities {
        arm_gate(&mut out, &[&q.name], "            ");
        w!(
            out,
            "            Self::{n}(_) => QuantityKind::{n},",
            n = q.name
        );
    }
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out);
    w!(out, "    /// Names and abbreviations.");
    w!(out, "    pub const fn info(self) -> &'static UnitInfo {{");
    w!(out, "        match self {{");
    for q in quantities {
        arm_gate(&mut out, &[&q.name], "            ");
        w!(out, "            Self::{n}(u) => u.info(),", n = q.name);
    }
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out);
    w!(out, "    /// The primary abbreviation.");
    w!(
        out,
        "    pub const fn abbreviation(self) -> &'static str {{"
    );
    w!(out, "        self.info().abbreviation");
    w!(out, "    }}");
    w!(out);
    w!(
        out,
        "    /// The unit of `kind` written as `s`: an abbreviation or a name."
    );
    w!(
        out,
        "    pub fn parse_in(kind: QuantityKind, s: &str) -> Result<Self, ParseError> {{"
    );
    w!(out, "        match kind {{");
    for q in quantities {
        arm_gate(&mut out, &[&q.name], "            ");
        w!(
            out,
            "            QuantityKind::{n} => s.parse::<{n}Unit>().map(Self::{n}),",
            n = q.name
        );
    }
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out, "}}");
    w!(out);
    for q in quantities {
        let g = gate(&q.name);
        if !g.is_empty() {
            w!(out, "{g}");
        }
        w!(out, "impl From<{n}Unit> for AnyUnit {{", n = q.name);
        w!(out, "    #[inline]");
        w!(out, "    fn from(unit: {n}Unit) -> Self {{", n = q.name);
        w!(out, "        Self::{n}(unit)", n = q.name);
        w!(out, "    }}");
        w!(out, "}}");
    }
    w!(out);

    // AnyQuantity
    w!(
        out,
        "/// A quantity of any kind, with `f64` storage. See the [module documentation](self)."
    );
    w!(out, "#[derive(Debug, Clone, Copy, PartialEq)]");
    w!(
        out,
        "#[cfg_attr(feature = \"serde\", derive(serde::Serialize, serde::Deserialize))]"
    );
    w!(out, "#[non_exhaustive]");
    w!(out, "pub enum AnyQuantity {{");
    for q in quantities {
        let g = gate(&q.name);
        if !g.is_empty() {
            w!(out, "    {g}");
        }
        w!(out, "    #[allow(missing_docs)]");
        w!(out, "    {n}({n}),", n = q.name);
    }
    w!(out, "}}");
    w!(out);

    // Applies `$body`, an expression of the typed quantity `q`, to every
    // variant.
    let each = |out: &mut String, indent: &str, body: &str| {
        for q in quantities {
            arm_gate(out, &[&q.name], indent);
            w!(out, "{indent}Self::{n}(q) => {body},", n = q.name);
        }
    };

    w!(out, "impl AnyQuantity {{");
    w!(out, "    /// `value` in `unit`.");
    w!(out, "    pub fn new(value: f64, unit: AnyUnit) -> Self {{");
    w!(out, "        match unit {{");
    for q in quantities {
        arm_gate(&mut out, &[&q.name], "            ");
        w!(
            out,
            "            AnyUnit::{n}(u) => Self::{n}({n}::new(value, u)),",
            n = q.name
        );
    }
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out);
    w!(out, "    /// The kind of quantity.");
    w!(out, "    pub fn kind(&self) -> QuantityKind {{");
    w!(out, "        match self {{");
    for q in quantities {
        arm_gate(&mut out, &[&q.name], "            ");
        w!(
            out,
            "            Self::{n}(_) => QuantityKind::{n},",
            n = q.name
        );
    }
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out);
    w!(out, "    /// The value in the base unit.");
    w!(out, "    pub fn as_base(&self) -> f64 {{");
    w!(out, "        match *self {{");
    each(&mut out, "            ", "q.as_base()");
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out);
    w!(
        out,
        "    /// The value in `unit`, which must be of the same kind."
    );
    w!(
        out,
        "    pub fn convert_to(&self, unit: AnyUnit) -> Result<f64, KindMismatch> {{"
    );
    w!(out, "        match (*self, unit) {{");
    for q in quantities {
        arm_gate(&mut out, &[&q.name], "            ");
        w!(
            out,
            "            (Self::{n}(q), AnyUnit::{n}(u)) => Ok(q.convert_to(u)),",
            n = q.name
        );
    }
    w!(out, "            #[allow(unreachable_patterns)]");
    w!(
        out,
        "            _ => Err(KindMismatch {{ expected: self.kind(), found: unit.kind() }}),"
    );
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out);
    w!(
        out,
        "    /// The value in `unit`, for display (`\"1.5 km\"`); the unit must be of the same kind."
    );
    w!(
        out,
        "    pub fn in_unit(&self, unit: AnyUnit) -> Result<InUnit<f64, AnyUnit>, KindMismatch> {{"
    );
    w!(
        out,
        "        self.convert_to(unit).map(|value| InUnit {{ value, unit }})"
    );
    w!(out, "    }}");
    w!(out);
    w!(
        out,
        "    /// The value in the SI-prefixed unit that reads best, like the typed `auto_unit`."
    );
    w!(
        out,
        "    pub fn auto_unit(&self) -> InUnit<f64, AnyUnit> {{"
    );
    w!(out, "        match *self {{");
    each(
        &mut out,
        "            ",
        "{ let s = q.auto_unit(); InUnit { value: s.value, unit: s.unit.into() } }",
    );
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out);

    // Same-kind arithmetic: amounts add; points subtract into their delta.
    w!(
        out,
        "    /// `self + rhs` for two amounts of the same kind, or a point plus its"
    );
    w!(
        out,
        "    /// difference kind (`Temperature + TemperatureDelta`); `None` otherwise."
    );
    w!(
        out,
        "    pub fn checked_add(self, rhs: Self) -> Option<Self> {{"
    );
    w!(out, "        match (self, rhs) {{");
    for q in quantities {
        match point_delta(&q.name) {
            None => {
                arm_gate(&mut out, &[&q.name], "            ");
                w!(
                    out,
                    "            (Self::{n}(a), Self::{n}(b)) => Some(Self::{n}(a + b)),",
                    n = q.name
                );
            }
            Some(d) => {
                arm_gate(&mut out, &[&q.name, d], "            ");
                w!(
                    out,
                    "            (Self::{n}(a), Self::{d}(b)) => Some(Self::{n}(a + b)),",
                    n = q.name
                );
            }
        }
    }
    w!(out, "            _ => None,");
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out);
    w!(
        out,
        "    /// `self - rhs` for two amounts of the same kind, two points (giving their"
    );
    w!(
        out,
        "    /// difference kind) or a point minus a difference; `None` otherwise."
    );
    w!(
        out,
        "    pub fn checked_sub(self, rhs: Self) -> Option<Self> {{"
    );
    w!(out, "        match (self, rhs) {{");
    for q in quantities {
        match point_delta(&q.name) {
            None => {
                arm_gate(&mut out, &[&q.name], "            ");
                w!(
                    out,
                    "            (Self::{n}(a), Self::{n}(b)) => Some(Self::{n}(a - b)),",
                    n = q.name
                );
            }
            Some(d) => {
                arm_gate(&mut out, &[&q.name, d], "            ");
                w!(
                    out,
                    "            (Self::{n}(a), Self::{n}(b)) => Some(Self::{d}(a - b)),",
                    n = q.name
                );
                arm_gate(&mut out, &[&q.name, d], "            ");
                w!(
                    out,
                    "            (Self::{n}(a), Self::{d}(b)) => Some(Self::{n}(a - b)),",
                    n = q.name
                );
            }
        }
    }
    w!(out, "            _ => None,");
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out);
    w!(
        out,
        "    /// `self · k` for amounts; `None` for points, which cannot be scaled."
    );
    w!(
        out,
        "    pub fn checked_scale(self, k: f64) -> Option<Self> {{"
    );
    w!(out, "        match self {{");
    for q in quantities {
        if point_delta(&q.name).is_none() {
            arm_gate(&mut out, &[&q.name], "            ");
            w!(
                out,
                "            Self::{n}(q) => Some(Self::{n}(q * k)),",
                n = q.name
            );
        }
    }
    w!(out, "            #[allow(unreachable_patterns)]");
    w!(out, "            _ => None,");
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out);

    // Products and quotients: the relations between quantities.
    for (method, kind, sym, doc) in [
        ("checked_mul", CrossKind::Mul, "*", "product"),
        ("checked_div", CrossKind::Div, "/", "quotient"),
    ] {
        w!(
            out,
            "    /// The {doc} of two quantities, when a relation defines it"
        );
        w!(out, "    /// (`Length {sym} Duration`); `None` otherwise.");
        w!(
            out,
            "    pub fn {method}(self, rhs: Self) -> Option<Self> {{"
        );
        w!(out, "        match (self, rhs) {{");
        for op in ops.iter().filter(|o| o.kind == kind) {
            arm_gate(&mut out, &[&op.lhs, &op.rhs, &op.output], "            ");
            w!(
                out,
                "            (Self::{l}(a), Self::{r}(b)) => Some(Self::{o}(a {sym} b)),",
                l = op.lhs,
                r = op.rhs,
                o = op.output
            );
        }
        w!(out, "            _ => None,");
        w!(out, "        }}");
        w!(out, "    }}");
        w!(out);
    }

    // Parsing.
    w!(
        out,
        "    /// Parses `s` as a quantity of `kind`, like the typed `FromStr`."
    );
    w!(
        out,
        "    pub fn parse_as(s: &str, kind: QuantityKind) -> Result<Self, ParseError> {{"
    );
    w!(out, "        match kind {{");
    for q in quantities {
        arm_gate(&mut out, &[&q.name], "            ");
        w!(
            out,
            "            QuantityKind::{n} => s.parse::<{n}>().map(Self::{n}),",
            n = q.name
        );
    }
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out);
    w!(
        out,
        "    /// `value` in the unit of `kind` named by `unit` (an abbreviation or a name)."
    );
    w!(
        out,
        "    pub(crate) fn listed(kind: QuantityKind, value: f64, unit: &str) -> Option<Self> {{"
    );
    w!(
        out,
        "        AnyUnit::parse_in(kind, unit).ok().map(|unit| Self::new(value, unit))"
    );
    w!(out, "    }}");
    w!(out);
    w!(
        out,
        "    /// `value` in the unit expression `expr`, if it fits `kind` (never a point kind)."
    );
    w!(
        out,
        "    pub(crate) fn expressed(kind: QuantityKind, value: f64, expr: UnitExpr) -> Option<Self> {{"
    );
    w!(out, "        match kind {{");
    for q in quantities {
        if point_delta(&q.name).is_none() {
            arm_gate(&mut out, &[&q.name], "            ");
            w!(
                out,
                "            QuantityKind::{n} => expr.factor_to_base::<{n}>().map(|k| Self::{n}({n}::from_base(value * k))),",
                n = q.name
            );
        }
    }
    w!(out, "            #[allow(unreachable_patterns)]");
    w!(out, "            _ => None,");
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out, "}}");
    w!(out);

    w!(out, "/// In the base unit: \"1.5 m\".");
    w!(out, "impl core::fmt::Display for AnyQuantity {{");
    w!(
        out,
        "    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {{"
    );
    w!(out, "        match *self {{");
    for q in quantities {
        arm_gate(&mut out, &[&q.name], "            ");
        w!(
            out,
            "            Self::{n}(q) => core::fmt::Display::fmt(&q, f),",
            n = q.name
        );
    }
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out, "}}");
    w!(out);
    w!(
        out,
        "/// Quantities of the same kind compare; different kinds are unordered."
    );
    w!(out, "impl PartialOrd for AnyQuantity {{");
    w!(
        out,
        "    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {{"
    );
    w!(out, "        match (self, other) {{");
    for q in quantities {
        arm_gate(&mut out, &[&q.name], "            ");
        w!(
            out,
            "            (Self::{n}(a), Self::{n}(b)) => a.partial_cmp(b),",
            n = q.name
        );
    }
    w!(out, "            #[allow(unreachable_patterns)]");
    w!(out, "            _ => None,");
    w!(out, "        }}");
    w!(out, "    }}");
    w!(out, "}}");
    w!(out);

    // Conversions with the typed quantities.
    for q in quantities {
        let g = gate(&q.name);
        if !g.is_empty() {
            w!(out, "{g}");
        }
        w!(out, "impl From<{n}> for AnyQuantity {{", n = q.name);
        w!(out, "    #[inline]");
        w!(out, "    fn from(q: {n}) -> Self {{", n = q.name);
        w!(out, "        Self::{n}(q)", n = q.name);
        w!(out, "    }}");
        w!(out, "}}");
        if !g.is_empty() {
            w!(out, "{g}");
        }
        w!(out, "impl TryFrom<AnyQuantity> for {n} {{", n = q.name);
        w!(out, "    type Error = KindMismatch;");
        w!(
            out,
            "    fn try_from(q: AnyQuantity) -> Result<Self, KindMismatch> {{"
        );
        w!(out, "        match q {{");
        w!(out, "            AnyQuantity::{n}(q) => Ok(q),", n = q.name);
        w!(out, "            #[allow(unreachable_patterns)]");
        w!(
            out,
            "            other => Err(KindMismatch {{ expected: QuantityKind::{n}, found: other.kind() }}),",
            n = q.name
        );
        w!(out, "        }}");
        w!(out, "    }}");
        w!(out, "}}");
    }
    out
}
