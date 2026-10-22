//! `*` and `/` between any two quantities: the output types, and the values
//! where base units aren't coherent SI.

use qty::constants::{GRAVITATIONAL_CONSTANT as G, STANDARD_GRAVITY};
use qty::formula::{Dims, Expr};
use qty::*;

fn close(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-12 * b.abs().max(1.0), "{a} != {b}");
}

#[test]
fn formulas_with_plain_operators() {
    let (m, v) = (
        Mass::from_kilograms(70.0),
        Speed::from_meters_per_second(3.0),
    );
    let t = Duration::from_seconds(2.0);

    let e: Energy = 0.5 * m * v * v;
    assert_eq!(e.as_joules(), 315.0);
    let p: Power = m * v * v / t;
    assert_eq!(p.as_watts(), 315.0);
    let x: Length = v * t + 0.5 * (v / t) * t * t;
    assert_eq!(x.as_meters(), 9.0);

    let (earth, r) = (
        Mass::from_kilograms(5.972e24),
        Length::from_kilometers(6371.0),
    );
    let weight: Force = G * earth * m / (r * r);
    assert_eq!(format!("{weight:.0}"), "687 N");
    let same: Force = formula!(G * earth * m / (r * r));
    assert_eq!(weight, same);
}

#[test]
fn dimensions_pick_the_quantity() {
    let l = Length::from_meters(2.0);
    let volume: Volume = l * l * l;
    assert_eq!(volume.as_cubic_meters(), 8.0);
    let a: Acceleration = l / (Duration::from_seconds(1.0) * Duration::from_seconds(2.0));
    assert_eq!(a.as_meters_per_second_squared(), 1.0);
    // Dimensions shared with `Torque`: the energy.
    let e: Energy = Pressure::from_pascals(3.0) * Volume::from_cubic_meters(2.0);
    assert_eq!(e.as_joules(), 6.0);
    let f: Frequency = 1.0 / Duration::from_seconds(4.0);
    assert_eq!(f.as_hertz(), 0.25);
    let f: Frequency = Speed::from_meters_per_second(3.0) / l;
    assert_eq!(f.as_hertz(), 1.5);
    // An angle is dimensionless: an arc is a length.
    let arc: Length = Angle::from_radians(0.5) * l;
    assert_eq!(arc.as_meters(), 1.0);
}

#[test]
fn unnamed_dimensions_give_an_expr() {
    let m = Mass::from_kilograms(3.0);
    let mm: Expr<Dims<0, 2>> = m * m;
    assert_eq!(mm.to_string(), "9 kg²");
    let back: Mass = mm / m;
    assert_eq!(back, m);
    let weight: Force = mm * STANDARD_GRAVITY / m;
    close(weight.as_newtons(), 3.0 * 9.806_65);
}

#[cfg(not(feature = "mechanics"))]
#[test]
fn quantities_behind_a_disabled_feature_give_an_expr() {
    // `Impulse` is in `mechanics`.
    let p: Expr<Dims<1, 1, -1>> = Mass::from_kilograms(2.0) * Speed::from_meters_per_second(3.0);
    assert_eq!(p.value(), 6.0);
}

#[test]
fn constants_times_quantities_are_quantities() {
    let w: Force = Mass::from_kilograms(10.0) * STANDARD_GRAVITY;
    close(w.as_newtons(), 98.0665);
    let w: Force = STANDARD_GRAVITY * Mass::from_kilograms(10.0);
    close(w.as_newtons(), 98.0665);
    let t: Duration = Speed::from_meters_per_second(9.806_65) / STANDARD_GRAVITY;
    close(t.as_seconds(), 1.0);
}

