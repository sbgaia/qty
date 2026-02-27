use unitsnet_rs::*;

const EPS: f64 = 1e-9;

#[test]
fn kilogram_is_base() {
    let m = Mass::from_kilograms(1.0);
    assert!((m.as_kilograms() - 1.0).abs() < EPS);
}

#[test]
fn gram_to_kilogram() {
    let m = Mass::from_grams(1000.0);
    assert!((m.as_kilograms() - 1.0).abs() < EPS);
}

#[test]
fn pound_to_kilogram() {
    let m = Mass::from_pounds(1.0);
    assert!((m.as_kilograms() - 0.45359237).abs() < EPS);
}

#[test]
fn tonne_to_kilogram() {
    let m = Mass::from_tonnes(1.0);
    assert!((m.as_kilograms() - 1000.0).abs() < EPS);
}

#[test]
fn milligram_conversion() {
    let m = Mass::from_milligrams(1_000_000.0);
    assert!((m.as_kilograms() - 1.0).abs() < EPS);
}

#[test]
fn addition() {
    let a = Mass::from_kilograms(2.0);
    let b = Mass::from_kilograms(3.0);
    assert!(((a + b).as_kilograms() - 5.0).abs() < EPS);
}

#[test]
fn from_str_grams() {
    let m: Mass = "500 g".parse().unwrap();
    assert!((m.as_grams() - 500.0).abs() < EPS);
}
