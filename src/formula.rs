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
//! error[E0277]: this formula gives `(kg, m<2>, s<-2>)`, not a `qty::Force`
//!   |
//!   |     let f: Force = formula!(0.5 * m * v * v); // an energy, not a force
//!   |                    ^^^^^^^^^^^^^^^^^^^^^^^^^ a `qty::Force` is `(kg, m, s<-2>)`
//! ```
//!
//! Dimensions in errors are tuples of SI base units ([`si`]), with their
//! exponents: `(kg, m<2>, s<-2>)` is kg·m²·s⁻², the dimensions of an energy,
//! and `()` is dimensionless.
//!
//! Adding values of different dimensions doesn't compile either, nor does
//! a square root that would leave a fractional exponent:
//!
//! ```rust,compile_fail,E0277
//! # use qty::*;
//! # let (m, v) = (Mass::from_kilograms(1.0), Speed::from_meters_per_second(1.0));
//! let e: Energy = formula!(m * v * v + m * v);
//! ```
//!
//! ```text
//! error[E0277]: can't add `(kg, m, s<-1>)` to `(kg, m<2>, s<-2>)`
//! ```
//!
//! ```rust,compile_fail,E0277
//! # use qty::*;
//! let side: Length = formula!(Volume::from_cubic_meters(8.0).sqrt());
//! ```
//!
//! ```text
//! error[E0277]: the square root of `(m<3>,)` has a fractional exponent
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
//! [`expr!`](crate::expr!) keeps an intermediate result as an `Expr`, with
//! no type annotation. It prints in SI units:
//!
//! ```rust
//! # use qty::*;
//! let (m, v) = (Mass::from_kilograms(2.0), Speed::from_meters_per_second(3.0));
//! let mv2 = expr!(m * v * v);
//! assert_eq!(mv2.to_string(), "18 kg·m²/s²");
//! let energy: Energy = formula!(0.5 * mv2);
//! assert_eq!(energy.as_joules(), 9.0);
//! ```
//!
//! `.powi::<N>()` raises any value in a formula to a whole power, negative
//! ones too: `formula!(0.5 * m * v.powi::<2>())`.

use core::fmt;
use core::marker::PhantomData;
use core::ops::{Add, Div, Mul, Neg, Sub};

use crate::Dimensions;

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

/// Half of an exponent, rounded down, and the remainder.
pub trait HalfE {
    /// The half.
    type Out;
    /// `E<0>` or `E<1>`.
    type Rem;
}

/// A third of an exponent, rounded down, and the remainder.
pub trait ThirdE {
    /// The third.
    type Out;
    /// `E<0>`, `E<1>` or `E<2>`.
    type Rem;
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
pub trait SqrtDim {
    /// Half of every exponent.
    type Out;
}

/// The dimensions of a cube root.
pub trait CbrtDim {
    /// A third of every exponent.
    type Out;
}

/// Half of every exponent, rounded down, and the remainders, for
/// [`SqrtDim`].
pub trait HalfDim {
    /// The halves.
    type Out;
    /// The remainders, a tuple of seven [`E`]s.
    type Rem;
}

/// A third of every exponent, rounded down, and the remainders, for
/// [`CbrtDim`].
pub trait ThirdDim {
    /// The thirds.
    type Out;
    /// The remainders, a tuple of seven [`E`]s.
    type Rem;
}

/// Checks that the remainders `Self` of halving the exponents of [`InBaseUnits`]
/// `D` are all zero. One check for all of them gives one error.
#[diagnostic::on_unimplemented(
    message = "the square root of `{D}` has a fractional exponent",
    label = "a square root needs even exponents",
    note = "dimensions are written in SI base units: `(kg, m<2>, s<-2>)` is kg·m²·s⁻²"
)]
pub trait WholeSqrt<D, H> {
    /// `H`, the halves. Going through the check, a fractional root has no
    /// dimensions at all, rather than rounded ones that would give more
    /// errors later in the formula.
    type Out;
}

impl<D, H> WholeSqrt<D, H> for (E<0>, E<0>, E<0>, E<0>, E<0>, E<0>, E<0>) {
    type Out = H;
}

