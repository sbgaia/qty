use unitsnet_rs::*;

const EPS: f64 = 1e-9;

#[test]
fn meter_per_second_is_base() {
    let s = Speed::from_meters_per_second(1.0);
    assert!((s.as_meters_per_second() - 1.0).abs() < EPS);
}

#[test]
fn kmh_to_ms() {
    // 1 km/h = 1000/3600 m/s
    let s = Speed::from_kilometers_per_hour(3.6);
    assert!((s.as_meters_per_second() - 1.0).abs() < EPS);
}

#[test]
fn mph_to_ms() {
    let s = Speed::from_miles_per_hour(1.0);
    assert!((s.as_meters_per_second() - 0.44704).abs() < EPS);
}

#[test]
fn knot_conversion() {
    // 1 knot = 1852/3600 m/s
    let s = Speed::from_knots(1.0);
    assert!((s.as_meters_per_second() - 1852.0 / 3600.0).abs() < EPS);
}

#[test]
fn scalar_mul() {
    let s = Speed::from_meters_per_second(2.0);
    assert!(((s * 3.0).as_meters_per_second() - 6.0).abs() < EPS);
}

#[test]
fn from_str_kmh() {
    let s: Speed = "100 km/h".parse().unwrap();
    // 100 km/h in m/s
    let expected = 100.0 * 1000.0 / 3600.0;
    assert!((s.as_meters_per_second() - expected).abs() < EPS);
}
