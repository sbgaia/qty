//! Tests for the crate-wide behaviour: layout, storage types, operators,
//! formatting, parsing, and the relations between quantities whose base
//! units aren't coherent.

// Uses quantities from every feature group.
#![cfg(feature = "full")]

use std::collections::HashSet;
use std::mem::{align_of, size_of};

use qty::*;

// Layout

#[test]
fn quantities_have_the_layout_of_their_storage() {
    assert_eq!(size_of::<Length>(), size_of::<f64>());
    assert_eq!(align_of::<Length>(), align_of::<f64>());
    assert_eq!(size_of::<Length<f32>>(), size_of::<f32>());
    assert_eq!(size_of::<Length<u8>>(), 1);
    assert_eq!(size_of::<Option<Length<std::num::NonZeroU32>>>(), 4);
    assert_eq!(size_of::<[Speed; 16]>(), 16 * 8);
}

#[test]
fn zero_copy_slice_views() {
    let mut raw = [1.0, 2.0, 3.0];
    let typed = Length::from_base_slice(&raw);
    assert_eq!(typed[1], Length::from_meters(2.0));
    assert_eq!(typed.iter().sum::<Length>(), Length::from_meters(6.0));
    assert_eq!(Length::as_base_slice(typed).as_ptr(), raw.as_ptr());

    for l in Length::from_base_slice_mut(&mut raw) {
        *l *= 2.0;
    }
    assert_eq!(raw, [2.0, 4.0, 6.0]);

    let mut typed = [Mass::from_kilograms(1.0), Mass::from_grams(500.0)];
    Mass::as_base_slice_mut(&mut typed)[0] = 3.0;
    assert_eq!(typed[0], Mass::from_kilograms(3.0));
}

#[test]
fn const_construction() {
    const LIMIT: Speed = Speed::from_base(13.9);
    const LIMIT_VALUE: f64 = LIMIT.as_base();
    const SLICE: &[Length] = Length::from_base_slice(&[1.0, 2.0]);
    assert_eq!(LIMIT_VALUE, 13.9);
    assert_eq!(SLICE.len(), 2);
}

// Storage types

#[test]
fn f32_storage() {
    let l: Length<f32> = Length::from_feet(3.0);
    assert!(l.approx_eq_rel(Length::from_inches(36.0), 1e-6));
    let v: Speed<f32> = l / Duration::from_seconds(2.0);
    assert!((v.as_feet_per_second() - 1.5).abs() < 1e-6);
    assert_eq!((l * 2.0).as_base(), l.as_base() * 2.0);
}

#[test]
fn integer_storage_is_eq_ord_and_hash() {
    let a = Mass::<i64>::from_base(1_500);
    let b = Mass::<i64>::from_base(500);
    assert_eq!(a + b, Mass::from_base(2_000));
    assert_eq!(a - b, Mass::from_base(1_000));
    assert_eq!(3 * b, a);
    assert_eq!(a / b, 3);
    assert_eq!(a.max(b), a); // Ord::max
    let set: HashSet<Mass<i64>> = [a, b, a].into_iter().collect();
    assert_eq!(set.len(), 2);

    let force: Force<i64> = a * Acceleration::from_base(2);
    assert_eq!(force.as_base(), 3_000);
    assert_eq!(force / a, Acceleration::from_base(2));
}

#[test]
fn integer_storage_rounds_from_floats() {
    let r = |cm: f64, mode| Length::<i64>::round_from(Length::from_centimeters(cm), mode);
    assert_eq!(
        r(250.0, Rounding::NearestTiesEven),
        Ok(Length::from_base(2))
    );
    assert_eq!(
        r(250.0, Rounding::NearestTiesAway),
        Ok(Length::from_base(3))
    );
    assert_eq!(r(-120.0, Rounding::Floor), Ok(Length::from_base(-2)));
    assert_eq!(r(120.0, Rounding::Exact), Err(RoundError::Inexact));
    assert_eq!(r(f64::NAN, Rounding::Floor), Err(RoundError::NotFinite));
    assert_eq!(
        Length::<i8>::round_from(Length::from_meters(300.0), Rounding::Floor),
        Err(RoundError::Overflow)
    );
}

#[test]
fn lossless_cast() {
    let m: Mass<f64> = Mass::<i32>::from_base(3).cast();
    assert_eq!(m.as_grams(), 3_000.0);
    let l: Length<f64> = Length::<f32>::from_base(0.5).cast();
    assert_eq!(l.as_meters(), 0.5);
}

#[test]
fn non_copy_storage_supports_arithmetic() {
    // A numeric type that is not Copy.
    #[derive(Debug, Clone, PartialEq, Default)]
    struct Big(Box<i128>);
    impl std::ops::Mul for Big {
        type Output = Big;
        fn mul(self, rhs: Big) -> Big {
            Big(Box::new(*self.0 * *rhs.0))
        }
    }
    impl std::ops::Add for Big {
        type Output = Big;
        fn add(self, rhs: Big) -> Big {
            Big(Box::new(*self.0 + *rhs.0))
        }
    }
    let f = Mass::from_base(Big(Box::new(2))) * Acceleration::from_base(Big(Box::new(5)));
    let f: Force<Big> = f + Force::from_base(Big(Box::new(1)));
    assert_eq!(f.into_base(), Big(Box::new(11)));
}

// Operators

#[test]
fn assignment_operators_and_sum() {
    let mut total = Energy::default();
    for e in [Energy::from_joules(1.0), Energy::from_kilojoules(1.0)] {
        total += e;
    }
    total -= Energy::from_joules(1.0);
    total *= 2.0;
    total /= 4.0;
    assert_eq!(total, Energy::from_joules(500.0));

    let parts = [Duration::from_minutes(1.0), Duration::from_seconds(30.0)];
    assert_eq!(parts.iter().sum::<Duration>(), Duration::from_seconds(90.0));
    assert_eq!(
        parts.into_iter().sum::<Duration>(),
        Duration::from_seconds(90.0)
    );
}

#[test]
fn remainder_and_float_helpers() {
    let a = Angle::from_degrees(370.0) % Angle::from_degrees(360.0);
    assert!(a.approx_eq_rel(Angle::from_degrees(10.0), 1e-12));
    let n = Length::from_meters(-2.0);
    assert_eq!(n.abs(), Length::from_meters(2.0));
    assert_eq!(n.max(Length::from_meters(1.0)), Length::from_meters(1.0));
    assert_eq!(n.min(Length::from_meters(1.0)), n);
}

#[test]
fn reciprocal_and_dimensionless_relations() {
    let f: Frequency = 1.0 / Duration::from_milliseconds(10.0);
    assert!(f.approx_eq_rel(Frequency::from_hertz(100.0), 1e-12));
    let ratio: f64 = Length::from_meters(4.0) * ReciprocalLength::from_inverse_meters(0.25);
    assert_eq!(ratio, 1.0);
}

/// UnitsNet stores some quantities in units that are not coherent SI base
/// units (`ElectricApparentEnergy` in VAh, …). Relations between them must
/// include the conversion factor.
#[test]
fn relations_with_non_coherent_base_units() {
    let m: Mass = MassFlow::from_kilograms_per_second(2.0) * Duration::from_seconds(3.0);
    assert!(m.approx_eq_rel(Mass::from_kilograms(6.0), 1e-15));

    let q: MassFlow = Mass::from_kilograms(6.0) / Duration::from_seconds(3.0);
    assert!(q.approx_eq_rel(MassFlow::from_kilograms_per_second(2.0), 1e-15));

    let e: ElectricApparentEnergy =
        ElectricApparentPower::from_voltamperes(100.0) * Duration::from_hours(2.0);
    assert!(e.approx_eq_rel(ElectricApparentEnergy::from_voltampere_hours(200.0), 1e-15));

    let t: Duration = Mass::from_kilograms(6.0) / MassFlow::from_kilograms_per_second(2.0);
    assert!(t.approx_eq_rel(Duration::from_seconds(3.0), 1e-15));
}

// Conversions

#[test]
fn exact_factors_are_correctly_rounded() {
    // × or ÷ by an exactly representable constant: one correctly rounded op.
    assert_eq!(Length::from_meters(300.0).as_kilometers(), 0.3);
    assert_eq!(Length::from_meters(0.3).as_millimeters(), 300.0);
    assert_eq!(Length::from_millimeters(300.0).as_meters(), 0.3);
    assert_eq!(Duration::from_seconds(5400.0).as_hours(), 1.5);
    assert_eq!(Duration::from_minutes(0.1).as_seconds(), 6.0);
}

/// When neither the factor nor its inverse is exactly representable (inch =
/// 0.0254 m), no single-operation scheme round-trips every value; sharing one
/// rounded constant between both directions keeps the error within an ulp
/// and round-trips the large majority of values exactly.
#[test]
fn round_trips_are_within_one_ulp() {
    let within_ulp = |out: f64, v: f64| (out - v).abs() <= v.abs() * f64::EPSILON;
    for v in [0.1, 0.3, 1.0, 12.0, 72.0, 1234.5678] {
        assert!(within_ulp(Length::from_inches(v).as_inches(), v));
        assert!(within_ulp(Length::from_feet(v).as_feet(), v));
        assert!(within_ulp(Length::from_miles(v).as_miles(), v));
        assert!(within_ulp(Mass::from_pounds(v).as_pounds(), v));
    }
    assert_eq!(Length::from_inches(36.0).as_inches(), 36.0);
}

#[test]
fn temperatures_are_points_and_deltas_are_amounts() {
    let readings = [18.0, 21.0, 24.0].map(Temperature::from_degrees_celsius);
    // Averaging points: a reference point plus the mean of the deltas.
    let reference = readings[0];
    let total: TemperatureDelta = readings.iter().map(|&t| t - reference).sum();
    let mean = reference + total / readings.len() as f64;
    assert!(mean.approx_eq_rel(Temperature::from_degrees_celsius(21.0), 1e-12));

    // A change of 1 °C is a change of 1 K, and of 1.8 °F.
    let rise = Temperature::from_degrees_celsius(25.0) - Temperature::from_degrees_celsius(24.0);
    assert!((rise.as_kelvins() - 1.0).abs() < 1e-12);
    assert!((rise.as_degrees_fahrenheit() - 1.8).abs() < 1e-12);

    // Deltas feed the physics relations: heat = entropy × ΔT.
    let heat: Energy = rise * Entropy::from_joules_per_kelvin(4.0);
    assert!(heat.approx_eq_rel(Energy::from_joules(4.0), 1e-12));

    // The ratio of absolute temperatures still works, as in the Carnot
    // efficiency 1 − Tc/Th.
    let (cold, hot) = (
        Temperature::from_kelvins(300.0),
        Temperature::from_kelvins(600.0),
    );
    assert_eq!(1.0 - cold / hot, 0.5);
}

