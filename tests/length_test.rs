use unitsnet_rs::*;

const EPS: f64 = 1e-9;

#[test]
fn from_meters_roundtrip() {
    let l = Length::from_meters(1.0);
    assert!((l.as_meters() - 1.0).abs() < EPS);
}

#[test]
fn kilometer_to_meter() {
    let l = Length::from_kilometers(1.0);
    assert!((l.as_meters() - 1000.0).abs() < EPS);
}

#[test]
fn foot_to_meter() {
    let l = Length::from_feet(1.0);
    assert!((l.as_meters() - 0.3048).abs() < EPS);
}

#[test]
fn mile_to_kilometer() {
    let l = Length::from_miles(1.0);
    assert!((l.as_kilometers() - 1.609344).abs() < EPS);
}

#[test]
fn addition() {
    let a = Length::from_meters(3.0);
    let b = Length::from_meters(2.0);
    assert!(((a + b).as_meters() - 5.0).abs() < EPS);
}

#[test]
fn subtraction() {
    let a = Length::from_meters(5.0);
    let b = Length::from_meters(3.0);
    assert!(((a - b).as_meters() - 2.0).abs() < EPS);
}

#[test]
fn scalar_mul() {
    let l = Length::from_meters(4.0);
    assert!(((l * 2.5).as_meters() - 10.0).abs() < EPS);
}

#[test]
fn scalar_div() {
    let l = Length::from_meters(10.0);
    assert!(((l / 2.0).as_meters() - 5.0).abs() < EPS);
}

#[test]
fn ratio() {
    let a = Length::from_meters(6.0);
    let b = Length::from_meters(2.0);
    assert!((a / b - 3.0).abs() < EPS);
}

#[test]
fn negation() {
    let l = Length::from_meters(5.0);
    assert!(((-l).as_meters() + 5.0).abs() < EPS);
}

#[test]
fn display() {
    let l = Length::from_meters(2.5);
    assert_eq!(l.to_string(), "2.5 m");
}

#[test]
fn from_str_meters() {
    let l: Length = "5 m".parse().unwrap();
    assert!((l.as_meters() - 5.0).abs() < EPS);
}

#[test]
fn from_str_kilometers() {
    let l: Length = "2.5 km".parse().unwrap();
    assert!((l.as_meters() - 2500.0).abs() < EPS);
}

#[test]
fn from_str_invalid() {
    assert!("abc".parse::<Length>().is_err());
}

#[test]
fn nanometer_conversion() {
    let l = Length::from_nanometers(1_000_000_000.0);
    assert!((l.as_meters() - 1.0).abs() < EPS);
}
