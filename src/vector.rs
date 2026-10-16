//! 2D and 3D vectors of quantities.

use core::fmt;
use core::ops;

#[cfg(any(feature = "std", feature = "libm"))]
use crate::{Angle, Float, FloatMath};
use crate::{Quantity, Unit};

macro_rules! vector {
    ($(#[$doc:meta])* $V:ident, $n:literal: $($c:ident),+) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[repr(C)]
        pub struct $V<Q> {
            $(
                #[doc = concat!("The ", stringify!($c), " component.")]
                pub $c: Q,
            )+
        }

        impl<Q> $V<Q> {
            /// A vector from its components.
            #[inline(always)]
            pub const fn new($($c: Q),+) -> Self {
                Self { $($c),+ }
            }

            /// Applies `f` to every component, e.g. `v.map(|c| c.as_kilometers())`.
            #[inline]
            pub fn map<R>(self, mut f: impl FnMut(Q) -> R) -> $V<R> {
                $V { $($c: f(self.$c)),+ }
            }

            /// The components as an array.
            #[inline(always)]
            pub fn to_array(self) -> [Q; $n] {
                [$(self.$c),+]
            }

            /// The dot product. Its type follows the quantities' relation:
            /// `Force · Speed` is `Power`, `Length · Length` is `Area`.
            ///
            /// The relations are UnitsNet's, which define `Force × Length`
            /// as `Torque` only: for the work done by a force (an `Energy`,
            /// same unit N·m), convert with `Energy::from_base(t.as_base())`.
            #[inline]
            pub fn dot<R, O>(self, other: $V<R>) -> O
            where
                Q: ops::Mul<R, Output = O>,
                O: ops::Add<Output = O>,
            {
                vector!(@sum $(self.$c * other.$c),+)
            }

            /// The squared length, `self · self`: cheaper than `norm`, for
            /// comparisons.
            #[inline]
            pub fn norm_squared<O>(self) -> O
            where
                Q: ops::Mul<Output = O> + Copy,
                O: ops::Add<Output = O>,
            {
                self.dot(self)
            }
        }

        impl<Q: Quantity> $V<Q> {
            /// A vector from base-unit components.
            #[inline]
            pub fn from_base(components: [Q::Value; $n]) -> Self {
                let [$($c),+] = components;
                Self { $($c: Q::from_base($c)),+ }
            }

            /// The base-unit components.
            #[inline]
            pub fn as_base(self) -> [Q::Value; $n] {
                [$(self.$c.as_base()),+]
            }
        }

        impl<Q: Quantity> $V<Q>
        where
            Q::Value: crate::Float,
        {
            /// Linear interpolation, component by component: `self` at
            /// `t = 0`, `other` at `t = 1`.
            #[inline]
            pub fn lerp(self, other: Self, t: Q::Value) -> Self {
                let (a, b) = (self.as_base(), other.as_base());
                Self::from_base(core::array::from_fn(|i| a[i] + (b[i] - a[i]) * t))
            }
        }

        #[cfg(any(feature = "std", feature = "libm"))]
        impl<Q: Quantity> $V<Q>
        where
            Q::Value: FloatMath,
        {
            /// The length (magnitude), through `hypot`: no intermediate
            /// overflow or underflow, and with [`Uncertain`](crate::Uncertain)
            /// storage the components count as independent.
            #[inline]
            pub fn norm(self) -> Q {
                Q::from_base(vector!(@hypot $(self.$c.as_base()),+))
            }

            /// The unit vector in the same direction: plain numbers. NaN for
            /// the zero vector.
            #[inline]
            pub fn direction(self) -> $V<Q::Value> {
                let n = self.norm().as_base();
                self.map(|c| c.as_base() / n)
            }

            /// The angle between two vectors, in 0°…180°.
            #[inline]
            pub fn angle_between(self, other: Self) -> Angle<Q::Value> {
                let (a, b) = (self.map(Q::as_base), other.map(Q::as_base));
                let dot = vector!(@sum $(a.$c * b.$c),+);
                let cross = vector!(@cross_norm a, b; $($c),+);
                Angle::from_base(cross.atan2(dot))
            }
        }

        impl<Q> From<[Q; $n]> for $V<Q> {
            #[inline]
            fn from([$($c),+]: [Q; $n]) -> Self {
                Self { $($c),+ }
            }
        }

        impl<Q> From<$V<Q>> for [Q; $n] {
            #[inline]
            fn from(v: $V<Q>) -> Self {
                [$(v.$c),+]
            }
        }

        // ── Arithmetic ──

        impl<Q: ops::Add<Output = Q>> ops::Add for $V<Q> {
            type Output = Self;
            #[inline(always)]
            fn add(self, rhs: Self) -> Self {
                Self { $($c: self.$c + rhs.$c),+ }
            }
        }
        impl<Q: ops::Sub<Output = Q>> ops::Sub for $V<Q> {
            type Output = Self;
            #[inline(always)]
            fn sub(self, rhs: Self) -> Self {
                Self { $($c: self.$c - rhs.$c),+ }
            }
        }
        impl<Q: ops::Neg<Output = Q>> ops::Neg for $V<Q> {
            type Output = Self;
            #[inline(always)]
            fn neg(self) -> Self {
                Self { $($c: -self.$c),+ }
            }
        }
        impl<Q: ops::AddAssign> ops::AddAssign for $V<Q> {
            #[inline(always)]
            fn add_assign(&mut self, rhs: Self) {
                $(self.$c += rhs.$c;)+
            }
        }
        impl<Q: ops::SubAssign> ops::SubAssign for $V<Q> {
            #[inline(always)]
            fn sub_assign(&mut self, rhs: Self) {
                $(self.$c -= rhs.$c;)+
            }
        }

        /// Scaling by a number or a quantity: `velocity * 2.0`,
        /// `velocity * duration` (a displacement).
        impl<Q: ops::Mul<S>, S: Copy> ops::Mul<S> for $V<Q> {
            type Output = $V<Q::Output>;
            #[inline(always)]
            fn mul(self, rhs: S) -> Self::Output {
                $V { $($c: self.$c * rhs),+ }
            }
        }
        /// Dividing by a number or a quantity: `displacement / duration` (a
        /// velocity).
        impl<Q: ops::Div<S>, S: Copy> ops::Div<S> for $V<Q> {
            type Output = $V<Q::Output>;
            #[inline(always)]
            fn div(self, rhs: S) -> Self::Output {
                $V { $($c: self.$c / rhs),+ }
            }
        }
        impl<Q: ops::MulAssign<S>, S: Copy> ops::MulAssign<S> for $V<Q> {
            #[inline(always)]
            fn mul_assign(&mut self, rhs: S) {
                $(self.$c *= rhs;)+
            }
        }
        impl<Q: ops::DivAssign<S>, S: Copy> ops::DivAssign<S> for $V<Q> {
            #[inline(always)]
            fn div_assign(&mut self, rhs: S) {
                $(self.$c /= rhs;)+
            }
        }
        /// `2.0 * vector`.
        impl<Q> ops::Mul<$V<Q>> for f64
        where
            f64: ops::Mul<Q>,
        {
            type Output = $V<<f64 as ops::Mul<Q>>::Output>;
            #[inline(always)]
            fn mul(self, rhs: $V<Q>) -> Self::Output {
                $V { $($c: self * rhs.$c),+ }
            }
        }

        impl<Q: ops::Add<Output = Q> + Default> core::iter::Sum for $V<Q> {
            #[inline]
            fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
                iter.fold(Self::default(), |acc, v| acc + v)
            }
        }
        impl<'a, Q: ops::Add<Output = Q> + Default + Copy + 'a> core::iter::Sum<&'a $V<Q>> for $V<Q> {
            #[inline]
            fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
                iter.copied().sum()
            }
        }

        /// `(x, y, …) unit`, with the components in the base unit and the
        /// format flags applied to each: `(1.00, 2.00, 0.50) m`.
        impl<Q: Quantity> fmt::Display for $V<Q>
        where
            Q::Value: fmt::Display,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("(")?;
                let mut first = true;
                for c in self.as_base() {
                    if !first {
                        f.write_str(", ")?;
                    }
                    first = false;
                    fmt::Display::fmt(&c, f)?;
                }
                f.write_str(")")?;
                let abbr = Q::BASE_UNIT.abbreviation();
                if abbr.is_empty() {
                    Ok(())
                } else {
                    write!(f, " {abbr}")
                }
            }
        }

        // SAFETY: `#[repr(C)]` with every field of type `Q`, so there is no
        // padding, and any bit pattern valid for each `Q` is valid.
        #[cfg(feature = "bytemuck")]
        unsafe impl<Q: bytemuck::Zeroable> bytemuck::Zeroable for $V<Q> {}
        // SAFETY: as above.
        #[cfg(feature = "bytemuck")]
        unsafe impl<Q: bytemuck::Pod> bytemuck::Pod for $V<Q> {}
    };

    (@sum $first:expr $(, $rest:expr)*) => { $first $(+ $rest)* };
    (@hypot $x:expr, $y:expr) => { $x.hypot($y) };
    (@hypot $x:expr, $y:expr, $z:expr) => { $x.hypot($y).hypot($z) };
    // |a × b|: for 2D the absolute perp-dot product.
    (@cross_norm $a:ident, $b:ident; x, y) => { Float::abs($a.x * $b.y - $a.y * $b.x) };
    (@cross_norm $a:ident, $b:ident; x, y, z) => {
        ($a.y * $b.z - $a.z * $b.y)
            .hypot($a.z * $b.x - $a.x * $b.z)
            .hypot($a.x * $b.y - $a.y * $b.x)
    };
}

