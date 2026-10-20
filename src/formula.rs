//! Formulas whose intermediate results aren't named quantities.
//!
//! `Mass * Mass` has no type of its own, so `G * m1 * m2 / r²` doesn't
//! compile with the operators alone. [`formula!`](crate::formula!) works out
//! the dimensions of each step at compile time instead, and converts the
//! result to the quantity you ask for:
//!
//! ```rust
//! use qty::*;
//! use qty::constants::GRAVITATIONAL_CONSTANT as G;
//!
//! let (earth, person) = (Mass::from_kilograms(5.972e24), Mass::from_kilograms(70.0));
//! let r = Length::from_kilometers(6371.0);
//!
//! let weight: Force = formula!(G * earth * person / (r * r));
//! assert_eq!(format!("{weight:.0}"), "687 N");
//!
//! let v = Speed::from_meters_per_second(3.0);
//! let energy: Energy = formula!(0.5 * person * v * v);
//! assert_eq!(energy.as_joules(), 315.0);
//! ```
//!
//! A formula with the wrong dimensions for its target doesn't compile, and
//! `cargo check` reports it:
//!
//! ```rust,compile_fail,E0277
//! # use qty::*;
//! # let (m, v) = (Mass::from_kilograms(1.0), Speed::from_meters_per_second(1.0));
//! let f: Force = formula!(0.5 * m * v * v); // an energy, not a force
//! ```
//!
//! ```text
//! error[E0277]: this formula doesn't give a `qty::Force`
//!   |     formula!(0.5 * m * v * v)
//!   |     the formula has dimensions `Dim<E<2>, E<1>, E<-2>, E<0>, E<0>, E<0>, E<0>>`
//! ```
//!
//! The numbers in `Dim` are the exponents of length, mass, time, electric
//! current, temperature, amount of substance and luminous intensity, so the
//! formula above is m²·kg·s⁻².
//!
//! Adding values of different dimensions doesn't compile either, nor does
//! a square root that would leave a fractional exponent:
//!
//! ```rust,compile_fail,E0308
//! # use qty::*;
//! # let (m, v) = (Mass::from_kilograms(1.0), Speed::from_meters_per_second(1.0));
//! let e: Energy = formula!(m * v * v + m * v);
//! ```
//!
//! ```rust,compile_fail,E0599
//! # use qty::*;
//! let side: Length = formula!(Volume::from_cubic_meters(8.0).sqrt());
//! ```
//!
//! # How it works
//!
//! Each value in a formula becomes an [`Expr`]: the value in coherent SI
//! units, with its dimensions in the type. Multiplying two `Expr`s adds the
//! exponents in their types, and dividing subtracts them, so the compiler
//! knows the dimensions of the result. At run time an `Expr` is just the
//! number, and a formula compiles to the same code as the arithmetic on
//! plain floats.
//!
//! Quantities whose base unit isn't coherent SI (grams per second for
//! `MassFlow`) are converted on the way in and out, with
//! one multiplication by a constant.
//!
//! Quantities with the same dimensions are interchangeable here: `force *
//! length` can become a `Torque` or an [`Energy`](crate::Energy), whichever
//! the target is. Angles are
//! dimensionless, as in SI.
//!
//! Exponents can be up to ±[`MAX_EXPONENT`], twice the largest exponent of
//! any quantity, so the product or quotient of any two quantities fits. A
//! formula that goes beyond that doesn't compile.
//!
//! # Without the macro
//!
//! The constants in [`constants`](crate::constants) are `Expr`s, and an
//! `Expr` multiplies and divides with quantities, plain numbers and other
//! `Expr`s. `into()` converts the result:
//!
//! ```rust
//! # use qty::*;
//! use qty::constants::STANDARD_GRAVITY as G0;
//!
//! let weight: Force = (Mass::from_kilograms(10.0) * G0).into();
//! assert_eq!(format!("{weight:.1}"), "98.1 N");
//! ```
//!
//! An intermediate result can be kept in a variable as an `Expr<_>`:
//!
//! ```rust
//! # use qty::*;
//! use qty::formula::Expr;
//!
//! let (m, v) = (Mass::from_kilograms(2.0), Speed::from_meters_per_second(3.0));
//! let mv2: Expr<_> = formula!(m * v * v);
//! let energy: Energy = formula!(0.5 * mv2);
//! assert_eq!(energy.as_joules(), 9.0);
//! ```

