# Changelog

All notable changes are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions
follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.4] - 2026-10-20

### Added

- `formula!` computes formulas whose intermediate results aren't named
  quantities, such as `G * m1 * m2 / (r * r)` or `0.5 * m * v * v`, and
  converts the result to the quantity the context expects. The dimensions
  are checked at compile time, and the generated code is the same as with
  plain floats. Custom quantities with dimensions work in it too.
- `qty::constants`: the SI defining constants and CODATA 2022 values, such
  as `SPEED_OF_LIGHT`, `PLANCK_CONSTANT` and `GRAVITATIONAL_CONSTANT`.
- `qty::formula`, with `Expr`, the value type of formulas, for keeping an
  intermediate result or defining a constant of your own.

## [0.2.3] - 2026-10-20

### Fixed

- Unit expressions parse for quantities whose UnitsNet base unit isn't one,
  such as `"3 mK"` as a `TemperatureDelta`, `"2 Pa*s"` as a
  `DynamicViscosity` and `"5 J/(kg*K)"` as a `SpecificEntropy`. The
  generator now works these base units out from UnitsNet's data instead of
  a hand-written list.

### Changed

- `"3 kV*A"` and other expressions that fit power, apparent power and
  reactive power are now `ParseError::Ambiguous` for `AnyQuantity`, rather
  than a `Power`.
- `ElectricApparentEnergy`, `ElectricReactiveEnergy` and `Information` can
  no longer be used in `relation!`: UnitsNet has no base-unit data for them.

## [0.2.2] - 2026-10-20

### Added

- `custom_quantity!` takes an optional `dimensions { mass: 1, length: -2 }`
  clause. The quantity then has those dimensions in its metadata, and unit
  expressions with them, like `"80 g/m^2"`, parse as it. Its base unit has
  to be such an expression, which is checked at compile time.
- `relation!(Grammage * Area = Mass)` implements `*` or `/` between a
  custom quantity and another quantity, custom or built in. The factor
  between the base units is worked out at compile time, and a relation
  whose dimensions don't match fails to compile.
- `UnitExpr::parse` is a `const fn`.

### Changed

- Plainer docs throughout. The per-unit methods now name the unit by its
  abbreviation (`from_kilometers`: "a value in `km`").
- `RoundError::Inexact` displays as "result is not a whole number".

## [0.2.1] - 2026-10-19

### Fixed

- The README, which is also the crates.io page, told you to depend on
  `qty = "0.1"`.

## [0.2.0] - 2026-10-19

### Added

- `custom_unit!` defines a unit for an existing quantity, with a factor
  and, for points such as temperatures, an offset. `new`, `convert_to` and
  `in_unit` take it like the built-in units, through the new `UnitOf`
  trait, which the generated unit enums implement too.
- `custom_quantity!` defines a new quantity and its unit enum. It gets the
  built-in quantities' arithmetic, conversions, formatting, parsing,
  `Quantity` and metadata, and qty's `serde`, `approx` and `bytemuck`
  integrations; not `from_<unit>` methods, `Fixed`, `AnyQuantity`, defmt or
  schemars.

### Changed

- The crate builds about 25% faster: from 15.8 s to 11.7 s in debug and
  from 18.1 s to 13.3 s in release, for every quantity on four cores. The `Fixed` unit
  markers, such as `unit::length::Millimeter`, made up half of the crate's
  trait impls; they now implement only `Clone` and `Copy`.

### Removed

- `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash` and `Default` on
  the unit markers. `Fixed` values keep all of them.
- `FixedUnit::ABBREVIATION` and `FixedUnit::ABBREVIATIONS`: use
  `U::UNIT.abbreviation()` and `U::UNIT.info().abbreviations`.

## [0.1.0] - 2026-10-16

The first release.

- 128 quantities and about 1,700 units, generated from the
  [UnitsNet](https://github.com/angularsen/UnitsNet) definitions. Each
  quantity wraps one number in its base unit and is generic over the number
  type: `f64` by default, `f32`, integers, or your own `Float` type.
- Arithmetic between quantities (`Length / Duration` is a `Speed`) for
  UnitsNet's relations and 15 more, all checked against the base
  dimensions when the code is generated.
- Conversions reduced to one constant per unit, correctly rounded where the
  factor is exact.
- Temperatures and decibel levels are points: their difference is a delta,
  and adding two of them does not compile.
- `Fixed<U, I>`, integers counted in a chosen unit: exact arithmetic and
  comparisons across units, explicit `Rounding` where precision would be
  lost, physics between counts (`mul_to`, `div_to`), exact decimal display
  and means.
- Display in any unit, with SI prefixes (`auto_unit`), from a list of units
  (`auto_unit_in`) or with significant digits; parsing of abbreviations,
  unit names and unit expressions (`"3 kg*m/s^2"`).
- Math: roots and powers between quantities, `hypot`, trigonometry on
  `Angle`, angle wrapping, rounding to a unit, `clamp`, `lerp` and `mean`.
- `Vector2` and `Vector3` of quantities, and `Uncertain` for propagating
  measurement uncertainty.
- Runtime quantities with the `dynamic` feature: `AnyQuantity`, `AnyUnit`
  and `QuantityKind`.
- Names, abbreviations and dimensions of every quantity and unit.
- `no_std` without allocation. Optional features for `serde` (as numbers, or
  as text through `serde_str`), `approx`, `bytemuck`, `defmt`, `chrono`,
  `time` and `schemars`, and quantity groups to cut compile time.
- Minimum supported Rust version: 1.82.

One UnitsNet abbreviation is corrected: the calorie-based heat transfer
coefficient was labelled `kcal/(h·m²·°C)`, which read 1000 times too large.