vector! {
    /// A 2D vector of quantities: a position, a velocity, a force in a plane.
    ///
    /// See [`Vector3`] for the operations; `Vector2` has the same ones, with
    /// [`perp_dot`](Vector2::perp_dot) in place of the cross product, plus
    /// polar coordinates.
    Vector2, 2: x, y
}

vector! {
    /// A 3D vector of quantities: a displacement, a velocity, a force, …
    ///
    /// Components are any quantity type, so the physics carries over:
    /// dividing a displacement by a duration gives a velocity, and the dot
    /// product of a force and a displacement is an energy.
    ///
    /// ```rust
    /// use unitsnet_rs::*;
    ///
    /// let displacement = Vector3::new(
    ///     Length::from_meters(3.0),
    ///     Length::from_meters(4.0),
    ///     Length::from_meters(0.0),
    /// );
    /// let velocity: Vector3<Speed> = displacement / Duration::from_seconds(2.0);
    /// assert_eq!(velocity.norm(), Speed::from_meters_per_second(2.5));
    ///
    /// let force: Vector3<Force> = Mass::from_kilograms(2.0) * Vector3::new(
    ///     Acceleration::from_meters_per_second_squared(0.0),
    ///     Acceleration::from_meters_per_second_squared(0.0),
    ///     Acceleration::from_meters_per_second_squared(-9.81),
    /// );
    /// let power: Power = force.dot(velocity); // F · v
    /// assert_eq!(power, Power::from_watts(0.0));
    ///
    /// // The cross product of two edges: the area of their parallelogram.
    /// let a = Vector3::<Length>::from_base([2.0, 0.0, 0.0]);
    /// let b = Vector3::<Length>::from_base([0.0, 3.0, 0.0]);
    /// let normal: Vector3<Area> = a.cross(b);
    /// assert_eq!(normal.norm(), Area::from_square_meters(6.0));
    ///
    /// assert_eq!(format!("{:.1}", displacement), "(3.0, 4.0, 0.0) m");
    /// ```
    Vector3, 3: x, y, z
}

