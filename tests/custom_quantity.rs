//! `custom_quantity!` used as another crate would: through `qty::` paths
//! only, under strict lints, next to a `Result` alias.

#![deny(missing_docs, unsafe_code)]

/// Quantities of this crate.
pub mod paper {
    /// A crate-wide `Result`, which generated code must not pick up.
    #[allow(dead_code)]
    pub type Result<T> = core::result::Result<T, ()>;

    qty::custom_quantity! {
        /// Printed paper,
        /// counted in sheets.
        ///
        /// Not part of the description.
        pub struct Paper;

        /// Paper is sold by the ream.
        pub enum PaperUnit {
            Sheet: "sh",
            Quire: "qr" = 25.0,
            /// 500 sheets.
            Ream: "rm" = 500.0,
            Box: "box" (plural "Boxes") = 5000.0,
        }
    }
}

use paper::{Paper, PaperUnit};
use qty::{InUnit, Quantity, Unit, UnitEnum, Vector2};

#[test]
fn conversions_and_constants() {
    let p = Paper::new(2.0, PaperUnit::Ream);
    assert_eq!(p.as_base(), 1000.0);
    assert_eq!(p.convert_to(PaperUnit::Quire), 40.0);
    assert_eq!(Paper::const_new(2.0, PaperUnit::Ream), p);
    const BOX: Paper = Paper::const_new(1.0, PaperUnit::Box);
    assert_eq!(BOX.convert_to(PaperUnit::Ream), 10.0);
    assert_eq!(Paper::ZERO, Paper::default());
    let small: Paper<f32> = Paper::new(3.0, PaperUnit::Quire);
    assert_eq!(small.as_base(), 75.0_f32);
}

#[test]
fn arithmetic() {
    let a = Paper::new(1.0, PaperUnit::Ream);
    let b = Paper::new(100.0, PaperUnit::Sheet);
    assert_eq!((a + b).as_base(), 600.0);
    assert_eq!((a - b).as_base(), 400.0);
    assert_eq!((a * 2.0).as_base(), 1000.0);
    assert_eq!((2.0 * a).as_base(), 1000.0);
    assert_eq!(a / b, 5.0);
    assert_eq!((-b).as_base(), -100.0);
    let total: Paper = [a, b, b].iter().sum();
    assert_eq!(total.as_base(), 700.0);
    assert_eq!(Paper::mean([a, b]).unwrap().as_base(), 300.0);
    assert!(b < a);
    assert_eq!(a.max(b), a);
    let v = Vector2::new(a, b);
    assert_eq!((v * 2.0).x, a * 2.0);
    let counted: Paper<i64> = Paper::from_base(12);
    assert_eq!(counted.checked_add(Paper::from_base(i64::MAX)), None);
}

#[test]
fn formatting() {
    let p = Paper::new(1250.0, PaperUnit::Sheet);
    assert_eq!(p.to_string(), "1250 sh");
    assert_eq!(format!("{:.1}", p.in_unit(PaperUnit::Ream)), "2.5 rm");
    assert_eq!(format!("{:>5}|", PaperUnit::Box), "  box|");
    let InUnit { value, unit } =
        p.auto_unit_in(&[PaperUnit::Quire, PaperUnit::Ream, PaperUnit::Box]);
    assert_eq!((value, unit), (2.5, PaperUnit::Ream));
    assert_eq!(
        p.in_unit(PaperUnit::Ream).significant(2).to_string(),
        "2.5 rm"
    );
}

#[test]
fn parsing() {
    assert_eq!(
        "3 qr".parse::<Paper>(),
        Ok(Paper::new(75.0, PaperUnit::Sheet))
    );
    assert_eq!(
        "1 Ream".parse::<Paper>(),
        Ok(Paper::new(500.0, PaperUnit::Sheet))
    );
    assert_eq!(
        "2 boxes".parse::<Paper>(),
        Ok(Paper::new(2.0, PaperUnit::Box))
    );
    assert_eq!(
        "4sh".parse::<Paper>(),
        Ok(Paper::new(4.0, PaperUnit::Sheet))
    );
    assert_eq!(" sheet ".parse::<PaperUnit>(), Ok(PaperUnit::Sheet));
    assert_eq!(PaperUnit::from_name("QUIRES"), Some(PaperUnit::Quire));
    // Abbreviations are exact; names are lenient.
    assert_eq!("RM".parse::<PaperUnit>(), Err(qty::ParseError::UnknownUnit));
    assert_eq!("1 m".parse::<Paper>(), Err(qty::ParseError::UnknownUnit));
    // No dimensions, so no unit expressions.
    assert_eq!(
        "1 sh/s*s".parse::<Paper>(),
        Err(qty::ParseError::UnknownUnit)
    );
    assert_eq!("x sh".parse::<Paper>(), Err(qty::ParseError::InvalidNumber));
}