#[test]
fn temperature_is_affine() {
    let boiling = Temperature::from_degrees_celsius(100.0);
    assert!(boiling.approx_eq_rel(Temperature::from_degrees_fahrenheit(212.0), 1e-15));
    assert!(boiling.approx_eq_rel(Temperature::from_kelvins(373.15), 1e-15));
    assert!((boiling.as_degrees_fahrenheit() - 212.0).abs() < 1e-12);
}

// Formatting and parsing

#[test]
fn expressions_for_base_units_unitsnet_writes_differently() {
    // UnitsNet writes these base units as ∆K, Ns/m², J/kg·K, Cd/m², VA and
    // m²K/W, which aren't unit expressions. The generator works them out
    // from UnitsNet's data instead.
    assert_eq!("3 mK".parse(), Ok(TemperatureDelta::from_kelvins(0.003)));
    assert_eq!(
        "2 Pa*s".parse(),
        Ok(DynamicViscosity::from_newton_seconds_per_meter_squared(2.0))
    );
    assert_eq!(
        "2 N*s/m^2".parse(),
        Ok(DynamicViscosity::from_newton_seconds_per_meter_squared(2.0))
    );
    assert_eq!(
        "5 J/(kg*K)".parse(),
        Ok(SpecificEntropy::from_joules_per_kilogram_kelvin(5.0))
    );
    assert_eq!(
        "100 cd/m^2".parse(),
        Ok(Luminance::from_candelas_per_square_meter(100.0))
    );
    assert_eq!(
        "3 kV*A".parse(),
        Ok(ElectricApparentPower::from_kilovoltamperes(3.0))
    );
    assert_eq!(
        "2 m^2*K/W".parse(),
        Ok(ThermalInsulance::from_square_meter_kelvins_per_watt(2.0))
    );
    // Plain numbers and counts still don't take expressions.
    assert_eq!("5 m/m".parse::<Ratio>(), Err(ParseError::UnknownUnit));
    assert_eq!("5 m/m".parse::<Information>(), Err(ParseError::UnknownUnit));
    // UnitsNet gives standard volume flow the dimensions of a mass flow.
    assert_eq!(
        "3 kg/s".parse::<StandardVolumeFlow>(),
        Err(ParseError::UnknownUnit)
    );
}

#[test]
fn display_honours_format_flags() {
    assert_eq!(Length::from_meters(1.5).to_string(), "1.5 m");
    assert_eq!(format!("{:.2}", Length::from_meters(1.0)), "1.00 m");
    assert_eq!(format!("{:e}", Length::from_meters(1500.0)), "1.5e3 m");
    assert_eq!(format!("{}", Length::<i32>::from_base(7)), "7 m");
    assert_eq!(format!("[{:>4}]", LengthUnit::Kilometer), "[  km]");
}

#[test]
fn display_in_a_chosen_or_automatic_unit() {
    let d = Length::from_meters(1234.5);
    assert_eq!(d.in_unit(LengthUnit::Kilometer).to_string(), "1.2345 km");
    assert_eq!(format!("{:.1}", d.in_unit(LengthUnit::Kilometer)), "1.2 km");
    assert_eq!(
        format!("{:>8.2}", d.in_unit(LengthUnit::Mile)),
        "    0.77 mi"
    );
    assert_eq!(
        format!("{:e}", d.in_unit(LengthUnit::Millimeter)),
        "1.2345e6 mm"
    );

    let auto = |m: f64| format!("{:.3}", Length::from_meters(m).auto_unit());
    assert_eq!(auto(1234.5), "1.234 km");
    assert_eq!(auto(0.0025), "2.500 mm");
    assert_eq!(auto(-0.0025), "-2.500 mm");
    assert_eq!(auto(3e-7), "300.000 nm");
    assert_eq!(auto(1.0), "1.000 m");
    assert_eq!(auto(999.0), "999.000 m");
    assert_eq!(auto(0.0), "0.000 m");
    assert_eq!(auto(5e12), "5000.000 Gm"); // beyond the largest prefix
    assert_eq!(auto(f64::INFINITY), "inf m");
    // Prefixes of a base unit that is itself prefixed: mass is in kg, the
    // prefixes are on the gram.
    assert_eq!(format!("{}", Mass::from_grams(2.5).auto_unit()), "2.5 g");
    assert_eq!(
        format!("{}", Mass::from_kilograms(80.0).auto_unit()),
        "80 kg"
    );
    assert_eq!(
        format!("{}", Duration::from_seconds(0.004).auto_unit()),
        "4 ms"
    );
    // f32 storage.
    assert_eq!(
        format!("{}", Length::from_meters(1500.0_f32).auto_unit()),
        "1.5 km"
    );
    // Generic code can read abbreviations through `UnitEnum`.
    fn abbreviations<U: UnitEnum>() -> usize {
        U::ALL
            .iter()
            .filter(|u| !u.abbreviation().is_empty())
            .count()
    }
    assert_eq!(abbreviations::<LengthUnit>(), LengthUnit::ALL.len());
}

#[test]
fn parsing() {
    assert_eq!("1.5 km".parse::<Length>(), Ok(Length::from_kilometers(1.5)));
    assert_eq!("1.5km".parse::<Length>(), Ok(Length::from_kilometers(1.5)));
    assert_eq!(
        "  -2e3   m ".parse::<Length>(),
        Ok(Length::from_meters(-2000.0))
    );
    assert_eq!("+.5 mi".parse::<Length<f32>>(), Ok(Length::from_miles(0.5)));
    assert_eq!("5 furlongs".parse::<Length>(), Err(ParseError::UnknownUnit));
    assert_eq!("km".parse::<Length>(), Err(ParseError::InvalidNumber));
    assert_eq!("".parse::<Length>(), Err(ParseError::InvalidNumber));
    assert_eq!("km/h".parse::<SpeedUnit>(), Ok(SpeedUnit::KilometerPerHour));
    assert_eq!(ParseError::UnknownUnit.to_string(), "unknown unit");
}

#[test]
fn parsing_unit_names() {
    assert_eq!(
        "5 kilometers".parse::<Length>(),
        Ok(Length::from_kilometers(5.0))
    );
    assert_eq!(
        "5 Kilometres".parse::<Length>(),
        Ok(Length::from_kilometers(5.0))
    );
    assert_eq!("1 meter".parse::<Length>(), Ok(Length::from_meters(1.0)));
    assert_eq!(
        "2 square_meters".parse::<Area>(),
        Ok(Area::from_square_meters(2.0))
    );
    assert_eq!(
        "2 Square Metres".parse::<Area>(),
        Ok(Area::from_square_meters(2.0))
    );
    assert_eq!("3 LITRES".parse::<Volume>(), Ok(Volume::from_liters(3.0)));
    assert_eq!(
        "10 degrees celsius".parse::<Temperature>(),
        Ok(Temperature::from_degrees_celsius(10.0))
    );
    assert_eq!(
        LengthUnit::from_name("nautical-mile"),
        Some(LengthUnit::NauticalMile)
    );
    // Abbreviations stay exact: `mm` and `Mm` differ, and are matched first.
    assert_eq!("mm".parse::<LengthUnit>(), Ok(LengthUnit::Millimeter));
    assert_eq!("Mm".parse::<LengthUnit>(), Ok(LengthUnit::Megameter));
    assert_eq!("MM".parse::<LengthUnit>(), Err(ParseError::UnknownUnit));
    assert_eq!(LengthUnit::from_name("métre"), None); // not ASCII
}

#[test]
fn split_value_unit_handles_digits_in_units() {
    assert_eq!(split_value_unit("2 m3"), ("2", "m3"));
    assert_eq!(split_value_unit("2m3"), ("2", "m3"));
    assert_eq!(split_value_unit("1e-3 em"), ("1e-3", "em"));
    assert_eq!(split_value_unit("4 e"), ("4", "e"));
    assert_eq!(split_value_unit("4em"), ("4", "em"));
}

// Generic code

/// Code generic over any quantity, using only the `Quantity` trait.
fn total_in_base<Q: Quantity<Value = f64>>(items: &[Q]) -> f64 {
    items.iter().map(|q| q.as_base()).sum()
}

/// Code generic over the float type.
fn kinetic_energy<T: Float>(m: Mass<T>, v: Speed<T>) -> Energy<T> {
    // ½·m·v²: Mass × Speed × Speed has no single relation, so go through
    // SpecificEnergy (v² = J/kg).
    let v2: SpecificEnergy<T> = v * v;
    m * v2 * T::from_f64(0.5)
}

#[test]
fn generic_code() {
    assert_eq!(
        total_in_base(&[Length::from_meters(1.0), Length::from_kilometers(1.0)]),
        1001.0
    );
    assert_eq!(<Length as Quantity>::BASE_UNIT, LengthUnit::Meter);

    let e64 = kinetic_energy(
        Mass::from_kilograms(2.0),
        Speed::from_meters_per_second(3.0),
    );
    assert_eq!(e64, Energy::from_joules(9.0));
    let e32 = kinetic_energy(
        Mass::from_kilograms(2.0_f32),
        Speed::from_meters_per_second(3.0),
    );
    assert_eq!(e32, Energy::from_joules(9.0_f32));
}

#[test]
fn unit_enum_is_complete_and_round_trips() {
    for &unit in LengthUnit::ALL {
        let l = Length::new(2.0, unit);
        assert!((l.convert_to(unit) - 2.0).abs() < 1e-12, "{unit:?}");
        assert!(unit.all_abbreviations().contains(&unit.abbreviation()));
    }
}

#[test]
fn time_alias() {
    let t: Time = Time::from_minutes(2.0);
    assert_eq!(t, Duration::from_seconds(120.0));
}

#[cfg(feature = "serde")]
#[test]
fn serde_is_transparent() {
    let l = Length::from_kilometers(1.5);
    assert_eq!(serde_json::to_string(&l).unwrap(), "1500.0");
    assert_eq!(serde_json::from_str::<Length>("1500.0").unwrap(), l);
    assert_eq!(
        serde_json::to_string(&LengthUnit::Kilometer).unwrap(),
        "\"Kilometer\""
    );
}

// Ordering float quantities

mod ordering {
    use std::collections::{BTreeMap, HashMap, HashSet};

    use qty::*;

