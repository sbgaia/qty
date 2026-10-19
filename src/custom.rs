//! Units of a quantity, built in or defined with [`custom_unit!`](crate::custom_unit).

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
