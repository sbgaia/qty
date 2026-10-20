//! Textbook formulas through `formula!`, checked against known values.

#![cfg(all(feature = "full", any(feature = "std", feature = "libm")))]

use qty::constants::*;
use qty::formula::{Dims, Expr};
use qty::*;

fn close(actual: f64, expected: f64, rel: f64) {
    assert!(
        (actual - expected).abs() <= rel * expected.abs(),
        "{actual} is not within {rel} of {expected}"
    );
}

#[test]
fn gravity() {
    let (earth, person) = (Mass::from_kilograms(5.972e24), Mass::from_kilograms(70.0));
    let r = Length::from_kilometers(6371.0);
    let f: Force = formula!(GRAVITATIONAL_CONSTANT * earth * person / (r * r));
    close(f.as_newtons(), 686.9, 1e-3);

    let g: Acceleration = formula!(GRAVITATIONAL_CONSTANT * earth / (r * r));
    close(g.as_meters_per_second_squared(), 9.82, 1e-3);
}

#[test]
fn mechanics() {
    let (m, v) = (
        Mass::from_kilograms(2.0),
        Speed::from_meters_per_second(3.0),
    );
    let e: Energy = formula!(0.5 * m * v * v);
    assert_eq!(e.as_joules(), 9.0);

    // Centripetal force.
    let r = Length::from_meters(1.5);
    let f: Force = formula!(m * v * v / r);
    assert_eq!(f.as_newtons(), 12.0);

    // Force times length is the work done, an energy. `formula!` gives a
    // torque, of the same dimensions, when that is asked for.
    let w: Energy = f * Length::from_meters(0.5);
    assert_eq!(w.as_joules(), 6.0);
    assert_eq!(w / Length::from_meters(0.5), f);
    let lever = Length::from_meters(0.5);
    let torque: Torque = formula!(f * lever);
    let work: Energy = formula!(f * lever);
    assert_eq!(torque.as_newton_meters(), 6.0);
    assert_eq!(work.as_joules(), 6.0);

    // Dynamic pressure.
    let rho = Density::from_kilograms_per_cubic_meter(1.225);
    let q: Pressure = formula!(0.5 * rho * v * v);
    close(q.as_pascals(), 5.5125, 1e-12);
}

#[test]
fn roots() {
    let length = Length::from_meters(1.0);
    let period: Duration =
        formula!(2.0 * core::f64::consts::PI * (length / STANDARD_GRAVITY).sqrt());
    close(period.as_seconds(), 2.006, 1e-3);

    let (m, r) = (
        Mass::from_kilograms(5.972e24),
        Length::from_kilometers(6371.0),
    );
    let escape: Speed = formula!((2.0 * GRAVITATIONAL_CONSTANT * m / r).sqrt());
    close(escape.as_kilometers_per_second(), 11.19, 1e-3);

    let area = Area::from_square_meters(16.0);
    let side: Length = formula!(area.sqrt());
    assert_eq!(side.as_meters(), 4.0);

    let volume = Volume::from_cubic_meters(27.0);
    let side: Length = formula!(volume.cbrt());
    close(side.as_meters(), 3.0, 1e-15);
}

#[test]
fn electromagnetism() {
    let (q1, q2) = (
        ElectricCharge::from_microcoulombs(1.0),
        ElectricCharge::from_microcoulombs(2.0),
    );
    let r = Length::from_centimeters(10.0);
    let f: Force = formula!(COULOMB_CONSTANT * q1 * q2 / (r * r));
    close(f.as_newtons(), 1.7975, 1e-4);

    // Same force from the permittivity.
    let f2: Force = formula!(q1 * q2 / (4.0 * core::f64::consts::PI * VACUUM_PERMITTIVITY * r * r));
    close(f2.as_newtons(), f.as_newtons(), 1e-9);

    // µ₀ε₀c² = 1.
    let one: f64 =
        formula!(VACUUM_PERMEABILITY * VACUUM_PERMITTIVITY * SPEED_OF_LIGHT * SPEED_OF_LIGHT);
    close(one, 1.0, 1e-9);
}

#[test]
fn thermal_and_chemistry() {
    let (area, t) = (
        Area::from_square_meters(1.0),
        TemperatureDelta::from_kelvins(300.0),
    );
    let p: Power = formula!(STEFAN_BOLTZMANN_CONSTANT * area * t * t * t * t);
    close(p.as_watts(), 459.3, 1e-3);

    // Ideal gas: one mole at 0 °C in 22.4 L.
    let n = AmountOfSubstance::from_moles(1.0);
    let t = TemperatureDelta::from_kelvins(273.15);
    let v = Volume::from_liters(22.414);
    let pressure: Pressure = formula!(n * MOLAR_GAS_CONSTANT * t / v);
    close(pressure.as_pascals(), 101_325.0, 1e-4);

    let r: f64 = formula!(MOLAR_GAS_CONSTANT / (AVOGADRO_CONSTANT * BOLTZMANN_CONSTANT));
    close(r, 1.0, 1e-15);
}

