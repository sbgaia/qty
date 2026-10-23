//! Quantities written with a unit, checked at compile time: the machinery
//! behind [`qty!`](crate::qty!).
//!
//! A unit written as one symbol, like `km`, is a constant in [`sym`]: a
//! [`Symbol`] whose dimensions are in its type and whose conversion is the
//! one the matching `from_` method uses, so that the literal is rounded the
//! same way. A unit written as tokens, like `m/s^2`, becomes an expression
//! on the constants in [`unit`](mod@unit),
//! `unit::m.div(unit::s.powi::<2>())`. Each is a [`Unit`] with its size as
//! an exact fraction, so the compiler works out the dimensions with the
//! same traits as [`formula!`](crate::formula!) and the size exactly, and
//! an unknown symbol is an unknown name. Nothing is parsed at compile time,
//! which keeps the macro cheap to build.

use core::marker::PhantomData;

pub use crate::factor::{Conversion, Factor};
use crate::formula::{AddE, Dims, DivDim, Expr, MulDim, MulE, PowDim, SubE, E};
use crate::{Number, UnitExpr};

#[allow(
    non_upper_case_globals,
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
#[rustfmt::skip] // Generated, one constant per line.
pub mod sym;

#[allow(
    non_upper_case_globals,
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
#[rustfmt::skip] // Generated, one constant per line.
pub mod unit;

/// `x · factor`, for [`Conv`].
pub const MUL: u8 = 0;
/// `x / divisor`, for [`Conv`].
pub const DIV: u8 = 1;
/// [`Number::mul_ratio`], for [`Conv`].
pub const MUL_RATIO: u8 = 2;
/// [`Number::div_ratio`], for [`Conv`].
pub const DIV_RATIO: u8 = 3;
/// Any of the four, for [`Conv`], chosen by the constants (see
/// [`Conversion::encoded`]): for units that are only known at compile time.
pub const ANY: u8 = 4;

/// A conversion to coherent SI units: [`MUL`], [`DIV`], [`MUL_RATIO`] or
/// [`DIV_RATIO`], with its constants, the way the generated `from_` method
/// of the unit converts, or [`ANY`] of them.
#[derive(Debug, Clone, Copy)]
pub struct Conv<const K: u8>(pub f64, pub f64);

impl<const K: u8> Conv<K> {
    /// `x` in coherent SI units.
    #[inline]
    pub fn apply(self, x: f64) -> f64 {
        // Most units multiply: a body small enough to inline and fold
        // before code generation.
        if K == MUL {
            x * self.0
        } else {
            self.apply_other(x)
        }
    }

    #[inline]
    fn apply_other(self, x: f64) -> f64 {
        match K {
            DIV => x / self.0,
            MUL_RATIO => <f64 as Number>::mul_ratio(x, self.0, self.1),
            DIV_RATIO => <f64 as Number>::div_ratio(x, self.0, self.1),
            _ => self.decode().apply(x),
        }
    }

    /// `Conv<ANY>` as the conversion it encodes.
    #[inline(always)]
    fn decode(self) -> Conversion {
        let (a, b) = (self.0, self.1);
        if b > 0.0 {
            Conversion::MulRatio(a, b)
        } else if b < 0.0 {
            Conversion::DivRatio(a, -b)
        } else if b.is_sign_negative() {
            Conversion::Div(a)
        } else {
            Conversion::Mul(a)
        }
    }

    /// The factor, rounded.
    #[inline]
    pub fn factor(self) -> f64 {
        match K {
            MUL => self.0,
            DIV => 1.0 / self.0,
            MUL_RATIO => self.0 / self.1,
            DIV_RATIO => self.1 / self.0,
            _ => self.decode().factor(),
        }
    }
}

impl Conversion {
    /// `x` in coherent SI units.
    #[inline(always)]
    fn apply(self, x: f64) -> f64 {
        match self {
            Self::Mul(factor) => x * factor,
            Self::Div(divisor) => x / divisor,
            Self::MulRatio(numerator, denominator) => {
                <f64 as Number>::mul_ratio(x, numerator, denominator)
            }
            Self::DivRatio(numerator, denominator) => {
                <f64 as Number>::div_ratio(x, numerator, denominator)
            }
        }
    }

    /// The factor, rounded.
    fn factor(self) -> f64 {
        match self {
            Self::Mul(factor) => factor,
            Self::Div(divisor) => 1.0 / divisor,
            Self::MulRatio(numerator, denominator) => numerator / denominator,
            Self::DivRatio(numerator, denominator) => denominator / numerator,
        }
    }
}

