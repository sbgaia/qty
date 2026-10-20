//! Code generator for `qty`.
//!
//! Reads `data/quantities/*.json` and writes one `src/units/<quantity>.rs`
//! per quantity, plus `src/cross_type.rs` from
//! `data/relations/UnitRelations.json`. The tests go to
//! `tests/units/<quantity>.rs` and `tests/cross_type.rs`.
//!
//! Run with: `cargo run -p codegen`

mod case;
mod expr;
mod formula;
mod generator;
mod parser;
mod si;

use std::collections::{HashMap, HashSet};
use std::path::Path;

use case::ToSnakeCase;

use expr::{Const, Form};
use generator::{generate_cross_type_tests, generate_quantity_tests, CrossKind, CrossOp};
use parser::QuantityDef;

fn main() {
    let def_dir = Path::new("data/quantities");
    let out_dir = Path::new("src/units");

    std::fs::create_dir_all(out_dir).expect("create src/units/");

    let mut entries: Vec<_> = std::fs::read_dir(def_dir)
        .expect("read data/quantities/")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();

    // Sorted, so the output doesn't depend on the directory order.
    entries.sort_by_key(|e| e.path());

    // Parsed up front: `generate_tests` needs all of them.
    let quantities: Vec<QuantityDef> = entries
        .iter()
        .map(|e| {
            let path = e.path();
            println!("codegen: processing {}", path.display());
            parser::parse(&path)
        })
        .collect();
    si::check_system(&quantities);

    let mut generated_modules: Vec<String> = Vec::new();

    for qty in &quantities {
        let source = generator::generate(qty, &quantities);

        let module_name = qty.name.to_snake_case();
        let out_path = out_dir.join(format!("{module_name}.rs"));

        std::fs::write(&out_path, &source)
            .unwrap_or_else(|e| panic!("write {}: {e}", out_path.display()));
        rustfmt(&out_path);

        println!(
            "codegen: wrote {} ({} units)",
            out_path.display(),
            qty.units.len()
        );
        generated_modules.push(module_name);
    }

    // src/units.rs declares the modules.
    let names: Vec<&str> = quantities.iter().map(|q| q.name.as_str()).collect();
    let units_mod_path = Path::new("src/units.rs");
    write_units_mod(units_mod_path, &generated_modules, &names);
    rustfmt(units_mod_path);
    println!("codegen: wrote src/units.rs");

    let unit_mod_path = Path::new("src/unit.rs");
    write_unit_markers_mod(unit_mod_path, &generated_modules, &names);
    rustfmt(unit_mod_path);
    println!("codegen: wrote src/unit.rs");

    let cross_ops = parse_relations(&quantities);
    let any_source = generator::generate_any(&quantities, &cross_ops);
    let any_path = Path::new("src/any.rs");
    std::fs::write(any_path, any_source)
        .unwrap_or_else(|e| panic!("write {}: {e}", any_path.display()));
    rustfmt(any_path);
    println!("codegen: wrote src/any.rs");

    let exponents_path = Path::new("src/formula/exponents.rs");
    std::fs::create_dir_all("src/formula").expect("create src/formula/");
    std::fs::write(
        exponents_path,
        generator::generate_formula_exponents(&quantities),
    )
    .unwrap_or_else(|e| panic!("write {}: {e}", exponents_path.display()));
    rustfmt(exponents_path);
    println!("codegen: wrote src/formula/exponents.rs");

    let cross_source = generator::generate_cross_type(&cross_ops);
    let cross_path = Path::new("src/cross_type.rs");
    std::fs::write(cross_path, &cross_source)
        .unwrap_or_else(|e| panic!("write {}: {e}", cross_path.display()));
    rustfmt(cross_path);
    println!(
        "codegen: wrote src/cross_type.rs ({} cross-type ops)",
        cross_ops.len()
    );

    // One test file per quantity, and one for the relations.
    let tests_units_dir = Path::new("tests/units");
    std::fs::create_dir_all(tests_units_dir).expect("create tests/units/");

    let mut test_modules: Vec<String> = Vec::new();
    for qty in &quantities {
        let module_name = qty.name.to_snake_case();
        let path = tests_units_dir.join(format!("{module_name}.rs"));
        let source = generate_quantity_tests(qty, &quantities);
        std::fs::write(&path, &source).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
        rustfmt(&path);
        test_modules.push(module_name);
    }

    let units_test_mod_path = Path::new("tests/units.rs");
    write_test_units_mod(units_test_mod_path, &test_modules);
    rustfmt(units_test_mod_path);

    let cross_test_path = Path::new("tests/cross_type.rs");
    let cross_test_source = generate_cross_type_tests(&cross_ops, &quantities);
    std::fs::write(cross_test_path, &cross_test_source)
        .unwrap_or_else(|e| panic!("write {}: {e}", cross_test_path.display()));
    rustfmt(cross_test_path);

    println!(
        "codegen: wrote tests/units.rs + tests/units/ ({} quantity files) + tests/cross_type.rs",
        quantities.len()
    );
}

