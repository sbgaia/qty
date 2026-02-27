//! # unitsnet-rs
//!
//! Strongly-typed physical quantities and unit conversions, generated from
//! [UnitsNet](https://github.com/angularsen/UnitsNet) definition files.
//!
//! ## Quick start
//!
//! ```rust
//! use unitsnet_rs::*;
//!
//! let dist = Length::from_kilometers(10.0);
//! let time = Duration::from_hours(0.5);
//! let speed: Speed = dist / time;
//!
//! assert!((speed.as_kilometers_per_hour() - 20.0).abs() < 1e-9);
//!
//! let temp = Temperature::from_degrees_celsius(100.0);
//! assert!((temp.as_degrees_fahrenheit() - 212.0).abs() < 1e-9);
//! ```

/// The underlying numeric type used by every quantity struct.
///
/// Defaults to `f64` (~15 significant digits). Enable the `f32` Cargo feature
/// to switch to `f32` (~7 significant digits) for lower memory usage.
#[cfg(feature = "f32")]
pub type Num = f32;
#[cfg(not(feature = "f32"))]
pub type Num = f64;

// Generated quantity modules — re-exports every public item from each module.
pub mod units;
pub use units::*;

// Generated cross-type arithmetic (Length / Duration = Speed, etc.).
// These are pure `impl` blocks — no named items to re-export.
pub mod cross_type;

// Convenience alias: the JSON quantity is named "Duration"; `Time` is the colloquial name.
pub use units::duration::Duration as Time;