impl Factor {
    /// `x · self`, rounded like the generated `from_` methods.
    pub fn apply(self, x: f64) -> f64 {
        self.conversion().apply(x)
    }
}

/// A unit with dimensions `D` (a [`Dim`](crate::formula::Dim)) and `A`
/// radians (an [`E`]), converted to coherent SI units by a [`Conv`]: what
/// [`qty!`](crate::qty!) scales the number by.
#[derive(Debug)]
pub struct Symbol<D, A, const K: u8> {
    conv: Conv<K>,
    unit: PhantomData<(D, A)>,
}

impl<D, A, const K: u8> Clone for Symbol<D, A, K> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<D, A, const K: u8> Copy for Symbol<D, A, K> {}

impl<D, A, const K: u8> Symbol<D, A, K> {
    /// A unit converted by `conv`.
    pub const fn new(conv: Conv<K>) -> Self {
        Self {
            conv,
            unit: PhantomData,
        }
    }
}

/// A unit with dimensions `D` (a [`Dim`](crate::formula::Dim)) and `A`
/// radians (an [`E`]), and its exact size in coherent SI units, for unit
/// expressions.
#[derive(Debug)]
pub struct Unit<D, A> {
    factor: Factor,
    unit: PhantomData<(D, A)>,
}

impl<D, A> Clone for Unit<D, A> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<D, A> Copy for Unit<D, A> {}

/// The unit `1`, as in `1/s`.
pub const ONE: Unit<Dims, E<0>> = Unit::new(Factor::ONE);

// The factors are computed the way `UnitExpr::parse` does, so that a unit
// written as tokens and the same unit in a string give the same value.
impl<D, A> Unit<D, A> {
    /// A unit `factor` times the coherent SI unit.
    pub const fn new(factor: Factor) -> Self {
        Self {
            factor,
            unit: PhantomData,
        }
    }

    /// A unit `numerator / denominator` times the coherent SI unit, with
    /// `value` the nearest `f64`, as in [`unit`](mod@unit).
    pub const fn exact(value: f64, numerator: u128, denominator: u128) -> Self {
        Self::new(Factor::exact(value, numerator, denominator, 0))
    }

    /// A unit `numerator / denominator · π^pi` times the coherent SI unit,
    /// with `value` the nearest `f64`, as in [`unit`](mod@unit).
    pub const fn exact_pi(value: f64, numerator: u128, denominator: u128, pi: i8) -> Self {
        Self::new(Factor::exact(value, numerator, denominator, pi))
    }

    /// A unit `factor` times the coherent SI unit, where that isn't kept as
    /// a fraction, as in [`unit`](mod@unit).
    pub const fn approx(factor: f64) -> Self {
        Self::new(Factor::approx(factor))
    }

    /// The size in coherent SI units.
    pub const fn factor(self) -> Factor {
        self.factor
    }

    /// The unit as a [`Symbol`], converted like the generated `from_`
    /// methods.
    pub const fn symbol(self) -> Symbol<D, A, ANY> {
        let (a, b) = self.factor.conversion().encoded();
        Symbol::new(Conv(a, b))
    }

    /// The product of two units.
    pub const fn mul<D2, A2>(self, other: Unit<D2, A2>) -> Unit<D::Out, A::Out>
    where
        D: MulDim<D2>,
        A: AddE<A2>,
    {
        Unit::new(self.factor.mul(other.factor))
    }

    /// The quotient of two units.
    pub const fn div<D2, A2>(self, other: Unit<D2, A2>) -> Unit<D::Out, A::Out>
    where
        D: DivDim<D2>,
        A: SubE<A2>,
    {
        Unit::new(self.factor.mul(other.factor.recip()))
    }

    /// The unit to the power `N`, which is positive.
    pub const fn powi<const N: i8>(self) -> Unit<D::Out, A::Out>
    where
        D: PowDim<N>,
        A: MulE<N>,
    {
        Unit::new(self.factor.powi(N))
    }

    /// `1 / self`.
    pub const fn recip(self) -> Unit<<Dims as DivDim<D>>::Out, <E<0> as SubE<A>>::Out>
    where
        Dims: DivDim<D>,
        E<0>: SubE<A>,
    {
        Unit::new(self.factor.recip())
    }
}