/// The products and quotients in `data/relations/UnitRelations.json` whose
/// quantities were all generated.
///
/// An entry `"C.Unit = A.Unit * B.Unit"` gives `A * B = C` and `B * A = C`,
/// and also `C / B = A` and `C / A = B` unless marked `NoInferredDivision`.
/// A dimensionless entry, `"1 = A.Unit * B.Unit"`, gives `A * B` and `B * A`
/// as plain numbers, and `1 / A = B` and `1 / B = A`.
///
/// An entry holds in the units it names, while the operators work on base
/// units, so the factor between the two is computed exactly. For coherent
/// SI base units it is 1 and the operator is a bare multiplication or
/// division.
fn parse_relations(quantities: &[QuantityDef]) -> Vec<CrossOp> {
    let path = Path::new("data/relations/UnitRelations.json");
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!(
                "warning: {} not found; no cross-type ops generated",
                path.display()
            );
            return Vec::new();
        }
    };

    let mut entries: Vec<String> = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()));
    // Relations added in this repository, after upstream's so that
    // upstream's win on any conflicting operator.
    let extra = Path::new("codegen/relations/ExtraRelations.json");
    let extra: Vec<String> = serde_json::from_str(
        &std::fs::read_to_string(extra)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", extra.display())),
    )
    .unwrap_or_else(|e| panic!("failed to parse {}: {e}", extra.display()));
    entries.extend(extra);

    // The exact to-base factor of each "Quantity.Unit" (linear units only).
    let factors: HashMap<String, Const> = quantities
        .iter()
        .flat_map(|q| {
            q.units.iter().filter_map(move |u| match &u.to_base {
                Form::Affine { a, b } if b.is_zero() => {
                    Some((format!("{}.{}", q.name, u.singular), a.clone()))
                }
                _ => None,
            })
        })
        .collect();
    let known: HashSet<&str> = quantities.iter().map(|q| q.name.as_str()).collect();

    let mut ops: Vec<CrossOp> = Vec::new();
    // Every `C = A * B` relation, as `(A, B, C, k)`.
    let mut products: Vec<(String, String, String, Const)> = Vec::new();
    let op = |kind, lhs: &str, rhs: &str, output: &str, factor: &Const| CrossOp {
        kind,
        lhs: lhs.to_owned(),
        rhs: rhs.to_owned(),
        output: output.to_owned(),
        factor: (!factor.is_one()).then(|| factor.to_string()),
        k: factor.clone(),
    };

    for entry in &entries {
        // Strip the optional annotation.
        let (body, no_div) = match entry.strip_suffix(" -- NoInferredDivision") {
            Some(s) => (s, true),
            None => (entry.as_str(), false),
        };

        // Parse "C.Unit = A.Unit * B.Unit".
        let Some((out_str, rhs)) = body.split_once(" = ") else {
            continue;
        };
        let Some((a_str, b_str)) = rhs.split_once(" * ") else {
            continue;
        };
        let qty_of = |s: &str| s.split('.').next().unwrap_or("").to_owned();
        let (a_qty, b_qty) = (qty_of(a_str), qty_of(b_str));
        if !known.contains(a_qty.as_str()) || !known.contains(b_qty.as_str()) {
            continue;
        }
        let dimensionless = matches!(out_str, "1" | "double" | "QuantityValue");
        let out_qty = qty_of(out_str);
        if !dimensionless && !known.contains(out_qty.as_str()) {
            continue;
        }

        // C_base = k · A_base · B_base, where k = f_C / (f_A · f_B).
        let lookup = |s: &str| factors.get(s).cloned();
        let f_out = if dimensionless {
            Some(Const::one())
        } else {
            lookup(out_str)
        };
        let (Some(fa), Some(fb), Some(fc)) = (lookup(a_str), lookup(b_str), f_out) else {
            eprintln!("codegen: note: skipping relation with a non-linear unit: {entry}");
            continue;
        };
        let k = fc / (fa * fb);
        let k_inv = k.recip();

        if dimensionless {
            if !k.is_one() {
                eprintln!("codegen: note: skipping non-coherent dimensionless relation: {entry}");
                continue;
            }
            ops.push(op(CrossKind::MulScalar, &a_qty, &b_qty, "", &k));
            if a_qty != b_qty {
                ops.push(op(CrossKind::MulScalar, &b_qty, &a_qty, "", &k));
            }
            ops.push(op(CrossKind::Recip, "", &a_qty, &b_qty, &k));
            if a_qty != b_qty {
                ops.push(op(CrossKind::Recip, "", &b_qty, &a_qty, &k));
            }
            continue;
        }

        products.push((a_qty.clone(), b_qty.clone(), out_qty.clone(), k.clone()));

        // A * B = C and B * A = C.
        ops.push(op(CrossKind::Mul, &a_qty, &b_qty, &out_qty, &k));
        if a_qty != b_qty {
            ops.push(op(CrossKind::Mul, &b_qty, &a_qty, &out_qty, &k));
        }

        // C / B = A and C / A = B, unless suppressed. When the two sides are
        // the same type, division already gives a plain number.
        if !no_div {
            if out_qty != b_qty {
                ops.push(op(CrossKind::Div, &out_qty, &b_qty, &a_qty, &k_inv));
            }
            if a_qty != b_qty && out_qty != a_qty {
                ops.push(op(CrossKind::Div, &out_qty, &a_qty, &b_qty, &k_inv));
            }
        }
    }

    // Roots. `C = A * A` gives `C.sqrt() = A`, with `C_base = k · A_base²`.
    // `C = A * B` with `A = B * B` gives `C.cbrt() = B`, `C_base = k_A · k · B_base³`.
    let squares: Vec<(String, String, Const)> = products
        .iter()
        .filter(|(a, b, _, _)| a == b)
        .map(|(a, _, c, k)| (c.clone(), a.clone(), k.clone()))
        .collect();
    for (c, a, k) in &squares {
        ops.push(op(CrossKind::Sqrt, c, "", a, k));
    }
    for (a, b, c, k) in &products {
        for (x, y) in [(a, b), (b, a)] {
            if let Some((_, root, k_sq)) = squares.iter().find(|(sq, root, _)| sq == x && root == y)
            {
                ops.push(op(
                    CrossKind::Cbrt,
                    c,
                    "",
                    root,
                    &(k_sq.clone() * k.clone()),
                ));
            }
        }
    }

    check_dimensions(&ops, quantities);

    // Keep the first relation for each operator signature; later duplicates
    // would be conflicting impls.
    let mut seen = HashSet::new();
    ops.retain(|o| seen.insert((o.kind.symbol(), o.lhs.clone(), o.rhs.clone())));
    ops.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then(a.lhs.cmp(&b.lhs))
            .then(a.rhs.cmp(&b.rhs))
    });

    ops
}