#[test]
fn metadata() {
    let info = Paper::<f64>::INFO;
    assert_eq!(info.name, "Paper");
    assert_eq!(info.description, "Printed paper, counted in sheets.");
    assert_eq!(info.dimensions, qty::Dimensions::DIMENSIONLESS);
    assert_eq!(info.base_unit.abbreviation, "sh");
    let names: Vec<_> = info.units.iter().map(|u| (u.singular, u.plural)).collect();
    assert_eq!(
        names,
        [
            ("Sheet", "Sheets"),
            ("Quire", "Quires"),
            ("Ream", "Reams"),
            ("Box", "Boxes")
        ]
    );
    assert_eq!(<PaperUnit as UnitEnum>::ALL.len(), 4);
    assert_eq!(<Paper as Quantity>::BASE_UNIT, PaperUnit::Sheet);
    assert_eq!(PaperUnit::Box.info().plural, "Boxes");
    assert_eq!(Unit::abbreviation(PaperUnit::Quire), "qr");
}

#[test]
fn generic_code_and_custom_units() {
    fn twice<Q: Quantity<Value = f64>>(q: Q) -> Q {
        Q::from_base(q.as_base() * 2.0)
    }
    assert_eq!(twice(Paper::new(1.0, PaperUnit::Ream)).as_base(), 1000.0);

    qty::custom_unit! {
        /// A bundle of 10 sheets.
        struct Bundle for paper::Paper {
            abbreviation: "bdl",
            singular: "Bundle",
            plural: "Bundles",
            factor: 10.0,
        }
    }
    assert_eq!(Paper::new(3.0, Bundle).convert_to(PaperUnit::Quire), 1.2);
    assert_eq!(
        format!("{:.0}", Paper::new(1.0, PaperUnit::Ream).in_unit(Bundle)),
        "50 bdl"
    );
}

#[test]
fn private_quantities() {
    qty::custom_quantity! {
        struct Steps;
        enum StepUnit {
            Step: "step",
            Flight: "flight" = 12.0,
        }
    }
    assert_eq!(Steps::new(2.0, StepUnit::Flight).as_base(), 24.0);
    assert_eq!(Steps::<f64>::INFO.description, "");
}

#[cfg(feature = "serde")]
#[test]
fn serde() {
    let p = Paper::new(1.5, PaperUnit::Ream);
    assert_eq!(serde_json::to_string(&p).unwrap(), "750.0");
    assert_eq!(serde_json::from_str::<Paper>("750.0").unwrap(), p);
    assert_eq!(serde_json::to_string(&PaperUnit::Box).unwrap(), "\"Box\"");
    assert_eq!(
        serde_json::from_str::<PaperUnit>("\"Quire\"").unwrap(),
        PaperUnit::Quire
    );
    let err = serde_json::from_str::<PaperUnit>("\"Crate\"")
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("unknown variant `Crate`, expected one of `Sheet`, `Quire`, `Ream`, `Box`"),
        "{err}"
    );
    // The same shape as the built-in quantities' derives.
    assert_eq!(
        serde_json::to_string(&(qty::LengthUnit::Meter, PaperUnit::Sheet)).unwrap(),
        r#"["Meter","Sheet"]"#
    );
}

#[cfg(feature = "approx")]
#[test]
fn approx() {
    let a = Paper::new(0.1, PaperUnit::Ream) + Paper::new(0.2, PaperUnit::Ream);
    approx::assert_relative_eq!(a, Paper::new(0.3, PaperUnit::Ream));
    approx::assert_abs_diff_ne!(a, Paper::new(151.0, PaperUnit::Sheet));
}

#[cfg(feature = "bytemuck")]
#[test]
fn bytemuck() {
    let sheets = [
        Paper::new(1.0, PaperUnit::Sheet),
        Paper::new(2.0, PaperUnit::Quire),
    ];
    let raw: &[f64] = bytemuck::cast_slice(&sheets);
    assert_eq!(raw, [1.0, 50.0]);
    assert_eq!(Paper::from_base_slice(raw), sheets);
}

/// A quantity with dimensions, so that unit expressions parse as it.
pub mod grammage {
    qty::custom_quantity! {
        /// The weight of paper per area.
        pub struct Grammage;
        dimensions { mass: 1, length: -2, }

        /// Units of paper weight.
        pub enum GrammageUnit {
            GramPerSquareMeter: "g/m²" (plural "GramsPerSquareMeter"),
            Gsm: "gsm" = 1.0,
            OuncePerSquareYard: "oz/yd²" (plural "OuncesPerSquareYard") = 33.9057474748823,
        }
    }
}

use grammage::{Grammage, GrammageUnit};

