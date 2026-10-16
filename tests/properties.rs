//! Property tests: the library against independent references (exact `i128`
//! rational arithmetic, `f64`'s own rounding), over the full input ranges.

use std::cmp::Ordering;

use proptest::prelude::*;
use qty::unit::{
    area::SquareMeter,
    duration::Millisecond,
    length::{Inch, Meter, Millimeter},
    speed::MeterPerSecond,
    temperature::{DegreeCelsius, DegreeFahrenheit, Kelvin, MillidegreeCelsius},
    temperature_delta,
};
use qty::*;

fn rounding() -> impl Strategy<Value = Rounding> {
    prop_oneof![
        Just(Rounding::Floor),
        Just(Rounding::Ceil),
        Just(Rounding::TowardZero),
        Just(Rounding::AwayFromZero),
        Just(Rounding::NearestTiesEven),
        Just(Rounding::NearestTiesAway),
        Just(Rounding::Exact),
    ]
}

/// Counts: anywhere in `i64`, small, at the extremes, and on exact ties
/// (`… 500 / 1000`, also after the 273 150 m°C offset), which uniformly
/// random values would almost never hit.
fn count() -> impl Strategy<Value = i64> {
    prop_oneof![
        3 => any::<i64>(),
        1 => -3000i64..3000,
        1 => prop_oneof![Just(i64::MIN), Just(i64::MAX), Just(0), Just(-1), Just(1)],
        1 => any::<i32>().prop_map(|k| i64::from(k) * 1000 + 500),
        1 => any::<i32>().prop_map(|k| i64::from(k) * 1000 - 272_650),
    ]
}

/// `n / d` rounded with `mode`, then narrowed to `i64`: the reference.
fn reference(n: i128, d: i128, mode: Rounding) -> Result<i64, RoundError> {
    if d == 0 {
        return Err(RoundError::DivisionByZero);
    }
    let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
    let floor = n.div_euclid(d);
    let rem = n.rem_euclid(d);
    let rounded = if rem == 0 {
        floor
    } else {
        let up = floor + 1;
        match mode {
            Rounding::Floor => floor,
            Rounding::Ceil => up,
            Rounding::TowardZero => {
                if n < 0 {
                    up
                } else {
                    floor
                }
            }
            Rounding::AwayFromZero => {
                if n < 0 {
                    floor
                } else {
                    up
                }
            }
            Rounding::NearestTiesEven | Rounding::NearestTiesAway => match (2 * rem).cmp(&d) {
                Ordering::Less => floor,
                Ordering::Greater => up,
                Ordering::Equal if mode == Rounding::NearestTiesAway => {
                    if n < 0 {
                        floor
                    } else {
                        up
                    }
                }
                Ordering::Equal => {
                    if floor % 2 == 0 {
                        floor
                    } else {
                        up
                    }
                }
            },
            Rounding::Exact => return Err(RoundError::Inexact),
        }
    };
    i64::try_from(rounded).map_err(|_| RoundError::Overflow)
}

