# Changelog

All notable changes are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions
follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `*` and `/` between any two quantities. The result is the quantity with
  the dimensions of the product or quotient, so formulas compile with the
  plain operators: `let e: Energy = 0.5 * m * v * v`,
  `let f: Force = G * m1 * m2 / (r * r)`. The relations of earlier
  versions keep their outputs (`power / rotational_speed` is a `Torque`);
  between quantities without a relation, the dimensions pick the common
  quantity (`Energy` rather than `Torque`, `Frequency` rather than
  `RotationalSpeed`). Dimensions that no quantity has, like those of
  `m * m`, give an `Expr`.

### Changed

- An `Expr` times or over a quantity is the quantity with the dimensions of
  the result, when there is one: `mass * STANDARD_GRAVITY` is a `Force`.
  `Expr` times `Expr` or a number is still an `Expr`, and `formula!` and
  `expr!` work as before.
- A quantity times or over a plain number of a storage type of your own
  needs that type to implement `Number`. Products of quantities still work
  for any storage type with the operator, as long as the base units need no
  conversion factor.
- Where the quantity with the dimensions of a product is behind a feature
  that isn't enabled, the product is an `Expr`, which converts with `.into()`
  once the feature is on: `mass * speed` is an `Impulse` with `mechanics`,
  and an `Expr` without it.

## [0.3.0] - 2026-10-21

### Migrating from 0.2

- A `Force * Length` product is now an `Energy`. Where you wanted a torque,
  name it: `let t: Torque = formula!(force * lever)`. The same goes for the
  cross product `r × F`.
- A custom storage type that implements `Float` must also implement
  `Number`: move its `from_f64` from the `Float` impl to a new `Number`
  impl. Generic code bounded on `Float` keeps working; bound it on `Number`
  instead to accept decimals and rationals too.
- `expr + Expr::new(1.0)` needs the type of the constant, for example from
  a `let` with a type annotation. Code that names
  `FromFormula`, `HalfE` or `ThirdE` in its own bounds may need the new
  `Gives`, `AddUnits`, `SubUnits` or `Rem` bounds; code that only calls
  `formula!` and `expr!` is unaffected.

### Added

- `qty!` writes a quantity with its unit, checked at compile time:
  `let g: Acceleration = qty!(9.81 m/s^2)`, `qty!(5 "°C")` for symbols that
  aren't Rust tokens. It gives the quantity the context expects, works in
  `formula!`, and rejects unknown units and wrong dimensions in
  `cargo check`. Angles count, as when parsing text, so `rev/min` is a
  `RotationalSpeed` and not a `Frequency`.
- `expr!` computes a formula like `formula!` but gives the `Expr` itself,
  so `let x = expr!(m * m)` needs no type annotation.
- `Expr` implements `Display` (and `LowerExp`, `UpperExp`), in SI units:
  `4 kg²`, `9 kg·m²/s²`, `6.6743e-11 m³/(kg·s²)`. The text parses back as a
  quantity with the same dimensions.
- `.powi::<N>()` in formulas and on `Expr`, for whole powers, negative ones
  included: `formula!(0.5 * m * v.powi::<2>())`.
- `Expr::dimensions`, and the `Dimension`, `Exponent`, `MulE` and `PowDim`
  traits behind them.
- Complex, decimal and rational storage, each behind a feature:
  `num-complex` (`Complex<f32>`, `Complex<f64>`), `rust_decimal`
  (`Decimal`) and `num-rational` (`Ratio<i32>`, `Ratio<i64>`,
  `Ratio<i128>`). Conversions, relations and `formula!` work with them.
  Decimals and rationals get each conversion constant as written in
  decimal (a foot is exactly 0.3048 m), and constants with no exact decimal,
  such as 5/9 for °F and 5/18 for km/h, as fractions, so rationals convert
  exactly. A constant that doesn't fit the type panics: the solar mass in
  `Decimal`, the electronvolt in `Ratio<i64>` (use `Ratio<i128>`).
- The `Number` trait: the arithmetic operators and `from_f64`, which is
  all that conversions need.
- Number literals in `formula!` work with every storage type, so
  `formula!(0.5 * m * v * v)` also works for `Uncertain` and the types
  above.

### Changed

- **Breaking:** `Force * Length` and `Length * Force` give an `Energy`, the
  work done, instead of a `Torque`, so W = F·d has the right type, and
  `Energy / Length = Force` and `Energy / Force = Length` are new. They need
  only the core quantities, where the torque needed `mechanics`. For a
  torque, write `let t: Torque = formula!(force * lever)`; `Torque / Length`,
  `Torque / Force` and `Torque * RotationalSpeed` are unchanged. The vector
  cross product `r × F` follows: it gives energies, converted with
  `formula!`. `AnyQuantity` arithmetic changes the same way.
- **Breaking:** conversions (`new`, `convert_to`, `in_unit`, the
  `from_`/`as_` methods, `UnitOf`) require `Number` instead of `Float`, and
  `Float` requires `Number`. A custom `Float` type must now implement
  `Number` too, which takes the `from_f64` method moved there from `Float`.
  Ordering, parsing, rounding and `auto_unit` still require `Float`.
- Formula errors name the dimensions in SI base units, as tuples such as
  `(kg, m<2>, s<-2>)` for kg·m²·s⁻², where they compared `Dim<E<…>, …>`
  types: ``this formula gives `(kg, m<2>, s<-2>)`, not a `Force` ``,
  ``can't add `(kg, m, s<-1>)` to `(kg, m<2>, s<-2>)` ``, and
  ``the square root of `(m<3>,)` has a fractional exponent``, once per
  root. The units are the types in `qty::formula::si`, and `InBaseUnits`
  gives them for a `Dim`.
- **Breaking**, for code naming the formula traits: `FromFormula` and
  `From<Expr>` are implemented for an `Expr` of any dimensions, with a
  `Gives` bound that checks them, and `Expr + Expr` and `Expr - Expr` take
  an `Expr` of any dimensions on the right, with an `AddUnits` or
  `SubUnits` bound, so `expr + Expr::new(1.0)` needs the type of the
  constant. `HalfE` and `ThirdE` are implemented for every exponent, with
  the remainder in `Rem`; `SqrtDim` and `CbrtDim` check it.
- `formula!`, `expr!` and `qty!` expand faster: a name or number followed
  by an operator takes one step, and a unit of one symbol, as in
  `qty!(12.5 km)`, needs no `const` block. Longer formulas fit in the
  default recursion limit.
- The `serde` feature builds faster: qty implements the traits by hand,
  with the same format and the same errors, and depends on `serde_core`
  (which `serde` re-exports, from 1.0.220) instead of `serde` with its
  `derive` feature. qty no longer waits for `syn` and `serde_derive` to
  compile, also when another crate enables `derive`.

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