#[test]
fn dimensions_enable_unit_expressions() {
    let g = |grams: f64| Grammage::new(grams, GrammageUnit::GramPerSquareMeter);
    // Listed units and names first, then any expression with the dimensions.
    assert_eq!("80 gsm".parse::<Grammage>(), Ok(g(80.0)));
    assert_eq!("80 grams per square meter".parse::<Grammage>(), Ok(g(80.0)));
    assert_eq!("80 g/m^2".parse::<Grammage>(), Ok(g(80.0)));
    assert_eq!("80 g·m⁻²".parse::<Grammage>(), Ok(g(80.0)));
    assert_eq!("0.08 kg/m²".parse::<Grammage>(), Ok(g(80.0)));
    assert_eq!("8 mg/cm^2".parse::<Grammage>(), Ok(g(80.0)));
    // The listed ounce unit agrees with the expression.
    let listed: Grammage = "3 oz/yd²".parse().unwrap();
    let expr: Grammage = "3 oz/yd^2".parse().unwrap();
    assert!((listed / expr - 1.0).abs() < 1e-15);
    // Other dimensions are rejected.
    assert_eq!(
        "80 g/m".parse::<Grammage>(),
        Err(qty::ParseError::UnknownUnit)
    );
    assert_eq!(
        "80 N/m^2".parse::<Grammage>(),
        Err(qty::ParseError::UnknownUnit)
    );
}

#[test]
fn dimensions_metadata() {
    let info = Grammage::<f64>::INFO;
    assert_eq!(info.dimensions, qty::Dimensions::new(-2, 1, 0, 0, 0, 0, 0));
    assert_eq!(info.description, "The weight of paper per area.");
    // Without the clause, a quantity is dimensionless and takes no expressions.
    assert_eq!(
        Paper::<f64>::INFO.dimensions,
        qty::Dimensions::DIMENSIONLESS
    );
}

#[test]
fn private_quantities_with_dimensions() {
    qty::custom_quantity! {
        struct Cadence;
        dimensions { time: -1 }
        enum CadenceUnit {
            Hertz: "Hz",
            PerMinute: "spm" = 1.0 / 60.0,
        }
    }
    let c: Cadence = "180 spm".parse().unwrap();
    assert!((c.as_base() - 3.0).abs() < 1e-12);
    assert!((("180 min^-1".parse::<Cadence>().unwrap() / c) - 1.0).abs() < 1e-12);
}

mod relations {
    use super::grammage::{Grammage, GrammageUnit};
    use qty::{Area, Energy, Mass, Ratio, Speed, TemperatureDelta};

    qty::relation! {
        Grammage * Area = Mass;
        Area * Grammage = Mass;
    }
    // A qualified path in the output, and a quotient.
    qty::relation!(qty::Mass / Grammage = qty::Area);
    // Ratio's base unit is written "", so it uses the "1" override.
    qty::relation!(Grammage * Ratio = Grammage);

    fn gsm(grams: f64) -> Grammage {
        Grammage::new(grams, GrammageUnit::GramPerSquareMeter)
    }

    #[test]
    fn custom_and_builtin() {
        // g/m² × m² is grams: the factor to kilograms comes from the base units.
        let ream = gsm(80.0) * Area::from_square_meters(31.25);
        assert!((ream.as_kilograms() - 2.5).abs() < 1e-12);
        let area = Mass::from_kilograms(2.5) / gsm(80.0);
        assert!((area.as_square_meters() - 31.25).abs() < 1e-12);
        let half = gsm(80.0) * Ratio::from_decimal_fractions(0.5);
        assert_eq!(half, gsm(40.0));
        let small: Mass<f32> = Grammage::<f32>::new(80.0, GrammageUnit::GramPerSquareMeter)
            * Area::<f32>::from_square_meters(31.25);
        assert!((small.as_kilograms() - 2.5).abs() < 1e-6);
    }

    #[test]
    fn custom_and_custom() {
        qty::custom_quantity! {
            struct Cadence;
            dimensions { time: -1 }
            enum CadenceUnit { Hertz: "Hz", StepPerMinute: "spm" = 1.0 / 60.0 }
        }
        qty::custom_quantity! {
            struct Stride;
            dimensions { length: 1 }
            enum StrideUnit { Centimeter: "cm" }
        }
        qty::relation!(Cadence * Stride = Speed);
        let pace: Speed = Cadence::new(180.0, CadenceUnit::StepPerMinute)
            * Stride::new(80.0, StrideUnit::Centimeter);
        assert!((pace.as_meters_per_second() - 2.4).abs() < 1e-12);
    }

    #[test]
    fn builtins_whose_abbreviation_is_not_an_expression() {
        qty::custom_quantity! {
            struct ThermalMass;
            dimensions { length: 2, mass: 1, time: -2, temperature: -1 }
            enum ThermalMassUnit { JoulePerKelvin: "J/K" (plural "JoulesPerKelvin") }
        }
        // TemperatureDelta's base unit is written "∆K".
        qty::relation!(ThermalMass * TemperatureDelta = Energy);
        let heat: Energy = ThermalMass::new(4184.0, ThermalMassUnit::JoulePerKelvin)
            * TemperatureDelta::from_degrees_celsius(10.0);
        assert!((heat.as_kilojoules() - 41.84).abs() < 1e-9);
    }
}
