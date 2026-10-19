# Changelog

All notable changes are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions
follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

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

- The crate builds about 25% faster (debug 15.8 s → 11.7 s, release
  18.1 s → 13.3 s for every quantity on four cores). The `Fixed` unit
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