/// A type that [`qty!`](crate::qty!) can produce from a value in a unit with
/// dimensions `D` (a [`Dim`](crate::formula::Dim)) and `A` radians (an
/// [`E`]): the quantities with those dimensions and that angle, and
/// [`Expr`], which takes any angle.
#[diagnostic::on_unimplemented(
    message = "this unit doesn't give a `{Self}`",
    label = "the unit has dimensions `{D}` and angle `{A}`",
    note = "the exponents are of length, mass, time, electric current, temperature, amount of substance and luminous intensity, and the angle counts radians (2 for the steradian)"
)]
pub trait FromLiteral<D, A> {
    /// `value` converted to coherent SI units by `conv`.
    fn from_literal<const K: u8>(value: f64, conv: Conv<K>) -> Self;
}

impl<D, const A: i8, V: Number> FromLiteral<D, E<A>> for Expr<D, V> {
    #[inline]
    fn from_literal<const K: u8>(value: f64, conv: Conv<K>) -> Self {
        Self::new(V::from_f64(conv.apply(value)))
    }
}

/// `value` in `unit`, as the type the context expects.
#[inline]
pub fn literal<T: FromLiteral<D, A>, D, A, const K: u8>(value: f64, unit: Symbol<D, A, K>) -> T {
    T::from_literal(value, unit.conv)
}

/// `value` in `unit`, as an [`Expr`], for [`formula!`](crate::formula!).
#[inline]
pub fn literal_expr<D, A, const K: u8, V: Number>(value: f64, unit: Symbol<D, A, K>) -> Expr<D, V> {
    Expr::new(V::from_f64(unit.conv.apply(value)))
}

/// The radians in a quantity's base unit, for the `FromLiteral` impls.
pub const fn base_unit_angle(expr: Option<UnitExpr>) -> i8 {
    match expr {
        Some(expr) => expr.angle,
        None => panic!("a quantity in formulas needs a base unit in SI"),
    }
}

/// Lets `qty!` give a quantity. Part of `__qty_formula!`.
#[doc(hidden)]
#[macro_export]
macro_rules! __qty_literal_impl {
    ($Q:ident) => {
        impl<V: $crate::Number>
            $crate::literal::FromLiteral<
                <$Q<V> as $crate::formula::Operand<V>>::Dim,
                $crate::formula::E<
                    {
                        $crate::literal::base_unit_angle(
                            <$Q<f64> as $crate::Quantity>::BASE_UNIT_EXPR,
                        )
                    },
                >,
            > for $Q<V>
        {
            #[inline]
            fn from_literal<const K: u8>(value: f64, conv: $crate::literal::Conv<K>) -> Self {
                const BASE: f64 = <$Q<f64> as $crate::formula::SiFactor>::FACTOR;
                $Q::from_base(V::from_f64(if BASE == 1.0 {
                    conv.apply(value)
                } else {
                    // Both factors are constants, so this is one
                    // multiplication of the literal.
                    value * (conv.factor() / BASE)
                }))
            }
        }
    };
}

