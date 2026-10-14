//! Integer quantities against exact `i128` rational arithmetic, over the
//! whole `i64` range.
#![no_main]

use core::cmp::Ordering;

use libfuzzer_sys::fuzz_target;
use unitsnet_rs::unit::{
    area::SquareMeter,
    duration::Millisecond,
    length::{Inch, Meter, Millimeter},
    speed::MeterPerSecond,
    temperature::{DegreeCelsius, DegreeFahrenheit, Kelvin, MillidegreeCelsius},
};
use unitsnet_rs::*;

const MODES: [Rounding; 7] = [
    Rounding::Floor,
    Rounding::Ceil,
    Rounding::TowardZero,
    Rounding::AwayFromZero,
    Rounding::NearestTiesEven,
    Rounding::NearestTiesAway,
    Rounding::Exact,
];

/// `n / d` rounded with `mode`, narrowed to `i64`.
fn reference(n: i128, d: i128, mode: Rounding) -> Result<i64, RoundError> {
    if d == 0 {
        return Err(RoundError::DivisionByZero);
    }
    let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
    let (floor, rem) = (n.div_euclid(d), n.rem_euclid(d));
    let up = floor + 1;
    let rounded = if rem == 0 {
        floor
    } else {
        match mode {
            Rounding::Floor => floor,
            Rounding::Ceil => up,
            Rounding::TowardZero if n < 0 => up,
            Rounding::TowardZero => floor,
            Rounding::AwayFromZero if n < 0 => floor,
            Rounding::AwayFromZero => up,
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
                Ordering::Equal if floor % 2 == 0 => floor,
                Ordering::Equal => up,
            },
            Rounding::Exact => return Err(RoundError::Inexact),
        }
    };
    i64::try_from(rounded).map_err(|_| RoundError::Overflow)
}

fuzz_target!(|input: (i64, i64, u8)| {
    let (a, b, mode) = input;
    let mode = MODES[usize::from(mode) % MODES.len()];
    let (wa, wb) = (i128::from(a), i128::from(b));

    // Conversions between units.
    let got = Fixed::<Millimeter>::new(a)
        .to_unit_rounded::<Meter>(mode)
        .map(|f| f.count());
    assert_eq!(got, reference(wa, 1000, mode));
    let got = Fixed::<Inch>::new(a)
        .to_unit_rounded::<Millimeter>(mode)
        .map(|f| f.count());
    assert_eq!(got, reference(wa * 127, 5, mode));
    let got = Fixed::<Millimeter>::new(a)
        .to_unit_rounded::<Inch>(mode)
        .map(|f| f.count());
    assert_eq!(got, reference(wa * 5, 127, mode));
    let got = Fixed::<DegreeCelsius>::new(a)
        .to_unit_rounded::<DegreeFahrenheit>(mode)
        .map(|f| f.count());
    assert_eq!(got, reference(9 * wa + 160, 5, mode));
    let got = Fixed::<MillidegreeCelsius>::new(a)
        .to_unit_rounded::<Kelvin>(mode)
        .map(|f| f.count());
    assert_eq!(got, reference(wa + 273_150, 1000, mode));

    // Comparisons across units.
    let mm_vs_m = Fixed::<Millimeter>::new(a).partial_cmp(&Fixed::<Meter>::new(b));
    assert_eq!(mm_vs_m, Some(wa.cmp(&(wb * 1000))));
    let c_vs_f = Fixed::<DegreeCelsius>::new(a).partial_cmp(&Fixed::<DegreeFahrenheit>::new(b));
    assert_eq!(
        c_vs_f,
        Some((900 * wa + 245_835).cmp(&(500 * wb + 229_835)))
    );

    // Physics.
    let got = Fixed::<Millimeter>::new(a)
        .mul_to_rounded(Fixed::<Millimeter>::new(b), SquareMeter, mode)
        .map(|f| f.count());
    assert_eq!(got, reference(wa * wb, 1_000_000, mode));
    let got = Fixed::<Millimeter>::new(a)
        .div_to(Fixed::<Millisecond>::new(b), MeterPerSecond, mode)
        .map(|f| f.count());
    assert_eq!(got, reference(wa, wb, mode));

    // Floats → integers: never panics, and agrees with `f64` rounding.
    let x = f64::from_bits(a as u64);
    let got = Fixed::<Meter>::round_from(Length::from_meters(x), mode);
    if x.is_finite() && x.abs() < 9e15 {
        let expected = match mode {
            Rounding::Floor => x.floor(),
            Rounding::Ceil => x.ceil(),
            Rounding::TowardZero => x.trunc(),
            Rounding::AwayFromZero if x.fract() == 0.0 => x,
            Rounding::AwayFromZero => x.trunc() + x.signum(),
            Rounding::NearestTiesEven => x.round_ties_even(),
            Rounding::NearestTiesAway => x.round(),
            Rounding::Exact if x.fract() == 0.0 => x,
            Rounding::Exact => {
                assert_eq!(got, Err(RoundError::Inexact));
                return;
            }
        };
        assert_eq!(got.map(|f| f.count()), Ok(expected as i64), "{x} {mode:?}");
    } else if !x.is_finite() {
        assert_eq!(got, Err(RoundError::NotFinite));
    }
});
