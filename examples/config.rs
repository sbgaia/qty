//! Reading a configuration file whose values carry their units.
//!
//! Run with `cargo run --example config --features serde`.

use serde::Deserialize;
use unitsnet_rs::*;

#[derive(Debug, Deserialize)]
struct Config {
    /// Any unit and spelling: "30 s", "0.5 min", "2 minutes".
    #[serde(with = "unitsnet_rs::serde_str")]
    timeout: Duration,
    #[serde(with = "unitsnet_rs::serde_str")]
    max_speed: Speed,
    /// Optional; missing means no limit.
    #[serde(with = "unitsnet_rs::serde_str::option", default)]
    max_load: Option<Mass>,
}

fn main() {
    let text = r#"{
        "timeout": "1.5 minutes",
        "max_speed": "60 km/h",
        "max_load": "2.5 t"
    }"#;
    let config: Config = serde_json::from_str(text).expect("valid config");
    println!("timeout:   {}", config.timeout);
    println!(
        "max speed: {:.2}",
        config.max_speed.in_unit(SpeedUnit::MeterPerSecond)
    );
    if let Some(load) = config.max_load {
        println!("max load:  {}", load.auto_unit());
    }

    // The config drives std APIs directly.
    let timeout: std::time::Duration = config.timeout.try_into().expect("non-negative timeout");
    assert_eq!(timeout.as_secs(), 90);

    // Mistakes are reported with the offending text.
    let error =
        serde_json::from_str::<Config>(r#"{ "timeout": "5 parsecs", "max_speed": "1 m/s" }"#)
            .unwrap_err();
    println!("bad config: {error}");
    assert!(error.to_string().contains("unknown unit"));
}
