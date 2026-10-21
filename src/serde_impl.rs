//! `Serialize` and `Deserialize` for the built-in quantities and unit enums.
//!
//! They behave exactly like `#[derive]` would: quantities as their bare
//! base-unit number (`#[serde(transparent)]`), units as unit variants by
//! name. The derive expands to a field enum and two visitors per type, which
//! for 128 unit enums made `serde` the most expensive feature to build.
//! Here the visitors are written once and each enum only passes its names.

use core::fmt;

use serde::de::{
    self, DeserializeSeed, Deserializer, EnumAccess, Unexpected, VariantAccess, Visitor,
};
use serde::Serializer;

use crate::UnitInfo;

/// The variant names of a unit enum, from its `UnitInfo`s: each variant is
/// named after the unit's `singular`.
pub(crate) const fn variant_names<const N: usize>(units: &[UnitInfo]) -> [&'static str; N] {
    let mut names = [""; N];
    let mut i = 0;
    while i < N {
        names[i] = units[i].singular;
        i += 1;
    }
    names
}

/// What `#[derive(Serialize)]` does for a unit variant.
#[inline]
pub(crate) fn serialize_unit<S: Serializer>(
    serializer: S,
    enum_name: &'static str,
    index: usize,
    names: &'static [&'static str],
) -> Result<S::Ok, S::Error> {
    serializer.serialize_unit_variant(enum_name, index as u32, names[index])
}

/// What `#[derive(Deserialize)]` does for an enum of unit variants. `all`
/// lists the variants in declaration order, matching `names`.
pub(crate) fn deserialize_unit<'de, D: Deserializer<'de>, U: Copy + 'static>(
    deserializer: D,
    enum_name: &'static str,
    names: &'static [&'static str],
    all: &'static [U],
) -> Result<U, D::Error> {
    deserializer.deserialize_enum(
        enum_name,
        names,
        UnitVisitor {
            enum_name,
            names,
            all,
        },
    )
}

struct UnitVisitor<U: 'static> {
    enum_name: &'static str,
    names: &'static [&'static str],
    all: &'static [U],
}

impl<'de, U: Copy + 'static> Visitor<'de> for UnitVisitor<U> {
    type Value = U;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "enum {}", self.enum_name)
    }

    fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<U, A::Error> {
        let (index, variant) = data.variant_seed(VariantIndex(self.names))?;
        variant.unit_variant()?;
        Ok(self.all[index])
    }
}

/// Reads a variant identifier, by name or by index, as its index.
struct VariantIndex(&'static [&'static str]);

impl<'de> DeserializeSeed<'de> for VariantIndex {
    type Value = usize;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<usize, D::Error> {
        deserializer.deserialize_identifier(self)
    }
}

impl<'de> Visitor<'de> for VariantIndex {
    type Value = usize;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("variant identifier")
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<usize, E> {
        match usize::try_from(value) {
            Ok(index) if index < self.0.len() => Ok(index),
            _ => Err(E::invalid_value(
                Unexpected::Unsigned(value),
                &IndexRange(self.0.len()),
            )),
        }
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<usize, E> {
        self.0
            .iter()
            .position(|name| *name == value)
            .ok_or_else(|| E::unknown_variant(value, self.0))
    }

    fn visit_bytes<E: de::Error>(self, value: &[u8]) -> Result<usize, E> {
        match self.0.iter().position(|name| name.as_bytes() == value) {
            Some(index) => Ok(index),
            #[cfg(feature = "std")]
            None => Err(E::unknown_variant(
                &std::string::String::from_utf8_lossy(value),
                self.0,
            )),
            #[cfg(not(feature = "std"))]
            None => Err(E::unknown_variant(
                core::str::from_utf8(value).unwrap_or("\u{fffd}\u{fffd}\u{fffd}"),
                self.0,
            )),
        }
    }
}

/// "variant index 0 <= i < N", as the derive words it.
struct IndexRange(usize);

impl de::Expected for IndexRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "variant index 0 <= i < {}", self.0)
    }
}