/// A quantity written as a number and a unit, like `qty!(9.81 m/s^2)`.
///
/// The unit is a [unit expression](crate::UnitExpr), checked at compile
/// time, and the result is the quantity the context expects, from a type
/// annotation, an argument or the other side of a comparison, as with
/// [`formula!`](crate::formula!):
///
/// ```rust
/// use qty::*;
///
/// let g: Acceleration = qty!(9.81 m/s^2);
/// assert_eq!(g, Acceleration::from_meters_per_second_squared(9.81));
///
/// let trip = Length::from_kilometers(42.0);
/// assert_eq!(trip + qty!(195 m), qty!(42.195 km));
///
/// fn heater(power: Power) -> Energy {
///     power * Duration::from_hours(2.0)
/// }
/// assert_eq!(heater(qty!(1.5 kW)), qty!(3 kW h));
/// ```
///
/// Multiply with `*` or a space, divide with `/`, and raise to a whole
/// power with `^`, as in `kg*m^2/s^2`, `W/(m^2*K)`, `1/s` or `s^-1`. Units
/// whose symbol isn't a Rust identifier, such as `°C`, `m²`, `µs` or `Å`,
/// go in a string, which takes any unit expression:
///
/// ```rust
/// # use qty::*;
/// let rise: TemperatureDelta = qty!(5 "°C");
/// let floor: Area = qty!(12.5 "m²");
/// let tick: Duration = qty!(250 "µs");
/// assert_eq!(tick, qty!(250 us));
/// ```
///
/// In [`formula!`](crate::formula!) and [`expr!`](crate::expr!), `qty!`
/// needs no type of its own:
///
/// ```rust
/// # use qty::*;
/// let t = Duration::from_seconds(2.0);
/// let fall: Length = formula!(0.5 * qty!(9.81 m/s^2) * t * t);
/// assert_eq!(fall.as_meters(), 19.62);
/// ```
///
/// # Checks
///
/// An unknown unit, or one with the wrong dimensions for the target, is a
/// compile error, which `cargo check` reports:
///
/// ```rust,compile_fail,E0425
/// # use qty::*;
/// let g: Acceleration = qty!(9.81 m/z^2); // there is no `z`
/// ```
///
/// ```text
/// error[E0425]: cannot find value `z` in module `qty::literal::unit`
/// ```
///
/// ```rust,compile_fail,E0277
/// # use qty::*;
/// let g: Acceleration = qty!(9.81 m/s); // a speed
/// ```
///
/// ```text
/// error[E0277]: this unit doesn't give a `qty::Acceleration`
///   |     let g: Acceleration = qty!(9.81 m/s);
///   |                           ^^^^^^^^^^^^^^ the unit has dimensions `Dim<E<1>, E<0>, E<-1>, E<0>, E<0>, E<0>, E<0>>` and angle `E<0>`
/// ```
///
/// Unlike in formulas, angles count, the way they do when parsing text: a
/// revolution per minute is a `RotationalSpeed`, not a `Frequency`.
///
/// ```rust,compile_fail,E0277
/// # use qty::*;
/// let f: Frequency = qty!(1 rev/min);
/// ```
///
/// A unit in a string is parsed at compile time, and an unknown one fails
/// the same way:
///
/// ```rust,compile_fail,E0080
/// # use qty::*;
/// let g: Acceleration = qty!(9.81 "m/z²");
/// ```
///
/// # Precision and cost
///
/// The unit's size is computed at compile time as an exact fraction, and
/// the number is scaled by it the way the matching `from_` method does, so
/// `qty!(12 ft)` and `Length::from_feet(12.0)` give the same `f64`. It's
/// computed in `f64`, then converted to the storage type. It costs about as
/// much to build as a `from_` call.
///
/// The number is a literal, `-` included. Absolute temperatures and dB
/// levels aren't unit expressions, so write `Temperature::from_degrees_celsius`
/// for those.
#[macro_export]
macro_rules! qty {
    ($value:literal $unit:literal) => {
        $crate::literal::literal(($value) as f64, $crate::__qty_unit!(@string $unit))
    };
    // One symbol, the most common unit, in one step: it's a constant already.
    ($value:literal in) => {
        $crate::literal::literal(($value) as f64, $crate::literal::sym::r#in)
    };
    ($value:literal as) => {
        $crate::literal::literal(($value) as f64, $crate::literal::sym::r#as)
    };
    ($value:literal $symbol:ident) => {
        $crate::literal::literal(($value) as f64, $crate::literal::sym::$symbol)
    };
    ($value:literal $($unit:tt)+) => {
        $crate::literal::literal(($value) as f64, const { $crate::__qty_unit!($($unit)+).symbol() })
    };
    ($($t:tt)*) => {
        ::core::compile_error!("qty!: write a number and a unit, like `qty!(9.81 m/s^2)`")
    };
}

/// `qty!` in `formula!`: the `Expr`.
#[doc(hidden)]
#[macro_export]
macro_rules! __qty_expr {
    ($value:literal $unit:literal) => {
        $crate::literal::literal_expr(($value) as f64, $crate::__qty_unit!(@string $unit))
    };
    // One symbol, in one step.
    ($value:literal in) => {
        $crate::literal::literal_expr(($value) as f64, $crate::literal::sym::r#in)
    };
    ($value:literal as) => {
        $crate::literal::literal_expr(($value) as f64, $crate::literal::sym::r#as)
    };
    ($value:literal $symbol:ident) => {
        $crate::literal::literal_expr(($value) as f64, $crate::literal::sym::$symbol)
    };
    ($value:literal $($unit:tt)+) => {
        $crate::literal::literal_expr(($value) as f64, const { $crate::__qty_unit!($($unit)+).symbol() })
    };
    ($($t:tt)*) => {
        ::core::compile_error!("qty!: write a number and a unit, like `qty!(9.81 m/s^2)`")
    };
}

// The `Unit` for a unit written as tokens, read left to right like
// `UnitExpr::parse`: `@factor [acc] [op]` reads a symbol, `1` or a group,
// `@exponent` an optional `^n`, and `@next` the operator after it.
#[doc(hidden)]
#[macro_export]
macro_rules! __qty_unit {
    (@string $unit:literal) => {{
        const UNIT: ($crate::UnitExpr, $crate::literal::Factor) = match $crate::UnitExpr::parse_exact($unit) {
            ::core::result::Result::Ok(unit) => unit,
            ::core::result::Result::Err(_) => {
                ::core::panic!(::core::concat!("qty!: `", $unit, "` is not a unit"))
            }
        };
        const CONSTANTS: (f64, f64) = UNIT.1.conversion().encoded();
        $crate::literal::Symbol::<
            $crate::formula::Dims<
                { UNIT.0.dimensions.length },
                { UNIT.0.dimensions.mass },
                { UNIT.0.dimensions.time },
                { UNIT.0.dimensions.electric_current },
                { UNIT.0.dimensions.temperature },
                { UNIT.0.dimensions.amount_of_substance },
                { UNIT.0.dimensions.luminous_intensity },
            >,
            $crate::formula::E<{ UNIT.0.angle }>,
            { $crate::literal::ANY },
        >::new($crate::literal::Conv(CONSTANTS.0, CONSTANTS.1))
    }};

    (@factor [$($acc:tt)*] [$op:tt] ($($group:tt)+) $($rest:tt)*) => {
        $crate::__qty_unit!(@exponent [$($acc)*] [$op] [($crate::__qty_unit!($($group)+))] $($rest)*)
    };
    (@factor [$($acc:tt)*] [$op:tt] 1 $($rest:tt)*) => {
        $crate::__qty_unit!(@exponent [$($acc)*] [$op] [$crate::literal::ONE] $($rest)*)
    };
    // Keywords need a raw identifier.
    (@factor [$($acc:tt)*] [$op:tt] in $($rest:tt)*) => {
        $crate::__qty_unit!(@exponent [$($acc)*] [$op] [$crate::literal::unit::r#in] $($rest)*)
    };
    (@factor [$($acc:tt)*] [$op:tt] as $($rest:tt)*) => {
        $crate::__qty_unit!(@exponent [$($acc)*] [$op] [$crate::literal::unit::r#as] $($rest)*)
    };
    (@factor [$($acc:tt)*] [$op:tt] $symbol:ident $($rest:tt)*) => {
        $crate::__qty_unit!(@exponent [$($acc)*] [$op] [$crate::literal::unit::$symbol] $($rest)*)
    };
    (@factor [$($acc:tt)*] [$op:tt] $($rest:tt)*) => {
        ::core::compile_error!(::core::concat!(
            "qty!: expected a unit symbol, `1` or `(` before `", ::core::stringify!($($rest)*), "`",
        ))
    };

    (@exponent [$($acc:tt)*] [$op:tt] [$($f:tt)+] ^ - $n:literal $($rest:tt)*) => {
        $crate::__qty_unit!(@combine [$($acc)*] [$op] [$($f)+.powi::<$n>().recip()] $($rest)*)
    };
    (@exponent [$($acc:tt)*] [$op:tt] [$($f:tt)+] ^ $n:literal $($rest:tt)*) => {
        $crate::__qty_unit!(@combine [$($acc)*] [$op] [$($f)+.powi::<$n>()] $($rest)*)
    };
    (@exponent [$($acc:tt)*] [$op:tt] [$($f:tt)+] $($rest:tt)*) => {
        $crate::__qty_unit!(@combine [$($acc)*] [$op] [$($f)+] $($rest)*)
    };

    (@combine [] [*] [$($f:tt)+] $($rest:tt)*) => {
        $crate::__qty_unit!(@next [$($f)+] $($rest)*)
    };
    (@combine [$($acc:tt)+] [*] [$($f:tt)+] $($rest:tt)*) => {
        $crate::__qty_unit!(@next [$($acc)+.mul($($f)+)] $($rest)*)
    };
    (@combine [$($acc:tt)+] [/] [$($f:tt)+] $($rest:tt)*) => {
        $crate::__qty_unit!(@next [$($acc)+.div($($f)+)] $($rest)*)
    };

    (@next [$($acc:tt)+]) => {
        $($acc)+
    };
    (@next [$($acc:tt)+] * $($rest:tt)+) => {
        $crate::__qty_unit!(@factor [$($acc)+] [*] $($rest)+)
    };
    (@next [$($acc:tt)+] / $($rest:tt)+) => {
        $crate::__qty_unit!(@factor [$($acc)+] [/] $($rest)+)
    };
    // A space between factors multiplies.
    (@next [$($acc:tt)+] $($rest:tt)+) => {
        $crate::__qty_unit!(@factor [$($acc)+] [*] $($rest)+)
    };

    ($($unit:tt)+) => {
        $crate::__qty_unit!(@factor [] [*] $($unit)+)
    };
}
