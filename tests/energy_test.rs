use unitsnet_rs::*;

const EPS: f64 = 1e-6;

#[test]
fn joule_is_base() {
    let e = Energy::from_joules(1.0);
    assert!((e.as_joules() - 1.0).abs() < EPS);
}

#[test]
fn kilowatt_hour_to_joule() {
    // 1 kWh = 3 600 000 J
    let e = Energy::from_kilowatt_hours(1.0);
    assert!((e.as_joules() - 3_600_000.0).abs() < 1.0);
}

#[test]
fn calorie_to_joule() {
    let e = Energy::from_calories(1.0);
    assert!((e.as_joules() - 4.184).abs() < EPS);
}

#[test]
fn kilojoule_to_joule() {
    let e = Energy::from_kilojoules(1.0);
    assert!((e.as_joules() - 1000.0).abs() < EPS);
}

#[test]
fn megajoule_to_kilojoule() {
    let e = Energy::from_megajoules(1.0);
    assert!((e.as_kilojoules() - 1000.0).abs() < EPS);
}

#[test]
fn btu_to_joule() {
    let e = Energy::from_british_thermal_units(1.0);
    assert!((e.as_joules() - 1055.05585262).abs() < EPS);
}

#[test]
fn addition() {
    let a = Energy::from_joules(100.0);
    let b = Energy::from_joules(200.0);
    assert!(((a + b).as_joules() - 300.0).abs() < EPS);
}

#[test]
fn from_str_joules() {
    let e: Energy = "42 J".parse().unwrap();
    assert!((e.as_joules() - 42.0).abs() < EPS);
}