    #[test]
    fn float_keys_and_sorting() {
        // Sorting floats: total order.
        let mut v = [
            Length::from_meters(2.0),
            Length::from_inches(1.0),
            Length::from_meters(f64::NAN),
        ];
        v.sort_by(Length::total_cmp);
        assert_eq!(v[0], Length::from_inches(1.0));
        assert!(v[2].as_base().is_nan());

        // Float keys: Ordered gives Eq + Ord + Hash.
        let mut m = HashMap::new();
        m.insert(Ordered(Length::from_meters(0.0)), "zero");
        assert_eq!(m[&Ordered(Length::from_meters(-0.0))], "zero"); // -0.0 == 0.0
        m.insert(Ordered(Length::from_meters(f64::NAN)), "nan");
        assert_eq!(m[&Ordered(Length::from_meters(-f64::NAN))], "nan"); // all NaNs are one key

        let tree: BTreeMap<Ordered<Length>, &str> = [
            (Ordered(Length::from_kilometers(1.0)), "km"),
            (Ordered(Length::from_meters(1.0)), "m"),
        ]
        .into();
        assert_eq!(tree.values().copied().collect::<Vec<_>>(), ["m", "km"]);

        let set: HashSet<Ordered<Length<f32>>> = [1.0_f32, 1.0, 2.0]
            .map(|m| Ordered(Length::from_meters(m)))
            .into();
        assert_eq!(set.len(), 2);
    }
}

// Integers in a chosen unit, rounding

#[cfg(feature = "fixed")]
mod fixed {
    use std::collections::{BTreeSet, HashMap};

    use qty::unit::duration::{Microsecond, Millisecond, Second};
    use qty::unit::length::{Inch, Meter, Millimeter};
    use qty::*;

    type Millimeters = Fixed<Millimeter>;
    type Meters = Fixed<Meter>;

    #[test]
    fn layout_and_arithmetic_are_those_of_the_integer() {
        assert_eq!(std::mem::size_of::<Fixed<Millimeter>>(), 8);
        assert_eq!(std::mem::size_of::<Fixed<Millimeter, u16>>(), 2);

        let a = Fixed::<Millimeter>::new(1_250);
        let b = Fixed::<Millimeter>::new(300);
        assert_eq!(a + b, Millimeters::new(1_550));
        assert_eq!(a - b, Millimeters::new(950));
        assert_eq!(-a, Millimeters::new(-1_250));
        assert_eq!(a * 2, Millimeters::new(2_500));
        assert_eq!(2 * a, Millimeters::new(2_500));
        assert_eq!(a / 3, Millimeters::new(416)); // truncates, like integers
        assert_eq!(a / b, 4);
        assert_eq!(a % b, Millimeters::new(50));
        let mut c = a;
        c += b;
        c -= b;
        c *= 4;
        c /= 2;
        assert_eq!(c, Millimeters::new(2_500));
        assert_eq!(
            [a, b].iter().sum::<Fixed<Millimeter>>(),
            Millimeters::new(1_550)
        );
        assert_eq!(a.count(), 1_250);
    }

    #[test]
    fn mixing_units_is_allowed_exactly_where_it_is_lossless() {
        let mut total = Millimeters::new(250);
        total += Meters::new(2); // 1 m = 1000 mm: exact
        assert_eq!(total, Millimeters::new(2_250));
        assert_eq!(total - Meters::new(1), Millimeters::new(1_250));
        assert_eq!(
            total + Fixed::<unit::length::Kilometer>::new(1),
            Millimeters::new(1_002_250)
        );
        // Inches into millimetres is not exact (25.4), but into microinches it is.
        let x = Fixed::<unit::length::Microinch>::new(5) + Fixed::<Inch>::new(1);
        assert_eq!(x.count(), 1_000_005);
        // `Meters + Millimeters` does not compile: it would need rounding.
    }

    #[test]
    fn comparisons_across_units_are_exact() {
        assert_eq!(Meters::new(2), Millimeters::new(2_000));
        assert_ne!(Meters::new(2), Millimeters::new(2_001));
        assert!(Meters::new(1) < Millimeters::new(1_001));
        assert!(Millimeters::new(999) < Meters::new(1));
        assert!(Millimeters::new(-1_001) < Meters::new(-1));
        assert!(Meters::new(0) == Millimeters::new(0));
        // Not exactly representable either way (1 in = 25.4 mm), still exact.
        assert!(Fixed::<Inch>::new(5) == Millimeters::new(127));
        assert!(Fixed::<Inch>::new(1) > Millimeters::new(25));
        assert!(Fixed::<Inch>::new(1) < Millimeters::new(26));
        // Extreme ratios: no overflow (compared in 256 bits).
        let ly = Fixed::<unit::length::LightYear>::new(i64::MAX);
        let fm = Fixed::<unit::length::Femtometer>::new(i64::MAX);
        assert!(ly > fm);
        assert!(-ly < fm);
        assert_eq!(
            Fixed::<unit::length::LightYear, i128>::new(1),
            Fixed::<unit::length::Femtometer, i128>::new(9_460_730_472_580_000_000_000_000_000_000)
        );
    }

    #[test]
    fn integer_temperatures_in_celsius() {
        use qty::unit::temperature::{DegreeCelsius, DegreeFahrenheit, Kelvin, MillidegreeCelsius};
        use qty::unit::temperature_delta as delta;
        type MilliC = Fixed<MillidegreeCelsius, i32>;

        // A sensor reporting millidegrees Celsius.
        let reading = MilliC::new(21_375);
        assert_eq!(reading.to_f64(), Temperature::from_degrees_celsius(21.375));
        assert_eq!(reading.to_string(), "21375 m°C");

        // Exact conversions need no rounding mode.
        assert_eq!(
            Fixed::<DegreeCelsius, i32>::new(-40).to_unit::<MillidegreeCelsius>(),
            Ok(MilliC::new(-40_000))
        );
        assert_eq!(
            Fixed::<Kelvin, i32>::new(0).to_unit::<MillidegreeCelsius>(),
            Ok(MilliC::new(-273_150))
        );
        // Lossy ones take a mode.
        assert_eq!(
            reading.to_unit_rounded::<DegreeCelsius>(Rounding::NearestTiesEven),
            Ok(Fixed::new(21))
        );
        assert_eq!(
            reading.to_unit_rounded::<Kelvin>(Rounding::Floor),
            Ok(Fixed::new(294))
        );
        assert_eq!(
            Fixed::<DegreeCelsius, i32>::new(100)
                .to_unit_rounded::<DegreeFahrenheit>(Rounding::Exact),
            Ok(Fixed::new(212))
        );
        assert_eq!(
            Fixed::<DegreeCelsius, i32>::new(37)
                .to_unit_rounded::<DegreeFahrenheit>(Rounding::Exact),
            Err(RoundError::Inexact) // 98.6 °F
        );

        // Exact comparisons across scales: -40 °C is -40 °F.
        assert_eq!(
            Fixed::<DegreeCelsius>::new(-40),
            Fixed::<DegreeFahrenheit>::new(-40)
        );
        assert!(Fixed::<DegreeCelsius>::new(20) > Fixed::<Kelvin>::new(293));
        assert!(Fixed::<DegreeCelsius>::new(20) < Fixed::<Kelvin>::new(294));
        assert!(reading < Fixed::<DegreeCelsius, i32>::new(22));

        // Point arithmetic: differences are deltas; deltas move points.
        let earlier = MilliC::new(19_000);
        let rise: Fixed<delta::MillidegreeCelsius, i32> = reading - earlier;
        assert_eq!(rise.count(), 2_375);
        assert_eq!(earlier + rise, reading);
        // A delta in a coarser unit converts exactly: +1 Δ°C = +1000 m°C.
        assert_eq!(
            earlier + Fixed::<delta::DegreeCelsius, i32>::new(1),
            MilliC::new(20_000)
        );
        let mut t = earlier;
        t += Fixed::<delta::Kelvin, i32>::new(2);
        t -= Fixed::<delta::MillidegreeCelsius, i32>::new(500);
        assert_eq!(t, MilliC::new(20_500));
        assert_eq!(
            t.sub_delta(Fixed::<delta::DegreeCelsius, i32>::new(1)),
            MilliC::new(19_500)
        );
        // Differences across scales are exact too.
        let d: Fixed<delta::MillidegreeCelsius, i32> =
            reading - Fixed::<DegreeCelsius, i32>::new(20);
        assert_eq!(d.count(), 1_375);

        // From floats, with explicit rounding.
        assert_eq!(
            MilliC::round_from(
                Temperature::from_degrees_celsius(21.2346),
                Rounding::NearestTiesEven
            ),
            Ok(MilliC::new(21_235))
        );
    }

    #[test]
    fn integer_decibel_levels() {
        use qty::unit::level::Decibel;
        use qty::unit::power_ratio::{DecibelMilliwatt, DecibelWatt};
        let tx = Fixed::<DecibelMilliwatt>::new(20);
        let gain = Fixed::<Decibel>::new(6);
        assert_eq!(tx + gain, Fixed::<DecibelMilliwatt>::new(26));
        assert_eq!(
            Fixed::<DecibelMilliwatt>::new(30),
            Fixed::<DecibelWatt>::new(0)
        );
        assert_eq!(
            Fixed::<DecibelWatt>::new(1).to_unit::<DecibelMilliwatt>(),
            Ok(Fixed::new(31))
        );
        let margin: Fixed<Decibel> = tx - Fixed::<DecibelWatt>::new(-20);
        assert_eq!(margin.count(), 10);
    }

    #[test]
    fn keys_in_hash_and_tree_collections() {
        let mut stock: HashMap<Fixed<Millimeter>, u32> = HashMap::new();
        *stock.entry(Millimeters::new(600)).or_default() += 2;
        *stock.entry(Millimeters::new(600)).or_default() += 1;
        assert_eq!(stock[&Millimeters::new(600)], 3);

        let sorted: BTreeSet<_> = [300, 100, 200].map(Fixed::<Millimeter>::new).into();
        assert_eq!(sorted.first(), Some(&Millimeters::new(100)));
    }

    #[test]
    fn rounding_from_floats() {
        let r = |m: f64, mode| Fixed::<Millimeter>::round_from(Length::from_meters(m), mode);
        assert_eq!(r(0.0125, Rounding::NearestTiesEven), Ok(Fixed::new(12)));
        assert_eq!(r(0.0125, Rounding::NearestTiesAway), Ok(Fixed::new(13)));
        assert_eq!(r(0.0127, Rounding::Floor), Ok(Fixed::new(12)));
        assert_eq!(r(-0.0121, Rounding::Floor), Ok(Fixed::new(-13)));
        assert_eq!(r(0.0121, Rounding::Ceil), Ok(Fixed::new(13)));
        assert_eq!(r(0.012, Rounding::Exact), Ok(Fixed::new(12)));
        assert_eq!(r(0.0121, Rounding::Exact), Err(RoundError::Inexact));
        assert_eq!(r(f64::NAN, Rounding::Floor), Err(RoundError::NotFinite));
        assert_eq!(
            Fixed::<Millimeter, i16>::round_from(Length::from_meters(40.0), Rounding::Floor),
            Err(RoundError::Overflow)
        );
        // A float count in the unit itself: ties are seen exactly.
        assert_eq!(
            Fixed::<Millimeter>::round_from_count(2.5, Rounding::NearestTiesEven),
            Ok(Fixed::new(2))
        );
    }