impl<Q> Vector3<Q> {
    /// The cross product. Its type follows the quantities' relation:
    /// `Length × Force` is `Torque` (`r × F`).
    #[inline]
    pub fn cross<R, O>(self, other: Vector3<R>) -> Vector3<O>
    where
        Q: ops::Mul<R, Output = O> + Copy,
        R: Copy,
        O: ops::Sub<Output = O>,
    {
        Vector3 {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }
}

impl<Q> Vector2<Q> {
    /// The 2D cross product `x₁·y₂ − y₁·x₂`: the z component of the 3D cross
    /// product, positive when `other` is counter-clockwise from `self`.
    #[inline]
    pub fn perp_dot<R, O>(self, other: Vector2<R>) -> O
    where
        Q: ops::Mul<R, Output = O>,
        O: ops::Sub<Output = O>,
    {
        self.x * other.y - self.y * other.x
    }
}

#[cfg(any(feature = "std", feature = "libm"))]
impl<Q: Quantity> Vector2<Q>
where
    Q::Value: FloatMath,
{
    /// A vector from its length and its angle from the x axis.
    ///
    /// ```rust
    /// use unitsnet_rs::*;
    ///
    /// let v = Vector2::from_polar(Speed::from_meters_per_second(2.0), Angle::from_degrees(90.0));
    /// assert!(v.x.approx_eq_abs(Speed::from_base(0.0), 1e-15));
    /// assert!(v.y.approx_eq_rel(Speed::from_meters_per_second(2.0), 1e-15));
    /// assert!((v.angle().as_degrees() - 90.0).abs() < 1e-12);
    /// ```
    #[inline]
    pub fn from_polar(length: Q, angle: Angle<Q::Value>) -> Self {
        let (sin, cos) = angle.sin_cos();
        let r = length.as_base();
        Self::from_base([r * cos, r * sin])
    }

    /// The angle from the x axis, in −180°…180°.
    #[inline]
    pub fn angle(self) -> Angle<Q::Value> {
        Angle::atan2(self.y, self.x)
    }
}
