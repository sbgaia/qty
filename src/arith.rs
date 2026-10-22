//! `*` and `/` between any two quantities.
//!
//! Each built-in quantity has one generic `Mul` and one generic `Div` impl,
//! for any right-hand side that implements [`Arith`]: a quantity, an
//! [`Expr`] or a plain number. The output type is worked out from the
//! dimensions, at compile time:
//!
//! 1. A number scales the quantity: `length * 2.0` is a `Length`.
//! 2. A relation between the two quantities gives its quantity, as listed in
//!    `cross_type.rs`: `power / energy` is a `Frequency`.
//! 3. Otherwise the dimensions of the result pick it: `mass * speed * speed`
//!    is an `Energy`. When several quantities have those dimensions, one of
//!    them is chosen (see `arith_table.rs`).
//! 4. Dimensions that no quantity has give an [`Expr`]: `mass * mass`.
//!
//! One impl per quantity, rather than one per pair of quantities, keeps the
//! library small: a pair of impls for each of the 12,000 pairs would make
//! it compile several times slower.

use core::marker::PhantomData;
use core::ops::{Div, Mul};

use crate::formula::{Dims, Expr};
use crate::Number;

/// A value on the right of `*` or `/` after a quantity.
#[diagnostic::on_unimplemented(
    message = "a quantity can't be multiplied or divided by `{Self}`",
    note = "the right-hand side can be a quantity, a plain number of the same storage type, or a `qty::formula::Expr`"
)]
pub trait Arith: Sized {
    /// The storage type.
    type Value;
    /// [`PlainNumber`], [`ExprKind`] or [`Quantity`], for picking the output.
    type Kind;
    /// The dimensions, a [`Dim`](crate::formula::Dim).
    type Dim;
    /// [`Coherent`] when [`FACTOR`](Self::FACTOR) is 1, else a marker for
    /// the factor.
    type Unit;
    /// The size of the base unit in coherent SI units.
    const FACTOR: f64;
    /// The value in the base unit.
    fn into_value(self) -> Self::Value;
    /// The value with `value` in the base unit.
    fn from_value(value: Self::Value) -> Self;
}

/// The [`Arith::Kind`] of plain numbers.
#[derive(Debug)]
pub struct PlainNumber;
/// The [`Arith::Kind`] of [`Expr`].
#[derive(Debug)]
pub struct ExprKind;
/// The [`Arith::Kind`] of quantities. `GM` and `GD` are [`NoGroup`], or
/// markers for the quantities on the right of a `*` or `/` relation whose
/// output isn't the one the dimensions pick.
#[derive(Debug)]
pub struct Quantity<GM, GD>(PhantomData<(GM, GD)>);
/// The group of most quantities: see [`Quantity`].
#[derive(Debug)]
pub struct NoGroup;

/// The operator `*`.
#[derive(Debug)]
pub struct MulOp;
/// The operator `/`.
#[derive(Debug)]
pub struct DivOp;

/// `*` or `/` on values.
pub trait BinOp<V> {
    /// `true` for `*`, `false` for `/`.
    const MUL: bool;
    /// `a * b` or `a / b`.
    fn apply(a: V, b: V) -> V;
}

impl<V: Mul<Output = V>> BinOp<V> for MulOp {
    const MUL: bool = true;
    #[inline(always)]
    fn apply(a: V, b: V) -> V {
        a * b
    }
}

impl<V: Div<Output = V>> BinOp<V> for DivOp {
    const MUL: bool = false;
    #[inline(always)]
    fn apply(a: V, b: V) -> V {
        a / b
    }
}

/// The factor from the base units of `a op b` to the base unit of the
/// output, `fa · fb / fc` or `fa / fb / fc` for base units `fa`, `fb` and
/// `fc` in coherent SI. `1` for coherent SI base units.
#[inline(always)]
pub const fn factor(mul: bool, fa: f64, fb: f64, fc: f64) -> f64 {
    if mul {
        fa * fb / fc
    } else {
        fa / fb / fc
    }
}

/// The [`Arith::Unit`] of values whose base unit is coherent SI. Other
/// sizes of base units have markers in `arith_table.rs`, such as grams per
/// second for `MassFlow`.
#[derive(Debug)]
pub struct Coherent;

/// Converts the result of `a op b` to the output's base unit, from the
/// [`Arith::Unit`]s of `a`, `b` and the output. Where the sizes cancel,
/// as between coherent units, it does nothing, so that any storage type
/// with the operator works; the others need a [`Number`] for the factor.
/// The impls are in `arith_table.rs`.
#[diagnostic::on_unimplemented(
    message = "this storage type can't be converted between the base units of these quantities",
    note = "a base unit that isn't coherent SI, like grams per second, needs a storage type that implements `qty::Number`"
)]
pub trait Rescale<V, Op> {
    /// `value * k`.
    fn rescale(value: V, k: f64) -> V;
}

/// A plain number on the right of `*` or `/`: a [`Number`] or a primitive
/// integer. One generic [`Arith`] impl for all of them, rather than one per
/// type, lets `length * 2` infer the type of `2` from the storage type.
pub trait PlainScalar {}

impl<T: Number> PlainScalar for T {}

