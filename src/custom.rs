//! Units and quantities defined outside qty: [`custom_unit!`](crate::custom_unit)
//! and [`custom_quantity!`](crate::custom_quantity).

use crate::{Quantity, Unit};

/// A unit of the quantity `Q`: the generated unit enums (`LengthUnit` for
/// `Length`, …) and units defined with [`custom_unit!`](crate::custom_unit).
/// Quantities take any of them in `new`, `convert_to` and `in_unit`.
pub trait UnitOf<Q: Quantity>: Unit {
    /// A value in this unit, in the quantity's base unit.
    fn value_to_base(self, value: Q::Value) -> Q::Value;

    /// A value in the quantity's base unit, in this unit.
    fn value_from_base(self, value: Q::Value) -> Q::Value;
}

/// Defines a unit for an existing quantity.
///
/// `factor` is the number of base units in one of this unit (the base unit
/// is the one a quantity is stored in: metres for `Length`, kelvins for
/// `Temperature`; see `Quantity::BASE_UNIT`). Point quantities such as
/// temperatures may add an `offset`, in base units: a value `v` in this
/// unit is `v × factor + offset` base units.
///
/// ```rust
/// use qty::*;
///
/// custom_unit! {
///     /// The furlong: 220 yards.
///     pub struct Furlong for Length {
///         abbreviation: "fur",
///         singular: "Furlong",
///         plural: "Furlongs",
///         factor: 201.168,
///     }
/// }
///
/// let race = Length::new(5.0, Furlong);
/// assert!((race.as_kilometers() - 1.00584).abs() < 1e-12);
/// assert_eq!(format!("{:.1}", Length::from_meters(402.336).in_unit(Furlong)), "2.0 fur");
/// ```
///
/// The unit works wherever a quantity takes a unit: `new`, `convert_to` and
/// `in_unit`. It is a type of your crate, so it is not known to parsing,
/// `Fixed` or `AnyQuantity`.
#[macro_export]
macro_rules! custom_unit {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident for $($q:ident)::+ {
            abbreviation: $abbreviation:literal,
            singular: $singular:literal,
            plural: $plural:literal,
            factor: $factor:expr
            $(, offset: $offset:expr)?
            $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        $vis struct $name;

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.pad($abbreviation)
            }
        }

        impl $crate::Unit for $name {
            fn abbreviation(self) -> &'static str {
                $abbreviation
            }
            fn info(self) -> &'static $crate::UnitInfo {
                static INFO: $crate::UnitInfo = $crate::UnitInfo {
                    singular: $singular,
                    plural: $plural,
                    abbreviation: $abbreviation,
                    abbreviations: &[$abbreviation],
                };
                &INFO
            }
        }

        impl<T: $crate::Float> $crate::UnitOf<$($q)::+<T>> for $name {
            #[inline]
            fn value_to_base(self, value: T) -> T {
                value * T::from_f64($factor) $(+ T::from_f64($offset))?
            }
            #[inline]
            fn value_from_base(self, value: T) -> T {
                (value $(- T::from_f64($offset))?) / T::from_f64($factor)
            }
        }
    };
}