use core::fmt;
use core::marker::PhantomData;
use core::ops::{Add, Div, Mul, Neg, Sub};

mod exponents;
pub use exponents::MAX_EXPONENT;

/// An exponent as a type, such as `E<-2>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct E<const N: i8>;

/// Dimensions as types: the exponents ([`E`]) of length, mass, time,
/// electric current, temperature, amount of substance and luminous
/// intensity.
pub struct Dim<L, M, T, I, Th, N, J>(PhantomData<(L, M, T, I, Th, N, J)>);

/// [`Dim`] from plain exponents, with the trailing ones defaulting to 0.
/// `Dims<3, -1, -2>` is m³·kg⁻¹·s⁻².
pub type Dims<
    const L: i8 = 0,
    const M: i8 = 0,
    const T: i8 = 0,
    const I: i8 = 0,
    const TH: i8 = 0,
    const N: i8 = 0,
    const J: i8 = 0,
> = Dim<E<L>, E<M>, E<T>, E<I>, E<TH>, E<N>, E<J>>;

impl<L, M, T, I, Th, N, J> fmt::Debug for Dim<L, M, T, I, Th, N, J> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Dim")
    }
}

/// `A + B` for exponents.
#[diagnostic::on_unimplemented(
    message = "an exponent in this formula is out of range",
    note = "formulas support exponents up to qty::formula::MAX_EXPONENT"
)]
pub trait AddE<B> {
    /// The sum.
    type Out;
}

/// `A - B` for exponents.
#[diagnostic::on_unimplemented(
    message = "an exponent in this formula is out of range",
    note = "formulas support exponents up to qty::formula::MAX_EXPONENT"
)]
pub trait SubE<B> {
    /// The difference.
    type Out;
}

/// Half of an even exponent.
#[diagnostic::on_unimplemented(
    message = "the square root of these dimensions has a fractional exponent",
    label = "`{Self}` is odd"
)]
pub trait HalfE {
    /// The half.
    type Out;
}

/// A third of an exponent that is a multiple of 3.
#[diagnostic::on_unimplemented(
    message = "the cube root of these dimensions has a fractional exponent",
    label = "`{Self}` isn't a multiple of 3"
)]
pub trait ThirdE {
    /// The third.
    type Out;
}

/// The dimensions of a product.
#[diagnostic::on_unimplemented(
    message = "an exponent in this formula is out of range",
    note = "formulas support exponents up to qty::formula::MAX_EXPONENT"
)]
pub trait MulDim<R> {
    /// The dimensions of `Self × R`.
    type Out;
}

/// The dimensions of a quotient.
#[diagnostic::on_unimplemented(
    message = "an exponent in this formula is out of range",
    note = "formulas support exponents up to qty::formula::MAX_EXPONENT"
)]
pub trait DivDim<R> {
    /// The dimensions of `Self / R`.
    type Out;
}

/// The dimensions of a square root.
#[diagnostic::on_unimplemented(
    message = "the square root of these dimensions has a fractional exponent"
)]
pub trait SqrtDim {
    /// Half of every exponent.
    type Out;
}

/// The dimensions of a cube root.
#[diagnostic::on_unimplemented(
    message = "the cube root of these dimensions has a fractional exponent"
)]
pub trait CbrtDim {
    /// A third of every exponent.
    type Out;
}

macro_rules! dim_op {
    ($Op:ident, $E:ident) => {
        impl<L1, M1, T1, I1, H1, N1, J1, L2, M2, T2, I2, H2, N2, J2>
            $Op<Dim<L2, M2, T2, I2, H2, N2, J2>> for Dim<L1, M1, T1, I1, H1, N1, J1>
        where
            L1: $E<L2>,
            M1: $E<M2>,
            T1: $E<T2>,
            I1: $E<I2>,
            H1: $E<H2>,
            N1: $E<N2>,
            J1: $E<J2>,
        {
            type Out = Dim<L1::Out, M1::Out, T1::Out, I1::Out, H1::Out, N1::Out, J1::Out>;
        }
    };
    ($Op:ident, $E:ident, root) => {
        impl<L, M, T, I, H, N, J> $Op for Dim<L, M, T, I, H, N, J>
        where
            L: $E,
            M: $E,
            T: $E,
            I: $E,
            H: $E,
            N: $E,
            J: $E,
        {
            type Out = Dim<L::Out, M::Out, T::Out, I::Out, H::Out, N::Out, J::Out>;
        }
    };
}
dim_op!(MulDim, AddE);
dim_op!(DivDim, SubE);
dim_op!(SqrtDim, HalfE, root);
dim_op!(CbrtDim, ThirdE, root);

