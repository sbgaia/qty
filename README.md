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

Each quantity stores a single number in its base unit (metres, kilograms,
seconds and so on). It takes the same space as that number and compiles to
the same code. Converting to another unit is usually a single
multiplication.

```toml
[dependencies]
qty = "0.5"
```

## Compile times

Checking units costs compile time, because the compiler has to check every
operation. With qty, rebuilding 1,000 functions takes 5 to 6 seconds longer
than with plain `f64` numbers, and 4 to 5 times less time than with
[uom](https://crates.io/crates/uom):

![Compile times of 1,000 functions of unit code with plain f64, qty and uom](https://raw.githubusercontent.com/sbgaia/qty/main/docs/compile-times.svg)

| 1,000 functions | `f64` | qty | uom |
| --- | ---: | ---: | ---: |
| Rebuild, `cargo check` | 1.6 s | 6.3 s | 27.9 s |
| Rebuild, debug build | 2.0 s | 7.9 s | 39.5 s |
| Edit one function, `cargo check` | 0.5 s | 1.4 s | 1.4 s |
| Edit one function, debug build | 0.7 s | 1.9 s | 6.5 s |
| From scratch, debug build | 2.1 s | 20.0 s | 52.5 s |

Each function contains the kind of arithmetic, unit literals and
conversions you find in everyday code. Both libraries are built with every
quantity and `f64` storage. This program only needs qty's core quantities,
and with just those, qty builds from scratch in 10.9 s. The times are the
mean of 10 runs on 8 cores with rustc 1.99. The raw data is in
[`docs/compile-times.json`](https://github.com/sbgaia/qty/blob/main/docs/compile-times.json),
and [`scripts/compile_time.py`](https://github.com/sbgaia/qty/blob/main/scripts/compile_time.py)
runs the same measurement on your machine.

## Units and arithmetic

Each quantity has a `from_` and an `as_` method per unit, and `new` and
`convert_to` for units chosen at runtime:

```rust
use qty::*;

let tyre = Pressure::from_bars(2.4);
assert!((tyre.as_pounds_force_per_square_inch() - 34.8).abs() < 0.1);
assert_eq!(Pressure::new(2.4, PressureUnit::Bar), tyre);
```

`*` and `/` work between any two quantities, and the result has the
dimensions of the product or quotient. Intermediate steps don't need a type
of their own, and at run time the code is the same arithmetic as on plain
floats:

```rust
use qty::*;
use qty::constants::GRAVITATIONAL_CONSTANT as G;

let force: Force = Mass::from_kilograms(1200.0) * Acceleration::from_meters_per_second_squared(2.0);
assert_eq!(force, Force::from_newtons(2400.0));
let energy: Energy = Power::from_kilowatts(3.0) * Duration::from_hours(2.0);
assert!((energy.as_kilowatt_hours() - 6.0).abs() < 1e-12);

let (earth, person) = (Mass::from_kilograms(5.972e24), Mass::from_kilograms(70.0));
let r = Length::from_kilometers(6371.0);
let weight: Force = G * earth * person / (r * r);
assert_eq!(format!("{weight:.0}"), "687 N");

let v = Speed::from_meters_per_second(3.0);
let energy: Energy = 0.5 * person * v.powi::<2>();
assert_eq!(energy.as_joules(), 315.0);
```

Sometimes several quantities share the same dimensions. Then the relations
between quantities decide: power over angular speed is a `Torque`, and force
times length is an `Energy`. Otherwise you get the more common one, so an
`Energy` rather than a `Torque`, and a `Frequency` rather than a
`RotationalSpeed`. When no quantity has the dimensions of a result, as with
`person * person`, you get a
[`formula::Expr`](https://docs.rs/qty/latest/qty/formula/struct.Expr.html).
It keeps multiplying, prints in SI units and converts with `into()`.
`.powi::<N>()` follows the same rules, so `side.powi::<2>()` is an `Area`.

Adding a length to a duration is a compile error, and so is asking for the
wrong type: `let f: Force = mass * speed` reports that it found an
`Impulse`. Operations that only make sense for some quantities only exist
on those. `Area::sqrt` gives a `Length`, and `Angle` has `sin` and `cos`.

## Unit literals

`qty!` writes a quantity as a number followed by a unit. The unit is
checked at compile time, even by `cargo check`. The result is whatever
quantity the surrounding code expects, whether that comes from a typed
`let`, a function argument, or the other side of a `+` or `==`:

```rust
use qty::*;

let g: Acceleration = qty!(9.81 m/s^2);
let trip = Length::from_kilometers(42.0) + qty!(195 m);
assert_eq!(trip, qty!(42.195 km));

let rise: TemperatureDelta = qty!(5 "°C"); // symbols that aren't Rust tokens
let fall: Length = formula!(0.5 * qty!(9.81 m/s^2) * qty!(2 s).powi::<2>());
assert_eq!(fall.as_meters(), 19.62);
```

Like `parse`, it needs to know the target type. On a line of its own,
write `let g: Acceleration = qty!(9.81 m/s^2)`, not
`let g = qty!(9.81 m/s^2)`. A misspelt unit doesn't compile, and neither
does one with the wrong dimensions, such as
`let g: Acceleration = qty!(9.81 m/s)`. A literal is rounded exactly like
the matching `from_` method, so `qty!(12 ft)` is `Length::from_feet(12.0)`
down to the last bit. Units are constants that carry their dimensions and
conversions in their types, so a literal costs about as much to compile as
a `from_` call.

## Formulas

The operators cover most formulas. `formula!` does the same arithmetic and
then converts the result to the type you ask for. That helps in the cases
the operators don't handle:

- The target isn't the quantity the dimensions pick. `force * lever` is an
  `Energy`, but `let t: Torque = formula!(force * lever)` gives a torque.
- Plain numbers work with every storage type. `formula!(0.5 * m * v * v)`
  also works with `f32`, while the operators only accept `f64` numbers on
  the left.
- `.sqrt()`, `.cbrt()` and `.powi::<N>()` work on any intermediate step, and
  errors name the dimensions in SI base units.

```rust
use qty::*;
use qty::constants::GRAVITATIONAL_CONSTANT as G;

let (m, r) = (Mass::from_kilograms(5.972e24), Length::from_kilometers(6371.0));
let escape: Speed = formula!((2.0 * G * m / r).sqrt());
assert_eq!(format!("{:.1}", escape.in_unit(SpeedUnit::KilometerPerSecond)), "11.2 km/s");

# #[cfg(feature = "mechanics")] {
let (force, lever) = (Force::from_newtons(20.0), Length::from_meters(0.5));
let t: Torque = formula!(force * lever);
assert_eq!(t.as_newton_meters(), 10.0);
# }
```

If a formula's dimensions don't match the target, `cargo check` fails. For
an energy, kg·m²·s⁻², it says
``this formula gives `(kg, m<2>, s<-2>)`, not a `Force` ``. At run time the
code is the same as the arithmetic on plain floats. The physical constants
are in `qty::constants`.

`expr!` computes a formula and keeps the
[`Expr`](https://docs.rs/qty/latest/qty/formula/struct.Expr.html), even when
a quantity with those dimensions exists:

```rust
use qty::*;

let (m, v) = (Mass::from_kilograms(70.0), Speed::from_meters_per_second(3.0));
assert_eq!(expr!(m * v * v).to_string(), "630 kg·m²/s²");
```

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

With the `serde` feature, quantities serialize as plain numbers in their
base unit. With `#[serde(with = "qty::serde_str")]` they read and write text
such as `"30 s"` or `"60 km/h"` instead, which is handy for configuration
files. Quantities implement `FromStr`, so they also work as command-line
arguments. With clap's derive, a field `distance: Length` accepts
`--distance "12 km"` and rejects `"12 kg"` with an error.

## Temperatures

A temperature is a point on a scale, so you can't add two temperatures.
Subtracting them gives a `TemperatureDelta`, which you can add:

```rust
use qty::*;

let morning = Temperature::from_degrees_celsius(12.0);
let noon = Temperature::from_degrees_celsius(20.0);
let rise: TemperatureDelta = noon - morning;
assert!((rise.as_degrees_fahrenheit() - 14.4).abs() < 1e-12);
assert!((morning + rise).approx_eq_rel(noon, 1e-12));
```

Decibel levels (dBm, dBV) work the same way.

## Storage types

Quantities store an `f64` unless you choose something else: `Length<f32>`,
`Mass<i64>`, complex numbers, decimals or rationals (behind features), or
your own type through the `Number` and `Float` traits. With decimals and
rationals, conversions are exact, including fractions such as km/h to m/s
(5/18):

```rust
# #[cfg(feature = "num-rational")] {
use num_rational::Ratio;
use qty::*;

let limit = Speed::from_kilometers_per_hour(Ratio::<i64>::from_integer(50));
assert_eq!(limit.as_meters_per_second(), Ratio::new(125, 9));
assert_eq!(Length::from_feet(Ratio::from_integer(1)).as_meters(), Ratio::new(381, 1250));
# }
```

With `f32` and `f64`, a conversion takes one or two operations and no
division on targets with an FMA instruction. That's every aarch64 target,
and x86-64 with `-C target-cpu=x86-64-v3` or `native`. The result is then
correctly rounded, so `Length::from_feet(12.0)` is exactly 3.6576 m. On other
targets it multiplies or divides once by the rounded constant, which stays
within one ulp.

Complex storage is useful for AC circuits. A current phasor times a complex
impedance, `ElectricCurrent<Complex64> * ElectricResistance<Complex64>`,
gives the voltage phasor.

## Your own units and quantities

You can add units that UnitsNet doesn't have in your own crate:

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

You can also add whole quantities, with their own units:

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

A custom quantity can also declare its physical dimensions, like
`dimensions { mass: 1, length: -2 }`. Unit expressions such as `"80 g/m^2"`
then parse as that quantity, it works in `formula!`, and `relation!` can
connect it to other quantities. The conversion factor is computed at compile
time, and a relation whose dimensions don't add up doesn't compile:

```rust
use qty::*;

custom_quantity! {
    /// The weight of paper per area.
    pub struct Grammage;
    dimensions { mass: 1, length: -2 }

    pub enum GrammageUnit {
        GramPerSquareMeter: "g/m²" (plural "GramsPerSquareMeter"),
    }
}

relation!(Grammage * Area = Mass);

let ream: Mass = "80 g/m^2".parse::<Grammage>()? * Area::from_square_meters(31.25);
assert_eq!(format!("{:.1}", ream.in_unit(MassUnit::Kilogram)), "2.5 kg");
# Ok::<(), ParseError>(())
```

## Integers in a fixed unit

With the `fixed` feature, `Fixed<U, I>` counts whole units of `U` in an
integer, which is what most sensors and file formats produce. Exact
operations just work, and anything that would need rounding asks you how to
round:

```rust
# #[cfg(feature = "fixed")] {
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
# }
```

Mixing units where the result wouldn't be exact, such as
`Fixed<Meter> + Fixed<Millimeter>`, doesn't compile. Without the feature you
still have integer storage in the base unit (`Length<i64>`) and
`round_from`.

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
| `fixed` | | `Fixed` integer counts in any unit |
| `dynamic` | | `AnyQuantity` and friends |
| `approx` | | `approx` traits, for `assert_relative_eq!` |
| `bytemuck` | | Zero-copy casts to and from bytes |
| `defmt` | | Logging on embedded targets |
| `chrono`, `time` | | Conversions with their duration types |
| `schemars` | | JSON schemas |
| `num-complex` | | Complex storage, `Length<Complex<f64>>` |
| `rust_decimal` | | Decimal storage, `Length<Decimal>` |
| `num-rational` | | Rational storage, `Length<Ratio<i64>>` |

The 18 core quantities (length, mass, duration, speed, force, energy,
pressure, temperature, angle and a few more) are always available. Building
only those takes about a quarter of the time `full` takes, so if you only
need a few groups, list them:

```toml
qty = { version = "0.5", default-features = false, features = ["std", "electrical"] }
```

Without default features the crate is `no_std` and never allocates. Add
`libm` for the math functions. No feature is required, and the default
features pull in no dependencies.

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
in [`examples/`](https://github.com/sbgaia/qty/tree/main/examples) run in CI.

## License

MIT, see [`LICENSE`](https://github.com/sbgaia/qty/blob/main/LICENSE). The unit definitions and descriptions come
from [UnitsNet](https://github.com/angularsen/UnitsNet) by Andreas Gullberg
Larsen and contributors, under the MIT No Attribution license.
