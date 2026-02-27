use unitsnet_rs::*;

const EPS: f64 = 1e-9;

#[test]
fn second_is_base() {
    let t = Duration::from_seconds(1.0);
    assert!((t.as_seconds() - 1.0).abs() < EPS);
}

#[test]
fn hour_to_seconds() {
    let t = Duration::from_hours(1.0);
    assert!((t.as_seconds() - 3600.0).abs() < EPS);
}

#[test]
fn minute_to_seconds() {
    let t = Duration::from_minutes(60.0);
    assert!((t.as_seconds() - 3600.0).abs() < EPS);
}

#[test]
fn day_to_hours() {
    let t = Duration::from_days(1.0);
    assert!((t.as_hours() - 24.0).abs() < EPS);
}

#[test]
fn week_to_days() {
    let t = Duration::from_weeks(1.0);
    assert!((t.as_days() - 7.0).abs() < EPS);
}

#[test]
fn millisecond_conversion() {
    let t = Duration::from_milliseconds(1000.0);
    assert!((t.as_seconds() - 1.0).abs() < EPS);
}

#[test]
fn nanosecond_conversion() {
    let t = Duration::from_nanoseconds(1_000_000_000.0);
    assert!((t.as_seconds() - 1.0).abs() < EPS);
}

#[test]
fn time_alias() {
    // `Time` is an alias for `Duration`.
    let t: Time = Time::from_hours(2.0);
    assert!((t.as_hours() - 2.0).abs() < EPS);
}

#[test]
fn from_str_seconds() {
    let t: Duration = "30 s".parse().unwrap();
    assert!((t.as_seconds() - 30.0).abs() < EPS);
}
