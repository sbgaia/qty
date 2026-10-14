# Relations between quantities

The generator reads two lists of relations:

- `data/relations/UnitRelations.json` — UnitsNet's curated relations,
  downloaded by `scripts/download.sh` at the pinned commit (not committed).
- `ExtraRelations.json`, in this directory — relations added here, in the
  same format, for physics UnitsNet does not cover yet: momentum
  (`Mass × Speed = Impulse`), Faraday's and Ohm's laws in field form,
  `τ = R·C`, `n = m·b`, … Each entry `"C.Unit = A.Unit * B.Unit"` gives
  `A * B`, `B * A`, `C / A` and `C / B`.

Both files go through the same expansion and dimension check in the
generator. Where an extra relation would define an operator UnitsNet already
defines, UnitsNet's wins.

Relations are chosen by hand rather than derived from dimensions: several
quantities share dimensions (energy and torque, frequency and rotational
speed), and most dimension-only matches are coincidences.
