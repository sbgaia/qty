# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- 15 relations UnitsNet does not define yet, chosen by hand
  (`data/relations/ExtraRelations.json`, 60 operators): momentum
  (`Mass * Speed = Impulse`), `Φ = V·t`, `Φ = L·I`, `V = L·dI/dt`,
  `I = C·dV/dt`, `τ = R·C`, `J = σ·E`, `E = ρ·J`, `B = μ·H`, `I = H·l`,
  `C = ε·l`, `L = μ·l`, `n = m·b`, `S = n·S_m`, `k = h·l`. They are
  dimension-checked like UnitsNet's.

- Conversions with `std::time::Duration` (`core::time::Duration`):
  `From` into `Duration<f64>`/`Duration<f32>`, `TryFrom` back (negative, NaN
  or too large fail), and exact `TryFrom` both ways with
  `Fixed<Nanosecond, I>`.

- **Generic storage.** Quantities are `#[repr(transparent)]` wrappers generic
  over their number type: `Length<f64>` (default), `Length<f32>`, integers,
  or any type implementing `Float`. Arithmetic compiles to the same machine
  code as the bare numbers.
- **`Fixed<U, I>`**: integers counted in a chosen unit (`Fixed<Millimeter>`),
  with exact cross-unit arithmetic where it is lossless (`mm + m`), exact
  comparisons across units, and explicit `Rounding` wherever precision would be
  lost (float → integer, fine → coarse unit, division). Markers for 1,700+
  units under `unit::<quantity>`, including affine units of point quantities
  (°C, m°C, °F, dBm, …).
- **Physics on `Fixed`:** `mm.mul_to(mm, SquareMillimeter)`,
  `km_per_h.mul_to(hours, Kilometer)` (exact, checked at compile time),
  `mul_to_rounded` and `div_to` with a rounding mode, for every relation
  between quantities, using exact factors between their base units.
- **Points and deltas.** `Temperature`, `AmplitudeRatio` and `PowerRatio` are
  points on a scale: `t1 - t0` is a `TemperatureDelta` (or a `Level` in dB),
  `t + delta` is a temperature, and adding two points does not compile. The
  same rules apply to their `Fixed` forms (`PointUnit`).
- **Display in a unit:** `q.in_unit(unit)` and `q.auto_unit()` (SI prefix in
  steps of 1000), honouring format flags; `UnitEnum` trait for generic code.
- **Parsing unit names:** `"5 kilometres"`, `"2 square_meters"`, case
  insensitive; abbreviations stay exact (`mm` ≠ `Mm`).
- **Math** (`std` feature, or `libm` on `no_std`): `Area::sqrt() → Length`,
  `Volume::cbrt() → Length` and other roots derived from the relations,
  `hypot`, and trigonometry on `Angle` (`sin`, `cos`, `Angle::atan2(y, x)`, …).
- **Ordering and keys:** `total_cmp` for sorting; `Ordered<Q>` makes float
  quantities usable as `HashMap`/`BTreeMap` keys.
- **Vectors:** `Vector2<Q>` and `Vector3<Q>` of any quantity, with the
  physics carried over (`Vector3<Length> / Duration = Vector3<Speed>`,
  `Mass * Vector3<Acceleration> = Vector3<Force>`), `dot`/`cross`/`perp_dot`
  typed by the relations, overflow-free `norm`, `direction`,
  `angle_between`, polar coordinates, `Display`, `serde` and `bytemuck`.
- **Uncertainty propagation:** `Uncertain<F>` (value ± standard uncertainty)
  is a storage type, so `Length<Uncertain>` propagates σ through
  conversions, relations, sums and math functions (first order, independent
  operands).
- `Quantity::const_new(value, unit)` for `f64`: `const` values in any unit,
  bit-identical to `new`.
- `to_f64()` / `to_f32()`, `cast()`, `const fn from_base/as_base`, zero-copy
  slice views, `+=`/`-=`/`*=`/`/=`/`%`, `Sum`, `abs/min/max`, `checked_*`.
- **Cargo features:** `std` (default), `libm`, `serde`, `approx`,
  `num-traits`, `bytemuck`, `defmt`, and quantity groups (`mechanics`,
  `fluids`, `electrical`, `thermal`, `chemistry`, `light`, `radiation`,
  `information`, `misc`; all enabled by the default `full`) — compiling only
  the core quantities takes about a fifth of the time.
- Property tests (`tests/properties.rs`, proptest): `Fixed` conversions,
  comparisons and physics against exact `i128` rational arithmetic over the
  whole `i64` range (biased toward rounding ties), float rounding against
  `f64`'s own, display/parse round trips, parsing arbitrary text, and
  invariants of `Uncertain` and vectors. Fuzz targets (`fuzz/`, cargo-fuzz)
  for parsing and integer arithmetic, run for a minute each in CI.
- Minimum supported Rust version: 1.82.
- CI (formatting, clippy, docs, tests in several feature sets, MSRV,
  `no_std`, feature groups alone, generated code up to date), a README and
  this changelog.

### Changed

- Conversions are folded at code-generation time into a single correctly
  rounded constant (one multiplication or division; two operations for affine
  units), instead of evaluating each upstream formula step by step.
- Generated tests compare every unit against the upstream formula evaluated
  independently, in `f64` and `f32`.
- The generator checks every relation it emits (including inferred
  divisions and roots) against the upstream base dimensions, and fails on an
  inconsistent one. All 120 upstream relations pass. Relations are not
  derived from dimensions alone: 21 groups of quantities share dimensions
  (energy and torque, frequency and rotational speed, …), and most
  dimension-only matches are coincidences (volume × volume is not a warping
  moment of inertia).
- The UnitsNet definitions are pinned to a commit, so regenerating is
  reproducible (`scripts/download.sh`, `UNITSNET_REF`).
- Unit enums are `#[non_exhaustive]`; parsing errors use one shared
  `ParseError` and do not allocate.

### Removed

- The `f32` Cargo feature and the `Num` alias: use `Length<f32>` instead.
- `scalar * quantity` for `f32` (write `quantity * scalar`), so that `f64`
  code needs no type annotations.
- Addition, negation and scaling of `Temperature`, `AmplitudeRatio` and
  `PowerRatio` (see *Points and deltas*).

### Fixed

- Cross-quantity relations ignored that some UnitsNet base units are not
  coherent SI units: `MassFlow * Duration` returned a `Mass` 1000× too large
  (MassFlow is stored in g/s), and volt-ampere-hour energies were off by
  3600×. Relation factors are now computed exactly.
- Adding two temperatures returned nonsense (20 °C + 20 °C = 313 °C); it is now
  a compile error.