    #[test]
    fn converting_between_units() {
        // Exact direction: no rounding argument, only overflow can fail.
        assert_eq!(
            Fixed::<Second>::new(3).to_unit::<Millisecond>(),
            Ok(Fixed::new(3_000))
        );
        assert_eq!(
            Fixed::<Second, i32>::new(3_000_000).to_unit::<Microsecond>(),
            Err(RoundError::Overflow)
        );
        // Lossy direction: explicit rounding.
        let t = Fixed::<Millisecond>::new(2_500);
        assert_eq!(
            t.to_unit_rounded::<Second>(Rounding::NearestTiesEven),
            Ok(Fixed::new(2))
        );
        assert_eq!(
            t.to_unit_rounded::<Second>(Rounding::Ceil),
            Ok(Fixed::new(3))
        );
        assert_eq!(
            t.to_unit_rounded::<Second>(Rounding::Exact),
            Err(RoundError::Inexact)
        );
        // Non-integer ratios (1 in = 25.4 mm) are exact rational arithmetic.
        assert_eq!(
            Fixed::<Millimeter>::new(127).to_unit_rounded::<Inch>(Rounding::Exact),
            Ok(Fixed::new(5))
        );
        assert_eq!(
            Fixed::<Inch>::new(1).to_unit_rounded::<Millimeter>(Rounding::Floor),
            Ok(Fixed::new(25))
        );
    }

    #[test]
    fn rounded_division_and_overflow_helpers() {
        let a = Fixed::<Millimeter>::new(10);
        assert_eq!(
            a.div_rounded(4, Rounding::NearestTiesEven),
            Ok(Fixed::new(2))
        );
        assert_eq!(
            a.div_rounded(4, Rounding::NearestTiesAway),
            Ok(Fixed::new(3))
        );
        assert_eq!(
            a.div_rounded(0, Rounding::Floor),
            Err(RoundError::DivisionByZero)
        );
        assert_eq!(a.ratio_rounded(Fixed::new(4), Rounding::Ceil), Ok(3));
        let max = Fixed::<Millimeter, u8>::new(250);
        assert_eq!(max.checked_add(Fixed::new(10)), None);
        assert_eq!(
            max.saturating_add(Fixed::new(10)),
            Fixed::<Millimeter, u8>::new(255)
        );
        assert_eq!(max.checked_mul(2), None);
    }

    #[test]
    fn bridges_to_floats_and_base_quantities() {
        let a = Fixed::<Millimeter>::new(1_250);
        assert_eq!(a.to_f64(), Length::from_meters(1.25));
        assert_eq!(a.to_f32(), Length::from_meters(1.25_f32));
        // Fixed<Meter, I> and Length<I> are the same thing.
        let l: Length<i64> = Fixed::<Meter>::new(3).into();
        assert_eq!(l, Length::from_base(3));
        let back: Fixed<Meter> = l.into();
        assert_eq!(back.to_unit::<Millimeter>(), Ok(Fixed::new(3_000)));
        // `to_f64` on any quantity, no unwrapping.
        assert_eq!(
            Length::<i64>::from_base(3).to_f64(),
            Length::from_meters(3.0)
        );
        assert_eq!(
            Length::from_meters(1.5_f32).to_f64(),
            Length::from_meters(1.5)
        );
    }

    #[test]
    fn display_and_parse() {
        let a = Fixed::<Millimeter>::new(1_550);
        assert_eq!(a.to_string(), "1550 mm");
        assert_eq!(format!("{a:>6}"), "  1550 mm");
        assert_eq!(format!("{a:?}"), "1550 mm");
        assert_eq!("1550 mm".parse(), Ok(a));
        assert_eq!("1550mm".parse(), Ok(a));
        assert_eq!(
            "1550 m".parse::<Fixed<Millimeter>>(),
            Err(ParseError::UnknownUnit)
        );
        assert_eq!(
            "15.5 mm".parse::<Fixed<Millimeter>>(),
            Err(ParseError::InvalidNumber)
        );
    }

    #[test]
    fn zero_copy_count_slices() {
        let raw = [1, 2, 3];
        let typed = Fixed::<Millisecond>::from_count_slice(&raw);
        assert_eq!(
            typed.iter().sum::<Fixed<Millisecond>>(),
            Fixed::<Millisecond>::new(6)
        );
        assert_eq!(Fixed::as_count_slice(typed), &raw);
    }
}

// Math

#[cfg(any(feature = "std", feature = "libm"))]
mod math {
    use qty::*;

    #[test]
    fn roots_follow_the_relations() {
        let side: Length = Area::from_square_meters(16.0).sqrt();
        assert_eq!(side, Length::from_meters(4.0));
        let edge: Length = Volume::from_liters(1.0).cbrt();
        assert!(edge.approx_eq_rel(Length::from_centimeters(10.0), 1e-15));
        // v = sqrt(2·E/m): specific energy (J/kg) is a speed squared.
        let v: Speed = (Energy::from_joules(100.0) * 2.0 / Mass::from_kilograms(2.0)).sqrt();
        assert!(v.approx_eq_rel(Speed::from_meters_per_second(10.0), 1e-15));
        // f32 storage.
        assert_eq!(
            Area::from_square_meters(9.0_f32).sqrt(),
            Length::from_meters(3.0_f32)
        );
    }

    #[test]
    fn hypot_and_trigonometry() {
        let (east, north) = (Length::from_meters(3.0), Length::from_meters(4.0));
        assert_eq!(east.hypot(north), Length::from_meters(5.0));
        let heading = Angle::atan2(north, east);
        assert!((heading.as_degrees() - 53.13010235415598).abs() < 1e-12);
        assert!((heading.cos() - 0.6).abs() < 1e-15);
        let (s, c) = Angle::from_degrees(90.0).sin_cos();
        assert!((s - 1.0).abs() < 1e-15 && c.abs() < 1e-15);
        assert!(Angle::acos(0.0).approx_eq_rel(Angle::from_degrees(90.0), 1e-15));
        assert!(Angle::atan(1.0).approx_eq_rel(Angle::from_degrees(45.0), 1e-15));
        // Velocity components from a speed and a heading.
        let v = Speed::from_meters_per_second(10.0);
        let vx = v * Angle::from_degrees(60.0).cos();
        assert!(vx.approx_eq_rel(Speed::from_meters_per_second(5.0), 1e-12));
    }
}

// Optional integrations

#[cfg(feature = "approx")]
#[test]
fn approx_assertions() {
    use approx::{assert_abs_diff_eq, assert_relative_eq, assert_ulps_eq};
    let a = Length::from_feet(3.0);
    let b = Length::from_yards(1.0);
    assert_relative_eq!(a, b, max_relative = 1e-15);
    assert_abs_diff_eq!(a, b, epsilon = 1e-12);
    assert_ulps_eq!(a, b, max_ulps = 4);
}

#[cfg(feature = "bytemuck")]
#[test]
fn bytemuck_casts() {
    use qty::unit::length::Millimeter;
    let lengths = [Length::from_meters(1.0), Length::from_meters(2.5)];
    let bytes: &[u8] = bytemuck::cast_slice(&lengths);
    assert_eq!(bytes.len(), 16);
    let back: &[Length] = bytemuck::cast_slice(bytes);
    assert_eq!(back, lengths);
    let raw: &[f64] = bytemuck::TransparentWrapper::peel_slice(&lengths[..]);
    assert_eq!(raw, [1.0, 2.5]);
    let counts: &[Fixed<Millimeter, u32>] = bytemuck::cast_slice(&[7u32, 8]);
    assert_eq!(counts[1].count(), 8);
}

#[cfg(feature = "defmt")]
#[test]
fn defmt_format_is_implemented() {
    // Formatting itself needs a defmt logger (firmware); check the bounds.
    fn is_format<T: defmt::Format>() {}
    is_format::<Length>();
    is_format::<LengthUnit>();
    is_format::<Fixed<qty::unit::length::Millimeter>>();
}

// Uncertainty propagation

mod uncertain {
    use qty::*;

    fn u(value: f64, sigma: f64) -> Uncertain {
        Uncertain::new(value, sigma)
    }

    fn close(a: Uncertain, value: f64, sigma: f64) {
        assert!(
            (a.value() - value).abs() <= 1e-12 * value.abs().max(1.0),
            "{a} vs value {value}"
        );
        assert!(
            (a.sigma() - sigma).abs() <= 1e-12 * sigma.abs().max(1.0),
            "{a} vs sigma {sigma}"
        );
    }

    #[test]
    fn propagation_rules() {
        // Sums and differences: absolute uncertainties add in quadrature.
        close(u(10.0, 3.0) + u(5.0, 4.0), 15.0, 5.0);
        close(u(10.0, 3.0) - u(5.0, 4.0), 5.0, 5.0);
        // Products and quotients: relative uncertainties add in quadrature.
        close(
            u(10.0, 0.3) * u(5.0, 0.2),
            50.0,
            50.0 * (0.03f64.hypot(0.04)),
        );
        close(u(10.0, 0.3) / u(5.0, 0.2), 2.0, 2.0 * (0.03f64.hypot(0.04)));
        // Exact constants scale σ.
        close(u(10.0, 0.3) * Uncertain::from(-2.0), -20.0, 0.6);
        close(-u(10.0, 0.3), -10.0, 0.3);
        // Functions: σ·|f′(x)|.
        close(u(16.0, 0.8).sqrt(), 4.0, 0.1);
        close(u(27.0, 2.7).cbrt(), 3.0, 0.1);
        close(u(0.0, 0.01).sin(), 0.0, 0.01);
        close(u(0.0, 0.01).cos(), 1.0, 0.0);
        close(u(3.0, 0.3).hypot(u(4.0, 0.4)), 5.0, 0.9f64.hypot(1.6) / 5.0);
        close(
            u(1.0, 0.1).atan2(u(1.0, 0.0)),
            core::f64::consts::FRAC_PI_4,
            0.05,
        );
        // Sums of many independent values.
        let total: Uncertain = [u(1.0, 0.3), u(2.0, 0.4)].into_iter().sum();
        close(total, 3.0, 0.5);
        // The sign of sigma is ignored; intervals.
        assert_eq!(u(1.0, -0.5).sigma(), 0.5);
        assert_eq!(u(10.0, 0.5).interval(2.0), (9.0, 11.0));
        assert_eq!(u(10.0, 0.5).relative(), 0.05);
    }