/// A value in coherent SI units, with dimensions `D` (a [`Dim`]).
///
/// It's what [`formula!`](crate::formula!) computes with. Build one with
/// [`new`](Self::new) for a constant of your own:
///
/// ```rust
/// # use qty::*;
/// use qty::formula::{Dims, Expr};
///
/// // The speed of sound in dry air at 20 °C, in m/s.
/// const SOUND: Expr<Dims<1, 0, -1>> = Expr::new(343.0);
///
/// let distance: Length = formula!(SOUND * Duration::from_seconds(2.0));
/// assert_eq!(distance.as_meters(), 686.0);
/// ```
pub struct Expr<D, V = f64>(V, PhantomData<D>);

impl<D, V> Expr<D, V> {
    /// A value in coherent SI units.
    #[inline(always)]
    pub const fn new(value: V) -> Self {
        Self(value, PhantomData)
    }

    /// The value in coherent SI units.
    #[inline(always)]
    pub fn value(self) -> V {
        self.0
    }
}

#[cfg(any(feature = "std", feature = "libm"))]
impl<D, V: crate::FloatMath> Expr<D, V> {
    /// Square root. Every exponent must be even.
    ///
    /// ```rust
    /// # use qty::*;
    /// use qty::constants::STANDARD_GRAVITY as G0;
    ///
    /// // The period of a 1 m pendulum.
    /// let length = Length::from_meters(1.0);
    /// let period: Duration = formula!(2.0 * core::f64::consts::PI * (length / G0).sqrt());
    /// assert_eq!(format!("{period:.3}"), "2.006 s");
    /// ```
    #[inline(always)]
    pub fn sqrt(self) -> Expr<D::Out, V>
    where
        D: SqrtDim,
    {
        Expr::new(self.0.sqrt())
    }

    /// Cube root. Every exponent must be a multiple of 3.
    #[inline(always)]
    pub fn cbrt(self) -> Expr<D::Out, V>
    where
        D: CbrtDim,
    {
        Expr::new(self.0.cbrt())
    }
}

impl<D, V: Clone> Clone for Expr<D, V> {
    fn clone(&self) -> Self {
        Self::new(self.0.clone())
    }
}
impl<D, V: Copy> Copy for Expr<D, V> {}

impl<D, V: fmt::Debug> fmt::Debug for Expr<D, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Expr").field(&self.0).finish()
    }
}

impl<D, V: PartialEq> PartialEq for Expr<D, V> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<D, V: PartialOrd> PartialOrd for Expr<D, V> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

/// A value that can be used in a formula: an [`Expr`], a quantity, or a
/// plain number.
pub trait Operand<V> {
    /// Its dimensions, a [`Dim`].
    type Dim;
    /// The value in coherent SI units.
    fn into_expr(self) -> Expr<Self::Dim, V>;
}

impl<D, V> Operand<V> for Expr<D, V> {
    type Dim = D;
    #[inline(always)]
    fn into_expr(self) -> Self {
        self
    }
}

macro_rules! number_operand {
    ($($t:ty),*) => {$(
        impl Operand<$t> for $t {
            type Dim = Dims;
            #[inline(always)]
            fn into_expr(self) -> Expr<Dims, $t> {
                Expr::new(self)
            }
        }

        impl FromFormula<Dims, $t> for $t {
            #[inline(always)]
            fn from_formula(expr: Expr<Dims, $t>) -> $t {
                expr.0
            }
        }

        impl<D> Mul<Expr<D, $t>> for $t {
            type Output = Expr<D, $t>;
            #[inline(always)]
            fn mul(self, rhs: Expr<D, $t>) -> Expr<D, $t> {
                Expr::new(self * rhs.0)
            }
        }
    )*};
}
number_operand!(f32, f64);

