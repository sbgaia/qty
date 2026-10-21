//! The hand-written serde impls behave like `#[derive]`: the same output,
//! and the same results and errors for good and bad input, checked against
//! derived copies of the types.

#![cfg(feature = "serde")]

use std::fmt::Debug;

use qty::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// `ours` and `derived` serialize the same, and every input deserializes to
/// equal values or fails with the same message for both. Derived messages
/// name the Rust type, so the mirrors' `Derived` prefix is dropped.
fn same<A, B>(ours: &A, derived: &B, inputs: &[&str])
where
    A: Serialize + DeserializeOwned + Debug,
    B: Serialize + DeserializeOwned + Debug,
{
    assert_eq!(
        serde_json::to_string(ours).unwrap(),
        serde_json::to_string(derived).unwrap()
    );
    for input in inputs {
        let a = serde_json::from_str::<A>(input).map(|v| serde_json::to_string(&v).unwrap());
        let b = serde_json::from_str::<B>(input).map(|v| serde_json::to_string(&v).unwrap());
        match (a, b) {
            (Ok(a), Ok(b)) => assert_eq!(a, b, "{input}"),
            (Err(a), Err(b)) => assert_eq!(
                a.to_string(),
                b.to_string().replace("Derived", ""),
                "{input}"
            ),
            (a, b) => panic!("{input}: {a:?} vs {b:?}"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "Dimensions")]
struct DerivedDimensions {
    length: i8,
    mass: i8,
    time: i8,
    electric_current: i8,
    temperature: i8,
    amount_of_substance: i8,
    luminous_intensity: i8,
}

#[test]
fn dimensions() {
    let ours = Force::<f64>::INFO.dimensions;
    let derived = DerivedDimensions {
        length: 1,
        mass: 1,
        time: -2,
        electric_current: 0,
        temperature: 0,
        amount_of_substance: 0,
        luminous_intensity: 0,
    };
    let json = serde_json::to_string(&ours).unwrap();
    assert_eq!(serde_json::from_str::<Dimensions>(&json).unwrap(), ours);
    same(
        &ours,
        &derived,
        &[
            "[1,1,-2,0,0,0,0]",
            "[1,1]",
            "[]",
            r#"{"length":1}"#,
            r#"{"length":1,"length":2}"#,
            r#"{"zzz":3,"length":1,"mass":1,"time":-2,"electric_current":0,"temperature":0,"amount_of_substance":0,"luminous_intensity":0}"#,
            r#"{"length":"a"}"#,
            r#"{"length":300}"#,
            r#""x""#,
            "null",
            "{}",
        ],
    );
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "Vector3")]
struct DerivedVector3<Q> {
    x: Q,
    y: Q,
    z: Q,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "Vector2")]
struct DerivedVector2<Q> {
    x: Q,
    y: Q,
}

#[test]
fn vectors() {
    let ours = Vector3::new(1.0, 2.0, 3.0).map(Length::from_meters);
    let derived = DerivedVector3 {
        x: Length::from_meters(1.0),
        y: Length::from_meters(2.0),
        z: Length::from_meters(3.0),
    };
    same(
        &ours,
        &derived,
        &[
            r#"{"x":1,"y":2,"z":3}"#,
            "[1,2,3]",
            "[1,2]",
            r#"{"x":1,"y":2}"#,
            r#"{"x":1,"x":2}"#,
            r#"{"x":1,"y":2,"z":3,"w":4}"#,
            "1",
        ],
    );
    // A missing `Option` is `None`, as with the derive.
    same(
        &Vector2::new(Some(1.0), None::<f64>),
        &DerivedVector2 {
            x: Some(1.0),
            y: None::<f64>,
        },
        &[r#"{"x":1}"#, "{}", "[1]", r#"{"y":null}"#],
    );
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "Uncertain")]
struct DerivedUncertain {
    value: f64,
    sigma: f64,
}

#[test]
fn uncertain() {
    same(
        &Uncertain::new(1.5, 0.25),
        &DerivedUncertain {
            value: 1.5,
            sigma: 0.25,
        },
        &[
            r#"{"value":1,"sigma":2}"#,
            "[1,2]",
            "[1]",
            r#"{"value":1}"#,
            r#"{"sigma":2,"value":1}"#,
        ],
    );
}

#[cfg(feature = "dynamic")]
mod dynamic {
    use super::*;
    use serde::de::value::{Error, U32Deserializer};

    /// A few kinds, enough to compare the representation.
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename = "AnyUnit")]
    enum DerivedAnyUnit {
        Length(LengthUnit),
        Mass(MassUnit),
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename = "AnyQuantity")]
    enum DerivedAnyQuantity {
        Length(Length),
        Mass(Mass),
    }

    #[test]
    fn any_unit_and_quantity() {
        let inputs = [
            r#"{"Length":"Kilometer"}"#,
            r#"{"Length":"Nope"}"#,
            r#"{"Length":3}"#,
            r#""Length""#,
            "4",
        ];
        same(
            &AnyUnit::from(LengthUnit::Kilometer),
            &DerivedAnyUnit::Length(LengthUnit::Kilometer),
            &inputs,
        );
        let inputs = [
            r#"{"Length":2500.0}"#,
            r#"{"Length":"x"}"#,
            r#""Length""#,
            "{}",
        ];
        same(
            &AnyQuantity::new(2.5, LengthUnit::Kilometer.into()),
            &DerivedAnyQuantity::Length(Length::from_kilometers(2.5)),
            &inputs,
        );
        for kind in QuantityKind::all() {
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(json, format!("\"{}\"", kind.info().name));
            assert_eq!(serde_json::from_str::<QuantityKind>(&json).unwrap(), kind);
            let unit = kind.base_unit();
            let json = serde_json::to_string(&unit).unwrap();
            assert_eq!(serde_json::from_str::<AnyUnit>(&json).unwrap(), unit);
            let q = AnyQuantity::new(1.5, unit);
            let json = serde_json::to_string(&q).unwrap();
            assert_eq!(json, format!(r#"{{"{}":1.5}}"#, kind.info().name));
            assert_eq!(serde_json::from_str::<AnyQuantity>(&json).unwrap(), q);
        }
    }

    /// Variants are numbered in declaration order among the kinds compiled
    /// in, as the derive numbers them, for formats that write the index.
    #[test]
    fn variant_indices() {
        let mut kinds: Vec<QuantityKind> = QuantityKind::all().collect();
        kinds.sort();
        for (i, kind) in kinds.iter().enumerate() {
            let read = QuantityKind::deserialize(U32Deserializer::<Error>::new(i as u32));
            assert_eq!(read.unwrap(), *kind);
        }
        let past = QuantityKind::deserialize(U32Deserializer::<Error>::new(kinds.len() as u32));
        assert!(past.is_err());
    }
}