/// Checks that the remainders `Self` of dividing the exponents of
/// [`InBaseUnits`] `D` by 3 are all zero.
#[diagnostic::on_unimplemented(
    message = "the cube root of `{D}` has a fractional exponent",
    label = "a cube root needs exponents that are multiples of 3",
    note = "dimensions are written in SI base units: `(kg, m<2>, s<-2>)` is kg·m²·s⁻²"
)]
pub trait WholeCbrt<D, T> {
    /// `T`, the thirds.
    type Out;
}

impl<D, T> WholeCbrt<D, T> for (E<0>, E<0>, E<0>, E<0>, E<0>, E<0>, E<0>) {
    type Out = T;
}

impl<D> SqrtDim for D
where
    D: HalfDim + InBaseUnits,
    D::Rem: WholeSqrt<D::Units, <D as HalfDim>::Out>,
{
    type Out = <D::Rem as WholeSqrt<D::Units, <D as HalfDim>::Out>>::Out;
}

impl<D> CbrtDim for D
where
    D: ThirdDim + InBaseUnits,
    D::Rem: WholeCbrt<D::Units, <D as ThirdDim>::Out>,
{
    type Out = <D::Rem as WholeCbrt<D::Units, <D as ThirdDim>::Out>>::Out;
}

/// An exponent's value, for [`Dimension`].
pub trait Exponent {
    /// The exponent.
    const VALUE: i8;
}

impl<const N: i8> Exponent for E<N> {
    const VALUE: i8 = N;
}

/// The dimensions of a [`Dim`] as a value, for printing an [`Expr`].
pub trait Dimension {
    /// The exponents.
    const DIMENSIONS: Dimensions;
}

impl<L, M, T, I, Th, N, J> Dimension for Dim<L, M, T, I, Th, N, J>
where
    L: Exponent,
    M: Exponent,
    T: Exponent,
    I: Exponent,
    Th: Exponent,
    N: Exponent,
    J: Exponent,
{
    const DIMENSIONS: Dimensions = Dimensions::new(
        L::VALUE,
        M::VALUE,
        T::VALUE,
        I::VALUE,
        Th::VALUE,
        N::VALUE,
        J::VALUE,
    );
}

/// The SI base units in the readable dimensions of error messages, such as
/// `(kg, m<2>, s<-2>)` for kg·m²·s⁻². `N` is the exponent; 1 is left out.
#[allow(non_camel_case_types)]
pub mod si {
    macro_rules! base_unit {
        ($($u:ident: $name:literal),*) => {$(
            #[doc = ::core::concat!($name, ", to the power `N`.")]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
            pub struct $u<const N: i8 = 1>;

            impl super::BaseUnit for $u {
                type Pow<const N: i8> = $u<N>;
            }
        )*};
    }
    base_unit!(
        kg: "Kilogram",
        m: "Metre",
        s: "Second",
        A: "Ampere",
        K: "Kelvin",
        mol: "Mole",
        cd: "Candela"
    );
}

/// A unit of [`si`], to any power.
pub trait BaseUnit {
    /// The unit to the power `N`.
    type Pow<const N: i8>;
}

/// The tuple with `H` in front of `Self`, for [`InBaseUnits`].
pub trait Cons<H> {
    /// `(H, …Self)`.
    type Out;
}

macro_rules! cons {
    ($($t:ident),*) => {
        impl<H, $($t),*> Cons<H> for ($($t,)*) {
            type Out = (H, $($t),*);
        }
    };
}
cons!();
cons!(B);
cons!(B, C);
cons!(B, C, D);
cons!(B, C, D, F);
cons!(B, C, D, F, G);
cons!(B, C, D, F, G, I);

/// The tuple `R` with this exponent of the unit `U` in front, or `R` itself
/// for a zero exponent.
pub trait Factor<U, R> {
    /// `(U<exponent>, …R)`.
    type Out;
}

/// The dimensions of a [`Dim`] as a tuple of [`si`] units, such as
/// `(kg, m<2>, s<-2>)`, which reads better than the exponents in compiler
/// errors. Dimensionless is `()`.
pub trait InBaseUnits {
    /// The units with nonzero exponents, mass first as in `kg·m²`.
    type Units;
}

impl<L, M, T, I, Th, N, J> InBaseUnits for Dim<L, M, T, I, Th, N, J>
where
    J: Factor<si::cd, ()>,
    N: Factor<si::mol, J::Out>,
    Th: Factor<si::K, N::Out>,
    I: Factor<si::A, Th::Out>,
    T: Factor<si::s, I::Out>,
    L: Factor<si::m, T::Out>,
    M: Factor<si::kg, L::Out>,
{
    type Units = M::Out;
}