/// Panics unless every operation is dimensionally consistent, checked with
/// UnitsNet's `BaseDimensions` (`[A] + [B] = [C]` for `A * B = C`). This
/// catches bad upstream relations and mistakes in how they're expanded here,
/// such as the inferred divisions and roots.
fn check_dimensions(ops: &[CrossOp], quantities: &[QuantityDef]) {
    let dims = |name: &str| -> parser::Dimensions {
        quantities
            .iter()
            .find(|q| q.name == name)
            .unwrap_or_else(|| panic!("relation names unknown quantity {name}"))
            .dimensions
    };
    let mut errors = Vec::new();
    for op in ops {
        let (expected, actual): (parser::Dimensions, parser::Dimensions) = match op.kind {
            CrossKind::Mul => (
                dims(&op.output),
                dimension_sum(dims(&op.lhs), dims(&op.rhs), 1),
            ),
            CrossKind::Div => (
                dims(&op.output),
                dimension_sum(dims(&op.lhs), dims(&op.rhs), -1),
            ),
            CrossKind::MulScalar => ([0; 7], dimension_sum(dims(&op.lhs), dims(&op.rhs), 1)),
            CrossKind::Recip => (dims(&op.output), dimension_sum([0; 7], dims(&op.rhs), -1)),
            CrossKind::Sqrt => (
                dims(&op.lhs),
                dimension_sum(dims(&op.output), dims(&op.output), 1),
            ),
            CrossKind::Cbrt => {
                let o = dims(&op.output);
                (dims(&op.lhs), dimension_sum(dimension_sum(o, o, 1), o, 1))
            }
        };
        if expected != actual {
            errors.push(format!(
                "{:?} {} {} {} = {}: dimensions {actual:?} vs {expected:?}",
                op.kind,
                op.lhs,
                op.kind.symbol(),
                op.rhs,
                op.output
            ));
        }
    }
    assert!(
        errors.is_empty(),
        "dimensionally inconsistent relations:\n{}",
        errors.join("\n")
    );
}

