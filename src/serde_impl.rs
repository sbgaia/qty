//! `Serialize` and `Deserialize` for every type of the crate.
//!
//! They behave exactly like `#[derive]` would: quantities as their bare
//! base-unit number (`#[serde(transparent)]`), units as unit variants by
//! name, structs as structs and the enums of `dynamic` as enums with one
//! newtype variant per kind. The derive expands to a field enum and two
//! visitors per type, which for 128 unit enums made `serde` the most
//! expensive feature to build, and needs `serde_derive`, which pulls in
//! `syn` and must be built before qty can start. Here the visitors are
//! written once and each type only passes its names.

use core::fmt;
use core::marker::PhantomData;

use serde::de::{
    self, DeserializeSeed, Deserializer, EnumAccess, IgnoredAny, MapAccess, SeqAccess, Unexpected,
    VariantAccess, Visitor,
};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

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

/// What `#[derive(Serialize)]` does for a struct whose fields all have the
/// same type: `values` in the order of `fields`.
pub(crate) fn serialize_struct<S: Serializer, T: Serialize, const N: usize>(
    serializer: S,
    name: &'static str,
    fields: &'static [&'static str; N],
    values: [&T; N],
) -> Result<S::Ok, S::Error> {
    let mut state = serializer.serialize_struct(name, N)?;
    for (field, value) in fields.iter().zip(values) {
        state.serialize_field(field, value)?;
    }
    state.end()
}

/// What `#[derive(Deserialize)]` does for a struct whose fields all have the
/// same type: the values in the order of `fields`, from a sequence or a map.
/// Unknown fields are skipped, and a missing one is an error, except for an
/// `Option`, which is `None`.
pub(crate) fn deserialize_struct<'de, D: Deserializer<'de>, T: Deserialize<'de>, const N: usize>(
    deserializer: D,
    name: &'static str,
    fields: &'static [&'static str; N],
) -> Result<[T; N], D::Error> {
    deserializer.deserialize_struct(
        name,
        fields,
        StructVisitor {
            name,
            fields,
            value: PhantomData,
        },
    )
}

struct StructVisitor<T, const N: usize> {
    name: &'static str,
    fields: &'static [&'static str; N],
    value: PhantomData<T>,
}

impl<'de, T: Deserialize<'de>, const N: usize> Visitor<'de> for StructVisitor<T, N> {
    type Value = [T; N];

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "struct {}", self.name)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<[T; N], A::Error> {
        let mut values: [Option<T>; N] = [const { None }; N];
        for (i, value) in values.iter_mut().enumerate() {
            match seq.next_element()? {
                Some(v) => *value = Some(v),
                None => return Err(de::Error::invalid_length(i, &StructLength(self.name, N))),
            }
        }
        Ok(values.map(|v| v.unwrap()))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<[T; N], A::Error> {
        let mut values: [Option<T>; N] = [const { None }; N];
        while let Some(field) = map.next_key_seed(FieldIndex(self.fields))? {
            match field {
                Some(i) if values[i].is_some() => {
                    return Err(de::Error::duplicate_field(self.fields[i]))
                }
                Some(i) => values[i] = Some(map.next_value()?),
                None => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }
        for (value, field) in values.iter_mut().zip(self.fields) {
            if value.is_none() {
                *value = Some(T::deserialize(MissingField(field, PhantomData))?);
            }
        }
        Ok(values.map(|v| v.unwrap()))
    }
}

/// "struct Name with N elements", as the derive words it.
struct StructLength(&'static str, usize);

impl de::Expected for StructLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "struct {} with {} elements", self.0, self.1)
    }
}

/// Reads a field identifier, by name or by index, as its index, or `None`
/// for a field the struct doesn't have.
struct FieldIndex(&'static [&'static str]);

impl<'de> DeserializeSeed<'de> for FieldIndex {
    type Value = Option<usize>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Option<usize>, D::Error> {
        deserializer.deserialize_identifier(self)
    }
}

impl<'de> Visitor<'de> for FieldIndex {
    type Value = Option<usize>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("field identifier")
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Option<usize>, E> {
        Ok(usize::try_from(value).ok().filter(|&i| i < self.0.len()))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Option<usize>, E> {
        Ok(self.0.iter().position(|name| *name == value))
    }

    fn visit_bytes<E: de::Error>(self, value: &[u8]) -> Result<Option<usize>, E> {
        Ok(self.0.iter().position(|name| name.as_bytes() == value))
    }
}

/// The value of a field that isn't there: `None` for an `Option`, an error
/// for anything else, as with the derive.
struct MissingField<E>(&'static str, PhantomData<E>);

impl<'de, E: de::Error> Deserializer<'de> for MissingField<E> {
    type Error = E;

    fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, E> {
        Err(de::Error::missing_field(self.0))
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        visitor.visit_none()
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

/// The enums of `dynamic`, with one newtype variant per quantity kind, in
/// the order of `QuantityKind`.
#[cfg(feature = "dynamic")]
pub(crate) trait KindEnum: Sized {
    /// The enum's name.
    const NAME: &'static str;

    /// The variant of `kind`, with its value from `variant`.
    fn variant<'de, A: VariantAccess<'de>>(
        kind: crate::QuantityKind,
        variant: A,
    ) -> Result<Self, A::Error>;
}

/// What `#[derive(Serialize)]` does for the variant of `kind` of a
/// [`KindEnum`].
#[cfg(feature = "dynamic")]
#[inline]
pub(crate) fn serialize_kind_variant<S: Serializer, T: ?Sized + Serialize>(
    serializer: S,
    name: &'static str,
    kind: crate::QuantityKind,
    value: &T,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_newtype_variant(name, kind as u32, kind.info().name, value)
}

/// What `#[derive(Deserialize)]` does for a [`KindEnum`].
#[cfg(feature = "dynamic")]
pub(crate) fn deserialize_kind_enum<'de, D: Deserializer<'de>, T: KindEnum>(
    deserializer: D,
) -> Result<T, D::Error> {
    deserializer.deserialize_enum(T::NAME, crate::any::KIND_NAMES, KindVisitor(PhantomData))
}

#[cfg(feature = "dynamic")]
struct KindVisitor<T>(PhantomData<T>);

#[cfg(feature = "dynamic")]
impl<'de, T: KindEnum> Visitor<'de> for KindVisitor<T> {
    type Value = T;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "enum {}", T::NAME)
    }

    fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<T, A::Error> {
        let (index, variant) = data.variant_seed(VariantIndex(crate::any::KIND_NAMES))?;
        T::variant(crate::any::KINDS[index], variant)
    }
}
