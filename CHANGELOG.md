# Changelog

All notable changes are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions
follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

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