/// Checks that a formula giving [`InBaseUnits`] `Self` can become a `Q`, of
/// units `U`.
#[diagnostic::on_unimplemented(
    message = "this formula gives `{Self}`, not a `{Q}`",
    label = "a `{Q}` is `{U}`",
    note = "dimensions are written in SI base units: `(kg, m<2>, s<-2>)` is kg·m²·s⁻², and `()` is dimensionless"
)]
pub trait Gives<Q, U> {}

impl<Q, U> Gives<Q, U> for U {}

/// Checks that a value of [`InBaseUnits`] `Self` can be added to one of `U`.
#[diagnostic::on_unimplemented(
    message = "can't add `{Self}` to `{U}`",
    label = "the dimensions differ",
    note = "dimensions are written in SI base units: `(kg, m<2>, s<-2>)` is kg·m²·s⁻², and `()` is dimensionless"
)]
pub trait AddUnits<U> {}

impl<U> AddUnits<U> for U {}

/// Checks that a value of [`InBaseUnits`] `Self` can be subtracted from one
/// of `U`.
#[diagnostic::on_unimplemented(
    message = "can't subtract `{Self}` from `{U}`",
    label = "the dimensions differ",
    note = "dimensions are written in SI base units: `(kg, m<2>, s<-2>)` is kg·m²·s⁻², and `()` is dimensionless"
)]
pub trait SubUnits<U> {}

impl<U> SubUnits<U> for U {}

/// `A × N` for exponents.
#[diagnostic::on_unimplemented(
    message = "an exponent in this formula is out of range",
    note = "formulas support exponents up to qty::formula::MAX_EXPONENT"
)]
pub trait MulE<const N: i8> {
    /// The product.
    type Out;
}

/// The dimensions of a power.
#[diagnostic::on_unimplemented(
    message = "an exponent in this formula is out of range",
    note = "formulas support exponents up to qty::formula::MAX_EXPONENT"
)]
pub trait PowDim<const N: i8> {
    /// Every exponent times `N`.
    type Out;
}

impl<L, M, T, I, H, N, J, const P: i8> PowDim<P> for Dim<L, M, T, I, H, N, J>
where
    L: MulE<P>,
    M: MulE<P>,
    T: MulE<P>,
    I: MulE<P>,
    H: MulE<P>,
    N: MulE<P>,
    J: MulE<P>,
{
    type Out = Dim<L::Out, M::Out, T::Out, I::Out, H::Out, N::Out, J::Out>;
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
            type Rem = (L::Rem, M::Rem, T::Rem, I::Rem, H::Rem, N::Rem, J::Rem);
        }
    };
}
dim_op!(MulDim, AddE);
dim_op!(DivDim, SubE);
dim_op!(HalfDim, HalfE, root);
dim_op!(ThirdDim, ThirdE, root);

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

impl<D: Dimension, V> Expr<D, V> {
    /// The dimensions.
    ///
    /// ```rust
    /// # use qty::*;
    /// use qty::formula::Expr;
    ///
    /// let m = Mass::from_kilograms(2.0);
    /// let v = Speed::from_meters_per_second(3.0);
    /// assert_eq!(Expr::dimensions(&expr!(m * v)), Dimensions::new(1, 1, -1, 0, 0, 0, 0));
    /// ```
    pub const fn dimensions(&self) -> Dimensions {
        D::DIMENSIONS
    }
}

