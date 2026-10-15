//! Integer temperature readings, as a sensor reports them, processed exactly.
//!
//! Run with `cargo run --example sensor`.

use unitsnet_rs::unit::temperature::{DegreeCelsius, MillidegreeCelsius};
use unitsnet_rs::unit::temperature_delta;
use unitsnet_rs::*;

/// A reading in millidegrees Celsius, the sensor's native integer format.
type Reading = Fixed<MillidegreeCelsius, i32>;
/// A temperature difference in millidegrees.
type Change = Fixed<temperature_delta::MillidegreeCelsius, i32>;

fn main() -> Result<(), RoundError> {
    let raw: [i32; 6] = [21_375, 21_500, 22_125, 23_000, 22_750, 22_250];
    let readings: Vec<Reading> = raw.iter().map(|&m| Fixed::new(m)).collect();

    // Integers compare and sort exactly; no NaN to worry about.
    let min = *readings.iter().min().unwrap();
    let max = *readings.iter().max().unwrap();
    // Temperatures are points: their difference is a change, not a temperature.
    let spread: Change = max - min;

    // Points cannot be summed (20 °C + 20 °C is not 40 °C), so the mean is the
    // first reading plus the mean change from it, rounded explicitly.
    let first = readings[0];
    let total_change: Change = readings.iter().map(|&r| r - first).sum();
    let mean_change = total_change.div_rounded(readings.len() as i32, Rounding::NearestTiesEven)?;
    let mean = first + mean_change;

    // Exact decimal display in °C, no floats involved.
    println!("readings: {}", readings.len());
    println!("min:      {}", min.display_in::<DegreeCelsius>());
    println!("max:      {}", max.display_in::<DegreeCelsius>());
    println!(
        "spread:   {}",
        spread.display_in::<temperature_delta::DegreeCelsius>()
    );
    println!("mean:     {:.2}", mean.display_in::<DegreeCelsius>());

    // For anything else, convert to a float quantity.
    let mean_f = mean.to_f64();
    println!(
        "mean:     {:.1}",
        mean_f.in_unit(TemperatureUnit::DegreeFahrenheit)
    );

    // An alarm threshold in whole degrees compares exactly with millidegrees.
    let alarm = Fixed::<DegreeCelsius, i32>::new(23);
    let over = readings.iter().filter(|&&r| r >= alarm).count();
    println!("readings at or above {}: {over}", alarm);

    assert_eq!(min.count(), 21_375);
    assert_eq!(spread.count(), 1_625);
    assert_eq!(mean.count(), 22_167); // 22 166.67 m°C, rounded
    assert_eq!(over, 1);
    Ok(())
}