/// Converts a formula's result to a quantity with the same dimensions.
#[diagnostic::on_unimplemented(
    message = "this formula doesn't give a `{Self}`",
    label = "the formula has dimensions `{D}`",
    note = "the exponents are of length, mass, time, electric current, temperature, amount of substance and luminous intensity"
)]
pub trait FromFormula<D, V> {
    /// The quantity with value `expr`.
    fn from_formula(expr: Expr<D, V>) -> Self;
}

impl<D, V> FromFormula<D, V> for Expr<D, V> {
    #[inline(always)]
    fn from_formula(expr: Expr<D, V>) -> Self {
        expr
    }
}

/// Wraps a value of a formula. Used by [`formula!`](crate::formula!).
#[doc(hidden)]
#[inline(always)]
pub fn operand<X: Operand<V>, V>(x: X) -> Expr<X::Dim, V> {
    x.into_expr()
}

impl<D, V, R> Mul<R> for Expr<D, V>
where
    V: Mul<Output = V>,
    R: Operand<V>,
    D: MulDim<R::Dim>,
{
    type Output = Expr<D::Out, V>;
    #[inline(always)]
    fn mul(self, rhs: R) -> Self::Output {
        Expr::new(self.0 * rhs.into_expr().0)
    }
}

impl<D, V, R> Div<R> for Expr<D, V>
where
    V: Div<Output = V>,
    R: Operand<V>,
    D: DivDim<R::Dim>,
{
    type Output = Expr<D::Out, V>;
    #[inline(always)]
    fn div(self, rhs: R) -> Self::Output {
        Expr::new(self.0 / rhs.into_expr().0)
    }
}

impl<D, V: Add<Output = V>> Add for Expr<D, V> {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self {
        Expr::new(self.0 + rhs.0)
    }
}

impl<D, V: Sub<Output = V>> Sub for Expr<D, V> {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self {
        Expr::new(self.0 - rhs.0)
    }
}

impl<D, V: Neg<Output = V>> Neg for Expr<D, V> {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Self {
        Expr::new(-self.0)
    }
}

/// The size of a quantity's base unit in coherent SI, for the formula impls.
#[doc(hidden)]
pub const fn base_unit_factor(expr: Option<crate::UnitExpr>) -> f64 {
    match expr {
        Some(expr) => expr.factor,
        None => panic!("a quantity in formulas needs a base unit in SI"),
    }
}

