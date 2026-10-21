//! Quantities written with a unit, checked at compile time: the machinery
//! behind [`qty!`](crate::qty!).
//!
//! A unit written as tokens, like `m/s^2`, becomes an expression on the
//! constants in [`sym`]: `sym::m.div(sym::s.powi::<2>())`. Each constant is
//! a [`Unit`] whose dimensions are in its type, so the compiler works out
//! the dimensions with the same traits as [`formula!`](crate::formula!), and
//! an unknown symbol is an unknown name. Nothing is parsed at compile time,
//! which keeps the macro cheap to build.

use core::marker::PhantomData;

use crate::formula::{AddE, Dims, DivDim, Expr, MulDim, MulE, PowDim, SubE, E};
use crate::{Number, UnitExpr};

#[allow(
    non_upper_case_globals,
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::unreadable_literal
)]
pub mod sym;

/// A unit with dimensions `D` (a [`Dim`](crate::formula::Dim)) and `A`
/// radians (an [`E`]), and its size in coherent SI units.
#[derive(Debug)]
pub struct Unit<D, A> {
    factor: f64,
    unit: PhantomData<(D, A)>,
}

impl<D, A> Clone for Unit<D, A> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<D, A> Copy for Unit<D, A> {}

/// The unit `1`, as in `1/s`.
pub const ONE: Unit<Dims, E<0>> = Unit::new(1.0);

// The factors are computed the way `UnitExpr::parse` does, so that a unit
// written as tokens and the same unit in a string give the same value.
impl<D, A> Unit<D, A> {
    /// A unit `factor` times the coherent SI unit.
    pub const fn new(factor: f64) -> Self {
        Self {
            factor,
            unit: PhantomData,
        }
    }

    /// The size in coherent SI units.
    pub const fn factor(self) -> f64 {
        self.factor
    }

    /// The product of two units.
    pub const fn mul<D2, A2>(self, other: Unit<D2, A2>) -> Unit<D::Out, A::Out>
    where
        D: MulDim<D2>,
        A: AddE<A2>,
    {
        Unit::new(self.factor * other.factor)
    }

    /// The quotient of two units.
    pub const fn div<D2, A2>(self, other: Unit<D2, A2>) -> Unit<D::Out, A::Out>
    where
        D: DivDim<D2>,
        A: SubE<A2>,
    {
        Unit::new(self.factor * (1.0 / other.factor))
    }

    /// The unit to the power `N`, which is positive.
    pub const fn powi<const N: i8>(self) -> Unit<D::Out, A::Out>
    where
        D: PowDim<N>,
        A: MulE<N>,
    {
        let mut factor = 1.0;
        let mut i = 0;
        while i < N {
            factor *= self.factor;
            i += 1;
        }
        Unit::new(factor)
    }

    /// `1 / self`.
    pub const fn recip(self) -> Unit<<Dims as DivDim<D>>::Out, <E<0> as SubE<A>>::Out>
    where
        Dims: DivDim<D>,
        E<0>: SubE<A>,
    {
        Unit::new(1.0 / self.factor)
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
    /// The value `value · factor` in coherent SI units.
    fn from_literal(value: f64, factor: f64) -> Self;
}

impl<D, const A: i8, V: Number> FromLiteral<D, E<A>> for Expr<D, V> {
    #[inline(always)]
    fn from_literal(value: f64, factor: f64) -> Self {
        Self::new(V::from_f64(value * factor))
    }
}

/// `value` in `unit`, as the type the context expects.
#[inline(always)]
pub fn literal<T: FromLiteral<D, A>, D, A>(value: f64, unit: Unit<D, A>) -> T {
    T::from_literal(value, unit.factor)
}

/// `value` in `unit`, as an [`Expr`], for [`formula!`](crate::formula!).
#[inline(always)]
pub fn literal_expr<D, A, V: Number>(value: f64, unit: Unit<D, A>) -> Expr<D, V> {
    Expr::new(V::from_f64(value * unit.factor))
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
            #[inline(always)]
            fn from_literal(value: f64, factor: f64) -> Self {
                // Both factors are constants, so this is one multiplication
                // of the literal.
                $Q::from_base(V::from_f64(
                    value * (factor / <$Q<V> as $crate::formula::SiFactor>::FACTOR),
                ))
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
/// error[E0425]: cannot find value `z` in module `qty::literal::sym`
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
/// The value is the number times the unit's size, which is computed at
/// compile time, so it's within about one ulp of the matching `from_`
/// method, and exact for units such as `km`, `mm` or `kW`. It's computed in
/// `f64`, then converted to the storage type. It costs about as much to
/// build as a `from_` call.
///
/// The number is a literal, `-` included. Absolute temperatures and dB
/// levels aren't unit expressions, so write `Temperature::from_degrees_celsius`
/// for those.
#[macro_export]
macro_rules! qty {
    ($value:literal $unit:literal) => {
        $crate::literal::literal(($value) as f64, $crate::__qty_unit!(@string $unit))
    };
    ($value:literal $($unit:tt)+) => {
        $crate::literal::literal(($value) as f64, const { $crate::__qty_unit!($($unit)+) })
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
    ($value:literal $($unit:tt)+) => {
        $crate::literal::literal_expr(($value) as f64, const { $crate::__qty_unit!($($unit)+) })
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
        const UNIT: $crate::UnitExpr = match $crate::UnitExpr::parse($unit) {
            ::core::result::Result::Ok(unit) => unit,
            ::core::result::Result::Err(_) => {
                ::core::panic!(::core::concat!("qty!: `", $unit, "` is not a unit"))
            }
        };
        $crate::literal::Unit::<
            $crate::formula::Dims<
                { UNIT.dimensions.length },
                { UNIT.dimensions.mass },
                { UNIT.dimensions.time },
                { UNIT.dimensions.electric_current },
                { UNIT.dimensions.temperature },
                { UNIT.dimensions.amount_of_substance },
                { UNIT.dimensions.luminous_intensity },
            >,
            $crate::formula::E<{ UNIT.angle }>,
        >::new(UNIT.factor)
    }};

    (@factor [$($acc:tt)*] [$op:tt] ($($group:tt)+) $($rest:tt)*) => {
        $crate::__qty_unit!(@exponent [$($acc)*] [$op] [($crate::__qty_unit!($($group)+))] $($rest)*)
    };
    (@factor [$($acc:tt)*] [$op:tt] 1 $($rest:tt)*) => {
        $crate::__qty_unit!(@exponent [$($acc)*] [$op] [$crate::literal::ONE] $($rest)*)
    };
    // Keywords need a raw identifier.
    (@factor [$($acc:tt)*] [$op:tt] in $($rest:tt)*) => {
        $crate::__qty_unit!(@exponent [$($acc)*] [$op] [$crate::literal::sym::r#in] $($rest)*)
    };
    (@factor [$($acc:tt)*] [$op:tt] as $($rest:tt)*) => {
        $crate::__qty_unit!(@exponent [$($acc)*] [$op] [$crate::literal::sym::r#as] $($rest)*)
    };
    (@factor [$($acc:tt)*] [$op:tt] $symbol:ident $($rest:tt)*) => {
        $crate::__qty_unit!(@exponent [$($acc)*] [$op] [$crate::literal::sym::$symbol] $($rest)*)
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