impl<D, V: crate::Number + Copy> Expr<D, V> {
    /// `self` to the power `N`, which can be negative. The exponents of
    /// the dimensions are multiplied by `N`.
    ///
    /// ```rust
    /// # use qty::*;
    /// // The power radiated by a black body.
    /// use qty::constants::STEFAN_BOLTZMANN_CONSTANT as SIGMA;
    ///
    /// let (area, t) = (Area::from_square_meters(1.0), TemperatureDelta::from_kelvins(300.0));
    /// let p: Power = formula!(SIGMA * area * t.powi::<4>());
    /// assert_eq!(format!("{p:.0}"), "459 W");
    /// ```
    #[inline(always)]
    pub fn powi<const N: i8>(self) -> Expr<D::Out, V>
    where
        D: PowDim<N>,
    {
        // `N` is a constant, so this loop unrolls to `|N| - 1`
        // multiplications.
        let one = V::from_f64(1.0);
        let mut result = one;
        let mut i = 0;
        while i < N.unsigned_abs() {
            result = result * self.0;
            i += 1;
        }
        Expr::new(if N < 0 { one / result } else { result })
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

/// The value in coherent SI units and those units, like `4 kg²` or
/// `6 kg·m²/s²`. Format flags apply to the number. The text parses back as
/// a quantity with these dimensions.
///
/// ```rust
/// # use qty::*;
/// let (m, v) = (Mass::from_kilograms(2.0), Speed::from_meters_per_second(3.0));
/// assert_eq!(expr!(m * m).to_string(), "4 kg²");
/// assert_eq!(format!("{:.1}", expr!(0.5 * m * v * v)), "9.0 kg·m²/s²");
/// assert_eq!(qty::constants::GRAVITATIONAL_CONSTANT.to_string(), "0.000000000066743 m³/(kg·s²)");
/// assert_eq!(format!("{:e}", qty::constants::GRAVITATIONAL_CONSTANT), "6.6743e-11 m³/(kg·s²)");
/// ```
impl<D: Dimension, V: fmt::Display> fmt::Display for Expr<D, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)?;
        write_si_units(f, D::DIMENSIONS)
    }
}

impl<D: Dimension, V: fmt::LowerExp> fmt::LowerExp for Expr<D, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerExp::fmt(&self.0, f)?;
        write_si_units(f, D::DIMENSIONS)
    }
}

impl<D: Dimension, V: fmt::UpperExp> fmt::UpperExp for Expr<D, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperExp::fmt(&self.0, f)?;
        write_si_units(f, D::DIMENSIONS)
    }
}

/// Writes ` kg·m²/s²` for `dimensions`, or nothing when dimensionless.
/// Mass comes first, as in `kg·m²`; the denominator gets parentheses when
/// it has more than one unit, and with no numerator the exponents are
/// negative, as in `s⁻¹`.
fn write_si_units(f: &mut fmt::Formatter<'_>, dimensions: Dimensions) -> fmt::Result {
    const UNITS: [&str; 7] = ["kg", "m", "s", "A", "K", "mol", "cd"];
    let [l, m, t, i, th, n, j] = dimensions.to_array();
    let exponents = [m, l, t, i, th, n, j];
    if exponents.iter().all(|&e| e == 0) {
        return Ok(());
    }
    f.write_str(" ")?;
    if exponents.iter().all(|&e| e <= 0) {
        return write_units(f, &UNITS, &exponents, -1, true);
    }
    write_units(f, &UNITS, &exponents, 1, false)?;
    let denominator = exponents.iter().filter(|&&e| e < 0).count();
    if denominator > 0 {
        f.write_str("/")?;
        if denominator > 1 {
            f.write_str("(")?;
        }
        write_units(f, &UNITS, &exponents, -1, false)?;
        if denominator > 1 {
            f.write_str(")")?;
        }
    }
    Ok(())
}

/// Writes the units whose exponents have `sign`, as `kg·m²`, or as `s⁻¹`
/// with `with_sign`.
fn write_units(
    f: &mut fmt::Formatter<'_>,
    units: &[&str; 7],
    exponents: &[i8; 7],
    sign: i8,
    with_sign: bool,
) -> fmt::Result {
    const SUPERSCRIPTS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    let mut first = true;
    for (unit, &exponent) in units.iter().zip(exponents) {
        if exponent.signum() != sign {
            continue;
        }
        if !first {
            f.write_str("·")?;
        }
        first = false;
        f.write_str(unit)?;
        let e = exponent.unsigned_abs();
        if with_sign {
            f.write_str("⁻")?;
        }
        if e >= 10 {
            write!(f, "{}", SUPERSCRIPTS[usize::from(e / 10)])?;
        }
        if e != 1 || with_sign {
            write!(f, "{}", SUPERSCRIPTS[usize::from(e % 10)])?;
        }
    }
    Ok(())
}

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

/// A plain number, such as the `0.5` in `0.5 * m * v * v`, in a formula of
/// any storage type: `f64`, `f32`, or a complex, decimal or rational.
impl<V: crate::Number> Operand<V> for f64 {
    type Dim = Dims;
    #[inline(always)]
    fn into_expr(self) -> Expr<Dims, V> {
        Expr::new(V::from_f64(self))
    }
}

