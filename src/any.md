For data whose kind is known only at runtime: a configuration value, a
column in a file, a unit picked in a user interface. [`AnyQuantity`] holds a
quantity of any kind (with `f64` storage), [`AnyUnit`] a unit of any kind and
[`QuantityKind`] the kind itself.

```rust
use qty::*;

// The unit decides the kind.
let q: AnyQuantity = "1.5 km".parse()?;
assert_eq!(q.kind(), QuantityKind::Length);
assert_eq!(q.convert_to(AnyUnit::Length(LengthUnit::Meter)), Ok(1500.0));
assert_eq!(q.to_string(), "1500 m");

// Back to the typed world.
let length: Length = q.try_into().unwrap();
assert_eq!(length, Length::from_kilometers(1.5));

// Arithmetic follows the same relations as the typed quantities, checked
// at runtime.
let t: AnyQuantity = Duration::from_seconds(60.0).into();
let v = q.checked_div(t).unwrap();
assert_eq!(v.kind(), QuantityKind::Speed);
assert_eq!(q.checked_add(t), None); // length + duration

// Some units belong to several quantities: "m" is also a minute.
assert_eq!("5 m".parse::<AnyQuantity>(), Err(ParseError::Ambiguous));
let five_minutes = AnyQuantity::parse_as("5 m", QuantityKind::Duration)?;
assert_eq!(five_minutes.as_base(), 300.0);

// Metadata for user interfaces: every kind and its units.
let kind: QuantityKind = "pressure".parse()?;
assert!(kind.info().units.iter().any(|u| u.abbreviation == "bar"));
# Ok::<(), ParseError>(())
```

Text whose unit is not listed for any quantity is read as a unit
expression ([`UnitExpr`](crate::UnitExpr)) and given the kind whose
dimensions it has, if there is exactly one: `"3 kg*m/s^2"` is a force.
