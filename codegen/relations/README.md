# Relations between quantities

The generator reads three lists of relations:

- `data/relations/UnitRelations.json`: UnitsNet's own list, downloaded by
  `scripts/download.sh` at the pinned commit. It isn't committed.
- `ExtraRelations.json` in this directory: relations added here, in the same
  format, for physics UnitsNet doesn't cover yet, such as momentum
  (`Mass × Speed = Impulse`), Faraday's and Ohm's laws in field form,
  `τ = R·C` and `n = m·b`. Each entry `"C.Unit = A.Unit * B.Unit"` gives
  `A * B`, `B * A`, `C / A` and `C / B`.

- `PreferredRelations.json` in this directory: relations that replace
  UnitsNet's for the same operator. UnitsNet has `Length × Force = Torque`;
  here `Force × Length` is the work done, an `Energy`, so that W = F·d has
  the right type. `Torque / Length = Force` and `Torque / Force = Length`
  still come from UnitsNet, and a torque comes from a force and a lever arm
  through `formula!`.

All three files go through the same expansion and dimension check in the
generator. For each operator the first definition wins: preferred, then
UnitsNet's, then the extra ones.

Relations are chosen by hand rather than derived from dimensions: several
quantities share dimensions (energy and torque, frequency and rotational
speed), and most dimension-only matches are coincidences.
