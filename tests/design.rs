//! Hand-written tests for the library-wide design guarantees: zero-cost
//! layout, generic storage, operators, formatting, parsing and the
//! cross-quantity relations whose base units are not coherent.

use std::collections::HashSet;
use std::mem::{align_of, size_of};

use unitsnet_rs::*;

// ── Layout ────────────────────────────────────────────────────────────────────

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

// ── Storage types ─────────────────────────────────────────────────────────────

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
fn lossless_cast() {
    let m: Mass<f64> = Mass::<i32>::from_base(3).cast();
    assert_eq!(m.as_grams(), 3_000.0);
    let l: Length<f64> = Length::<f32>::from_base(0.5).cast();
    assert_eq!(l.as_meters(), 0.5);
}

#[test]
fn non_copy_storage_supports_arithmetic() {
    // A deliberately non-Copy numeric type.
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

// ── Operators ─────────────────────────────────────────────────────────────────

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
/// units (`MassFlow` in g/s, `ElectricApparentEnergy` in VAh, …). Relations
/// between them must include the conversion factor.
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

// ── Conversions ───────────────────────────────────────────────────────────────

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

    // Absolute ratio (e.g. Carnot efficiency 1 − Tc/Th) stays available.
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

// ── Formatting & parsing ──────────────────────────────────────────────────────

#[test]
fn display_honours_format_flags() {
    assert_eq!(Length::from_meters(1.5).to_string(), "1.5 m");
    assert_eq!(format!("{:.2}", Length::from_meters(1.0)), "1.00 m");
    assert_eq!(format!("{:e}", Length::from_meters(1500.0)), "1.5e3 m");
    assert_eq!(format!("{}", Length::<i32>::from_base(7)), "7 m");
    assert_eq!(format!("[{:>4}]", LengthUnit::Kilometer), "[  km]");
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
    assert_eq!("5 parsecs".parse::<Length>(), Err(ParseError::UnknownUnit));
    assert_eq!("km".parse::<Length>(), Err(ParseError::InvalidNumber));
    assert_eq!("".parse::<Length>(), Err(ParseError::InvalidNumber));
    assert_eq!("km/h".parse::<SpeedUnit>(), Ok(SpeedUnit::KilometerPerHour));
    assert_eq!(ParseError::UnknownUnit.to_string(), "unknown unit");
}

#[test]
fn split_value_unit_handles_digits_in_units() {
    assert_eq!(split_value_unit("2 m3"), ("2", "m3"));
    assert_eq!(split_value_unit("2m3"), ("2", "m3"));
    assert_eq!(split_value_unit("1e-3 em"), ("1e-3", "em"));
    assert_eq!(split_value_unit("4 e"), ("4", "e"));
    assert_eq!(split_value_unit("4em"), ("4", "em"));
}

// ── Generic code ──────────────────────────────────────────────────────────────

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

// ── Integers in a chosen unit, rounding, ordering ─────────────────────────────

mod fixed {
    use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

    use unitsnet_rs::unit::duration::{Microsecond, Millisecond, Second};
    use unitsnet_rs::unit::length::{Inch, Meter, Millimeter};
    use unitsnet_rs::*;

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
        // Base-unit integer quantities round too.
        assert_eq!(
            Length::<i64>::round_from(Length::from_centimeters(250.0), Rounding::NearestTiesEven),
            Ok(Length::from_base(2))
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
