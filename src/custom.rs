//! Units, quantities and relations defined in other crates:
//! [`custom_unit!`](crate::custom_unit), [`custom_quantity!`](crate::custom_quantity)
//! and [`relation!`](crate::relation).

use crate::{Quantity, Unit};

/// A unit of `Q`.
///
/// Implemented by the generated unit enums (`LengthUnit` for `Length`) and
/// by units from [`custom_unit!`](crate::custom_unit). `new`, `convert_to`
/// and `in_unit` accept any of them.
pub trait UnitOf<Q: Quantity>: Unit {
    /// Converts `value` from this unit to the base unit.
    fn value_to_base(self, value: Q::Value) -> Q::Value;

    /// Converts `value` from the base unit to this unit.
    fn value_from_base(self, value: Q::Value) -> Q::Value;
}

/// Defines a unit for an existing quantity.
///
/// `factor` is how many base units make one of the new unit. The base unit
/// is the one the quantity is stored in, such as metres for `Length`. Units
/// of point quantities like `Temperature` can also take an `offset`, so
/// that `v` in the new unit is `v * factor + offset` base units.
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
/// The unit works with `new`, `convert_to` and `in_unit`. Parsing, `Fixed`
/// and `AnyQuantity` don't know about it.
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

        impl<T: $crate::Number> $crate::UnitOf<$($q)::+<T>> for $name {
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

/// Defines a new quantity and the enum of its units.
///
/// The first unit is the base unit, the one values are stored in. The
/// others give their size in base units after `=`. Unit names come from the
/// variant names, with an "s" added for the plural unless you give one with
/// `(plural "...")`. The doc comment becomes the quantity's description in
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
/// The new quantity gets most of what the built-in ones have: arithmetic,
/// `new`, `convert_to`, `in_unit`, `auto_unit_in`, `const_new`, `ZERO`,
/// formatting and parsing, the [`Quantity`] trait, and the `serde`, `approx`
/// and `bytemuck` impls when those features are on. It has no
/// `from_<unit>` or `as_<unit>` methods, and `Fixed`, `AnyQuantity`, defmt
/// and schemars don't support it.
///
/// # Dimensions
///
/// You can give the quantity its physical dimensions, as exponents of the
/// fields of [`Dimensions`](crate::Dimensions). The base unit's
/// abbreviation then has to be a [unit expression](crate::UnitExpr) with
/// those dimensions. Any unit with the same dimensions then parses as the
/// quantity, and it can be used in [`formula!`](crate::formula!) and
/// [`relation!`](crate::relation).
///
/// ```rust
/// use qty::*;
///
/// custom_quantity! {
///     /// The weight of paper per area.
///     pub struct Grammage;
///     dimensions { mass: 1, length: -2 }
///
///     pub enum GrammageUnit {
///         GramPerSquareMeter: "g/m²" (plural "GramsPerSquareMeter"),
///         OuncePerSquareYard: "oz/yd²" (plural "OuncesPerSquareYard") = 33.9057474748823,
///     }
/// }
///
/// let office: Grammage = "80 g/m^2".parse()?;
/// assert_eq!(office, "0.08 kg/m²".parse()?);
/// assert_eq!(format!("{:.2}", office.in_unit(GrammageUnit::OuncePerSquareYard)), "2.36 oz/yd²");
/// assert_eq!(Grammage::<f64>::INFO.dimensions, Dimensions::new(-2, 1, 0, 0, 0, 0, 0)); // L⁻² M
/// assert_eq!("80 g/m".parse::<Grammage>(), Err(ParseError::UnknownUnit));
/// # Ok::<(), ParseError>(())
/// ```
///
/// A base unit that isn't an expression fails to compile:
///
/// ```rust,compile_fail,E0080
/// # use qty::*;
/// custom_quantity! {
///     pub struct Grammage;
///     dimensions { mass: 1, length: -2 }
///     pub enum GrammageUnit { Gsm: "gsm" } // not a unit expression
/// }
/// ```
///
/// Without dimensions, write products with other quantities by hand:
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
/// [`custom_unit!`](crate::custom_unit) can add units to a custom quantity
/// too.
#[macro_export]
macro_rules! custom_quantity {
    (
        $(#[doc = $doc:literal])*
        $vis:vis struct $Q:ident;
        dimensions { $($dimension:ident: $exponent:literal),* $(,)? }
        $($rest:tt)*
    ) => {
        $crate::custom_quantity! {
            @define [$($doc),*] [$vis] $Q,
            [$crate::Dimensions { $($dimension: $exponent,)* ..$crate::Dimensions::DIMENSIONLESS }],
            Base;
            $($rest)*
        }
        $crate::custom_quantity!(@formula $Q, $($rest)*);
        const _: () = ::core::assert!(
            <$Q<f64> as $crate::Quantity>::BASE_UNIT_EXPR.is_some(),
            ::core::concat!(
                "custom_quantity! ", ::core::stringify!($Q), ": the base unit must be a unit ",
                "expression with the given dimensions, like \"kg/m^2\"",
            ),
        );
    };
    (
        $(#[doc = $doc:literal])*
        $vis:vis struct $Q:ident;
        $($rest:tt)*
    ) => {
        $crate::custom_quantity! {
            @define [$($doc),*] [$vis] $Q, [$crate::Dimensions::DIMENSIONLESS], Off;
            $($rest)*
        }
    };
    (@formula $Q:ident, $(#[$attr:meta])* $unit_vis:vis enum $U:ident $body:tt) => {
        $crate::__qty_formula!($Q, $U);
    };
    (
        @define [$($doc:literal),*] [$vis:vis] $Q:ident, [$($dimensions:tt)*], $exprs:ident;

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
            "Stored as `T` (by default `f64`) in [`", stringify!($U), "::", stringify!($base), "`].",
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
                    "`", $abbreviation, "`, ", stringify!($factor), " × [`",
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

            #[doc = concat!("Name, description, dimensions and units of [`", stringify!($Q), "`].")]
            pub const QUANTITY: $crate::QuantityInfo = $crate::QuantityInfo {
                name: stringify!($Q),
                description: $crate::__private::doc_str(&$U::DESCRIPTION),
                dimensions: $($dimensions)*,
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

        /// Accepts an abbreviation (case-sensitive) or a name (see
        /// `from_name`).
        impl ::core::str::FromStr for $U {
            type Err = $crate::ParseError;
            fn from_str(s: &str) -> ::core::result::Result<Self, $crate::ParseError> {
                $crate::__private::unit_from_str(s)
            }
        }

        impl<T: $crate::Number> $crate::UnitOf<$Q<T>> for $U {
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
            /// Same as `new`, but usable in constants.
            #[inline]
            pub const fn const_new(value: f64, unit: $U) -> Self {
                Self(match unit {
                    $U::$base => value,
                    $($U::$unit => value * $factor,)*
                })
            }
        }

        $crate::__qty_common!($Q, $U, $base, exprs: $exprs);
        $crate::__qty_vector_ops!($Q);
        $crate::__qty_scalar_ops!($Q);

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

/// Implements `*` or `/` between a custom quantity and another quantity.
///
/// `A * B = C` implements `Mul<B> for A` with `C` as the output, and
/// `A / B = C` implements `Div<B> for A`. The factor between the three base
/// units is worked out at compile time by reading their abbreviations as
/// [unit expressions](crate::UnitExpr), and a relation whose dimensions
/// don't add up fails to compile. Custom quantities need
/// [`dimensions`](crate::custom_quantity#dimensions) for this.
///
/// ```rust
/// use qty::*;
///
/// custom_quantity! {
///     /// The weight of paper per area.
///     pub struct Grammage;
///     dimensions { mass: 1, length: -2 }
///
///     pub enum GrammageUnit {
///         GramPerSquareMeter: "g/m²" (plural "GramsPerSquareMeter"),
///     }
/// }
///
/// relation! {
///     Grammage * Area = Mass;
///     Area * Grammage = Mass;
///     Mass / Grammage = Area;
/// }
///
/// let ream = "80 g/m^2".parse::<Grammage>()? * Area::from_square_meters(500.0 / 16.0);
/// assert!((ream.as_kilograms() - 2.5).abs() < 1e-12);
/// let sheets = Mass::from_kilograms(2.5) / Grammage::new(80.0, GrammageUnit::GramPerSquareMeter);
/// assert!((sheets.as_square_meters() - 31.25).abs() < 1e-12);
/// # Ok::<(), ParseError>(())
/// ```
///
/// Each line makes one impl, so list both orders of a product if you want
/// both. Because of Rust's orphan rule, one of the two operands has to be a
/// type from your crate: you can't add relations between built-in
/// quantities. Any float storage works, so `Grammage<f32> * Area<f32>` is a
/// `Mass<f32>`.
///
/// A relation whose dimensions don't match fails to compile:
///
/// ```rust,compile_fail,E0080
/// # use qty::*;
/// # custom_quantity! {
/// #     pub struct Grammage;
/// #     dimensions { mass: 1, length: -2 }
/// #     pub enum GrammageUnit { GramPerSquareMeter: "g/m²" (plural "GramsPerSquareMeter") }
/// # }
/// relation!(Grammage * Length = Mass); // M·L⁻² × L is not M
/// ```
#[macro_export]
macro_rules! relation {
    () => {};
    ($($a:ident)::+ * $($b:ident)::+ = $($c:ident)::+ $(; $($rest:tt)*)?) => {
        $crate::__qty_relation!(
            Mul, mul, *, false, [$($a)::+], [$($b)::+], [$($c)::+],
            ::core::stringify!($($a)::+ * $($b)::+ = $($c)::+)
        );
        $($crate::relation!($($rest)*);)?
    };
    ($($a:ident)::+ / $($b:ident)::+ = $($c:ident)::+ $(; $($rest:tt)*)?) => {
        $crate::__qty_relation!(
            Div, div, /, true, [$($a)::+], [$($b)::+], [$($c)::+],
            ::core::stringify!($($a)::+ / $($b)::+ = $($c)::+)
        );
        $($crate::relation!($($rest)*);)?
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __qty_relation {
    (
        $Trait:ident, $method:ident, $op:tt, $divide:literal,
        [$($a:tt)*], [$($b:tt)*], [$($c:tt)*], $text:expr
    ) => {
        impl<T: $crate::Number> ::core::ops::$Trait<$($b)*<T>> for $($a)*<T> {
            type Output = $($c)*<T>;
            #[inline]
            fn $method(self, rhs: $($b)*<T>) -> $($c)*<T> {
                const K: f64 = match $crate::__private::relation_factor(
                    <$($a)*<f64> as $crate::Quantity>::BASE_UNIT_EXPR,
                    <$($b)*<f64> as $crate::Quantity>::BASE_UNIT_EXPR,
                    <$($c)*<f64> as $crate::Quantity>::BASE_UNIT_EXPR,
                    $divide,
                ) {
                    ::core::result::Result::Ok(k) => k,
                    ::core::result::Result::Err($crate::__private::RelationError::Lhs) => ::core::panic!(
                        ::core::concat!("relation!(", $text, "): the base unit of `", ::core::stringify!($($a)*),
                            "` is not a unit expression")
                    ),
                    ::core::result::Result::Err($crate::__private::RelationError::Rhs) => ::core::panic!(
                        ::core::concat!("relation!(", $text, "): the base unit of `", ::core::stringify!($($b)*),
                            "` is not a unit expression")
                    ),
                    ::core::result::Result::Err($crate::__private::RelationError::Output) => ::core::panic!(
                        ::core::concat!("relation!(", $text, "): the base unit of `", ::core::stringify!($($c)*),
                            "` is not a unit expression")
                    ),
                    ::core::result::Result::Err($crate::__private::RelationError::Dimensions) => ::core::panic!(
                        ::core::concat!("relation!(", $text, "): dimensions differ")
                    ),
                };
                let value = $crate::Quantity::as_base(self) $op $crate::Quantity::as_base(rhs);
                <$($c)*<T> as $crate::Quantity>::from_base(if K == 1.0 { value } else { value * T::from_f64(K) })
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

// The exported macros expand in other crates, where `#[cfg(feature = ...)]`
// would test that crate's features. These switches test qty's instead.
// They're written out by hand because a macro defined by another macro
// can't be called by path from inside the crate.

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

/// Used by the exported macros. Not public API.
#[doc(hidden)]
pub mod private {
    #[cfg(feature = "approx")]
    pub use approx;
    #[cfg(feature = "bytemuck")]
    pub use bytemuck;
    #[cfg(feature = "serde")]
    pub use serde_core as serde;

    use core::fmt;

    use crate::{
        normalize_unit_name, Dimensions, Float, InUnit, Number, ParseError, Quantity, QuantityInfo,
        UnitEnum, UnitExpr,
    };

    /// Which unit expressions a quantity's `FromStr` accepts.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Exprs {
        /// None. For points and custom quantities without dimensions.
        Off,
        /// Any with the quantity's dimensions.
        Base,
    }

    /// The factor from the unit expression `unit` to `Q`'s base unit.
    pub fn expr_factor<Q: Quantity>(unit: &str, exprs: Exprs) -> Option<f64> {
        match exprs {
            Exprs::Off => None,
            Exprs::Base => UnitExpr::parse(unit).ok()?.factor_to_base::<Q>(),
        }
    }

    const fn same_dimensions(a: Dimensions, b: Dimensions) -> bool {
        let (a, b) = (a.to_array(), b.to_array());
        let mut i = 0;
        while i < 7 {
            if a[i] != b[i] {
                return false;
            }
            i += 1;
        }
        true
    }

    /// `Quantity::BASE_UNIT_EXPR`: the base unit's abbreviation read as an
    /// expression, if it has the quantity's dimensions, and `fallback`
    /// otherwise. Built-in quantities pass the base unit worked out from
    /// UnitsNet's data. A dimensionless quantity whose base unit has no
    /// abbreviation, like a ratio's decimal fraction, is a plain number.
    pub const fn base_unit_expr(
        info: QuantityInfo,
        fallback: Option<UnitExpr>,
    ) -> Option<UnitExpr> {
        let abbreviation = info.base_unit.abbreviation;
        if abbreviation.is_empty() && info.dimensions.is_dimensionless() {
            return Some(si_unit([0; 7], 1.0));
        }
        match UnitExpr::parse(abbreviation) {
            Ok(e) if same_dimensions(e.dimensions, info.dimensions) => Some(e),
            _ => fallback,
        }
    }

    /// A unit in coherent SI: `size` times the SI base units raised to
    /// `dims`, in the order L, M, T, I, Θ, N, J.
    pub const fn si_unit(dims: [i8; 7], size: f64) -> UnitExpr {
        UnitExpr {
            dimensions: Dimensions::new(
                dims[0], dims[1], dims[2], dims[3], dims[4], dims[5], dims[6],
            ),
            angle: 0,
            factor: size,
        }
    }

    /// Why `relation!` rejects a relation.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum RelationError {
        /// The first quantity's base unit is not a unit expression.
        Lhs,
        /// The second quantity's.
        Rhs,
        /// The result's.
        Output,
        /// The dimensions (or angles) of the two sides differ.
        Dimensions,
    }

    /// The factor `k` with `c = a * b * k` (or `a / b * k` when `divide`),
    /// in base units. Takes the three quantities' `BASE_UNIT_EXPR`s.
    pub const fn relation_factor(
        a: Option<UnitExpr>,
        b: Option<UnitExpr>,
        c: Option<UnitExpr>,
        divide: bool,
    ) -> Result<f64, RelationError> {
        let Some(a) = a else {
            return Err(RelationError::Lhs);
        };
        let Some(b) = b else {
            return Err(RelationError::Rhs);
        };
        let Some(c) = c else {
            return Err(RelationError::Output);
        };
        let product = a.mul(if divide { b.powi(-1) } else { b });
        if same_dimensions(product.dimensions, c.dimensions) && product.angle == c.angle {
            Ok(product.factor / c.factor)
        } else {
            Err(RelationError::Dimensions)
        }
    }

    // The bodies of methods every quantity has. `__qty_common!` delegates to
    // these, so the compiler checks each body once rather than once per
    // quantity.

    /// `auto_unit_in`.
    pub fn auto_unit_in<Q: Quantity>(quantity: Q, units: &[Q::Unit]) -> InUnit<Q::Value, Q::Unit>
    where
        Q::Value: Float,
    {
        let magnitude = quantity.as_base().abs();
        let one = |unit: Q::Unit| Q::new(Q::Value::from_f64(1.0), unit).as_base().abs();
        let mut best: Option<(Q::Unit, Q::Value)> = None;
        let mut smallest: Option<(Q::Unit, Q::Value)> = None;
        for &unit in units {
            let size = one(unit);
            if smallest.is_none_or(|(_, s)| size < s) {
                smallest = Some((unit, size));
            }
            if size <= magnitude && best.is_none_or(|(_, b)| size > b) {
                best = Some((unit, size));
            }
        }
        let unit = best.or(smallest).map_or(Q::BASE_UNIT, |(unit, _)| unit);
        InUnit {
            value: quantity.convert_to(unit),
            unit,
        }
    }

    /// The generated `auto_unit`: the first unit of `scale` (largest first)
    /// that doesn't exceed the value, else the last one. Zero, NaN and
    /// infinities stay in the base unit.
    pub fn auto_unit<Q: Quantity>(
        quantity: Q,
        scale: &[(f64, Q::Unit)],
    ) -> InUnit<Q::Value, Q::Unit>
    where
        Q::Value: Float,
    {
        let magnitude = quantity.as_base().abs();
        let unit = if magnitude == Q::Value::from_f64(0.0) || !magnitude.is_finite() {
            Q::BASE_UNIT
        } else {
            scale
                .iter()
                .find(|&&(factor, _)| magnitude >= Q::Value::from_f64(factor))
                .map_or(scale[scale.len() - 1].1, |&(_, unit)| unit)
        };
        InUnit {
            value: quantity.convert_to(unit),
            unit,
        }
    }

    /// `mean`.
    pub fn mean<Q: Quantity, I: IntoIterator<Item = Q>>(values: I) -> Option<Q>
    where
        Q::Value: Float,
    {
        let mut values = values.into_iter();
        let first = values.next()?.as_base();
        let mut sum = Q::Value::from_f64(0.0);
        let mut n = 1_u32;
        for q in values {
            sum = sum + (q.as_base() - first);
            n += 1;
        }
        Some(Q::from_base(first + sum / Q::Value::from_f64(f64::from(n))))
    }

    /// `clamp`, on base-unit values.
    #[inline]
    pub fn clamp<T: Float>(value: T, min: T, max: T) -> T {
        assert!(
            min.partial_cmp(&max) != Some(core::cmp::Ordering::Greater),
            "clamp: min > max"
        );
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }

    /// The unit part of a quantity's `Display`: a space and `abbreviation`,
    /// or nothing for an empty abbreviation.
    pub fn fmt_unit(f: &mut fmt::Formatter<'_>, abbreviation: &str) -> fmt::Result {
        if abbreviation.is_empty() {
            Ok(())
        } else {
            write!(f, " {abbreviation}")
        }
    }

    /// `FromStr`: a number and a unit, which can be an abbreviation, a name
    /// or, if `exprs` allows, a unit expression.
    pub fn parse_quantity<Q: Quantity>(s: &str, exprs: Exprs) -> Result<Q, ParseError>
    where
        Q::Value: Float + core::str::FromStr,
        Q::Unit: core::str::FromStr<Err = ParseError>,
    {
        let (number, unit) = crate::split_value_unit(s);
        let value: Q::Value = number.parse().map_err(|_| ParseError::InvalidNumber)?;
        match unit.parse::<Q::Unit>() {
            Ok(unit) => Ok(Q::new(value, unit)),
            Err(e) => {
                let factor = expr_factor::<Q>(unit, exprs).ok_or(e)?;
                Ok(Q::from_base(value * Q::Value::from_f64(factor)))
            }
        }
    }

    /// The generated `from_name`, for custom unit enums.
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

    /// The generated `FromStr`: an exact abbreviation, else a name.
    pub fn unit_from_str<U: UnitEnum>(s: &str) -> Result<U, ParseError> {
        U::ALL
            .iter()
            .copied()
            .find(|unit| unit.info().abbreviations.contains(&s))
            .or_else(|| unit_from_name(s))
            .ok_or(ParseError::UnknownUnit)
    }

    /// The first paragraph of a doc comment, on one line. `doc` has the
    /// comment's lines joined with newlines; the result is the first `.1`
    /// bytes of the array.
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

    /// The string in a [`doc_text`] result. It's only ever cut at ASCII
    /// bytes, so it's valid UTF-8.
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

    #[cfg(test)]
    mod tests {
        extern crate std;
        use super::*;

        #[test]
        fn relation_factors() {
            let p = |s| UnitExpr::parse(s).ok();
            let k = |a, b, c, divide| relation_factor(p(a), p(b), p(c), divide);
            assert_eq!(k("g/m²", "m²", "kg", false), Ok(1e-3));
            assert_eq!(k("kg", "g/m²", "m²", true), Ok(1e3));
            assert_eq!(k("N", "m", "J", false), Ok(1.0));
            assert_eq!(k("m²·K/kW", "W/m²", "K", false), Ok(1e-3));
            assert_eq!(k("N", "m", "rad", false), Err(RelationError::Dimensions));
            // Angles count: N·m is not N·m/rad.
            assert_eq!(k("N·m", "rad", "J", true), Err(RelationError::Dimensions));
            assert_eq!(k("gsm", "m²", "kg", false), Err(RelationError::Lhs));
            assert_eq!(k("g/m²", "", "kg", false), Err(RelationError::Rhs));
            assert_eq!(k("g/m²", "m²", "dB", false), Err(RelationError::Output));
        }
    }
}
