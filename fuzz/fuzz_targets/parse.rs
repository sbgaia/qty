//! Parsing arbitrary text never panics, and whatever parses displays and
//! parses back to the same value.
#![no_main]

use libfuzzer_sys::fuzz_target;
use unitsnet_rs::*;

fn round_trip<Q>(s: &str)
where
    Q: core::str::FromStr<Err = ParseError>
        + core::fmt::Display
        + PartialEq
        + core::fmt::Debug
        + Quantity<Value = f64>,
{
    if let Ok(q) = s.parse::<Q>() {
        if q.as_base().is_finite() {
            let shown = q.to_string();
            let back = shown.parse::<Q>();
            assert!(
                matches!(&back, Ok(b) if *b == q),
                "{s:?} -> {q:?} -> {shown:?} -> {back:?}"
            );
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let Ok(s) = core::str::from_utf8(data) else {
        return;
    };
    round_trip::<Length>(s);
    round_trip::<Temperature>(s);
    round_trip::<Pressure>(s);
    round_trip::<Speed>(s);
    round_trip::<Information>(s);
    let _ = s.parse::<Length<f32>>();
    let _ = s.parse::<LengthUnit>();
    let _ = s.parse::<TemperatureUnit>();
    let _ = LengthUnit::from_name(s);
    let _ = PressureUnit::from_name(s);
    let _ = split_value_unit(s);
});