proptest! {
    // ── Fixed: conversions between units ──

    #[test]
    fn rescaling_matches_exact_rationals(a in count(), mode in rounding()) {
        // 1 mm = 1/1000 m.
        let got = Fixed::<Millimeter>::new(a).to_unit_rounded::<Meter>(mode).map(|f| f.count());
        prop_assert_eq!(got, reference(a.into(), 1000, mode));
        // 1 in = 127/5 mm.
        let got = Fixed::<Inch>::new(a).to_unit_rounded::<Millimeter>(mode).map(|f| f.count());
        prop_assert_eq!(got, reference(i128::from(a) * 127, 5, mode));
        // 1 mm = 5/127 in.
        let got = Fixed::<Millimeter>::new(a).to_unit_rounded::<Inch>(mode).map(|f| f.count());
        prop_assert_eq!(got, reference(i128::from(a) * 5, 127, mode));
    }

    #[test]
    fn affine_rescaling_matches_exact_rationals(t in count(), mode in rounding()) {
        // °C → °F: 9c/5 + 32.
        let got = Fixed::<DegreeCelsius>::new(t).to_unit_rounded::<DegreeFahrenheit>(mode).map(|f| f.count());
        prop_assert_eq!(got, reference(9 * i128::from(t) + 160, 5, mode));
        // m°C → K: m/1000 + 273.15.
        let got = Fixed::<MillidegreeCelsius>::new(t).to_unit_rounded::<Kelvin>(mode).map(|f| f.count());
        prop_assert_eq!(got, reference(i128::from(t) + 273_150, 1000, mode));
        // K → m°C is exact: 1000k − 273150.
        let got = Fixed::<Kelvin>::new(t).to_unit::<MillidegreeCelsius>().map(|f| f.count());
        prop_assert_eq!(got, reference(i128::from(t) * 1000 - 273_150, 1, mode));
    }

    // ── Fixed: comparisons across units are exact ──

    #[test]
    fn comparisons_across_units_are_exact(
        (a, b) in prop_oneof![
            (count(), count()),
            // Exactly equal pairs, and their neighbours: k m = 1000k mm;
            // 5k in = 127k mm; 5k °C = (9k + 32) °F.
            (any::<i32>(), -1i64..=1).prop_map(|(k, d)| (1000 * i64::from(k) + d, i64::from(k))),
            (any::<i32>(), -1i64..=1).prop_map(|(k, d)| (5 * i64::from(k), 127 * i64::from(k) + d)),
            (any::<i32>(), -1i64..=1).prop_map(|(k, d)| (5 * i64::from(k), 9 * i64::from(k) + 32 + d)),
        ],
    ) {
        let (mm, m) = (Fixed::<Millimeter>::new(a), Fixed::<Meter>::new(b));
        let expected = i128::from(a).cmp(&(i128::from(b) * 1000));
        prop_assert_eq!(mm.partial_cmp(&m), Some(expected));
        prop_assert_eq!(mm == m, expected == Ordering::Equal);
        // a in vs b mm: 127a vs 5b.
        let expected = (i128::from(a) * 127).cmp(&(i128::from(b) * 5));
        prop_assert_eq!(Fixed::<Inch>::new(a).partial_cmp(&Fixed::<Millimeter>::new(b)), Some(expected));
        // a °C vs b °F, in units of K/900: 900a + 245835 vs 500b + 229835.
        let expected = (900 * i128::from(a) + 245_835).cmp(&(500 * i128::from(b) + 229_835));
        prop_assert_eq!(
            Fixed::<DegreeCelsius>::new(a).partial_cmp(&Fixed::<DegreeFahrenheit>::new(b)),
            Some(expected)
        );
    }

    // ── Fixed: physics ──

    #[test]
    fn products_and_quotients_match_exact_rationals(
        (a, b) in prop_oneof![
            (any::<i32>(), any::<i32>()).prop_map(|(a, b)| (i64::from(a), i64::from(b))),
            // Ties: (1000k + 500) · 1000 mm² = (k + ½) m²; (2k + 1) / 2.
            (-2_000_000i64..2_000_000).prop_map(|k| (1000 * k + 500, 1000)),
            (-2_000_000i64..2_000_000).prop_map(|k| (2 * k + 1, 2)),
        ],
        mode in rounding(),
    ) {
        // mm × mm → m²: ab / 10⁶.
        let got = Fixed::<Millimeter>::new(a)
            .mul_to_rounded(Fixed::<Millimeter>::new(b), SquareMeter, mode)
            .map(|f| f.count());
        prop_assert_eq!(got, reference(i128::from(a) * i128::from(b), 1_000_000, mode));
        // mm ÷ ms → m/s: a / b.
        let got = Fixed::<Millimeter>::new(a)
            .div_to(Fixed::<Millisecond>::new(b), MeterPerSecond, mode)
            .map(|f| f.count());
        prop_assert_eq!(got, reference(a.into(), b.into(), mode));
    }

    #[test]
    fn point_differences_move_points_back(t0 in -(1i64 << 61)..(1i64 << 61), t1 in -(1i64 << 61)..(1i64 << 61)) {
        let (p0, p1) = (Fixed::<MillidegreeCelsius>::new(t0), Fixed::<MillidegreeCelsius>::new(t1));
        let delta: Fixed<temperature_delta::MillidegreeCelsius> = p1 - p0;
        prop_assert_eq!(delta.count(), t1 - t0);
        prop_assert_eq!(p0 + delta, p1);
        prop_assert_eq!(p1.sub_delta(delta), p0);
    }

    // ── Floats → integers ──

    #[test]
    fn rounding_floats_matches_std(x in -1e15f64..1e15, mode in rounding()) {
        let got = Fixed::<Meter>::round_from(Length::from_meters(x), mode).map(|f| f.count());
        let expected = match mode {
            Rounding::Floor => Ok(x.floor()),
            Rounding::Ceil => Ok(x.ceil()),
            Rounding::TowardZero => Ok(x.trunc()),
            Rounding::AwayFromZero if x.fract() == 0.0 => Ok(x),
            Rounding::AwayFromZero => Ok(x.trunc() + x.signum()),
            Rounding::NearestTiesEven => Ok(x.round_ties_even()),
            Rounding::NearestTiesAway => Ok(x.round()),
            Rounding::Exact if x.fract() == 0.0 => Ok(x),
            Rounding::Exact => Err(RoundError::Inexact),
        };
        prop_assert_eq!(got, expected.map(|v| v as i64));
    }

    // ── Floats: conversions, display and parsing ──

    #[test]
    fn linear_conversions_round_trip(v in -1e250f64..1e250, i in 0usize..1000) {
        let unit = LengthUnit::ALL[i % LengthUnit::ALL.len()];
        let back = Length::new(v, unit).convert_to(unit);
        prop_assert!((back - v).abs() <= v.abs() * 4.0 * f64::EPSILON, "{} {:?} -> {}", v, unit, back);
    }

    #[test]
    fn display_then_parse_round_trips(v in prop::num::f64::NORMAL | prop::num::f64::ZERO, i in 0usize..1000) {
        fn check<Q>(q: Q, unit: Q::Unit) -> Result<(), TestCaseError>
        where
            Q: Quantity<Value = f64> + core::str::FromStr<Err = ParseError> + PartialEq + core::fmt::Debug,
            Q::Unit: core::str::FromStr<Err = ParseError>,
            Q: InUnitExt,
        {
            let abbreviation = unit.abbreviation();
            if abbreviation.is_empty() {
                return Ok(());
            }
            let (value, text) = q.shown_in(unit);
            if !value.is_finite() {
                return Ok(()); // e.g. 1e300 m in femtometres
            }
            // Abbreviations shared by several units parse to the first one.
            let parsed_unit: Q::Unit = abbreviation.parse().unwrap();
            prop_assert_eq!(text.parse::<Q>(), Ok(Q::make(value, parsed_unit)), "{}", text);
            Ok(())
        }
        check(Length::from_base(v), LengthUnit::ALL[i % LengthUnit::ALL.len()])?;
        check(Pressure::from_base(v), PressureUnit::ALL[i % PressureUnit::ALL.len()])?;
        check(Temperature::from_base(v), TemperatureUnit::ALL[i % TemperatureUnit::ALL.len()])?;
        check(Speed::from_base(v), SpeedUnit::ALL[i % SpeedUnit::ALL.len()])?;
    }

    #[test]
    fn parsing_never_panics(s in "\\PC*") {
        let _ = s.parse::<Length>();
        let _ = s.parse::<Temperature>();
        let _ = s.parse::<LengthUnit>();
        let _ = LengthUnit::from_name(&s);
        let _ = format!("1.5 {s}").parse::<Speed>();
        let _ = format!("{s} m").parse::<Length<f32>>();
    }

    // ── Exact decimal display ──

    #[test]
    fn decimal_display_matches_exact_rationals(n in count(), precision in prop::option::of(0usize..10)) {
        use qty::unit::length::{Kilometer, Micrometer};
        // Reference: n · 10^-k, rounded to `p` decimals (ties to even), as text.
        fn reference(n: i64, k: u32, precision: Option<usize>) -> String {
            let p = precision.unwrap_or(k as usize) as u32;
            let magnitude = i128::from(n).unsigned_abs();
            let shown = if p < k {
                let d = 10u128.pow(k - p);
                let (q, r) = (magnitude / d, magnitude % d);
                q + u128::from(2 * r > d || (2 * r == d && q % 2 == 1))
            } else {
                magnitude * 10u128.pow(p - k)
            };
            let sign = if n < 0 && shown != 0 { "-" } else { "" };
            let scale = 10u128.pow(p);
            if p == 0 {
                format!("{sign}{shown} m")
            } else {
                format!("{sign}{}.{:0w$} m", shown / scale, shown % scale, w = p as usize)
            }
        }
        let show = |d: qty::Decimal<Meter>| match precision {
            Some(p) => format!("{d:.p$}"),
            None => d.to_string(),
        };
        prop_assert_eq!(show(Fixed::<Millimeter>::new(n).display_in::<Meter>()), reference(n, 3, precision));
        prop_assert_eq!(show(Fixed::<Micrometer>::new(n).display_in::<Meter>()), reference(n, 6, precision));
        // Up-scaling: n km = n·1000 m, whatever the precision.
        let km = Fixed::<Kilometer>::new(n).display_in::<Meter>();
        let expected = match (n, precision) {
            (0, Some(p)) if p > 0 => format!("0.{} m", "0".repeat(p)),
            (_, Some(p)) if p > 0 => format!("{n}000.{} m", "0".repeat(p)),
            (0, _) => "0 m".to_owned(),
            _ => format!("{n}000 m"),
        };
        prop_assert_eq!(show(km), expected);
    }

    // ── Uncertainty and vectors ──

    #[test]
    fn uncertainty_stays_valid(a in -1e100f64..1e100, sa in 0f64..1e100, b in -1e100f64..1e100, sb in 0f64..1e100) {
        let (x, y) = (Uncertain::new(a, sa), Uncertain::new(b, sb));
        for r in [x + y, x - y, x * y, y + x] {
            prop_assert!(r.sigma() >= 0.0 || r.sigma().is_nan());
        }
        prop_assert_eq!(x + y, y + x);
        prop_assert_eq!(x * y, y * x);
        prop_assert_eq!((x * y).value(), a * b);
    }

    #[test]
    fn vector_identities(a in prop::array::uniform3(-1e6f64..1e6), b in prop::array::uniform3(-1e6f64..1e6)) {
        let (u, v) = (Vector3::<Length>::from_base(a), Vector3::<Length>::from_base(b));
        prop_assert_eq!(u.dot(v), v.dot(u));
        prop_assert_eq!(u.cross(v), -v.cross(u));
        // The cross product is perpendicular to both factors (|u·(u×v)| small).
        let scale = u.norm().as_base().powi(2) * v.norm().as_base() + 1e-300;
        let triple: Volume = u.cross(v).dot(u);
        prop_assert!(triple.as_base().abs() <= scale * 1e-12);
        // Triangle inequality.
        prop_assert!((u + v).norm().as_base() <= (u.norm() + v.norm()).as_base() * (1.0 + 1e-15));
    }

    #[test]
    fn fixed_means_match_exact_rationals(counts in prop::collection::vec(count(), 0..8), mode in rounding()) {
        let values = counts.iter().map(|&c| Fixed::<MillidegreeCelsius, i64>::new(c));
        let sum: i128 = counts.iter().map(|&c| i128::from(c)).sum();
        let expected = reference(sum, counts.len() as i128, mode);
        prop_assert_eq!(Fixed::mean(values, mode).map(|m| m.count()), expected);
    }

    #[test]
    fn float_means_of_points_shift_with_them(a in -1e3f64..1e3, b in -1e3f64..1e3, c in -1e3f64..1e3) {
        let t = |v: f64| Temperature::from_degrees_celsius(v);
        let mean = Temperature::mean([t(a), t(b), t(c)]).unwrap();
        let expected = (a + b + c) / 3.0;
        prop_assert!((mean.as_degrees_celsius() - expected).abs() <= 1e-9);
    }

    #[test]
    fn significant_digits_match_scientific_formatting(
        v in prop::num::f64::NORMAL | prop::num::f64::SUBNORMAL | prop::num::f64::ZERO,
        digits in 1usize..=17,
    ) {
        let shown = Length::from_meters(v).in_unit(LengthUnit::Meter).significant(digits).to_string();
        let number = shown.strip_suffix(" m").unwrap();
        let reference = format!("{:.*e}", digits - 1, v);
        // Same value as the correctly rounded scientific form.
        prop_assert_eq!(number.parse::<f64>().unwrap(), reference.parse::<f64>().unwrap());
        // And exactly `digits` significant digits.
        let mantissa: String = number.split('e').next().unwrap().chars().filter(char::is_ascii_digit).collect();
        let significant = mantissa.trim_start_matches('0');
        if v != 0.0 {
            prop_assert!(significant.len() >= digits, "{} has fewer than {} digits", number, digits);
            prop_assert!(significant.trim_end_matches('0').len() <= digits);
        }
    }
}

/// Helpers to show and rebuild a quantity in a unit, generically.
trait InUnitExt: Quantity {
    fn shown_in(self, unit: Self::Unit) -> (f64, String);
    fn make(value: f64, unit: Self::Unit) -> Self;
}

macro_rules! in_unit_ext {
    ($($Q:ident),*) => {$(
        impl InUnitExt for $Q {
            fn shown_in(self, unit: Self::Unit) -> (f64, String) {
                let shown = self.in_unit(unit);
                (shown.value, shown.to_string())
            }
            fn make(value: f64, unit: Self::Unit) -> Self {
                $Q::new(value, unit)
            }
        }
    )*};
}
in_unit_ext!(Length, Pressure, Temperature, Speed);
