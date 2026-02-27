use unitsnet_rs::*;

const EPS: f64 = 1e-6;

#[test]
fn kelvin_roundtrip() {
    let t = Temperature::from_kelvins(300.0);
    assert!((t.as_kelvins() - 300.0).abs() < EPS);
}

#[test]
fn celsius_to_kelvin() {
    let t = Temperature::from_degrees_celsius(0.0);
    assert!((t.as_kelvins() - 273.15).abs() < EPS);
}

#[test]
fn boiling_point_celsius_to_fahrenheit() {
    let t = Temperature::from_degrees_celsius(100.0);
    assert!((t.as_degrees_fahrenheit() - 212.0).abs() < EPS);
}

#[test]
fn freezing_point_fahrenheit_to_celsius() {
    let t = Temperature::from_degrees_fahrenheit(32.0);
    assert!((t.as_degrees_celsius() - 0.0).abs() < EPS);
}

#[test]
fn body_temp_celsius_to_fahrenheit() {
    let t = Temperature::from_degrees_celsius(37.0);
    assert!((t.as_degrees_fahrenheit() - 98.6).abs() < 1e-4);
}

#[test]
fn absolute_zero_kelvin_to_celsius() {
    let t = Temperature::from_kelvins(0.0);
    assert!((t.as_degrees_celsius() + 273.15).abs() < EPS);
}

#[test]
fn rankine_conversion() {
    // 0 °C = 491.67 °R
    let t = Temperature::from_degrees_celsius(0.0);
    assert!((t.as_degrees_rankine() - 491.67).abs() < 1e-4);
}

#[test]
fn from_str_kelvin() {
    let t: Temperature = "373.15 K".parse().unwrap();
    assert!((t.as_degrees_celsius() - 100.0).abs() < EPS);
}