    #[test]
    fn quantities_carry_uncertainty() {
        // Conversions scale σ by the unit factor.
        let d = Length::from_kilometers(u(1.5, 0.002));
        close(d.as_meters(), 1500.0, 2.0);
        close(
            Length::from_meters(u(1.0, 0.01)).as_centimeters(),
            100.0,
            1.0,
        );
        // Affine conversions shift without changing σ.
        let t = Temperature::from_degrees_celsius(u(20.0, 0.1));
        close(t.as_kelvins(), 293.15, 0.1);
        // Relations, roots and trigonometry.
        let v: Speed<Uncertain> =
            Length::from_meters(u(100.0, 1.0)) / Duration::from_seconds(u(10.0, 0.1));
        close(v.as_base(), 10.0, 10.0 * 0.01f64.hypot(0.01));
        let side: Length<Uncertain> = Area::from_square_meters(u(16.0, 0.8)).sqrt();
        close(side.as_base(), 4.0, 0.1);
        let heading = Angle::atan2(
            Length::from_meters(u(1.0, 0.1)),
            Length::from_meters(u(1.0, 0.0)),
        );
        close(heading.as_base(), core::f64::consts::FRAC_PI_4, 0.05);
        // Sums, comparisons, formatting.
        let laps = [
            Length::from_meters(u(400.0, 0.3)),
            Length::from_meters(u(400.0, 0.4)),
        ];
        close(
            laps.iter().copied().sum::<Length<Uncertain>>().as_base(),
            800.0,
            0.5,
        );
        assert!(laps[0] < Length::from_meters(u(401.0, 5.0)));
        assert_eq!(laps[0].partial_cmp(&laps[1]), None); // equal values, different σ
        assert_eq!(
            format!("{:.2}", Length::from_meters(u(1.5, 0.25))),
            "1.50 ± 0.25 m"
        );
        assert_eq!(
            Length::from_meters(u(1.5, 0.25)).to_f64(),
            Length::from_meters(1.5)
        );
        let mut x = Length::from_meters(u(1.0, 0.3));
        x += Length::from_meters(u(1.0, 0.4));
        close(x.as_base(), 2.0, 0.5);
    }

    #[test]
    fn same_value_twice_is_not_independent() {
        // Documented limitation: operands are assumed independent.
        let x = u(10.0, 1.0);
        close(x - x, 0.0, 2f64.sqrt());
    }
}

// Vectors

mod vectors {
    use qty::*;

    fn m(x: f64, y: f64, z: f64) -> Vector3<Length> {
        Vector3::from_base([x, y, z])
    }

    #[test]
    fn layout_and_conversions() {
        assert_eq!(core::mem::size_of::<Vector3<Length>>(), 24);
        assert_eq!(core::mem::size_of::<Vector2<Length<f32>>>(), 8);
        let v = m(1.0, 2.0, 3.0);
        assert_eq!(v.as_base(), [1.0, 2.0, 3.0]);
        assert_eq!(<[Length; 3]>::from(v), [v.x, v.y, v.z]);
        assert_eq!(Vector3::from([v.x, v.y, v.z]), v);
        assert_eq!(
            v.map(|c| c.as_millimeters()),
            Vector3::new(1000.0, 2000.0, 3000.0)
        );
        assert_eq!(v.to_array(), [v.x, v.y, v.z]);
    }

    #[test]
    fn arithmetic_follows_the_component_types() {
        let a = m(1.0, 2.0, 3.0);
        let b = m(0.5, 0.5, 0.5);
        assert_eq!(a + b, m(1.5, 2.5, 3.5));
        assert_eq!(a - b, m(0.5, 1.5, 2.5));
        assert_eq!(-a, m(-1.0, -2.0, -3.0));
        assert_eq!(a * 2.0, m(2.0, 4.0, 6.0));
        assert_eq!(2.0 * a, m(2.0, 4.0, 6.0));
        assert_eq!(a / 2.0, m(0.5, 1.0, 1.5));
        let mut c = a;
        c += b;
        c -= b;
        c *= 3.0;
        c /= 3.0;
        assert_eq!(c, a);
        // Physics: displacement / time = velocity; mass · acceleration = force.
        let v: Vector3<Speed> = a / Duration::from_seconds(2.0);
        assert_eq!(v, Vector3::from_base([0.5, 1.0, 1.5]));
        let back: Vector3<Length> = v * Duration::from_seconds(2.0);
        assert_eq!(back, a);
        let g = Vector3::<Acceleration>::from_base([0.0, 0.0, -9.81]);
        let w: Vector3<Force> = Mass::from_kilograms(2.0) * g;
        assert_eq!(w, Vector3::from_base([0.0, 0.0, -19.62]));
        let w32: Vector3<Force<f32>> =
            Mass::from_kilograms(2.0f32) * Vector3::<Acceleration<f32>>::from_base([1.0, 0.0, 0.0]);
        assert_eq!(w32.x, Force::from_newtons(2.0f32));
        // Dot and cross products.
        let area: Area = a.dot(b);
        assert_eq!(area, Area::from_square_meters(3.0));
        assert_eq!(a.norm_squared(), Area::from_square_meters(14.0));
        let x = m(1.0, 0.0, 0.0);
        let y = m(0.0, 1.0, 0.0);
        let z: Vector3<Area> = x.cross(y);
        assert_eq!(z, Vector3::from_base([0.0, 0.0, 1.0]));
        let power: Power = Vector3::<Force>::from_base([1.0, 2.0, 0.0]).dot(v);
        assert_eq!(power, Power::from_watts(2.5));
        // Sums.
        let total: Vector3<Length> = [a, b].iter().sum();
        assert_eq!(total, a + b);
        assert_eq!([a, b].into_iter().sum::<Vector3<Length>>(), a + b);
    }

    #[test]
    fn norms_and_angles() {
        assert_eq!(m(2.0, 3.0, 6.0).norm(), Length::from_meters(7.0));
        // `hypot`: no overflow for huge components.
        assert!(m(3e200, 4e200, 0.0)
            .norm()
            .approx_eq_rel(Length::from_meters(5e200), 1e-15));
        assert_eq!(m(0.0, 3.0, 4.0).direction(), Vector3::new(0.0, 0.6, 0.8));
        let right = m(1.0, 0.0, 0.0).angle_between(m(0.0, 0.0, 2.0));
        assert!((right.as_degrees() - 90.0).abs() < 1e-12);
        let opposite = m(1.0, 1.0, 0.0).angle_between(m(-2.0, -2.0, 0.0));
        assert!((opposite.as_degrees() - 180.0).abs() < 1e-12);
        let tiny = m(1.0, 0.0, 0.0).angle_between(m(1.0, 1e-9, 0.0));
        assert!((tiny.as_base() - 1e-9).abs() < 1e-20); // atan2: accurate for small angles
                                                        // 2D: polar coordinates and perp-dot.
        let v = Vector2::from_polar(Length::from_meters(2.0), Angle::from_degrees(30.0));
        assert!((v.angle().as_degrees() - 30.0).abs() < 1e-12);
        assert!(v.norm().approx_eq_rel(Length::from_meters(2.0), 1e-15));
        let a = Vector2::<Length>::from_base([1.0, 0.0]);
        let b = Vector2::<Length>::from_base([0.0, 1.0]);
        assert_eq!(a.perp_dot(b), Area::from_square_meters(1.0));
        assert!((a.angle_between(b).as_degrees() - 90.0).abs() < 1e-12);
        // Uncertain components are independent under `norm`.
        let u = Vector2::<Length<Uncertain>>::from_base([
            Uncertain::new(3.0, 0.3),
            Uncertain::new(4.0, 0.4),
        ]);
        let n = u.norm().as_base();
        assert_eq!(n.value(), 5.0);
        assert!((n.sigma() - 0.9f64.hypot(1.6) / 5.0).abs() < 1e-15);
    }

    #[test]
    fn display() {
        assert_eq!(m(1.0, 2.5, -3.0).to_string(), "(1, 2.5, -3) m");
        assert_eq!(
            format!("{:.2}", Vector2::<Speed>::from_base([1.0, 0.5])),
            "(1.00, 0.50) m/s"
        );
    }

    #[cfg(feature = "bytemuck")]
    #[test]
    fn bytemuck_casts() {
        let points = [m(1.0, 2.0, 3.0), m(4.0, 5.0, 6.0)];
        let raw: &[f64] = bytemuck::cast_slice(&points);
        assert_eq!(raw, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }
}

// std::time::Duration interop

mod std_time {
    #[cfg(feature = "fixed")]
    use qty::unit::duration::Nanosecond;
    use qty::*;
    use std::time::Duration as StdDuration;

    #[test]
    fn floats() {
        assert_eq!(
            Duration::from(StdDuration::from_millis(250)),
            Duration::from_seconds(0.25)
        );
        assert_eq!(
            Duration::<f32>::from(StdDuration::from_secs(3)),
            Duration::from_seconds(3.0f32)
        );
        assert_eq!(
            StdDuration::try_from(Duration::from_hours(1.0)),
            Ok(StdDuration::from_secs(3600))
        );
        assert_eq!(
            StdDuration::try_from(Duration::from_nanoseconds(1.4)),
            Ok(StdDuration::from_nanos(1))
        );
        assert!(StdDuration::try_from(Duration::from_seconds(-0.5)).is_err());
        assert!(StdDuration::try_from(Duration::from_seconds(f64::NAN)).is_err());
        assert!(StdDuration::try_from(Duration::from_seconds(1e30)).is_err());
        assert!(StdDuration::try_from(Duration::from_seconds(2.0f32)).is_ok());
    }

    #[cfg(feature = "fixed")]
    #[test]
    fn integer_nanoseconds_are_exact() {
        let d = StdDuration::new(1_234_567, 890_123_456);
        let ns: Fixed<Nanosecond, i128> = d.try_into().unwrap();
        assert_eq!(ns.count(), 1_234_567_890_123_456);
        assert_eq!(StdDuration::try_from(ns), Ok(d));
        // Extremes.
        let max: Fixed<Nanosecond, i128> = StdDuration::MAX.try_into().unwrap();
        assert_eq!(StdDuration::try_from(max), Ok(StdDuration::MAX));
        assert_eq!(
            StdDuration::try_from(max + Fixed::<Nanosecond, i128>::new(1)),
            Err(RoundError::Overflow)
        );
        assert_eq!(
            Fixed::<Nanosecond, i64>::try_from(StdDuration::MAX),
            Err(RoundError::Overflow)
        );
        assert_eq!(
            Fixed::<Nanosecond, u8>::try_from(StdDuration::from_nanos(255)).map(|f| f.count()),
            Ok(255)
        );
        assert_eq!(
            StdDuration::try_from(Fixed::<Nanosecond>::new(-1)),
            Err(RoundError::Overflow)
        );
        // Other units convert through `to_unit`, exactly.
        let ms = Fixed::<unit::duration::Millisecond>::new(1500);
        assert_eq!(
            StdDuration::try_from(ms.to_unit::<Nanosecond>().unwrap()),
            Ok(StdDuration::from_millis(1500))
        );
    }
}

// serde as text

#[cfg(feature = "serde")]
mod serde_text {
    use qty::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Wrap<Q: core::fmt::Display + core::str::FromStr<Err = ParseError>>(
        #[serde(with = "qty::serde_str")] Q,
    );

