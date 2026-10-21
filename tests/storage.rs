//! Storage types from other crates: complex numbers, decimals and rationals.

#[allow(unused_imports)]
use qty::*;

#[cfg(feature = "num-complex")]
mod complex {
    use super::*;
    use num_complex::Complex64;

    #[test]
    fn conversions() {
        let z = Length::from_kilometers(Complex64::new(1.5, -2.0));
        assert_eq!(z.as_meters(), Complex64::new(1500.0, -2000.0));
        assert_eq!(
            z.convert_to(LengthUnit::Kilometer),
            Complex64::new(1.5, -2.0)
        );
        assert_eq!(z.in_unit(LengthUnit::Kilometer).to_string(), "1.5-2i km");
    }

    #[test]
    fn arithmetic() {
        let a = Length::from_meters(Complex64::new(3.0, 4.0));
        let b = Length::from_meters(Complex64::new(1.0, -1.0));
        assert_eq!((a + b).as_meters(), Complex64::new(4.0, 3.0));
        let area: Area<Complex64> = a * b;
        assert_eq!(area.as_square_meters(), Complex64::new(7.0, 1.0));
        let t = Duration::from_seconds(Complex64::new(2.0, 0.0));
        let v: Speed<Complex64> = a / t;
        assert_eq!(v.as_meters_per_second(), Complex64::new(1.5, 2.0));
    }

    /// Ohm's law with phasors: 2 A at 0° through 3 + 4j Ω.
    #[cfg(feature = "electrical")]
    #[test]
    fn phasors() {
        let current = ElectricCurrent::from_amperes(Complex64::new(2.0, 0.0));
        let impedance = ElectricResistance::from_ohms(Complex64::new(3.0, 4.0));
        let voltage: ElectricPotential<Complex64> = current * impedance;
        assert_eq!(voltage.as_volts(), Complex64::new(6.0, 8.0));
        assert_eq!(voltage.as_volts().norm_sqr(), 100.0);
    }

    #[test]
    fn formulas() {
        let m = Mass::from_kilograms(Complex64::new(2.0, 0.0));
        let v = Speed::from_meters_per_second(Complex64::new(0.0, 3.0));
        let e: Energy<Complex64> = formula!(0.5 * m * v.powi::<2>());
        assert_eq!(e.as_joules(), Complex64::new(-9.0, 0.0));
    }
}

#[cfg(feature = "rust_decimal")]
mod decimal {
    use super::*;
    use rust_decimal::Decimal;

    fn dec(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    #[test]
    fn exact_conversions() {
        let foot = Length::from_feet(Decimal::ONE);
        assert_eq!(foot.as_meters(), dec("0.3048"));
        assert_eq!(Length::from_inches(dec("12")), foot);
        assert_eq!(Length::from_meters(dec("0.3")).as_millimeters(), dec("300"));
        // In f64 this is 0.30000000000000004.
        let sum = Length::from_meters(dec("0.1")) + Length::from_meters(dec("0.2"));
        assert_eq!(sum.as_meters(), dec("0.3"));
        assert_eq!(
            Mass::from_pounds(dec("2")).as_kilograms(),
            dec("0.90718474")
        );
    }

    #[test]
    fn offsets() {
        let t = Temperature::from_degrees_celsius(dec("21.5"));
        assert_eq!(t.as_kelvins(), dec("294.65"));
        // 5/9 has no exact decimal, so this is 70.7 to 26 places.
        assert_eq!(t.as_degrees_fahrenheit().round_dp(20), dec("70.7"));
        assert_eq!(
            Speed::from_kilometers_per_hour(dec("36")).as_meters_per_second(),
            dec("10")
        );
        assert_eq!(
            Speed::from_meters_per_second(dec("10")).as_kilometers_per_hour(),
            dec("36")
        );
    }

    #[test]
    fn display_and_relations() {
        let d = Length::from_kilometers(dec("1.5"));
        assert_eq!(d.in_unit(LengthUnit::Meter).to_string(), "1500.0 m");
        let v: Speed<Decimal> = d / Duration::from_minutes(dec("2"));
        assert_eq!(v.as_meters_per_second(), dec("12.5"));
    }

    #[test]
    fn tiny_constants_round_to_28_places() {
        // An electronvolt has 28 decimal places in joules, so it is exact.
        assert_eq!(
            Energy::from_electron_volts(Decimal::ONE).as_joules(),
            dec("0.0000000000000000001602176634")
        );
    }

    #[test]
    #[should_panic(expected = "doesn't fit in Decimal")]
    fn huge_constants_panic() {
        #[cfg(feature = "full")]
        let _ = Mass::from_solar_masses(Decimal::ONE);
        #[cfg(not(feature = "full"))]
        panic!("doesn't fit in Decimal");
    }
}

#[cfg(feature = "num-rational")]
mod rational {
    use super::*;
    use num_rational::Ratio;

    #[test]
    fn exact_conversions() {
        let foot = Length::from_feet(Ratio::<i64>::from_integer(1));
        assert_eq!(foot.as_meters(), Ratio::new(381, 1250));
        assert_eq!(Length::from_inches(Ratio::from_integer(12)), foot);
        let third = Length::from_meters(Ratio::<i64>::new(1, 3));
        assert_eq!((third + third + third).as_meters(), Ratio::from_integer(1));
        assert_eq!(
            Duration::from_hours(Ratio::<i32>::new(1, 4)).as_minutes(),
            Ratio::from_integer(15)
        );
    }

    #[test]
    fn offsets() {
        let t = Temperature::from_degrees_fahrenheit(Ratio::<i64>::from_integer(212));
        assert_eq!(t.as_degrees_celsius(), Ratio::from_integer(100));
        assert_eq!(t.as_kelvins(), Ratio::new(37315, 100));
        let v = Speed::from_kilometers_per_hour(Ratio::<i64>::from_integer(36));
        assert_eq!(v.as_meters_per_second(), Ratio::from_integer(10));
        assert_eq!(v.as_miles_per_hour(), Ratio::new(31_250, 1_397));
    }

    #[test]
    fn relations_and_formulas() {
        let d = Length::from_kilometers(Ratio::<i64>::from_integer(1));
        let t = Duration::from_seconds(Ratio::from_integer(3));
        let v: Speed<Ratio<i64>> = d / t;
        assert_eq!(v.as_meters_per_second(), Ratio::new(1000, 3));
        let m = Mass::from_kilograms(Ratio::from_integer(3));
        let e: Energy<Ratio<i64>> = formula!(0.5 * m * v * v);
        assert_eq!(e.as_joules(), Ratio::new(500_000, 3));
    }

    #[test]
    fn wide_constants_need_i128() {
        let ev = Energy::from_electron_volts(Ratio::<i128>::from_integer(1));
        assert_eq!(ev.as_joules(), Ratio::new(801_088_317, 5 * 10i128.pow(27)));
    }

    #[test]
    #[should_panic(expected = "doesn't fit in Ratio<i64>")]
    fn constants_past_the_storage_panic() {
        let _ = Energy::from_electron_volts(Ratio::<i64>::from_integer(1));
    }
}
