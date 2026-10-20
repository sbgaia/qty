//! Each quantity's base unit in coherent SI, worked out from UnitsNet's data.
//!
//! qty reads unit expressions by comparing them with the quantity's base
//! unit, which it normally gets by parsing the base unit's abbreviation. Some
//! abbreviations aren't expressions (`∆K`, `Ns/m²`, `VA`), so the generator
//! also derives the base unit from `BaseDimensions` and `BaseUnits`, and qty
//! falls back to that.

use crate::expr::{Const, Form};
use crate::parser::{QuantityDef, DIMENSIONS};

/// The quantity whose units each `BaseUnits` entry names, by dimension. Their
/// base units are the SI base units, so a unit's factor is its size in SI.
const SYSTEM: [(&str, &str); 7] = [
    ("L", "Length"),
    ("M", "Mass"),
    ("T", "Duration"),
    ("I", "ElectricCurrent"),
    ("Θ", "Temperature"),
    ("N", "AmountOfSubstance"),
    ("J", "LuminousIntensity"),
];

/// Quantities whose `BaseUnits` are known to be wrong upstream.
/// `GramPerKilonewtonSecond` lists metres and seconds, which would make it
/// 1 s/m, but g/(kN·s) is 1e-6 s/m. qty reads its abbreviation instead.
const WRONG_BASE_UNITS: &[&str] = &["SpecificFuelConsumption"];

/// The size of the base unit in coherent SI, or `None` if the data doesn't
/// say. Dimensionless quantities get `None` too: qty treats an empty base
/// abbreviation as a plain number, and the rest (bits, %RH, dB) aren't.
pub fn base_unit_size(qty: &QuantityDef, all: &[QuantityDef]) -> Option<Const> {
    if qty.dimensions == [0; 7] || WRONG_BASE_UNITS.contains(&qty.name.as_str()) {
        return None;
    }
    // The base unit's own entry first, then any other unit's.
    let mut units: Vec<_> = qty.units.iter().collect();
    units.sort_by_key(|u| u.singular != qty.base_unit);
    units.into_iter().find_map(|unit| {
        // 1 unit = k base units. Units with an offset or a reciprocal
        // conversion don't give a size.
        let Form::Affine { a: k, b } = &unit.to_base else {
            return None;
        };
        if !b.is_zero() {
            return None;
        }
        let size = unit_size(qty, unit.base_units.as_ref()?, all)?;
        Some(size / k.clone())
    })
}

/// The size in SI of a unit made of `base_units`, which must cover exactly
/// the quantity's dimensions. UnitsNet's standard volume flow, for one,
/// lists metres and seconds for the dimensions of a mass flow.
fn unit_size(
    qty: &QuantityDef,
    base_units: &std::collections::BTreeMap<String, String>,
    all: &[QuantityDef],
) -> Option<Const> {
    let used = qty.dimensions.iter().filter(|&&e| e != 0).count();
    if base_units.len() != used {
        return None;
    }
    let mut size = Const::one();
    for (i, &exponent) in qty.dimensions.iter().enumerate() {
        if exponent == 0 {
            continue;
        }
        let factor = system_unit_factor(DIMENSIONS[i], base_units.get(DIMENSIONS[i])?, all)?;
        for _ in 0..exponent.unsigned_abs() {
            size = if exponent > 0 {
                size * factor.clone()
            } else {
                size / factor.clone()
            };
        }
    }
    Some(size)
}

/// The size in SI of `unit`, a unit of the dimension `symbol`: 0.3048 for
/// `Foot`. For temperatures this is the size of a degree, without the offset.
fn system_unit_factor(symbol: &str, unit: &str, all: &[QuantityDef]) -> Option<Const> {
    let (_, name) = SYSTEM.iter().find(|(s, _)| *s == symbol)?;
    let qty = all.iter().find(|q| q.name == *name)?;
    match &qty.units.iter().find(|u| u.singular == unit)?.to_base {
        Form::Affine { a, .. } => Some(a.clone()),
        Form::Reciprocal { .. } => None,
    }
}

/// Panics unless the quantities of [`SYSTEM`] are stored in the SI base
/// units, which [`system_unit_factor`] relies on.
pub fn check_system(all: &[QuantityDef]) {
    let si = [
        "Meter", "Kilogram", "Second", "Ampere", "Kelvin", "Mole", "Candela",
    ];
    for ((_, name), base) in SYSTEM.iter().zip(si) {
        let qty = all.iter().find(|q| q.name == *name);
        assert!(
            qty.is_some_and(|q| q.base_unit == base),
            "{name} should be stored in {base}"
        );
    }
}