/// Lets a quantity take part in formulas. `$U::QUANTITY` gives the
/// dimensions and `BASE_UNIT_EXPR` the size of the base unit.
#[doc(hidden)]
#[macro_export]
macro_rules! __qty_formula {
    ($Q:ident, $U:ident) => {
        const _: () = ::core::assert!(
            <$Q<f64> as $crate::Quantity>::BASE_UNIT_EXPR.is_some(),
            ::core::concat!(::core::stringify!($Q), ": formulas need a base unit in SI"),
        );

        impl<V: $crate::Float> $crate::formula::Operand<V> for $Q<V> {
            type Dim = $crate::formula::Dims<
                { $U::QUANTITY.dimensions.length },
                { $U::QUANTITY.dimensions.mass },
                { $U::QUANTITY.dimensions.time },
                { $U::QUANTITY.dimensions.electric_current },
                { $U::QUANTITY.dimensions.temperature },
                { $U::QUANTITY.dimensions.amount_of_substance },
                { $U::QUANTITY.dimensions.luminous_intensity },
            >;
            #[inline(always)]
            fn into_expr(self) -> $crate::formula::Expr<Self::Dim, V> {
                $crate::formula::Expr::new(
                    self.into_base() * V::from_f64(<$Q<V> as $crate::formula::SiFactor>::FACTOR),
                )
            }
        }

        impl<V: $crate::Float> $crate::formula::SiFactor for $Q<V> {
            const FACTOR: f64 =
                $crate::formula::base_unit_factor(<$Q<f64> as $crate::Quantity>::BASE_UNIT_EXPR);
        }

        impl<V: $crate::Float>
            $crate::formula::FromFormula<<$Q<V> as $crate::formula::Operand<V>>::Dim, V> for $Q<V>
        {
            #[inline(always)]
            fn from_formula(
                expr: $crate::formula::Expr<<$Q<V> as $crate::formula::Operand<V>>::Dim, V>,
            ) -> Self {
                $Q::from_base(
                    expr.value() / V::from_f64(<$Q<V> as $crate::formula::SiFactor>::FACTOR),
                )
            }
        }

        impl<V: $crate::Float>
            ::core::convert::From<
                $crate::formula::Expr<<$Q<V> as $crate::formula::Operand<V>>::Dim, V>,
            > for $Q<V>
        {
            #[inline(always)]
            fn from(
                expr: $crate::formula::Expr<<$Q<V> as $crate::formula::Operand<V>>::Dim, V>,
            ) -> Self {
                $crate::formula::FromFormula::from_formula(expr)
            }
        }

        impl<D, V: $crate::Float> ::core::ops::Mul<$crate::formula::Expr<D, V>> for $Q<V>
        where
            <$Q<V> as $crate::formula::Operand<V>>::Dim: $crate::formula::MulDim<D>,
        {
            type Output = $crate::formula::Expr<
                <<$Q<V> as $crate::formula::Operand<V>>::Dim as $crate::formula::MulDim<D>>::Out,
                V,
            >;
            #[inline(always)]
            fn mul(self, rhs: $crate::formula::Expr<D, V>) -> Self::Output {
                $crate::formula::Expr::new(
                    $crate::formula::Operand::into_expr(self).value() * rhs.value(),
                )
            }
        }

        impl<D, V: $crate::Float> ::core::ops::Div<$crate::formula::Expr<D, V>> for $Q<V>
        where
            <$Q<V> as $crate::formula::Operand<V>>::Dim: $crate::formula::DivDim<D>,
        {
            type Output = $crate::formula::Expr<
                <<$Q<V> as $crate::formula::Operand<V>>::Dim as $crate::formula::DivDim<D>>::Out,
                V,
            >;
            #[inline(always)]
            fn div(self, rhs: $crate::formula::Expr<D, V>) -> Self::Output {
                $crate::formula::Expr::new(
                    $crate::formula::Operand::into_expr(self).value() / rhs.value(),
                )
            }
        }
    };
}

/// The size of a quantity's base unit in coherent SI.
#[doc(hidden)]
pub trait SiFactor {
    /// One base unit in coherent SI units.
    const FACTOR: f64;
}

/// Computes a formula and converts the result to the quantity the context
/// expects.
///
/// Each value in the formula can be a quantity, a plain number, a constant
/// from [`constants`](crate::constants) or another [`Expr`](crate::formula::Expr).
/// The intermediate results don't need to be named quantities. The formula
/// keeps Rust's precedence and parentheses, and `.sqrt()` and `.cbrt()` work
/// on any value or parenthesised group:
///
/// ```rust
/// use qty::*;
/// use qty::constants::{GRAVITATIONAL_CONSTANT as G, STEFAN_BOLTZMANN_CONSTANT as SIGMA};
///
/// // Escape velocity from the Earth.
/// let (m, r) = (Mass::from_kilograms(5.972e24), Length::from_kilometers(6371.0));
/// let v: Speed = formula!((2.0 * G * m / r).sqrt());
/// assert_eq!(format!("{:.1}", v.in_unit(SpeedUnit::KilometerPerSecond)), "11.2 km/s");
///
/// // Power radiated by a black body.
/// let (area, t) = (Area::from_square_meters(1.0), TemperatureDelta::from_kelvins(300.0));
/// let p: Power = formula!(SIGMA * area * t * t * t * t);
/// assert_eq!(format!("{p:.0}"), "459 W");
/// ```
///
/// The result's type has to be known, from a type annotation, a function's
/// return type or an argument. See [`formula`](mod@crate::formula) for how it works
/// and what it checks.
#[macro_export]
macro_rules! formula {
    ($($t:tt)+) => {
        $crate::formula::FromFormula::from_formula($crate::__formula!(@start [] $($t)+))
    };
}

