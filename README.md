# qty

[![CI](https://github.com/sbgaia/qty/actions/workflows/ci.yml/badge.svg)](https://github.com/sbgaia/qty/actions/workflows/ci.yml)

Typed physical quantities for Rust. A length, a duration and a speed are
different types, and the compiler checks the arithmetic between them. The
128 quantities and about 1,700 units come from the
[UnitsNet](https://github.com/angularsen/UnitsNet) definitions.

```rust
use qty::*;

let distance = Length::from_kilometers(10.0);
let time = Duration::from_minutes(30.0);
let speed: Speed = distance / time;

assert_eq!(format!("{:.1}", speed.in_unit(SpeedUnit::MilePerHour)), "12.4 mph");
```

A quantity stores one number, in its base unit (metres, kilograms,
seconds, ...). It has the same size as that number and compiles to the same
code; converting to another unit is a single multiplication.

```toml
[dependencies]
qty = "0.2"
```

## Units and arithmetic

Each quantity has a `from_` and an `as_` method per unit, and `new` and
`convert_to` for units chosen at runtime:

```rust
use qty::*;

let tyre = Pressure::from_bars(2.4);
assert!((tyre.as_pounds_force_per_square_inch() - 34.8).abs() < 0.1);
assert_eq!(Pressure::new(2.4, PressureUnit::Bar), tyre);

let force: Force = Mass::from_kilograms(1200.0) * Acceleration::from_meters_per_second_squared(2.0);
assert_eq!(force, Force::from_newtons(2400.0));
let energy: Energy = Power::from_kilowatts(3.0) * Duration::from_hours(2.0);
assert!((energy.as_kilowatt_hours() - 6.0).abs() < 1e-12);
```

Adding a length to a duration is a compile error. Operations that only make
sense for some quantities exist only there: `Area::sqrt` gives a `Length`,
`Length::squared` an `Area`, `Angle` has `sin` and `cos`.

The storage type is `f64` unless you pick another: `Length<f32>`,
`Mass<i64>`, or your own float type through the `Float` trait.

Units missing from UnitsNet can be added in your own crate:

```rust
use qty::*;

custom_unit! {
    /// The furlong: 220 yards.
    pub struct Furlong for Length {
        abbreviation: "fur",
        singular: "Furlong",
        plural: "Furlongs",
        factor: 201.168, // metres in one furlong
    }
}

let race = Length::new(5.0, Furlong);
assert_eq!(format!("{:.2}", race.in_unit(LengthUnit::Kilometer)), "1.01 km");
```

and so can whole quantities, with their own units:

```rust
use qty::*;

custom_quantity! {
    /// Printed paper, counted in sheets.
    pub struct Paper;

    pub enum PaperUnit {
        Sheet: "sh",          // the base unit
        Quire: "qr" = 25.0,   // sheets in a quire
        Ream: "rm" = 500.0,
    }
}

let order = Paper::new(3.0, PaperUnit::Ream) + "2 quires".parse::<Paper>()?;
assert_eq!(format!("{:.1}", order.in_unit(PaperUnit::Ream)), "3.1 rm");
# Ok::<(), ParseError>(())
```

A custom quantity can also declare its physical dimensions
(`dimensions { mass: 1, length: -2 }`), so that unit expressions such as
`"80 g/m^2"` parse as it.

## Text

Quantities print in their base unit, or in any unit you ask for. Parsing
accepts abbreviations, unit names and equivalent expressions:

```rust
use qty::*;

let d = Length::from_meters(1234.5);
assert_eq!(d.to_string(), "1234.5 m");
assert_eq!(format!("{:.1}", d.in_unit(LengthUnit::Kilometer)), "1.2 km");
assert_eq!(Length::from_meters(0.0025).auto_unit().to_string(), "2.5 mm");
assert_eq!(Force::from_newtons(1234.5).auto_unit().significant(3).to_string(), "1.23 kN");

assert_eq!("5 km".parse::<Length>(), Ok(Length::from_kilometers(5.0)));
assert_eq!("5 kilometres".parse::<Length>(), Ok(Length::from_kilometers(5.0)));
assert_eq!("3 kg*m/s^2".parse::<Force>(), Ok(Force::from_newtons(3.0)));
```

With the `serde` feature, quantities serialize as plain numbers in the base
unit; `#[serde(with = "qty::serde_str")]` reads and writes text such as
`"30 s"` or `"60 km/h"` instead, which suits configuration files.

## Temperatures

A temperature is a position on a scale, so two temperatures can't be added.
Subtracting them gives a `TemperatureDelta`, which can:

```rust
use qty::*;

let morning = Temperature::from_degrees_celsius(12.0);
let noon = Temperature::from_degrees_celsius(20.0);
let rise: TemperatureDelta = noon - morning;
assert!((rise.as_degrees_fahrenheit() - 14.4).abs() < 1e-12);
assert!((morning + rise).approx_eq_rel(noon, 1e-12));
```

Decibel levels (dBm, dBV) work the same way.

## Integers in a fixed unit

`Fixed<U, I>` counts whole units of `U` in an integer, which is what most
sensors and file formats deliver. Operations that are exact just work;
anything that would round asks how:

```rust
use qty::*;
use qty::unit::length::{Meter, Millimeter};
use qty::unit::temperature::{DegreeCelsius, MillidegreeCelsius};

let gap = Fixed::<Millimeter>::round_from(Length::from_meters(0.0125), Rounding::NearestTiesEven).unwrap();
assert_eq!(gap.count(), 12);
assert_eq!(gap + Fixed::<Meter>::new(1), Fixed::<Millimeter>::new(1_012));
let rounded = Fixed::<Millimeter>::new(1_500).to_unit_rounded::<Meter>(Rounding::Ceil).unwrap();
assert_eq!(rounded.count(), 2);

let reading = Fixed::<MillidegreeCelsius, i32>::new(21_375);
assert!(reading > Fixed::<DegreeCelsius, i32>::new(21));
assert_eq!(reading.display_in::<DegreeCelsius>().to_string(), "21.375 °C");
```

Mixing units where the result would not be exact, such as
`Fixed<Meter> + Fixed<Millimeter>`, does not compile.

## Also included

- `Vector2<Q>` and `Vector3<Q>`, with `dot`, `cross` and `norm` following
  the same rules: a vector of lengths divided by a duration is a vector of
  speeds.
- `Uncertain`, a number type that carries a standard uncertainty through
  every calculation: 100 ± 0.5 m divided by 9.58 ± 0.01 s prints as
  `37.6 ± 0.2 km/h`.
- `AnyQuantity` (feature `dynamic`), for values whose kind is only known at
  runtime: `"1.5 km".parse::<AnyQuantity>()` is a length.
- Metadata for every quantity and unit (`Length::INFO`, `unit.info()`):
  names, abbreviations and dimensions.

## Features

| Feature | Default | |
|---|---|---|
| `std` | yes | Math functions (`sqrt`, `hypot`, trigonometry) |
| `libm` | | The same math without `std` |
| `full` | yes | All quantity groups below |
| `mechanics`, `fluids`, `electrical`, `thermal`, `chemistry`, `light`, `radiation`, `information`, `misc` | through `full` | The quantities of each group |
| `serde` | | Serialization |
| `dynamic` | | `AnyQuantity` and friends |
| `approx` | | `approx` traits, for `assert_relative_eq!` |
| `bytemuck` | | Zero-copy casts to and from bytes |
| `defmt` | | Logging on embedded targets |
| `chrono`, `time` | | Conversions with their duration types |
| `schemars` | | JSON schemas |

The 18 core quantities (length, mass, duration, speed, force, energy,
pressure, temperature, angle and a few more) are always there. Building
only those takes about a fifth of the time of `full`, so if you need few
groups, list them:

```toml
qty = { version = "0.2", default-features = false, features = ["std", "electrical"] }
```

Without default features the crate is `no_std` and never allocates; add
`libm` for the math functions. No feature is required, and the defaults pull
in no dependencies.

The minimum supported Rust version is 1.82.

## Development

The files under `src/units/`, `src/unit.rs`, `src/cross_type.rs`,
`src/any.rs` and `tests/units/` are generated from the UnitsNet definitions
at a pinned commit, and CI checks that they are up to date:

```sh
./scripts/download.sh --current
cargo run -p codegen
```

The generated tests check every unit against the upstream formulas. Property
tests compare the integer arithmetic with exact rational arithmetic, and two
fuzz targets cover parsing and `Fixed` (see `fuzz/README.md`). The examples
in [`examples/`](https://github.com/sbgaia/qty/tree/master/examples) run in CI.

## License

MIT, see [`LICENSE`](https://github.com/sbgaia/qty/blob/master/LICENSE). The unit definitions and descriptions come
from [UnitsNet](https://github.com/angularsen/UnitsNet) by Andreas Gullberg
Larsen and contributors, under the MIT No Attribution license.
