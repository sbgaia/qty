use unitsnet_rs::*;

const EPS: f64 = 1e-9;

#[test]
fn length_div_duration_gives_speed() {
    let d = Length::from_kilometers(10.0);
    let t = Duration::from_hours(0.5);
    let s: Speed = d / t;
    assert!((s.as_kilometers_per_hour() - 20.0).abs() < EPS);
}

#[test]
fn speed_mul_duration_gives_length() {
    // 20 km/h × 0.5 h = 10 km
    let s = Speed::from_kilometers_per_hour(20.0);
    let t = Duration::from_hours(0.5);
    let l: Length = s * t;
    assert!((l.as_kilometers() - 10.0).abs() < EPS);
}

#[test]
fn duration_mul_speed_gives_length() {
    let t = Duration::from_hours(2.0);
    let s = Speed::from_meters_per_second(5.0);
    let l: Length = t * s;
    assert!((l.as_meters() - 36000.0).abs() < EPS);
}

#[test]
fn length_div_speed_gives_duration() {
    // 100 m / 25 m/s = 4 s
    let l = Length::from_meters(100.0);
    let s = Speed::from_meters_per_second(25.0);
    let t: Duration = l / s;
    assert!((t.as_seconds() - 4.0).abs() < EPS);
}

#[test]
fn round_trip_length_speed_duration() {
    let original = Length::from_kilometers(42.0);
    let speed = Speed::from_kilometers_per_hour(100.0);
    let t: Duration = original / speed;
    let recovered: Length = speed * t;
    assert!((recovered.as_kilometers() - 42.0).abs() < EPS);
}