/// Defines a new quantity: a type like `Length`, stored in its base unit,
/// and an enum of its units.
///
/// The first unit is the base unit, in which values are stored. Every other
/// unit gives its size in base units after `=`. A unit's singular name is
/// its variant name, and its plural adds an "s" unless given with
/// `(plural "…")`. The quantity's doc comment becomes its description in
/// [`QuantityInfo`](crate::QuantityInfo).
///
/// ```rust
/// use qty::*;
///
/// custom_quantity! {
///     /// Printed paper, counted in sheets.
///     pub struct Paper;
///
///     pub enum PaperUnit {
///         Sheet: "sh",             // the base unit
///         Quire: "qr" = 25.0,      // sheets in a quire
///         Ream: "rm" = 500.0,
///         Box: "box" (plural "Boxes") = 5000.0,
///     }
/// }
///
/// let order = Paper::new(3.0, PaperUnit::Ream) + Paper::new(2.0, PaperUnit::Quire);
/// assert_eq!(order.convert_to(PaperUnit::Sheet), 1550.0);
/// assert_eq!(order.in_unit(PaperUnit::Ream).to_string(), "3.1 rm");
/// assert_eq!("2 boxes".parse::<Paper>(), Ok(Paper::new(10_000.0, PaperUnit::Sheet)));
/// ```
///
/// The quantity has what the built-in ones have that does not depend on
/// their generated per-unit code: arithmetic with itself and with numbers,
/// `new`, `convert_to`, `in_unit`, `auto_unit_in`, `const_new`, `ZERO`,
/// formatting, parsing unit abbreviations and names, [`Quantity`], and
/// qty's `serde`, `approx` and `bytemuck` integrations when those features
/// are on. It has no `from_<unit>` / `as_<unit>` methods, and parsing,
/// `Fixed`, `AnyQuantity`, defmt and schemars do not know it. It has no
/// physical dimensions, so unit expressions such as `"km/h"` do not parse
/// as it, and products with other quantities are yours to define:
///
/// ```rust
/// # use qty::*;
/// # custom_quantity! { pub struct Paper; pub enum PaperUnit { Sheet: "sh" } }
/// impl core::ops::Mul<Length> for Paper {
///     type Output = Length;
///     /// The height of a stack, from the thickness of one sheet.
///     fn mul(self, thickness: Length) -> Length {
///         thickness * self.convert_to(PaperUnit::Sheet)
///     }
/// }
/// let stack = Paper::new(500.0, PaperUnit::Sheet) * Length::from_millimeters(0.1);
/// assert!((stack.as_centimeters() - 5.0).abs() < 1e-12);
/// ```
///
/// Units can also be added to a custom quantity with
/// [`custom_unit!`](crate::custom_unit).
#[macro_export]
macro_rules! custom_quantity {
    (
        $(#[doc = $doc:literal])*
        $vis:vis struct $Q:ident;

        $(#[doc = $unit_doc:literal])*
        $unit_vis:vis enum $U:ident {
            $(#[doc = $base_doc:literal])*
            $base:ident: $base_abbreviation:literal $((plural $base_plural:literal))?
            $(,
                $(#[doc = $variant_doc:literal])*
                $unit:ident: $abbreviation:literal $((plural $plural:literal))? = $factor:expr
            )*
            $(,)?
        }
    ) => {
        $(#[doc = $doc])*
        #[doc = ""]
        #[doc = concat!(
            "Stored as a value of type `T` (default `f64`) in the base unit, [`",
            stringify!($U), "::", stringify!($base), "`].",
        )]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        #[repr(transparent)]
        #[must_use]
        $vis struct $Q<T = f64>(T);

        #[doc = concat!("Units of [`", stringify!($Q), "`].")]
        #[doc = ""]
        $(#[doc = $unit_doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        $unit_vis enum $U {
            #[doc = concat!("`", $base_abbreviation, "`, the base unit.")]
            #[doc = ""]
            $(#[doc = $base_doc])*
            $base,
            $(
                #[doc = concat!(
                    "`", $abbreviation, "`: ", stringify!($factor), " × [`",
                    stringify!($base), "`](Self::", stringify!($base), ").",
                )]
                #[doc = ""]
                $(#[doc = $variant_doc])*
                $unit,
            )*
        }

        impl $U {
            /// Every unit of the quantity, the base unit first.
            pub const ALL: &'static [Self] = &[Self::$base, $(Self::$unit),*];

            const INFOS: &'static [$crate::UnitInfo] = &[
                $crate::UnitInfo {
                    singular: stringify!($base),
                    plural: $crate::__qty_plural!($base $(, $base_plural)?),
                    abbreviation: $base_abbreviation,
                    abbreviations: &[$base_abbreviation],
                },
                $($crate::UnitInfo {
                    singular: stringify!($unit),
                    plural: $crate::__qty_plural!($unit $(, $plural)?),
                    abbreviation: $abbreviation,
                    abbreviations: &[$abbreviation],
                },)*
            ];

            const DOC: &'static str = concat!($($doc, "\n",)*);
            const DESCRIPTION: ([u8; $U::DOC.len()], usize) = $crate::__private::doc_text($U::DOC);

            #[doc = concat!("Name, description and units of [`", stringify!($Q), "`].")]
            pub const QUANTITY: $crate::QuantityInfo = $crate::QuantityInfo {
                name: stringify!($Q),
                description: $crate::__private::doc_str(&$U::DESCRIPTION),
                dimensions: $crate::Dimensions::DIMENSIONLESS,
                base_unit: &Self::INFOS[0],
                units: Self::INFOS,
            };

            /// Names and abbreviation.
            #[inline]
            pub const fn info(self) -> &'static $crate::UnitInfo {
                &Self::INFOS[self as usize]
            }

            /// The abbreviation, as displayed.
            #[inline]
            pub const fn abbreviation(self) -> &'static str {
                self.info().abbreviation
            }

            /// Parses a unit name, singular or plural, ignoring case,
            /// spaces, `_` and `-`.
            pub fn from_name(name: &str) -> ::core::option::Option<Self> {
                $crate::__private::unit_from_name(name)
            }
        }

        /// Parses a unit abbreviation (exact) or name (lenient, see
        /// `from_name`).
        impl ::core::str::FromStr for $U {
            type Err = $crate::ParseError;
            fn from_str(s: &str) -> ::core::result::Result<Self, $crate::ParseError> {
                $crate::__private::unit_from_str(s)
            }
        }

        impl<T: $crate::Float> $crate::UnitOf<$Q<T>> for $U {
            #[inline]
            fn value_to_base(self, value: T) -> T {
                match self {
                    $U::$base => value,
                    $($U::$unit => value * T::from_f64($factor),)*
                }
            }
            #[inline]
            fn value_from_base(self, value: T) -> T {
                match self {
                    $U::$base => value,
                    $($U::$unit => value / T::from_f64($factor),)*
                }
            }
        }

        impl $Q<f64> {
            /// Like `new`, but `const`: for constants in any unit.
            ///
            /// The result is identical to `new(value, unit)`.
            #[inline]
            pub const fn const_new(value: f64, unit: $U) -> Self {
                Self(match unit {
                    $U::$base => value,
                    $($U::$unit => value * $factor,)*
                })
            }
        }

        $crate::__qty_common!($Q, $U, $base, names_only: true);
        $crate::__qty_vector_ops!($Q);

        $crate::__qty_if_serde! {
            /// The bare number in the base unit.
            impl<T: $crate::__private::serde::Serialize> $crate::__private::serde::Serialize for $Q<T> {
                fn serialize<S: $crate::__private::serde::Serializer>(
                    &self,
                    serializer: S,
                ) -> ::core::result::Result<S::Ok, S::Error> {
                    T::serialize(&self.0, serializer)
                }
            }
            impl<'de, T: $crate::__private::serde::Deserialize<'de>> $crate::__private::serde::Deserialize<'de>
                for $Q<T>
            {
                fn deserialize<D: $crate::__private::serde::Deserializer<'de>>(
                    deserializer: D,
                ) -> ::core::result::Result<Self, D::Error> {
                    T::deserialize(deserializer).map(Self)
                }
            }
            /// The variant name, like the built-in unit enums.
            impl $crate::__private::serde::Serialize for $U {
                fn serialize<S: $crate::__private::serde::Serializer>(
                    &self,
                    serializer: S,
                ) -> ::core::result::Result<S::Ok, S::Error> {
                    $crate::__private::serialize_unit(*self, serializer, stringify!($U))
                }
            }
            impl<'de> $crate::__private::serde::Deserialize<'de> for $U {
                fn deserialize<D: $crate::__private::serde::Deserializer<'de>>(
                    deserializer: D,
                ) -> ::core::result::Result<Self, D::Error> {
                    $crate::__private::deserialize_unit(
                        deserializer,
                        stringify!($U),
                        &[stringify!($base), $(stringify!($unit)),*],
                    )
                }
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __qty_plural {
    ($unit:ident) => {
        concat!(stringify!($unit), "s")
    };
    ($unit:ident, $plural:literal) => {
        $plural
    };
}

// Switches for code that the exported macros generate only with some of
// qty's features: a `cfg` in the generated code would test the features of
// the crate it expands in. (Written out: macros generated by a macro cannot
// be called by path from within the crate.)

#[cfg(any(feature = "std", feature = "libm"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __qty_if_math {
    ($($t:tt)*) => { $($t)* };
}
#[cfg(not(any(feature = "std", feature = "libm")))]
#[doc(hidden)]
#[macro_export]
macro_rules! __qty_if_math {
    ($($t:tt)*) => {};
}

#[cfg(feature = "serde")]
#[doc(hidden)]
#[macro_export]
macro_rules! __qty_if_serde {
    ($($t:tt)*) => { $($t)* };
}
#[cfg(not(feature = "serde"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __qty_if_serde {
    ($($t:tt)*) => {};
}

