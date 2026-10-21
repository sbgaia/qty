//! `qty!`: quantities written with a unit.

use qty::formula::{Dims, Expr};
use qty::*;

#[test]
fn gives_the_quantity_the_context_expects() {
    let g: Acceleration = qty!(9.81 m/s^2);
    assert_eq!(g, Acceleration::from_meters_per_second_squared(9.81));
    let d: Length = qty!(5 km);
    assert_eq!(d, Length::from_kilometers(5.0));
    let below: TemperatureDelta = qty!(-40 K);
    assert_eq!(below.as_kelvins(), -40.0);

    // Comparisons and arguments give the type too.
    assert!(d < qty!(5.1 km));
    assert_eq!(d + qty!(500 m), qty!(5.5 km));
    fn energy(power: Power, time: Duration) -> Energy {
        power * time
    }
    assert_eq!(energy(qty!(2 kW), qty!(3 h)), qty!(6 kWh));
}

#[test]
fn matches_the_from_methods() {
    assert_eq!(Length::from_miles(3.0), qty!(3 mi));
    assert_eq!(Length::from_feet(12.0), qty!(12 ft));
    assert_eq!(Length::from_inches(3.0), qty!(3 in));
    assert_eq!(Length::from_millimeters(2.5), qty!(2.5 mm));
    assert_eq!(Mass::from_kilograms(70.0), qty!(70 kg));
    assert_eq!(Mass::from_grams(250.0), qty!(250 g));
    assert_eq!(Duration::from_minutes(90.0), qty!(90 min));
    assert_eq!(Duration::from_seconds(1e-18), qty!(1 as));
    assert_eq!(Speed::from_kilometers_per_hour(100.0), qty!(100 km/h));
    assert_eq!(Pressure::from_bars(2.4), qty!(2.4 bar));
    assert_eq!(Power::from_kilowatts(1.5), qty!(1.5 kW));
    assert_eq!(Energy::from_joules(5.0), qty!(5 N m));
    assert_eq!(Angle::from_degrees(180.0), qty!(180 deg));
    assert_eq!(Frequency::from_hertz(50.0), qty!(50 Hz));
}

#[test]
#[cfg(all(feature = "mechanics", feature = "electrical", feature = "fluids"))]
fn matches_the_from_methods_in_groups() {
    assert_eq!(Torque::from_newton_meters(5.0), qty!(5 N*m));
    assert_eq!(ElectricResistance::from_kiloohms(10.0), qty!(10 kΩ));
    assert_eq!(ElectricResistance::from_kiloohms(10.0), qty!(10 kohm));
    // Within an ulp where the factor isn't exact.
    let rpm = RotationalSpeed::from_revolutions_per_minute(60.0);
    assert!(rpm.approx_eq_rel(qty!(60 rev/min), 1e-15));
    // A base unit that isn't coherent SI: grams per second.
    assert_eq!(MassFlow::from_kilograms_per_second(3.0), qty!(3 kg/s));
}

/// The unit as tokens and in a string give the same bits.
macro_rules! same_as_string {
    ($($Q:ty: $value:literal $text:literal = [$($unit:tt)+])+) => {$(
        let tokens: $Q = qty!($value $($unit)+);
        let string: $Q = qty!($value $text);
        assert_eq!(tokens.as_base().to_bits(), string.as_base().to_bits(), $text);
    )+};
}

#[test]
fn tokens_and_strings_give_the_same_value() {
    same_as_string! {
        Acceleration: 9.81 "m/s^2" = [m/s^2]
        Acceleration: 9.81 "m*s^-2" = [m*s^-2]
        Acceleration: 9.81 "km/h/s" = [km/h/s]
        Force: 3.0 "kg·m/s²" = [kg*m/s^2]
        Force: 3.0 "kg m s⁻²" = [kg m s^-2]
        Frequency: 120.0 "1/min" = [1/min]
        Frequency: 120.0 "min⁻¹" = [min^-1]
        Speed: 100.0 "km/h" = [km/h]
        Speed: 30.0 "mi/h" = [mi/h]
        Energy: 3.0 "kW·h" = [kW*h]
        Energy: 3.0 "kW h" = [kW h]
        Pressure: 2.0 "kN/m²" = [kN/m^2]
        Duration: 250.0 "µs" = [us]
        Duration: 250.0 "μs" = [μs]
        Area: 3.0 "ha" = [ha]
        Area: 3.0 "km²" = [(km)^2]
        Volume: 2.0 "L" = [L]
        Volume: 2.0 "dm³" = [dm^3]
        Length: 1.0 "Å" = [Å]
    }
}

#[test]
#[cfg(feature = "thermal")]
fn tokens_and_strings_give_the_same_value_in_groups() {
    same_as_string! {
        HeatTransferCoefficient: 25.0 "W/(m²·K)" = [W/(m^2*K)]
        HeatTransferCoefficient: 25.0 "W/m^2/K" = [W/m^2/K]
    }
}

#[test]
fn strings_take_any_unit_expression() {
    let rise: TemperatureDelta = qty!(9 "°F");
    assert_eq!(rise, TemperatureDelta::from_kelvins(5.0));
    let angle: Angle = qty!(90 "°");
    assert_eq!(angle, Angle::from_degrees(90.0));
    let area: Area = qty!(12.5 "m²");
    assert_eq!(area.as_square_meters(), 12.5);
}

#[test]
fn other_storage_types() {
    let d: Length<f32> = qty!(1.5 km);
    assert_eq!(d, Length::<f32>::from_meters(1500.0));
    let e: Expr<Dims<1, 0, -2>> = qty!(9.81 m/s^2);
    assert_eq!(e.value(), 9.81);
    let e: Expr<Dims<1>, f32> = qty!(1 mi);
    assert_eq!(e.value(), 1609.344);
}

#[test]
fn in_formulas() {
    let t = Duration::from_seconds(2.0);
    let fall: Length = formula!(0.5 * qty!(9.81 m/s^2) * t * t);
    assert_eq!(fall.as_meters(), 19.62);
    let weight: Force = formula!(qty!(70 kg) * qty!(9.81 m/s^2));
    assert_eq!(weight.as_newtons(), 70.0 * 9.81);
    let side: Length = formula!(qty!(16 "m²").sqrt());
    assert_eq!(side.as_meters(), 4.0);
    let m = Mass::from_kilograms(2.0);
    let e = expr!(m * qty!(3 m/s).powi::<2>());
    assert_eq!(e.to_string(), "18 kg·m²/s²");
}

/// The value is computed in `f64`, then converted to the storage type.
#[cfg(feature = "rust_decimal")]
#[test]
fn decimal_storage() {
    use rust_decimal::Decimal;

    let d: Length<Decimal> = qty!(1.5 km);
    assert_eq!(d, Length::from_kilometers(Decimal::new(15, 1)));
}