#[test]
fn numbers_scale_and_ratios_are_numbers() {
    let l = Length::from_meters(3.0);
    let twice: Length = l * 2.0;
    assert_eq!(twice / l, 2.0);
    let half: Length = l / 2.0;
    assert_eq!(half.as_meters(), 1.5);
    let r: f64 = Energy::from_joules(6.0) / Energy::from_joules(3.0);
    assert_eq!(r, 2.0);
    let n: f64 = Duration::from_seconds(2.0) * Frequency::from_hertz(3.0);
    assert_eq!(n, 6.0);
}

#[test]
fn integer_storage() {
    let a: Area<i32> = Length::from_base(2) * Length::from_base(3);
    assert_eq!(a.into_base(), 6);
    let f: Force<i64> = Mass::from_base(3) * Acceleration::from_base(4);
    assert_eq!(f.into_base(), 12);
    assert_eq!(Length::<u8>::from_base(4) * 2, Length::from_base(8));
    assert_eq!(Length::<i32>::from_base(9) / Length::from_base(3), 3);
}

#[test]
fn f32_storage() {
    let v: Speed<f32> = Length::from_base(6.0f32) / Duration::from_base(2.0f32);
    assert_eq!(v.into_base(), 3.0);
    let e: Energy<f32> = v * v * Mass::from_base(2.0f32) * 0.5;
    assert_eq!(e.into_base(), 9.0);
}

#[test]
fn generic_code() {
    fn product<A: core::ops::Mul<B>, B>(a: A, b: B) -> A::Output {
        a * b
    }
    let w: Energy = product(Force::from_newtons(2.0), Length::from_meters(3.0));
    assert_eq!(w.as_joules(), 6.0);
}

#[cfg(feature = "mechanics")]
#[test]
fn torque_stays_a_torque() {
    let t = Torque::from_newton_meters(4.0);
    let doubled: Torque = t * 2.0;
    assert_eq!(doubled.as_newton_meters(), 8.0);
    let r: f64 = t / Energy::from_joules(2.0);
    assert_eq!(r, 2.0);
    let k: RotationalStiffness = t / Angle::from_radians(2.0);
    assert_eq!(k.as_newton_meters_per_radian(), 2.0);
    let t: Torque = Power::from_watts(6.0) / RotationalSpeed::from_radians_per_second(2.0);
    assert_eq!(t.as_newton_meters(), 3.0);
    let t: Torque = Area::from_square_meters(2.0) * ForcePerLength::from_newtons_per_meter(3.0);
    assert_eq!(t.as_newton_meters(), 6.0);
    let p: Impulse = Mass::from_kilograms(2.0) * Speed::from_meters_per_second(3.0);
    assert_eq!(p.as_newton_seconds(), 6.0);
}

#[cfg(feature = "fluids")]
#[test]
fn base_units_that_are_not_coherent() {
    // `MassFlow` is stored in grams per second.
    let flow: MassFlow = Mass::from_kilograms(3.0) / Duration::from_seconds(2.0);
    assert_eq!(flow.as_grams_per_second(), 1500.0);
    let m: Mass = flow * Duration::from_seconds(2.0);
    assert_eq!(m.as_kilograms(), 3.0);
    // Without a relation, through the dimensions.
    let f: Frequency = flow / Mass::from_kilograms(3.0);
    assert_eq!(f.as_hertz(), 0.5);
    let mm: Expr<Dims<0, 2, -1>> = flow * Mass::from_kilograms(2.0);
    assert_eq!(mm.value(), 3.0);
    let r: f64 = flow / MassFlow::from_grams_per_second(500.0);
    assert_eq!(r, 3.0);
    let doubled: MassFlow = flow * 2.0;
    assert_eq!(doubled.as_grams_per_second(), 3000.0);
    assert_eq!(MassFlow::<i64>::from_base(6) / MassFlow::from_base(2), 3);
}

#[cfg(feature = "information")]
#[test]
fn relations_to_quantities_outside_formulas() {
    let bits: Information = BitRate::from_bits_per_second(8.0) * Duration::from_seconds(2.0);
    assert_eq!(bits.as_bits(), 16.0);
}