#[cfg(feature = "approx")]
#[doc(hidden)]
#[macro_export]
macro_rules! __qty_if_approx {
    ($($t:tt)*) => { $($t)* };
}
#[cfg(not(feature = "approx"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __qty_if_approx {
    ($($t:tt)*) => {};
}

#[cfg(feature = "bytemuck")]
#[doc(hidden)]
#[macro_export]
macro_rules! __qty_if_bytemuck {
    ($($t:tt)*) => { $($t)* };
}
#[cfg(not(feature = "bytemuck"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __qty_if_bytemuck {
    ($($t:tt)*) => {};
}

/// Support for the exported macros.
#[doc(hidden)]
pub mod private {
    #[cfg(feature = "approx")]
    pub use approx;
    #[cfg(feature = "bytemuck")]
    pub use bytemuck;
    #[cfg(feature = "serde")]
    pub use serde;

    use crate::{normalize_unit_name, ParseError, UnitEnum};

    /// Like the generated `from_name`: a singular or plural name.
    pub fn unit_from_name<U: UnitEnum>(name: &str) -> Option<U> {
        let mut buf = [0; 64];
        let name = normalize_unit_name(name, &mut buf)?;
        U::ALL.iter().copied().find(|unit| {
            let info = unit.info();
            [info.singular, info.plural].into_iter().any(|candidate| {
                let mut buf = [0; 64];
                normalize_unit_name(candidate, &mut buf) == Some(name)
            })
        })
    }