#[test]
fn photon() {
    let wavelength = Length::from_nanometers(532.0);
    let energy: Energy = formula!(PLANCK_CONSTANT * SPEED_OF_LIGHT / wavelength);
    close(energy.in_unit(EnergyUnit::ElectronVolt).value, 2.3305, 1e-4);

    let frequency: Frequency = formula!(SPEED_OF_LIGHT / wavelength);
    close(frequency.as_terahertz(), 563.5, 1e-4);
}

// Base units that aren't coherent SI are converted on the way in and out.
#[test]
fn non_si_base_units() {
    let flow: MassFlow = formula!(Mass::from_kilograms(1.0) / Duration::from_seconds(1.0));
    close(flow.as_grams_per_second(), 1000.0, 1e-15);

    let back: Mass = formula!(flow * Duration::from_minutes(1.0));
    close(back.as_kilograms(), 60.0, 1e-15);
}

#[test]
fn sums_and_signs() {
    let (m, v) = (
        Mass::from_kilograms(2.0),
        Speed::from_meters_per_second(3.0),
    );
    let e: Energy = formula!(0.5 * m * v * v);
    let zero: Energy = formula!(-(m * v * v) + 2.0 * e);
    assert_eq!(zero.as_joules(), 0.0);

    let h = Length::from_meters(10.0);
    let total: Energy = formula!(0.5 * m * v * v + m * STANDARD_GRAVITY * h);
    close(total.as_joules(), 9.0 + 196.133, 1e-12);

    let diff: Energy = formula!(m * STANDARD_GRAVITY * h - 0.5 * m * v * v);
    close(diff.as_joules(), 196.133 - 9.0, 1e-12);
}

#[test]
fn dimensionless_results() {
    // The Reynolds number of water in a pipe.
    let rho = Density::from_kilograms_per_cubic_meter(1000.0);
    let (v, d) = (
        Speed::from_meters_per_second(2.0),
        Length::from_centimeters(5.0),
    );
    let mu = DynamicViscosity::from_pascal_seconds(1e-3);
    let re: f64 = formula!(rho * v * d / mu);
    close(re, 100_000.0, 1e-12);
    let re: Ratio = formula!(rho * v * d / mu);
    close(re.as_decimal_fractions(), 100_000.0, 1e-12);

    // Arc length from an angle in degrees.
    let arc: Length = formula!(Length::from_meters(2.0) * Angle::from_degrees(90.0));
    close(arc.as_meters(), core::f64::consts::PI, 1e-15);
}

#[test]
fn intermediate_values() {
    let (m, v) = (
        Mass::from_kilograms(2.0),
        Speed::from_meters_per_second(3.0),
    );
    let momentum_speed: Expr<_> = formula!(m * v * v);
    let e: Energy = formula!(0.5 * momentum_speed);
    assert_eq!(e.as_joules(), 9.0);

    // Values from fields and method calls.
    struct Body {
        mass: Mass,
        speed: Speed,
    }
    let body = Body { mass: m, speed: v };
    let e: Energy = formula!(0.5 * body.mass * body.speed.abs() * body.speed);
    assert_eq!(e.as_joules(), 9.0);
}

#[test]
fn without_the_macro() {
    let field = ElectricField::from_volts_per_meter(1e4);
    let force: Force = (ELEMENTARY_CHARGE * field).into();
    close(force.as_newtons(), 1.602_176_634e-15, 1e-15);

    let weight: Force = (Mass::from_kilograms(10.0) * STANDARD_GRAVITY).into();
    close(weight.as_newtons(), 98.0665, 1e-15);

    let g: Acceleration = STANDARD_GRAVITY.into();
    assert_eq!(g.as_meters_per_second_squared(), 9.806_65);
}

#[test]
fn own_constants() {
    const SOUND: Expr<Dims<1, 0, -1>> = Expr::new(343.0);
    let d: Length = formula!(SOUND * Duration::from_seconds(2.0));
    assert_eq!(d.as_meters(), 686.0);
}

#[test]
fn f32_storage() {
    let (m, v) = (
        Mass::<f32>::from_kilograms(2.0),
        Speed::<f32>::from_meters_per_second(3.0),
    );
    let e: Energy<f32> = formula!(m * v * v * 0.5);
    assert_eq!(e.as_joules(), 9.0);
}