/// `a + sign · b`, exponent by exponent.
fn dimension_sum(a: parser::Dimensions, b: parser::Dimensions, sign: i32) -> parser::Dimensions {
    std::array::from_fn(|i| a[i] + sign * b[i])
}

/// Writes `src/unit.rs`, which re-exports each quantity's `Fixed` units
/// under a short path: `unit::<quantity>`.
fn write_unit_markers_mod(path: &Path, modules: &[String], names: &[&str]) {
    let mut content = String::from(
        "// Code generated by `cargo run -p codegen` \u{2014} DO NOT EDIT\n\
         //! Unit markers for [`Fixed`](crate::Fixed) integer quantities, one module per\n\
         //! quantity: `unit::length::Millimeter`, `unit::duration::Nanosecond`, \u{2026}\n\n",
    );
    for (m, name) in modules.iter().zip(names) {
        content.push_str(&gate(name));
        content.push_str(&format!("pub use crate::units::{m}::unit as {m};\n"));
    }
    std::fs::write(path, content).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

/// The `#[cfg]` and docs.rs badge lines for `name`'s feature group. Empty
/// for core quantities.
fn gate(name: &str) -> String {
    match generator::group(name) {
        None => String::new(),
        Some(g) => {
            format!("#[cfg(feature = \"{g}\")]\n#[cfg_attr(docsrs, doc(cfg(feature = \"{g}\")))]\n")
        }
    }
}

/// Writes `src/units.rs`, which declares every quantity module and
/// re-exports the quantities and their unit enums.
fn write_units_mod(path: &Path, modules: &[String], names: &[&str]) {
    let mut content =
        String::from("// Code generated by `cargo run -p codegen` \u{2014} DO NOT EDIT\n\n");

    for (m, name) in modules.iter().zip(names) {
        content.push_str(&gate(name));
        content.push_str(&format!("pub mod {m};\n"));
    }

    content.push('\n');

    for (m, name) in modules.iter().zip(names) {
        content.push_str(&gate(name));
        content.push_str(&format!("pub use {m}::{{{name}, {name}Unit}};\n"));
    }

    std::fs::write(path, content).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

/// Writes `tests/units.rs`, the root of the `units` test binary. Cargo only
/// finds the top-level `tests/*.rs`, so the files in `tests/units/` are its
/// modules. Their `mod` paths would resolve from `tests/`, hence `#[path]`.
fn write_test_units_mod(path: &Path, modules: &[String]) {
    let mut content =
        String::from("// Code generated by `cargo run -p codegen` \u{2014} DO NOT EDIT\n\n");
    content.push_str(TEST_HELPERS);
    for m in modules {
        content.push_str(&format!("#[path = \"units/{m}.rs\"]\nmod {m};\n"));
    }
    std::fs::write(path, content).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

/// Shared helpers for the generated per-quantity tests.
const TEST_HELPERS: &str = r#"/// Generated conversions vs. the upstream formulas evaluated step by step:
/// both are within a few ulps of the exact value.
const F64_TOL: f64 = 1e-14;
/// `from_unit(v).as_unit()` returns `v` to within an ulp or two.
const ROUND_TRIP_TOL: f64 = 1e-15;
/// f32 storage vs. the f64 reference (f32 has ~7 significant digits).
const F32_TOL: f64 = 1e-6;

/// Checks a `Fixed` unit marker: the float bridge, and that exact integer
/// rescaling to and from the base unit agrees with float arithmetic.
fn check_fixed<U: qty::FixedUnit>() {
    use qty::{Fixed, Rounding, Unit};
    let f = Fixed::<U>::new(7);
    assert_eq!(f.to_f64(), U::to_quantity(7.0_f64));
    assert_eq!(Fixed::<U>::round_from(f.to_f64(), Rounding::NearestTiesEven), Ok(f));
    assert_eq!(Fixed::<U, i32>::round_from_count(2.5_f64, Rounding::NearestTiesEven), Ok(Fixed::new(2)));
    assert_eq!(format!("{f}"), format!("7 {}", U::UNIT.abbreviation()));
}

/// Checks that exact integer rescaling between a `Fixed` unit and its base
/// unit agrees with float arithmetic. (Not for units related to the base by
/// a power of π: that ratio is irrational and refused at compile time.)
fn check_fixed_rescale<U: qty::FixedUnit>() {
    use qty::{Fixed, Quantity, RoundError, Rounding};
    let f = Fixed::<U>::new(7);

    // Integer rescaling is exact; compare with floats where those are
    // unambiguous (not near a tie, comfortably inside i64).
    let check = |exact: Result<i64, RoundError>, float: f64| {
        let frac = (float - float.trunc()).abs();
        if float.abs() < 1e15 && (frac - 0.5).abs() > 1e-6 {
            assert_eq!(exact, Ok(float.round() as i64), "{float}");
        } else if float.abs() > 1e19 {
            assert_eq!(exact, Err(RoundError::Overflow), "{float}");
        }
    };
    let to_base = f.to_unit_rounded::<U::Base>(Rounding::NearestTiesAway).map(|b| b.count());
    check(to_base, f.to_f64().as_base());
    let base = Fixed::<U::Base>::new(7_000_000_000);
    let from_base = base.to_unit_rounded::<U>(Rounding::NearestTiesAway).map(|u| u.count());
    check(from_base, U::from_quantity(U::Quantity::<f64>::from_base(7e9)));
}

/// Asserts `actual` is within `rel_tol` (relative) of `expected`.
#[track_caller]
fn assert_close(actual: f64, expected: f64, rel_tol: f64) {
    let scale = actual.abs().max(expected.abs());
    assert!(
        actual == expected || (actual - expected).abs() <= rel_tol * scale,
        "{actual:e} != {expected:e} (relative error {:e} > {rel_tol:e})",
        (actual - expected).abs() / scale
    );
}


/// Every abbreviation of every unit that parses as a unit expression must
/// mean the same as the generated conversion. Returns how many were checked.
#[allow(dead_code)]
fn check_unit_exprs<Q: qty::Quantity<Value = f64>>(new: fn(f64, Q::Unit) -> Q) -> usize {
    use qty::{Unit, UnitEnum, UnitExpr};
    let mut checked = 0;
    for &unit in <Q::Unit as UnitEnum>::ALL {
        if new(0.0, unit).as_base() != 0.0 {
            continue; // affine (°C): expressions mean differences
        }
        let one = new(1.0, unit).as_base();
        for abbreviation in unit.info().abbreviations {
            let Some(factor) = UnitExpr::parse(abbreviation)
                .ok()
                .and_then(|e| e.factor_to_base::<Q>())
            else {
                continue;
            };
            // UnitsNet defines lb/(in·s²) through a rounded standard gravity
            // (386.0886 in/s² instead of 386.088 582 677…): 4.5e-8 apart.
            let tolerance = if *abbreviation == "lb/(in·s²)" { 1e-7 } else { 1e-12 };
            assert!(
                (factor - one).abs() <= tolerance * one.abs(),
                "{} {:?} ({abbreviation:?}): expression {factor}, conversion {one}",
                Q::INFO.name,
                unit
            );
            checked += 1;
        }
    }
    checked
}
"#;

/// Formats a generated file, so it passes `cargo fmt --check`. Failures are
/// ignored: without rustfmt the code is still valid, and CI's format check
/// would catch the difference.
fn rustfmt(path: &Path) {
    std::process::Command::new("rustfmt")
        .arg("--edition=2021")
        .arg(path)
        .status()
        .ok();
}

#[cfg(test)]
mod dimension_tests {
    use super::*;

    fn quantity(name: &str, dimensions: parser::Dimensions) -> QuantityDef {
        QuantityDef {
            name: name.to_owned(),
            base_unit: String::new(),
            doc: String::new(),
            units: Vec::new(),
            dimensions,
        }
    }

    fn op(kind: CrossKind, lhs: &str, rhs: &str, output: &str) -> CrossOp {
        CrossOp {
            kind,
            lhs: lhs.to_owned(),
            rhs: rhs.to_owned(),
            output: output.to_owned(),
            factor: None,
            k: Const::one(),
        }
    }

    // L, M, T, …
    fn quantities() -> Vec<QuantityDef> {
        vec![
            quantity("Length", [1, 0, 0, 0, 0, 0, 0]),
            quantity("Area", [2, 0, 0, 0, 0, 0, 0]),
            quantity("Volume", [3, 0, 0, 0, 0, 0, 0]),
            quantity("Duration", [0, 0, 1, 0, 0, 0, 0]),
            quantity("Frequency", [0, 0, -1, 0, 0, 0, 0]),
            quantity("Speed", [1, 0, -1, 0, 0, 0, 0]),
        ]
    }

    #[test]
    fn consistent_relations_pass() {
        check_dimensions(
            &[
                op(CrossKind::Mul, "Length", "Length", "Area"),
                op(CrossKind::Div, "Length", "Duration", "Speed"),
                op(CrossKind::MulScalar, "Duration", "Frequency", ""),
                op(CrossKind::Recip, "", "Duration", "Frequency"),
                op(CrossKind::Sqrt, "Area", "", "Length"),
                op(CrossKind::Cbrt, "Volume", "", "Length"),
            ],
            &quantities(),
        );
    }

    #[test]
    #[should_panic(expected = "Mul Length * Duration = Speed")]
    fn inconsistent_product_fails() {
        check_dimensions(
            &[op(CrossKind::Mul, "Length", "Duration", "Speed")],
            &quantities(),
        );
    }

    #[test]
    #[should_panic(expected = "Sqrt Volume sqrt  = Length")]
    fn inconsistent_root_fails() {
        check_dimensions(
            &[op(CrossKind::Sqrt, "Volume", "", "Length")],
            &quantities(),
        );
    }
}