    /// Like the generated `FromStr`: an exact abbreviation, else a name.
    pub fn unit_from_str<U: UnitEnum>(s: &str) -> Result<U, ParseError> {
        U::ALL
            .iter()
            .copied()
            .find(|unit| unit.info().abbreviations.contains(&s))
            .or_else(|| unit_from_name(s))
            .ok_or(ParseError::UnknownUnit)
    }

    /// The first paragraph of a doc comment (lines joined by `"\n"`), its
    /// lines trimmed and joined by spaces, in the first `.1` bytes.
    pub const fn doc_text<const N: usize>(doc: &str) -> ([u8; N], usize) {
        const fn is_space(b: u8) -> bool {
            matches!(b, b' ' | b'\t' | b'\r')
        }
        let doc = doc.as_bytes();
        let mut out = [0; N];
        let mut len = 0;
        let mut start = 0;
        while start < doc.len() {
            let mut end = start;
            while end < doc.len() && doc[end] != b'\n' {
                end += 1;
            }
            let (mut a, mut b) = (start, end);
            while a < b && is_space(doc[a]) {
                a += 1;
            }
            while b > a && is_space(doc[b - 1]) {
                b -= 1;
            }
            if a == b {
                if len > 0 {
                    break;
                }
            } else {
                if len > 0 {
                    out[len] = b' ';
                    len += 1;
                }
                while a < b {
                    out[len] = doc[a];
                    len += 1;
                    a += 1;
                }
            }
            start = end + 1;
        }
        (out, len)
    }

    /// The text of [`doc_text`]. It is cut at ASCII bytes, so it is UTF-8.
    pub const fn doc_str<const N: usize>(text: &'static ([u8; N], usize)) -> &'static str {
        match core::str::from_utf8(text.0.split_at(text.1).0) {
            Ok(s) => s,
            Err(_) => "",
        }
    }

    /// Serializes a unit as its variant, like `#[derive(Serialize)]`.
    #[cfg(feature = "serde")]
    pub fn serialize_unit<U: UnitEnum, S: serde::Serializer>(
        unit: U,
        serializer: S,
        name: &'static str,
    ) -> Result<S::Ok, S::Error> {
        let index = U::ALL.iter().position(|&u| u == unit).unwrap_or(0);
        serializer.serialize_unit_variant(name, index as u32, unit.info().singular)
    }

    /// Deserializes a unit like `#[derive(Deserialize)]`: from its variant
    /// name or, in formats that write it, its index.
    #[cfg(feature = "serde")]
    pub fn deserialize_unit<'de, U: UnitEnum, D: serde::Deserializer<'de>>(
        deserializer: D,
        name: &'static str,
        variants: &'static [&'static str],
    ) -> Result<U, D::Error> {
        use core::{fmt, marker::PhantomData};
        use serde::de::{self, DeserializeSeed, EnumAccess, VariantAccess, Visitor};

        struct Variant<U>(&'static [&'static str], PhantomData<U>);
        impl<U: UnitEnum> Visitor<'_> for Variant<U> {
            type Value = U;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a unit name")
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<U, E> {
                usize::try_from(v)
                    .ok()
                    .and_then(|i| U::ALL.get(i).copied())
                    .ok_or_else(|| E::invalid_value(de::Unexpected::Unsigned(v), &self))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<U, E> {
                U::ALL
                    .iter()
                    .copied()
                    .find(|u| u.info().singular == v)
                    .ok_or_else(|| E::unknown_variant(v, self.0))
            }
            fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<U, E> {
                match core::str::from_utf8(v) {
                    Ok(s) => self.visit_str(s),
                    Err(_) => Err(E::invalid_value(de::Unexpected::Bytes(v), &self)),
                }
            }
        }
        impl<'de, U: UnitEnum> DeserializeSeed<'de> for Variant<U> {
            type Value = U;
            fn deserialize<D: serde::Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<U, D::Error> {
                deserializer.deserialize_identifier(self)
            }
        }

        struct Enum<U>(&'static [&'static str], PhantomData<U>);
        impl<'de, U: UnitEnum> Visitor<'de> for Enum<U> {
            type Value = U;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a unit")
            }
            fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<U, A::Error> {
                let (unit, variant) = data.variant_seed(Variant(self.0, PhantomData))?;
                variant.unit_variant()?;
                Ok(unit)
            }
        }

        deserializer.deserialize_enum(name, variants, Enum(variants, PhantomData))
    }
}