macro_rules! integer_scalar {
    ($($t:ty),*) => {$(
        impl PlainScalar for $t {}
    )*};
}
integer_scalar!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

impl<T: PlainScalar> Arith for T {
    type Value = T;
    type Kind = PlainNumber;
    type Dim = Dims;
    type Unit = Coherent;
    const FACTOR: f64 = 1.0;
    #[inline(always)]
    fn into_value(self) -> T {
        self
    }
    #[inline(always)]
    fn from_value(value: T) -> T {
        value
    }
}

impl<D, V> Arith for Expr<D, V> {
    type Value = V;
    type Kind = ExprKind;
    type Dim = D;
    type Unit = Coherent;
    const FACTOR: f64 = 1.0;
    #[inline(always)]
    fn into_value(self) -> V {
        self.value()
    }
    #[inline(always)]
    fn from_value(value: V) -> Self {
        Expr::new(value)
    }
}

/// Picks how `S op R` is computed, from the group of `S` (`Self`) and the
/// [`Arith::Kind`] of `R`: [`Scaled`], [`ByDimensions`] or one of the
/// relations of `arith_table.rs`.
pub trait Pick<K, Op> {
    /// The [`Select`] impl to use.
    type Sel;
}

/// `quantity op number`.
#[derive(Debug)]
pub struct Scaled;
/// The output that the dimensions pick.
#[derive(Debug)]
pub struct ByDimensions;

impl<G, Op> Pick<PlainNumber, Op> for G {
    type Sel = Scaled;
}

impl<G, Op> Pick<ExprKind, Op> for G {
    type Sel = ByDimensions;
}

impl<GM, GD, Op> Pick<Quantity<GM, GD>, Op> for NoGroup {
    type Sel = ByDimensions;
}

/// The output of `S op R`, where the result has the dimensions `D`.
#[diagnostic::on_unimplemented(
    message = "the result of this operation is a quantity behind a feature that isn't enabled",
    note = "enable the feature of the quantity with these dimensions, or use `formula!` or `expr!`"
)]
pub trait Select<S: Arith, R, Op, D> {
    /// The output type.
    type Out;
    /// The output, from the values of `S` and `R` in their base units.
    fn apply(a: S::Value, b: S::Value) -> Self::Out;
}

/// A quantity times a number is the same quantity.
macro_rules! scaled {
    ($($T:ty),*) => {$(
        impl<S, R, Op, D> Select<S, R, Op, D> for $T
        where
            S: Arith,
            R: Arith<Value = S::Value>,
            Op: BinOp<S::Value>,
        {
            type Out = S;
            #[inline(always)]
            fn apply(a: S::Value, b: S::Value) -> S {
                S::from_value(Op::apply(a, b))
            }
        }
    )*};
}
// The kinds select directly for quantities without a group.
scaled!(Scaled, PlainNumber);

/// The quantity that the dimensions `D` pick, or an [`Expr`].
pub trait Canon<V> {
    /// The output type.
    type Out;
    /// Its [`Arith::Unit`].
    type Unit;
    /// The size of its base unit in coherent SI.
    const FACTOR: f64;
    /// The output with `value` in its base unit.
    fn make(value: V) -> Self::Out;
}

macro_rules! by_dimensions {
    ($([$($G:ident),*] $T:ty),*) => {$(
        impl<$($G,)* S, R, Op, D> Select<S, R, Op, D> for $T
        where
            S: Arith,
            R: Arith<Value = S::Value>,
            Op: BinOp<S::Value>,
            D: Canon<S::Value>,
            (S::Unit, R::Unit, D::Unit): Rescale<S::Value, Op>,
        {
            type Out = D::Out;
            #[inline(always)]
            fn apply(a: S::Value, b: S::Value) -> D::Out {
                let k = const { factor(Op::MUL, S::FACTOR, R::FACTOR, D::FACTOR) };
                D::make(<(S::Unit, R::Unit, D::Unit) as Rescale<S::Value, Op>>::rescale(Op::apply(a, b), k))
            }
        }
    )*};
}
// The kinds select directly for quantities without a group.
by_dimensions!([] ByDimensions, [] ExprKind, [GM, GD] Quantity<GM, GD>);

/// A node of the lookup from dimensions to quantities: the next node for
/// the exponent `X`.
pub trait Step<X> {
    /// The next node.
    type Next;
}

/// The last node of the lookup: the quantity for dimensions `D`.
pub trait Leaf<D, V> {
    /// The quantity.
    type Out;
    /// Its [`Arith::Unit`].
    type Unit;
    /// The size of its base unit in coherent SI.
    const FACTOR: f64;
    /// The quantity with `value` in its base unit.
    fn make(value: V) -> Self::Out;
}

/// Dimensions that no quantity has.
#[derive(Debug)]
pub struct Unnamed;

impl<X> Step<X> for Unnamed {
    type Next = Unnamed;
}

impl<D, V> Leaf<D, V> for Unnamed {
    type Out = Expr<D, V>;
    type Unit = Coherent;
    const FACTOR: f64 = 1.0;
    #[inline(always)]
    fn make(value: V) -> Expr<D, V> {
        Expr::new(value)
    }
}

#[path = "arith_table.rs"]
mod table;
pub use table::*;
