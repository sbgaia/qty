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
qty = "0.4"
```

## Units and arithmetic

Each quantity has a `from_` and an `as_` method per unit, and `new` and
`convert_to` for units chosen at runtime:

```rust
use qty::*;

let tyre = Pressure::from_bars(2.4);
assert!((tyre.as_pounds_force_per_square_inch() - 34.8).abs() < 0.1);
assert_eq!(Pressure::new(2.4, PressureUnit::Bar), tyre);
```

`*` and `/` work between any two quantities, and the result is the
quantity with the dimensions of the product or quotient. Steps in between
don't need a type of their own, and at run time it's the arithmetic on
plain floats:

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

Where several quantities have the same dimensions, the relations between
quantities decide (power over angular speed is a `Torque`, force times
length an `Energy`), and otherwise the common one: an `Energy` rather than
a `Torque`, a `Frequency` rather than a `RotationalSpeed`. Dimensions that
no quantity has, like those of `person * person`, give a
[`formula::Expr`](https://docs.rs/qty/latest/qty/formula/struct.Expr.html),
which keeps multiplying, prints in SI units and converts with `into()`.
`.powi::<N>()` follows the same rules: `side.powi::<2>()` is an `Area`.

Adding a length to a duration is a compile error, and so is asking for the
wrong type: `let f: Force = mass * speed` reports that it found an
`Impulse`. Operations that only make sense for some quantities exist only
there: `Area::sqrt` gives a `Length`, `Angle` has `sin` and `cos`.

## Unit literals

`qty!` writes a quantity as a number and a unit. The unit is checked at
compile time, by `cargo check` too, and the result is the quantity the
code around it expects, from a `let` with a type, an argument, or the other
side of `+` or `==`:

```rust
use qty::*;

let g: Acceleration = qty!(9.81 m/s^2);
let trip = Length::from_kilometers(42.0) + qty!(195 m);
assert_eq!(trip, qty!(42.195 km));

let rise: TemperatureDelta = qty!(5 "°C"); // symbols that aren't Rust tokens
let fall: Length = formula!(0.5 * qty!(9.81 m/s^2) * qty!(2 s).powi::<2>());
assert_eq!(fall.as_meters(), 19.62);
```

Like `parse`, it needs that type: on its own, write `let g: Acceleration =
qty!(9.81 m/s^2)` rather than `let g = qty!(9.81 m/s^2)`. A misspelt unit,
or one with the wrong dimensions (`let g: Acceleration = qty!(9.81 m/s)`),
doesn't compile. The units are constants whose dimensions are in their
types, so a literal costs about as much to compile as a `from_` call.

## Formulas

The operators cover most formulas. `formula!` computes the same arithmetic
and converts the result to the type you ask for, which helps where the
operators don't:

- The target isn't the quantity the dimensions pick: `force * lever` is an
  `Energy`, and `let t: Torque = formula!(force * lever)` a torque.
- Plain numbers work with every storage type: `formula!(0.5 * m * v * v)`
  with `f32` too, where the operators only take `f64` numbers on the left.
- `.sqrt()`, `.cbrt()` and `.powi::<N>()` work on any step, and an error
  names the dimensions in SI base units.

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

A formula whose dimensions don't match the target fails `cargo check`:
``this formula gives `(kg, m<2>, s<-2>)`, not a `Force` `` for an energy,
kg·m²·s⁻². At run time it's the same code as the arithmetic on plain
floats. `qty::constants` has the physical constants.

`expr!` computes a formula and keeps the
[`Expr`](https://docs.rs/qty/latest/qty/formula/struct.Expr.html), even
where a quantity has its dimensions:

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

With the `serde` feature, quantities serialize as plain numbers in the base
unit; `#[serde(with = "qty::serde_str")]` reads and writes text such as
`"30 s"` or `"60 km/h"` instead, which suits configuration files.
Quantities implement `FromStr`, so they also work as command-line
arguments: with clap's derive, a field `distance: Length` takes
`--distance "12 km"` and rejects `"12 kg"` with an error.

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

## Storage types

The storage type is `f64` unless you pick another: `Length<f32>`,
`Mass<i64>`, complex numbers, decimals and rationals behind features, or
your own type through the `Number` and `Float` traits. With decimals and
rationals the conversions are exact, fractions such as km/h to m/s (5/18)
included:

```rust
# #[cfg(feature = "num-rational")] {
use num_rational::Ratio;
use qty::*;

let limit = Speed::from_kilometers_per_hour(Ratio::<i64>::from_integer(50));
assert_eq!(limit.as_meters_per_second(), Ratio::new(125, 9));
assert_eq!(Length::from_feet(Ratio::from_integer(1)).as_meters(), Ratio::new(381, 1250));
# }
```

Complex storage suits AC circuits: a current phasor times a complex
impedance, `ElectricCurrent<Complex64> * ElectricResistance<Complex64>`, is
the voltage phasor.

## Your own units and quantities

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
`"80 g/m^2"` parse as it, it works in `formula!`, and `relation!` connects it
to other quantities.
The conversion factor is worked out at compile time, and a relation whose
dimensions don't add up doesn't compile:

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
| `num-complex` | | Complex storage, `Length<Complex<f64>>` |
| `rust_decimal` | | Decimal storage, `Length<Decimal>` |
| `num-rational` | | Rational storage, `Length<Ratio<i64>>` |

The 18 core quantities (length, mass, duration, speed, force, energy,
pressure, temperature, angle and a few more) are always there. Building
only those takes about a fifth of the time of `full`, so if you need few
groups, list them:

```toml
qty = { version = "0.4", default-features = false, features = ["std", "electrical"] }
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
in [`examples/`](https://github.com/sbgaia/qty/tree/main/examples) run in CI.

`cargo bench -p qty-benches` compares run-time speed with other unit crates,
and its `patterns` benchmark with uom and plain `f64` on common uses.
`python3 scripts/compile_time.py` compares compile times with uom: cold
builds of each library, and rebuilds of a crate that uses it.
`python3 scripts/scaling.py` generates programs of 10 to 1,000 functions
(or more, with `--sizes`) in eight patterns, from plain arithmetic to
`formula!`, unit literals, conversions, generics and serde, and measures
how long they take to compile and to run with qty, with uom and with plain
`f64`. Both scripts build qty and uom with the same features. Run them on
an idle machine; `compile_time.py` takes a few minutes, `scaling.py` about
an hour.

## License

MIT, see [`LICENSE`](https://github.com/sbgaia/qty/blob/main/LICENSE). The unit definitions and descriptions come
from [UnitsNet](https://github.com/angularsen/UnitsNet) by Andreas Gullberg
Larsen and contributors, under the MIT No Attribution license.