impl Operand<f32> for f32 {
    type Dim = Dims;
    #[inline(always)]
    fn into_expr(self) -> Expr<Dims, f32> {
        Expr::new(self)
    }
}

macro_rules! number_operand {
    ($($t:ty),*) => {$(
        impl<D> FromFormula<D, $t> for $t
        where
            D: InBaseUnits,
            D::Units: Gives<$t, ()>,
        {
            #[inline(always)]
            fn from_formula(expr: Expr<D, $t>) -> $t {
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
///
/// A quantity implements it for every `D`, with a [`Gives`] bound that
/// compares the dimensions, so that a mismatch is reported in
/// [`InBaseUnits`] rather than as a mismatch of [`Dim`]s.
#[diagnostic::on_unimplemented(
    message = "a formula can't give a `{Self}`",
    note = "formulas give quantities whose base unit is in SI, plain numbers and `Expr`s; temperatures and levels are points, use `TemperatureDelta` or a ratio"
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

/// [`Expr::sqrt`], for [`formula!`](crate::formula!). A method with
/// unsatisfied bounds gets a generic error, a function the message of
/// [`WholeSqrt`]. The result is `R` rather than `D::Out`, whose bounds would
/// fail a second time.
#[doc(hidden)]
#[cfg(any(feature = "std", feature = "libm"))]
#[inline(always)]
pub fn sqrt<D: SqrtDim<Out = R>, R, V: crate::FloatMath>(x: Expr<D, V>) -> Expr<R, V> {
    Expr::new(x.0.sqrt())
}

/// [`Expr::cbrt`], for [`formula!`](crate::formula!).
#[doc(hidden)]
#[cfg(any(feature = "std", feature = "libm"))]
#[inline(always)]
pub fn cbrt<D: CbrtDim<Out = R>, R, V: crate::FloatMath>(x: Expr<D, V>) -> Expr<R, V> {
    Expr::new(x.0.cbrt())
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

// `Expr<D2>` rather than `Self` on the right, so that a difference in
// dimensions is reported by `AddUnits` and `SubUnits`, in base units.
impl<D, D2, V> Add<Expr<D2, V>> for Expr<D, V>
where
    V: Add<Output = V>,
    D: InBaseUnits,
    D2: InBaseUnits,
    D2::Units: AddUnits<D::Units>,
{
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Expr<D2, V>) -> Self {
        Expr::new(self.0 + rhs.0)
    }
}

impl<D, D2, V> Sub<Expr<D2, V>> for Expr<D, V>
where
    V: Sub<Output = V>,
    D: InBaseUnits,
    D2: InBaseUnits,
    D2::Units: SubUnits<D::Units>,
{
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Expr<D2, V>) -> Self {
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

        impl<V: $crate::Number> $crate::formula::Operand<V> for $Q<V> {
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

        $crate::__qty_literal_impl!($Q);

        impl<V: $crate::Number> $crate::formula::SiFactor for $Q<V> {
            const FACTOR: f64 =
                $crate::formula::base_unit_factor(<$Q<f64> as $crate::Quantity>::BASE_UNIT_EXPR);
        }

        // Any `D`, checked by `Gives`, for an error message in base units.
        impl<D, V: $crate::Number> $crate::formula::FromFormula<D, V> for $Q<V>
        where
            D: $crate::formula::InBaseUnits,
            D::Units: $crate::formula::Gives<
                $Q<V>,
                <<$Q<V> as $crate::formula::Operand<V>>::Dim as $crate::formula::InBaseUnits>::Units,
            >,
        {
            #[inline(always)]
            fn from_formula(expr: $crate::formula::Expr<D, V>) -> Self {
                $Q::from_base(
                    expr.value() / V::from_f64(<$Q<V> as $crate::formula::SiFactor>::FACTOR),
                )
            }
        }

        impl<D, V: $crate::Number> ::core::convert::From<$crate::formula::Expr<D, V>> for $Q<V>
        where
            $Q<V>: $crate::formula::FromFormula<D, V>,
        {
            #[inline(always)]
            fn from(expr: $crate::formula::Expr<D, V>) -> Self {
                $crate::formula::FromFormula::from_formula(expr)
            }
        }

        impl<D, V: $crate::Number> ::core::ops::Mul<$crate::formula::Expr<D, V>> for $Q<V>
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

        impl<D, V: $crate::Number> ::core::ops::Div<$crate::formula::Expr<D, V>> for $Q<V>
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
/// keeps Rust's precedence and parentheses, and `.sqrt()`, `.cbrt()` and
/// `.powi::<N>()` work on any value or parenthesised group:
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
/// return type or an argument. Where there is none, [`expr!`](crate::expr!)
/// gives the [`Expr`](crate::formula::Expr) itself. See
/// [`formula`](mod@crate::formula) for how it works and what it checks.
#[macro_export]
macro_rules! formula {
    ($($t:tt)+) => {
        $crate::formula::FromFormula::from_formula($crate::__formula!(@start [] $($t)+))
    };
}

/// Computes a formula like [`formula!`](crate::formula!), but gives the
/// [`Expr`](crate::formula::Expr) itself, so no type annotation is needed.
/// It prints in SI units, and works in later formulas or with `into()`:
///
/// ```rust
/// use qty::*;
///
/// let (m, v) = (Mass::from_kilograms(2.0), Speed::from_meters_per_second(3.0));
/// let twice_energy = expr!(m * v * v);
/// assert_eq!(twice_energy.to_string(), "18 kg·m²/s²");
///
/// let energy: Energy = formula!(0.5 * twice_energy);
/// assert_eq!(energy.as_joules(), 9.0);
/// ```
#[macro_export]
macro_rules! expr {
    ($($t:tt)+) => {
        $crate::__formula!(@start [] $($t)+)
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
        $crate::__formula!(@value [$($o)* $crate::formula::sqrt($crate::__formula!(@start [] $($g)+))] [] $($rest)*)
    };
    (@start [$($o:tt)*] ($($g:tt)+) . cbrt () $($rest:tt)*) => {
        $crate::__formula!(@value [$($o)* $crate::formula::cbrt($crate::__formula!(@start [] $($g)+))] [] $($rest)*)
    };
    (@start [$($o:tt)*] ($($g:tt)+) . powi :: < $n:literal > () $($rest:tt)*) => {
        $crate::__formula!(@value [$($o)* ($crate::__formula!(@start [] $($g)+)).powi::<{ $n }>()] [] $($rest)*)
    };
    // `<-` is a single token.
    (@start [$($o:tt)*] ($($g:tt)+) . powi :: <- $n:literal > () $($rest:tt)*) => {
        $crate::__formula!(@value [$($o)* ($crate::__formula!(@start [] $($g)+)).powi::<{ -$n }>()] [] $($rest)*)
    };
    // A unit literal: its `Expr`.
    (@start [$($o:tt)*] qty ! ($($g:tt)+) $($rest:tt)*) => {
        $crate::__formula!(@value [$($o)*] [$crate::__qty_expr!($($g)+)] $($rest)*)
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

    // `.sqrt()`, `.cbrt()` and `.powi::<N>()` on a value work on its `Expr`,
    // so the roots accept any dimensions with even (or multiple of 3)
    // exponents. The roots are functions, whose errors read better.
    (@value [$($o:tt)*] [$($v:tt)+] . sqrt () $($rest:tt)*) => {
        $crate::__formula!(@value [$($o)* $crate::formula::sqrt($crate::formula::operand($($v)+))] [] $($rest)*)
    };
    (@value [$($o:tt)*] [$($v:tt)+] . cbrt () $($rest:tt)*) => {
        $crate::__formula!(@value [$($o)* $crate::formula::cbrt($crate::formula::operand($($v)+))] [] $($rest)*)
    };
    (@value [$($o:tt)*] [$($v:tt)+] . powi :: < $n:literal > () $($rest:tt)*) => {
        $crate::__formula!(@value [$($o)* $crate::formula::operand($($v)+).powi::<{ $n }>()] [] $($rest)*)
    };
    (@value [$($o:tt)*] [$($v:tt)+] . powi :: <- $n:literal > () $($rest:tt)*) => {
        $crate::__formula!(@value [$($o)* $crate::formula::operand($($v)+).powi::<{ -$n }>()] [] $($rest)*)
    };
    (@value [$($o:tt)*] [] . $($rest:tt)*) => {
        ::core::compile_error!("formula!: only .sqrt(), .cbrt() and .powi::<N>() can follow parentheses")
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