    fn round_trip<Q>(q: Q) -> String
    where
        Q: core::fmt::Display
            + core::str::FromStr<Err = ParseError>
            + PartialEq
            + core::fmt::Debug
            + Copy,
    {
        let json = serde_json::to_string(&Wrap(q)).unwrap();
        let back: Wrap<Q> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.0, q, "{json}");
        json
    }

    #[test]
    fn round_trips_are_exact() {
        for v in [0.1, -2.5e-300, 1.0 / 3.0, f64::MAX, 6.02214076e23] {
            let json = serde_json::to_string(&Wrap(Length::from_meters(v))).unwrap();
            let back: Wrap<Length> = serde_json::from_str(&json).unwrap();
            assert_eq!(back.0.as_base().to_bits(), v.to_bits(), "{json}");
        }
        let json = serde_json::to_string(&Wrap(Length::from_meters(0.1f32))).unwrap();
        assert_eq!(json, r#""0.1 m""#);
        assert_eq!(
            serde_json::from_str::<Wrap<Length<f32>>>(&json).unwrap().0,
            Length::from_meters(0.1f32)
        );
        assert_eq!(
            round_trip(Temperature::from_kelvins(293.15)),
            r#""293.15 K""#
        );
        assert_eq!(
            round_trip(Fixed::<unit::length::Millimeter, i32>::new(-12)),
            r#""-12 mm""#
        );
        assert_eq!(round_trip(LengthUnit::Kilometer), r#""km""#);
    }

    #[test]
    fn reading_accepts_any_unit_and_names() {
        let read = |s: &str| serde_json::from_str::<Wrap<Length>>(s).map(|w| w.0);
        assert_eq!(read(r#""1.5 km""#).unwrap(), Length::from_meters(1500.0));
        assert_eq!(
            read(r#""2 Kilometres""#).unwrap(),
            Length::from_meters(2000.0)
        );
        assert!(read(r#""2 kg""#)
            .unwrap_err()
            .to_string()
            .contains("unknown unit"));
        assert!(read(r#""abc m""#)
            .unwrap_err()
            .to_string()
            .contains("invalid number"));
        // Bare numbers are rejected: the unit must be explicit.
        assert!(read("1.5")
            .unwrap_err()
            .to_string()
            .contains("a number with a unit"));
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Optional {
        #[serde(
            with = "qty::serde_str::option",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        limit: Option<Speed>,
    }

    #[test]
    fn optional_fields() {
        let some: Optional = serde_json::from_str(r#"{"limit":"36 km/h"}"#).unwrap();
        assert_eq!(some.limit, Some(Speed::from_meters_per_second(10.0)));
        assert_eq!(
            serde_json::to_string(&some).unwrap(),
            r#"{"limit":"10 m/s"}"#
        );
        let none: Optional = serde_json::from_str("{}").unwrap();
        assert_eq!(none.limit, None);
        assert_eq!(
            serde_json::from_str::<Optional>(r#"{"limit":null}"#)
                .unwrap()
                .limit,
            None
        );
        assert_eq!(serde_json::to_string(&none).unwrap(), "{}");
    }
}

// Angle wrapping, clamp, lerp

mod wrapping {
    use qty::*;

    #[test]
    fn angles_wrap_into_one_turn() {
        use core::f64::consts::{PI, TAU};
        for (input, unsigned, signed) in [
            (0.0, 0.0, 0.0),
            (PI, PI, PI),
            (-PI, PI, PI),
            (TAU, 0.0, 0.0),
            (-TAU, 0.0, 0.0),
            (3.0 * PI, PI, PI),
            (-0.5, TAU - 0.5, -0.5),
            (7.5, 7.5 - TAU, 7.5 - TAU),
        ] {
            let a = Angle::from_radians(input);
            assert!(
                (a.wrap_unsigned().as_base() - unsigned).abs() < 1e-12,
                "{input}"
            );
            assert!(
                (a.wrap_signed().as_base() - signed).abs() < 1e-12,
                "{input}"
            );
        }
        // Ranges hold even where rounding would push a value onto the bound.
        for x in [
            -1e-300,
            -f64::EPSILON,
            -1e-17,
            1e300,
            -1e300,
            2.0 * TAU,
            -3.0 * TAU,
        ] {
            let u = Angle::from_radians(x).wrap_unsigned().as_base();
            assert!((0.0..TAU).contains(&u), "{x} -> {u}");
            let s = Angle::from_radians(x).wrap_signed().as_base();
            assert!(s > -PI && s <= PI, "{x} -> {s}");
        }
        assert!(Angle::from_radians(f64::INFINITY)
            .wrap_signed()
            .as_base()
            .is_nan());
        let a = Angle::from_degrees(-30.0f32).wrap_unsigned();
        assert!((a.as_degrees() - 330.0).abs() < 1e-4);
    }

    #[test]
    fn clamp_and_lerp() {
        let (lo, hi) = (Length::from_meters(1.0), Length::from_meters(2.0));
        assert_eq!(Length::from_meters(0.5).clamp(lo, hi), lo);
        assert_eq!(
            Length::from_meters(1.5).clamp(lo, hi),
            Length::from_meters(1.5)
        );
        assert_eq!(Length::from_meters(9.0).clamp(lo, hi), hi);
        assert!(Length::from_meters(f64::NAN)
            .clamp(lo, hi)
            .as_base()
            .is_nan());
        assert_eq!(lo.lerp(hi, 0.25), Length::from_meters(1.25));
        assert_eq!(lo.lerp(hi, 2.0), Length::from_meters(3.0));
        let mid = Temperature::from_degrees_celsius(10.0)
            .lerp(Temperature::from_degrees_celsius(20.0), 0.5);
        assert!(mid.approx_eq_rel(Temperature::from_degrees_celsius(15.0), 1e-12));
    }

    #[test]
    #[should_panic(expected = "min > max")]
    fn clamp_rejects_inverted_bounds() {
        let _ = Length::from_meters(1.0).clamp(Length::from_meters(2.0), Length::from_meters(1.0));
    }
}

// Exact decimal display of Fixed

#[cfg(feature = "fixed")]
#[test]
fn decimal_display_flags() {
    use qty::unit::length::{Meter, Millimeter};
    let d = Fixed::<Millimeter>::new(1230).display_in::<Meter>();
    assert_eq!(format!("[{d:<9}]"), "[1.230 m  ]");
    assert_eq!(format!("[{d:^10}]"), "[ 1.230 m  ]");
    assert_eq!(format!("[{d:*>9}]"), "[**1.230 m]");
    assert_eq!(format!("{d:+}"), "+1.230 m");
    assert_eq!(format!("{d:.5}"), "1.23000 m");
    assert_eq!(format!("{d:.0}"), "1 m");
    assert_eq!(
        format!(
            "{:.0}",
            Fixed::<Millimeter>::new(-400).display_in::<Meter>()
        ),
        "0 m"
    );
    assert_eq!(
        format!(
            "{:.0}",
            Fixed::<Millimeter>::new(2500).display_in::<Meter>()
        ),
        "2 m"
    ); // ties to even
    assert_eq!(format!("{:?}", d), "1.230 m");
    let max = Fixed::<Millimeter, u128>::new(u128::MAX).display_in::<Meter>();
    assert_eq!(
        max.to_string(),
        "340282366920938463463374607431768211.455 m"
    );
    let min = Fixed::<Millimeter, i128>::new(i128::MIN).display_in::<Meter>();
    assert_eq!(
        min.to_string(),
        "-170141183460469231731687303715884105.728 m"
    );
}

// Runtime quantities

#[cfg(feature = "dynamic")]
mod runtime {
    use qty::*;

    #[test]
    fn expressions_can_be_ambiguous() {
        assert_eq!(
            "2 Pa*s".parse::<AnyQuantity>(),
            Ok(AnyQuantity::DynamicViscosity(
                DynamicViscosity::from_newton_seconds_per_meter_squared(2.0)
            ))
        );
        // V·A is power, apparent power and reactive power.
        assert_eq!("3 kV*A".parse::<AnyQuantity>(), Err(ParseError::Ambiguous));
        // A listed unit wins over expressions: K is the kelvin, a temperature.
        assert_eq!(
            "3 K".parse::<AnyQuantity>(),
            Ok(AnyQuantity::Temperature(Temperature::from_kelvins(3.0)))
        );
    }

    #[test]
    fn every_unit_round_trips() {
        let mut units = 0;
        for kind in QuantityKind::all() {
            let info = kind.info();
            assert_eq!(kind.to_string(), info.name);
            assert_eq!(info.name.to_lowercase().parse::<QuantityKind>(), Ok(kind));
            assert_eq!(kind.base_unit().info(), info.base_unit);
            for unit_info in info.units {
                let unit = AnyUnit::parse_in(kind, unit_info.abbreviation)
                    .or_else(|_| AnyUnit::parse_in(kind, unit_info.singular))
                    .unwrap();
                assert_eq!(unit.kind(), kind);
                let q = AnyQuantity::new(1.5, unit);
                assert_eq!(q.kind(), kind);
                let back = q.convert_to(unit).unwrap();
                // Affine units near their zero point lose digits to
                // cancellation: 1.5 m°C is 273.1515 K and back.
                assert!((back - 1.5).abs() <= 1e-9, "{kind} {unit:?}: {back}");
                units += 1;
            }
        }
        assert_eq!(QuantityKind::all().count(), 128);
        assert!(units > 1700, "{units}");
    }

    #[test]
    fn conversions_with_typed_quantities() {
        let q = AnyQuantity::from(Length::from_kilometers(2.0));
        assert_eq!(Length::try_from(q), Ok(Length::from_meters(2000.0)));
        assert_eq!(
            Mass::try_from(q),
            Err(KindMismatch {
                expected: QuantityKind::Mass,
                found: QuantityKind::Length
            })
        );
        assert_eq!(
            q.convert_to(AnyUnit::Mass(MassUnit::Gram))
                .unwrap_err()
                .to_string(),
            "expected a Length, found a Mass"
        );
        assert_eq!(
            q.in_unit(LengthUnit::Kilometer.into()).unwrap().to_string(),
            "2 km"
        );
        assert_eq!(q.auto_unit().to_string(), "2 km");
        assert_eq!(q.to_string(), "2000 m");
        assert_eq!(q.base_unit(), AnyUnit::Length(LengthUnit::Meter));
        assert_eq!(q.dimensions(), Length::<f64>::INFO.dimensions);
    }

    #[test]
    fn runtime_arithmetic_follows_the_relations() {
        let l: AnyQuantity = Length::from_meters(6.0).into();
        let t: AnyQuantity = Duration::from_seconds(2.0).into();
        assert_eq!(
            l.checked_mul(l),
            Some(Area::from_square_meters(36.0).into())
        );
        assert_eq!(
            l.checked_div(t),
            Some(Speed::from_meters_per_second(3.0).into())
        );
        assert_eq!(l.checked_add(l), Some(Length::from_meters(12.0).into()));
        assert_eq!(l.checked_add(t), None);
        assert_eq!(l.checked_scale(0.5), Some(Length::from_meters(3.0).into()));
        assert!(l < Length::from_meters(7.0).into());
        assert_eq!(l.partial_cmp(&t), None);
        // Points: differences and moves, but no sums or scaling.
        let t0: AnyQuantity = Temperature::from_degrees_celsius(20.0).into();
        let t1: AnyQuantity = Temperature::from_degrees_celsius(25.0).into();
        let rise = t1.checked_sub(t0).unwrap();
        assert_eq!(rise.kind(), QuantityKind::TemperatureDelta);
        assert!((rise.as_base() - 5.0).abs() < 1e-12);
        assert_eq!(t0.checked_add(rise), Some(t1));
        assert_eq!(t0.checked_add(t1), None);
        assert_eq!(t0.checked_scale(2.0), None);
    }

    #[test]
    fn parsing() {
        let q: AnyQuantity = "1.5 km".parse().unwrap();
        assert_eq!(q, Length::from_kilometers(1.5).into());
        assert_eq!(
            "2 kilograms".parse::<AnyQuantity>(),
            Ok(Mass::from_kilograms(2.0).into())
        );
        // Listed units win over expressions: "N·m" is listed for torque.
        assert_eq!(
            "2 N·m".parse::<AnyQuantity>().map(|q| q.kind()),
            Ok(QuantityKind::Torque)
        );
        // An expression fits one kind…
        assert_eq!(
            "3 kg*m/s^2".parse::<AnyQuantity>(),
            Ok(Force::from_newtons(3.0).into())
        );
        // …or several (energy and torque share dimensions).
        assert_eq!(
            "3 kg*m^2/s^2".parse::<AnyQuantity>(),
            Err(ParseError::Ambiguous)
        );
        let kinds: Vec<_> = AnyQuantity::candidates("3 kg*m^2/s^2")
            .map(|q| q.kind())
            .collect();
        assert!(kinds.contains(&QuantityKind::Energy) && kinds.contains(&QuantityKind::Torque));
        // Shared abbreviations.
        assert_eq!("5 m".parse::<AnyQuantity>(), Err(ParseError::Ambiguous));
        assert_eq!(
            AnyQuantity::parse_as("5 m", QuantityKind::Length),
            Ok(Length::from_meters(5.0).into())
        );
        let units: Vec<_> = AnyUnit::candidates("m").collect();
        assert!(units.contains(&AnyUnit::Length(LengthUnit::Meter)));
        assert!(units.contains(&AnyUnit::Duration(DurationUnit::Minute)));
        // Errors.
        assert_eq!(
            "x km".parse::<AnyQuantity>(),
            Err(ParseError::InvalidNumber)
        );
        assert_eq!(
            "1 parsec-ish".parse::<AnyQuantity>(),
            Err(ParseError::UnknownUnit)
        );
        assert_eq!(
            "nonsense".parse::<QuantityKind>(),
            Err(ParseError::UnknownUnit)
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip() {
        let q: AnyQuantity = Pressure::from_bars(2.0).into();
        let json = serde_json::to_string(&q).unwrap();
        assert_eq!(json, r#"{"Pressure":200000.0}"#);
        assert_eq!(serde_json::from_str::<AnyQuantity>(&json).unwrap(), q);
        let unit = AnyUnit::Speed(SpeedUnit::KilometerPerHour);
        let json = serde_json::to_string(&unit).unwrap();
        assert_eq!(json, r#"{"Speed":"KilometerPerHour"}"#);
        assert_eq!(serde_json::from_str::<AnyUnit>(&json).unwrap(), unit);
        assert_eq!(
            serde_json::to_string(&QuantityKind::Mass).unwrap(),
            r#""Mass""#
        );
    }
}

// Integrations

#[cfg(feature = "chrono")]
#[test]
fn chrono_time_delta() {
    use chrono::TimeDelta;
    use qty::unit::duration::Nanosecond;
    assert_eq!(
        Duration::from(TimeDelta::milliseconds(1500)),
        Duration::from_seconds(1.5)
    );
    assert_eq!(
        Duration::from(TimeDelta::milliseconds(-1500)),
        Duration::from_seconds(-1.5)
    );
    assert_eq!(
        TimeDelta::try_from(Duration::from_seconds(-1.25)),
        Ok(TimeDelta::milliseconds(-1250))
    );
    assert_eq!(
        TimeDelta::try_from(Duration::from_nanoseconds(0.6)),
        Ok(TimeDelta::nanoseconds(1))
    );
    assert_eq!(
        TimeDelta::try_from(Duration::from_seconds(f64::NAN)),
        Err(RoundError::NotFinite)
    );
    assert_eq!(
        TimeDelta::try_from(Duration::from_seconds(1e18)),
        Err(RoundError::Overflow)
    );
    // Exact integer nanoseconds, negative included.
    let d = TimeDelta::nanoseconds(-1_234_567_890_123);
    let ns: Fixed<Nanosecond, i64> = d.try_into().unwrap();
    assert_eq!(ns.count(), -1_234_567_890_123);
    assert_eq!(TimeDelta::try_from(ns), Ok(d));
    let max: Fixed<Nanosecond, i128> = TimeDelta::MAX.try_into().unwrap();
    assert_eq!(TimeDelta::try_from(max), Ok(TimeDelta::MAX));
    assert_eq!(
        TimeDelta::try_from(max + Fixed::<Nanosecond, i128>::new(1)),
        Err(RoundError::Overflow)
    );
}

#[cfg(feature = "time")]
#[test]
fn time_duration() {
    use qty::unit::duration::Nanosecond;
    assert_eq!(
        Duration::from(time::Duration::milliseconds(-250)),
        Duration::from_seconds(-0.25)
    );
    assert_eq!(
        time::Duration::try_from(Duration::from_seconds(-1.25)),
        Ok(time::Duration::milliseconds(-1250))
    );
    assert_eq!(
        time::Duration::try_from(Duration::from_seconds(f64::INFINITY)),
        Err(RoundError::NotFinite)
    );
    let d = time::Duration::new(-12, -345_678_901);
    let ns: Fixed<Nanosecond, i64> = d.try_into().unwrap();
    assert_eq!(ns.count(), -12_345_678_901);
    assert_eq!(time::Duration::try_from(ns), Ok(d));
    let max: Fixed<Nanosecond, i128> = time::Duration::MAX.try_into().unwrap();
    assert_eq!(time::Duration::try_from(max), Ok(time::Duration::MAX));
    assert_eq!(
        time::Duration::try_from(max + Fixed::<Nanosecond, i128>::new(1)),
        Err(RoundError::Overflow)
    );
    assert_eq!(
        Fixed::<Nanosecond, i64>::try_from(time::Duration::MAX),
        Err(RoundError::Overflow)
    );
}

#[cfg(all(feature = "schemars", feature = "serde"))]
#[test]
fn json_schemas() {
    #[derive(schemars::JsonSchema)]
    #[allow(dead_code)]
    struct Config {
        distance: Length,
        unit: LengthUnit,
        #[schemars(schema_with = "qty::serde_str::schema::<Duration>")]
        timeout: Duration,
    }
    let schema = serde_json::to_value(schemars::schema_for!(Config)).unwrap();
    let props = &schema["properties"];
    assert_eq!(props["distance"]["type"], "number");
    assert_eq!(props["distance"]["description"], "Length in Meters (m)");
    assert_eq!(props["timeout"]["type"], "string");
    assert_eq!(
        props["timeout"]["description"],
        "A number and a unit of Duration, such as \"1.5 s\""
    );
    let unit = serde_json::to_value(schemars::schema_for!(LengthUnit)).unwrap();
    let names = unit["enum"].as_array().unwrap();
    assert!(names.iter().any(|n| n == "Kilometer"));
    assert_eq!(names.len(), LengthUnit::ALL.len());
}

mod presentation {
    use qty::*;

    #[test]
    fn auto_unit_over_a_unit_list() {
        let shown = |m: f64| {
            Length::from_meters(m)
                .auto_unit_in(LengthUnit::IMPERIAL)
                .unit
        };
        assert_eq!(shown(1609.344), LengthUnit::Mile);
        assert_eq!(shown(1609.0), LengthUnit::Yard);
        assert_eq!(shown(0.5), LengthUnit::Foot);
        assert_eq!(shown(-0.5), LengthUnit::Foot);
        // Below every unit: the smallest.
        assert_eq!(shown(0.01), LengthUnit::Inch);
        assert_eq!(shown(0.0), LengthUnit::Inch);
        // No units: the base unit.
        assert_eq!(
            Length::from_meters(5.0).auto_unit_in(&[]).unit,
            LengthUnit::Meter
        );
        let q = Mass::from_kilograms(80.0).auto_unit_in(MassUnit::IMPERIAL);
        assert_eq!(q.unit, MassUnit::Stone);
        assert_eq!(format!("{:.1}", q), "12.6 st");
        let cup = Volume::from_liters(0.25).auto_unit_in(VolumeUnit::US_LIQUID);
        assert_eq!(cup.unit, VolumeUnit::UsCustomaryCup);
    }

    #[test]
    fn significant_digits() {
        let m = |v: f64, d: usize| {
            Length::from_meters(v)
                .in_unit(LengthUnit::Meter)
                .significant(d)
                .to_string()
        };
        assert_eq!(m(0.0, 3), "0.00 m");
        assert_eq!(m(-0.0, 3), "0.00 m");
        assert_eq!(m(1.0, 1), "1 m");
        assert_eq!(m(1.0, 4), "1.000 m");
        assert_eq!(m(-1234.5, 2), "-1200 m");
        assert_eq!(m(999.96, 4), "1000 m");
        assert_eq!(m(999.96, 5), "999.96 m");
        assert_eq!(m(1.25e-7, 2), "1.2e-7 m");
        assert_eq!(m(1e21, 2), "1.0e21 m");
        assert_eq!(m(f64::NAN, 3), "NaN m");
        assert_eq!(m(f64::NEG_INFINITY, 3), "-inf m");
        // Digits are clamped to what the type holds.
        assert_eq!(m(0.1, 0), "0.1 m");
        assert_eq!(m(0.1, 40), "0.10000000000000001 m");
        let f = Length::<f32>::from_meters(0.1).in_unit(LengthUnit::Meter);
        assert_eq!(f.significant(40).to_string(), "0.100000001 m");
        // Formatter flags apply to the whole text.
        let s = Force::from_newtons(1234.5).auto_unit().significant(3);
        assert_eq!(format!("{s:+}"), "+1.23 kN");
        assert_eq!(format!("{s:<9}|"), "1.23 kN  |");
        assert_eq!(format!("{s:*^11}"), "**1.23 kN**");
        // Dimensionless units have no abbreviation to separate.
        let r = Ratio::from_decimal_fractions(0.123_456).in_unit(RatioUnit::DecimalFraction);
        assert_eq!(r.significant(2).to_string(), "0.12");
    }

    #[test]
    fn means() {
        let t = Temperature::from_degrees_celsius;
        let mean = Temperature::mean([t(20.0), t(30.0)]).unwrap();
        assert!((mean.as_degrees_celsius() - 25.0).abs() < 1e-12);
        let f = Temperature::from_degrees_fahrenheit;
        let mean = Temperature::mean([f(50.0), f(70.0)]).unwrap();
        assert!((mean.as_degrees_fahrenheit() - 60.0).abs() < 1e-9);
        assert_eq!(Temperature::<f64>::mean([]), None);
        let lengths = [1.0, 2.0, 6.0].map(Length::from_meters);
        assert_eq!(Length::mean(lengths), Some(Length::from_meters(3.0)));
    }

    #[cfg(feature = "fixed")]
    #[test]
    fn fixed_means() {
        use qty::unit::temperature::MillidegreeCelsius;
        type Reading = Fixed<MillidegreeCelsius, i32>;
        let r = [2, -1].map(Reading::new);
        assert_eq!(Fixed::mean(r, Rounding::TowardZero).unwrap().count(), 0);
        assert_eq!(Fixed::mean(r, Rounding::Floor).unwrap().count(), 0);
        assert_eq!(Fixed::mean(r, Rounding::Ceil).unwrap().count(), 1);
        assert_eq!(
            Fixed::mean(r, Rounding::NearestTiesEven).unwrap().count(),
            0
        );
        assert_eq!(
            Fixed::mean(r, Rounding::NearestTiesAway).unwrap().count(),
            1
        );
        assert_eq!(Fixed::mean(r, Rounding::Exact), Err(RoundError::Inexact));
        assert_eq!(
            Fixed::mean(core::iter::empty::<Reading>(), Rounding::Floor),
            Err(RoundError::DivisionByZero)
        );
        // The sum is exact even when it leaves `i32`.
        let big = [i32::MAX, i32::MAX - 2].map(Reading::new);
        assert_eq!(
            Fixed::mean(big, Rounding::Floor).unwrap().count(),
            i32::MAX - 1
        );
    }
}

mod polish {
    #[cfg(feature = "fixed")]
    use qty::unit::length::Millimeter;
    use qty::*;

    #[test]
    fn generic_conversions_through_the_trait() {
        fn round_trip<Q: Quantity<Value = f64>>(value: f64, unit: Q::Unit) -> f64 {
            Q::new(value, unit).convert_to(unit)
        }
        assert!((round_trip::<Length>(3.5, LengthUnit::Foot) - 3.5).abs() < 1e-12);
        assert!(
            (round_trip::<Temperature>(-40.0, TemperatureUnit::DegreeFahrenheit) + 40.0).abs()
                < 1e-12
        );
        fn in_base<Q: Quantity<Value = f64>>(value: f64, unit: Q::Unit) -> f64 {
            Q::new(value, unit).as_base()
        }
        assert_eq!(in_base::<Length>(2.0, LengthUnit::Kilometer), 2000.0);
    }

    #[test]
    fn float_predicates_and_signs() {
        let m = Length::from_meters;
        assert!(m(f64::NAN).is_nan());
        assert!(!m(1.0).is_nan());
        assert!(!m(f64::INFINITY).is_finite());
        assert!(m(1.0).is_finite());
        assert_eq!(m(-2.0).signum(), -1.0);
        assert_eq!(m(0.0).signum(), 1.0);
        assert_eq!(m(-0.0).signum(), -1.0);
        assert!(m(f64::NAN).signum().is_nan());
        assert_eq!(m(3.0).copysign(m(-1.0)), m(-3.0));
        assert_eq!(m(-3.0).copysign(m(0.0)), m(3.0));
        assert!(Temperature::from_kelvins(f64::NAN).is_nan());
        assert!(Length::<f32>::from_meters(f32::NAN).is_nan());
        assert_eq!(Length::<f32>::from_meters(-0.5).signum(), -1.0);
    }

    #[test]
    fn zero_constants() {
        const ORIGIN: Length = Length::ZERO;
        assert_eq!(ORIGIN, Length::from_meters(0.0));
        assert_eq!(Length::<f32>::default(), Length::<f32>::from_meters(0.0));
        assert_eq!(
            Duration::ZERO + Duration::from_seconds(1.0),
            Duration::from_seconds(1.0)
        );
    }

    #[test]
    fn powers() {
        let side = Length::from_meters(3.0);
        let area: Area = side.squared();
        assert_eq!(area, Area::from_square_meters(9.0));
        let volume: Volume = side.cubed();
        assert_eq!(volume, Volume::from_cubic_meters(27.0));
        assert_eq!(area.sqrt(), side);
    }

    #[cfg(feature = "mechanics")]
    #[test]
    fn speed_squared_is_specific_energy() {
        let v = Speed::from_meters_per_second(4.0);
        assert_eq!(v.squared(), SpecificEnergy::from_joules_per_kilogram(16.0));
    }

    #[test]
    fn rounding_in_a_unit() {
        let l = Length::from_meters(1.264);
        assert!((l.round_in(LengthUnit::Centimeter).as_centimeters() - 126.0).abs() < 1e-9);
        assert_eq!(
            Length::from_meters(1.0)
                .floor_in(LengthUnit::Foot)
                .as_feet(),
            3.0
        );
        assert!((Length::from_meters(1.0).ceil_in(LengthUnit::Foot).as_feet() - 4.0).abs() < 1e-12);
        assert!(
            (Length::from_meters(-1.0)
                .floor_in(LengthUnit::Foot)
                .as_feet()
                + 4.0)
                .abs()
                < 1e-12
        );
        // Points round on their own scale.
        let t = Temperature::from_degrees_celsius(21.6);
        assert!(
            (t.round_in(TemperatureUnit::DegreeCelsius)
                .as_degrees_celsius()
                - 22.0)
                .abs()
                < 1e-9
        );
        // Ties away from zero.
        assert_eq!(
            Length::from_meters(2.5)
                .round_in(LengthUnit::Meter)
                .as_meters(),
            3.0
        );
        assert_eq!(
            Length::from_meters(-2.5)
                .round_in(LengthUnit::Meter)
                .as_meters(),
            -3.0
        );
        // Uncertain storage rounds the value and keeps the uncertainty.
        let u = Length::from_meters(Uncertain::new(1.26, 0.02)).round_in(LengthUnit::Meter);
        assert_eq!(u.as_meters().value(), 1.0);
        assert_eq!(u.as_meters().sigma(), 0.02);
    }

    #[cfg(feature = "fixed")]
    #[test]
    fn fixed_signs() {
        type Mm = Fixed<Millimeter, i32>;
        assert_eq!(Mm::new(-5).abs(), Mm::new(5));
        assert_eq!(Mm::new(i32::MIN).checked_abs(), None);
        assert_eq!(Mm::new(i32::MAX).checked_abs(), Some(Mm::new(i32::MAX)));
        assert_eq!(Mm::new(7).checked_neg(), Some(Mm::new(-7)));
        assert_eq!(Mm::new(i32::MIN).checked_neg(), None);
        assert_eq!(
            (
                Mm::new(-3).signum(),
                Mm::new(0).signum(),
                Mm::new(9).signum()
            ),
            (-1, 0, 1)
        );
        type Unsigned = Fixed<Millimeter, u32>;
        assert_eq!(Unsigned::new(5).checked_neg(), None);
        assert_eq!(Unsigned::new(0).checked_neg(), Some(Unsigned::new(0)));
        assert_eq!(Unsigned::new(5).abs(), Unsigned::new(5));
        assert_eq!(Unsigned::new(5).signum(), 1);
    }

    #[cfg(feature = "fixed")]
    #[test]
    #[should_panic(expected = "Fixed::abs overflowed")]
    fn fixed_abs_overflow_panics() {
        let _ = Fixed::<Millimeter, i8>::new(i8::MIN).abs();
    }

    #[test]
    fn vector_lerp() {
        let a = Vector2::new(Length::from_meters(0.0), Length::from_meters(10.0));
        let b = Vector2::new(Length::from_meters(4.0), Length::from_meters(0.0));
        let mid = a.lerp(b, 0.5);
        assert_eq!(
            mid,
            Vector2::new(Length::from_meters(2.0), Length::from_meters(5.0))
        );
        assert_eq!(a.lerp(b, 0.0), a);
    }
}

mod custom_units {
    use qty::*;

    custom_unit! {
        /// The furlong: 220 yards.
        pub struct Furlong for qty::Length {
            abbreviation: "fur",
            singular: "Furlong",
            plural: "Furlongs",
            factor: 201.168,
        }
    }

    // Degrees Celsius again, to check the offset against the built-in unit.
    custom_unit! {
        struct Celsius for Temperature {
            abbreviation: "°C*",
            singular: "Celsius",
            plural: "Celsius",
            factor: 1.0,
            offset: 273.15
        }
    }

    #[test]
    fn conversions() {
        let race = Length::new(5.0, Furlong);
        assert!((race.as_meters() - 1005.84).abs() < 1e-9);
        assert!((race.convert_to(Furlong) - 5.0).abs() < 1e-12);
        assert!((Length::from_miles(1.0).convert_to(Furlong) - 8.0).abs() < 1e-12);
        let small: Length<f32> = Length::new(1.0, Furlong);
        assert!((small.as_meters() - 201.168).abs() < 1e-4);
    }

    #[test]
    fn offsets_for_points() {
        let t = Temperature::new(21.5, Celsius);
        assert!(t.approx_eq_abs(Temperature::from_degrees_celsius(21.5), 1e-12));
        assert!((Temperature::from_kelvins(300.0).convert_to(Celsius) - 26.85).abs() < 1e-9);
    }

    #[test]
    fn display_and_metadata() {
        let d = Length::from_meters(402.336).in_unit(Furlong);
        assert_eq!(format!("{d:.1}"), "2.0 fur");
        assert_eq!(format!("{:>8}", Furlong), "     fur");
        assert_eq!(Furlong.info().plural, "Furlongs");
        assert_eq!(Furlong.abbreviation(), "fur");
    }

    #[test]
    fn generic_code_takes_any_unit() {
        fn in_unit_of<U: UnitOf<Length>>(meters: f64, unit: U) -> f64 {
            Length::from_meters(meters).convert_to(unit)
        }
        assert!((in_unit_of(201.168, Furlong) - 1.0).abs() < 1e-12);
        assert_eq!(in_unit_of(1500.0, LengthUnit::Kilometer), 1.5);
        // Built-in units still infer the storage type from the value.
        let l = Length::new(2.0, LengthUnit::Kilometer);
        assert_eq!(l, Length::from_meters(2000.0));
    }
}