custom_quantity! {
    /// The weight of paper per area.
    pub struct Grammage;
    dimensions { mass: 1, length: -2 }

    pub enum GrammageUnit {
        GramPerSquareMeter: "g/m²" (plural "GramsPerSquareMeter"),
    }
}

#[test]
fn custom_quantities() {
    let paper = Grammage::new(80.0, GrammageUnit::GramPerSquareMeter);
    let sheet = Area::from_square_meters(0.0625);
    let mass: Mass = formula!(paper * sheet * 500.0);
    close(mass.as_kilograms(), 2.5, 1e-12);

    let back: Grammage = formula!(mass / sheet / 500.0);
    close(
        back.convert_to(GrammageUnit::GramPerSquareMeter),
        80.0,
        1e-12,
    );
}

#[test]
fn powers() {
    let (m, v) = (
        Mass::from_kilograms(2.0),
        Speed::from_meters_per_second(3.0),
    );
    let e: Energy = formula!(0.5 * m * v.powi::<2>());
    assert_eq!(e.as_joules(), 9.0);

    let t = Duration::from_seconds(4.0);
    let f: Frequency = formula!(t.powi::<-1>());
    assert_eq!(f.as_hertz(), 0.25);

    // On a group, and outside the macro.
    let a: Area = formula!((Length::from_meters(1.0) + Length::from_meters(2.0)).powi::<2>());
    assert_eq!(a.as_square_meters(), 9.0);
    let r = Length::from_meters(2.0);
    let per_volume: Expr<Dims<-3>> = expr!(r).powi::<-3>();
    assert_eq!(per_volume.value(), 0.125);
    let one: f64 = formula!(r.powi::<0>());
    assert_eq!(one, 1.0);
}

#[test]
fn expressions_without_annotations() {
    let (m, v) = (
        Mass::from_kilograms(2.0),
        Speed::from_meters_per_second(3.0),
    );
    let m2 = expr!(m * m);
    assert_eq!(m2.dimensions(), Dimensions::new(0, 2, 0, 0, 0, 0, 0));
    assert_eq!(m2.value(), 4.0);

    let momentum = expr!(m * v);
    let impulse: Impulse = momentum.into();
    assert_eq!(impulse.as_newton_seconds(), 6.0);
    let e: Energy = formula!(0.5 * momentum * v);
    assert_eq!(e.as_joules(), 9.0);
}

#[test]
fn display() {
    let (m, v) = (
        Mass::from_kilograms(2.0),
        Speed::from_meters_per_second(3.0),
    );
    assert_eq!(expr!(m * m).to_string(), "4 kg²");
    assert_eq!(expr!(m * v).to_string(), "6 kg·m/s");
    assert_eq!(format!("{:.1}", expr!(0.5 * m * v * v)), "9.0 kg·m²/s²");
    assert_eq!(
        expr!(1.0 / Duration::from_seconds(4.0)).to_string(),
        "0.25 s⁻¹"
    );
    assert_eq!(
        expr!(Length::from_meters(2.0).powi::<-3>()).to_string(),
        "0.125 m⁻³"
    );
    assert_eq!(
        expr!(1.0 / (Mass::from_kilograms(1.0) * Duration::from_seconds(1.0))).to_string(),
        "1 kg⁻¹·s⁻¹"
    );
    assert_eq!(
        "0.25 s⁻¹".parse::<Frequency>(),
        Ok(Frequency::from_hertz(0.25))
    );
    assert_eq!(expr!(m / m).to_string(), "1");
    assert_eq!(
        format!("{:e}", GRAVITATIONAL_CONSTANT),
        "6.6743e-11 m³/(kg·s²)"
    );
    assert_eq!(
        format!("{:.3E}", BOLTZMANN_CONSTANT),
        "1.381E-23 kg·m²/(s²·K)"
    );
    assert_eq!(
        expr!(v.powi::<-12>()).dimensions(),
        Dimensions::new(-12, 0, 12, 0, 0, 0, 0)
    );
    assert!(expr!(v.powi::<-12>()).to_string().ends_with(" s¹²/m¹²"));

    // The text parses back as a quantity with the same dimensions.
    let text = expr!(0.5 * m * v * v).to_string();
    assert_eq!(text.parse::<Energy>(), Ok(Energy::from_joules(9.0)));
    let text = expr!(m / (Length::from_meters(2.0) * Duration::from_seconds(1.0))).to_string();
    assert_eq!(text, "1 kg/(m·s)");
    assert_eq!(
        text.parse::<DynamicViscosity>(),
        Ok(DynamicViscosity::from_newton_seconds_per_meter_squared(1.0))
    );
}
