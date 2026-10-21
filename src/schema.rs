//! `JsonSchema` for the built-in quantities and unit enums, through shared
//! functions so that each of the 128 quantities only passes its names.

use std::borrow::Cow;
use std::format;
use std::vec::Vec;

use schemars::{JsonSchema, Schema, SchemaGenerator};

use crate::UnitInfo;

/// `Length_double` for `Length<f64>`.
pub(crate) fn quantity_schema_name<T: JsonSchema>(quantity: &str) -> Cow<'static, str> {
    format!("{}_{}", quantity, T::schema_name()).into()
}

/// The storage type's schema (a number for floats), described with the base
/// unit.
pub(crate) fn quantity_schema<T: JsonSchema>(
    generator: &mut SchemaGenerator,
    quantity: &str,
    base: &UnitInfo,
) -> Schema {
    let mut schema = T::json_schema(generator);
    schema.insert(
        "description".into(),
        format!("{} in {} ({})", quantity, base.plural, base.abbreviation).into(),
    );
    schema
}

/// The variant names, as serde writes them.
pub(crate) fn unit_schema(units: &[UnitInfo]) -> Schema {
    let names: Vec<&str> = units.iter().map(|u| u.singular).collect();
    schemars::json_schema!({ "type": "string", "enum": names })
}