// Wraps every value of the formula in `formula::operand` and keeps the
// operators and parentheses in place, so Rust's precedence applies.
//
// `@start [out]` is at the start of a value. `@value [out] [tokens]` reads
// the tokens of a value up to the next operator.
#[doc(hidden)]
#[macro_export]
macro_rules! __formula {
    // A parenthesised group: a formula of its own.
    (@start [$($o:tt)*] ($($g:tt)+) . sqrt () $($rest:tt)*) => {
        $crate::__formula!(@value [$($o)* ($crate::__formula!(@start [] $($g)+)).sqrt()] [] $($rest)*)
    };
    (@start [$($o:tt)*] ($($g:tt)+) . cbrt () $($rest:tt)*) => {
        $crate::__formula!(@value [$($o)* ($crate::__formula!(@start [] $($g)+)).cbrt()] [] $($rest)*)
    };
    (@start [$($o:tt)*] ($($g:tt)+) $($rest:tt)*) => {
        $crate::__formula!(@value [$($o)* ($crate::__formula!(@start [] $($g)+))] [] $($rest)*)
    };
    (@start [$($o:tt)*] $($rest:tt)*) => {
        $crate::__formula!(@value [$($o)*] [] $($rest)*)
    };

    // The end.
    (@value [$($o:tt)*] []) => {
        $($o)*
    };
    (@value [$($o:tt)*] [$($v:tt)+]) => {
        $($o)* $crate::formula::operand($($v)+)
    };

    // `.sqrt()` and `.cbrt()` on a value work on its `Expr`, so they accept
    // any dimensions with even (or multiple of 3) exponents.
    (@value [$($o:tt)*] [$($v:tt)+] . sqrt () $($rest:tt)*) => {
        $crate::__formula!(@value [$($o)* $crate::formula::operand($($v)+).sqrt()] [] $($rest)*)
    };
    (@value [$($o:tt)*] [$($v:tt)+] . cbrt () $($rest:tt)*) => {
        $crate::__formula!(@value [$($o)* $crate::formula::operand($($v)+).cbrt()] [] $($rest)*)
    };
    (@value [$($o:tt)*] [] . $($rest:tt)*) => {
        ::core::compile_error!("formula!: only .sqrt() and .cbrt() can follow parentheses")
    };

    // An operator ends the value. With no value before it, it's a unary
    // minus or follows a parenthesised group.
    (@value [$($o:tt)*] [] $op:tt $($rest:tt)*) => {
        $crate::__formula!(@op $op [$($o)*] $($rest)*)
    };
    (@value [$($o:tt)*] [$($v:tt)+] * $($rest:tt)*) => {
        $crate::__formula!(@start [$($o)* $crate::formula::operand($($v)+) *] $($rest)*)
    };
    (@value [$($o:tt)*] [$($v:tt)+] / $($rest:tt)*) => {
        $crate::__formula!(@start [$($o)* $crate::formula::operand($($v)+) /] $($rest)*)
    };
    (@value [$($o:tt)*] [$($v:tt)+] + $($rest:tt)*) => {
        $crate::__formula!(@start [$($o)* $crate::formula::operand($($v)+) +] $($rest)*)
    };
    (@value [$($o:tt)*] [$($v:tt)+] - $($rest:tt)*) => {
        $crate::__formula!(@start [$($o)* $crate::formula::operand($($v)+) -] $($rest)*)
    };
    // Anything else is part of the value.
    (@value [$($o:tt)*] [$($v:tt)*] $t:tt $($rest:tt)*) => {
        $crate::__formula!(@value [$($o)*] [$($v)* $t] $($rest)*)
    };

    (@op * [$($o:tt)*] $($rest:tt)*) => { $crate::__formula!(@start [$($o)* *] $($rest)*) };
    (@op / [$($o:tt)*] $($rest:tt)*) => { $crate::__formula!(@start [$($o)* /] $($rest)*) };
    (@op + [$($o:tt)*] $($rest:tt)*) => { $crate::__formula!(@start [$($o)* +] $($rest)*) };
    (@op - [$($o:tt)*] $($rest:tt)*) => { $crate::__formula!(@start [$($o)* -] $($rest)*) };
    (@op $t:tt [$($o:tt)*] $($rest:tt)*) => {
        $crate::__formula!(@value [$($o)*] [$t] $($rest)*)
    };
}
