# unitsnet-rs

[![CI](https://github.com/sbgaia/unitsnet-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/sbgaia/unitsnet-rs/actions/workflows/ci.yml)

Strongly-typed physical quantities for Rust — 128 quantities and over 1,700
units generated from the [UnitsNet](https://github.com/angularsen/UnitsNet)
definitions — with **no runtime overhead** over the numbers they wrap.

```rust
use unitsnet_rs::*;

let distance = Length::from_kilometers(10.0);
let time = Duration::from_minutes(30.0);
let speed: Speed = distance / time;

assert!(speed.approx_eq_rel(Speed::from_kilometers_per_hour(20.0), 1e-12));
assert_eq!(format!("{:.1}", speed.in_unit(SpeedUnit::MilePerHour)), "12.4 mph");
```

## Why

- **Zero cost.** Every quantity is a `#[repr(transparent)]` wrapper. In release
  builds, typed code compiles to the same machine code as the bare `f64` code —
  for arithmetic, sums and in-place loops, LLVM even merges the two functions.
- **Exact conversions in one instruction.** Each upstream formula is reduced
  with exact rational arithmetic to a single constant: converting points to
  metres is one multiplication instead of a multiply and a divide. Conversions
  by an exact factor (km, mm, hours, …) are correctly rounded.
- **Any number type.** `f64` by default; `f32`, integers or your own float type
  work too (`Length<f32>`, `Mass<i64>`).
- **Integers in the unit you choose,** with explicit rounding: `Fixed<Millimeter>`
  counts millimetres exactly, and `Fixed<MillidegreeCelsius, i32>` is a
  temperature sensor's native reading.
- **Physics checked by the compiler.** `Length / Duration` is a `Speed`;
  `Length + Duration` does not compile, and neither does adding two
  temperatures (20 °C + 20 °C is not 40 °C).
- **`no_std`, no allocation, no required dependencies.**

## Numeric types

```rust
use unitsnet_rs::*;

// f32: half the memory.
let l: Length<f32> = Length::from_feet(3.0);
assert!(l.approx_eq_rel(Length::from_inches(36.0), 1e-6));

// Integers in the base unit: exact, Eq + Ord + Hash, physics included.
let force: Force<i64> = Mass::from_base(1_500) * Acceleration::from_base(2);
assert_eq!(force.as_base(), 3_000); // newtons

// Bulk data without copies.
let raw = [1.0, 2.0, 3.0];
let lengths: &[Length] = Length::from_base_slice(&raw);
assert_eq!(lengths.iter().sum::<Length>(), Length::from_meters(6.0));
```

## Vectors

`Vector2<Q>` and `Vector3<Q>` hold any quantity, and the physics carries over:

```rust
use unitsnet_rs::*;

let displacement = Vector3::<Length>::from_base([3.0, 4.0, 0.0]);
let velocity: Vector3<Speed> = displacement / Duration::from_seconds(2.0);
assert_eq!(velocity.norm(), Speed::from_meters_per_second(2.5));

let force: Vector3<Force> = Mass::from_kilograms(2.0) * Vector3::<Acceleration>::from_base([0.0, 0.0, -9.81]);
let power: Power = force.dot(velocity);
assert_eq!(power, Power::from_watts(0.0));
```

## Measurement uncertainty

`Uncertain` is a number type carrying a standard uncertainty, so any quantity
can hold one; conversions, relations and math functions propagate it (to
first order, for independent inputs):

```rust
use unitsnet_rs::*;

let distance = Length::from_meters(Uncertain::new(100.0, 0.5));
let time = Duration::from_seconds(Uncertain::new(9.58, 0.01));
let speed = distance / time;
assert_eq!(format!("{:.1}", speed.in_unit(SpeedUnit::KilometerPerHour)), "37.6 ± 0.2 km/h");
```

## Integers in a chosen unit, with explicit rounding

`Fixed<U, I = i64>` is a whole number of the unit `U`. Lossless operations
just work; anything that would lose precision asks for a `Rounding` mode.

```rust
use unitsnet_rs::*;
use unitsnet_rs::unit::length::{Meter, Millimeter};

type Millimeters = Fixed<Millimeter>;
type Meters = Fixed<Meter>;

let a = Millimeters::round_from(Length::from_meters(0.0125), Rounding::NearestTiesEven)?;
assert_eq!(a, Millimeters::new(12)); // 12.5 mm, ties to even

// Mixing units is allowed where it is exact: the result is in the left unit.
assert_eq!(a + Meters::new(1), Millimeters::new(1_012));
assert!(Meters::new(1) > a); // comparisons work across units, exactly

// Converting: exact needs no mode, lossy needs one.
assert_eq!(Meters::new(2).to_unit::<Millimeter>()?, Millimeters::new(2_000));
assert_eq!(Millimeters::new(1_500).to_unit_rounded::<Meter>(Rounding::Ceil)?, Meters::new(2));
# Ok::<(), RoundError>(())
```

Physics works on counts too, with the result's unit named explicitly:

```rust
use unitsnet_rs::*;
use unitsnet_rs::unit::{area::SquareMillimeter, length::Millimeter};
use unitsnet_rs::unit::{duration::Millisecond, speed::MillimeterPerSecond};

let side = Fixed::<Millimeter>::new(1_200);
assert_eq!(side.mul_to(side, SquareMillimeter).count(), 1_440_000);
let speed = side.div_to(Fixed::<Millisecond>::new(400), MillimeterPerSecond, Rounding::Exact)?;
assert_eq!(speed.count(), 3_000);
# Ok::<(), RoundError>(())
```

`Meters + Millimeters`, `Millimeters::to_unit::<Meter>()` and
`mm.mul_to(mm, SquareMeter)` are compile-time errors (they would need
rounding; the `_rounded` forms take a mode). Integer values are `Eq + Ord + Hash`, so they
work as `HashMap`/`BTreeMap` keys.

## Temperatures and decibel levels are points

A temperature is a point on a scale, not an amount. Differences between
points are deltas, which you can add back:

```rust
use unitsnet_rs::*;

let morning = Temperature::from_degrees_celsius(12.0);
let noon = Temperature::from_degrees_celsius(20.0);
let rise: TemperatureDelta = noon - morning;
assert!((rise.as_degrees_fahrenheit() - 14.4).abs() < 1e-12);
assert!((morning + rise).approx_eq_rel(noon, 1e-12));
// `morning + noon` does not compile.
```

The same holds for integer temperatures and for decibel levels (dBm, dBV),
whose differences are a `Level` in dB:

```rust
use unitsnet_rs::*;
use unitsnet_rs::unit::temperature::{DegreeCelsius, MillidegreeCelsius};
use unitsnet_rs::unit::temperature_delta as delta;

let reading = Fixed::<MillidegreeCelsius, i32>::new(21_375); // 21.375 °C
assert!(reading > Fixed::<DegreeCelsius, i32>::new(21)); // exact across scales
let warmer = reading + Fixed::<delta::DegreeCelsius, i32>::new(1); // +1000 m°C
assert_eq!(warmer.count(), 22_375);
```

## Display and parsing

```rust
use unitsnet_rs::*;

let d = Length::from_meters(1234.5);
assert_eq!(d.to_string(), "1234.5 m");
assert_eq!(format!("{:.1}", d.in_unit(LengthUnit::Kilometer)), "1.2 km");
assert_eq!(format!("{}", Length::from_meters(0.0025).auto_unit()), "2.5 mm");

assert_eq!("5 km".parse::<Length>(), Ok(Length::from_kilometers(5.0)));
assert_eq!("5 Kilometres".parse::<Length>(), Ok(Length::from_kilometers(5.0)));
```

## Math, sorting and keys

```rust
use std::collections::BTreeMap;
use unitsnet_rs::*;

let side: Length = Area::from_square_meters(16.0).sqrt();
assert_eq!(side, Length::from_meters(4.0));
let heading = Angle::atan2(Length::from_meters(1.0), Length::from_meters(1.0));
assert!((heading.as_degrees() - 45.0).abs() < 1e-12);

let mut v = vec![Length::from_meters(2.0), Length::from_inches(1.0)];
v.sort_by(Length::total_cmp);

let mut by_length = BTreeMap::new();
by_length.insert(Ordered(Length::from_meters(2.0)), "two"); // float keys
```

## Cargo features

| Feature | Default | Adds |
|---|---|---|
| `std` | ✓ | Math functions (`sqrt`, `cbrt`, `hypot`, trigonometry on `Angle`) |
| `libm` | | The same math on `no_std`, through `libm` |
| `full` | ✓ | Every quantity group (below) |
| `mechanics`, `fluids`, `electrical`, `thermal`, `chemistry`, `light`, `radiation`, `information`, `misc` | via `full` | The quantities of that group |
| `serde` | | `Serialize`/`Deserialize` |
| `approx` | | `assert_relative_eq!` and friends on quantities |
| `num-traits` | | `Zero` |
| `bytemuck` | | `Pod`/`Zeroable`/`TransparentWrapper` for zero-copy byte casts |
| `defmt` | | `defmt::Format` for embedded logging |

The 18 core quantities (length, area, volume, mass, duration, speed,
acceleration, force, energy, power, pressure, temperature, angle, frequency,
density, …) are always available. Compiling only the core takes about a fifth
of the time of `full`:

```toml
[dependencies]
unitsnet-rs = { version = "0.1", default-features = false, features = ["std", "electrical"] }
```

For `no_std`, use `default-features = false` (add `libm` for math).

## Code generation

`src/units/`, `src/unit.rs`, `src/cross_type.rs` and `tests/units/` are
generated from the UnitsNet definitions at a pinned commit:

```sh
./scripts/download.sh --current   # definitions at the pinned UnitsNet commit
cargo run -p codegen              # regenerate the code and its tests
```

CI fails if the committed code differs from what the generator produces.
To move to newer upstream definitions, change `UNITSNET_REF` in
`scripts/download.sh` and review the regenerated diff.

## Minimum supported Rust version

1.82 (for float arithmetic in `const fn`).

## License

MIT OR Apache-2.0.
