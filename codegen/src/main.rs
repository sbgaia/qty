//! Code generator for `unitsnet-rs`.
//!
//! Reads every `data/quantities/*.json` file and writes a corresponding
//! `src/units/<quantity_snake>.rs` file, plus `src/cross_type.rs` derived
//! from `data/relations/UnitRelations.json`. Integration tests are written to
//! `tests/units/<quantity_snake>.rs` (mirroring `src/units/`) and
//! `tests/cross_type.rs`.
//!
//! Run with: `cargo run -p codegen`

mod expr;
mod formula;
mod generator;
mod parser;

use std::collections::{HashMap, HashSet};
use std::path::Path;

use heck::ToSnakeCase;

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

    // Sort for deterministic output order.
    entries.sort_by_key(|e| e.path());

    // Parse all quantities up front so we can pass them to `generate_tests` later.
    let quantities: Vec<QuantityDef> = entries
        .iter()
        .map(|e| {
            let path = e.path();
            println!("codegen: processing {}", path.display());
            parser::parse(&path)
        })
        .collect();

    let mut generated_modules: Vec<String> = Vec::new();

    for qty in &quantities {
        let source = generator::generate(qty);

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

    // Write src/units.rs — the module declaration file.
    let names: Vec<&str> = quantities.iter().map(|q| q.name.as_str()).collect();
    let units_mod_path = Path::new("src/units.rs");
    write_units_mod(units_mod_path, &generated_modules, &names);
    rustfmt(units_mod_path);
    println!("codegen: wrote src/units.rs");

    let unit_mod_path = Path::new("src/unit.rs");
    write_unit_markers_mod(unit_mod_path, &generated_modules);
    rustfmt(unit_mod_path);
    println!("codegen: wrote src/unit.rs");

    // Read curated cross-type relationships from UnitRelations.json.
    let cross_ops = parse_relations(&quantities);
    let cross_source = generator::generate_cross_type(&cross_ops);
    let cross_path = Path::new("src/cross_type.rs");
    std::fs::write(cross_path, &cross_source)
        .unwrap_or_else(|e| panic!("write {}: {e}", cross_path.display()));
    rustfmt(cross_path);
    println!(
        "codegen: wrote src/cross_type.rs ({} cross-type ops)",
        cross_ops.len()
    );

    // Write one test file per quantity into tests/units/ (mirrors src/units/)
    // and one cross-type test file into tests/.
    let tests_units_dir = Path::new("tests/units");
    std::fs::create_dir_all(tests_units_dir).expect("create tests/units/");

    let mut test_modules: Vec<String> = Vec::new();
    for qty in &quantities {
        let module_name = qty.name.to_snake_case();
        let path = tests_units_dir.join(format!("{module_name}.rs"));
        let source = generate_quantity_tests(qty);
        std::fs::write(&path, &source).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
        rustfmt(&path);
        test_modules.push(module_name);
    }

    // Cargo only discovers top-level `tests/*.rs` binaries. Write `tests/units.rs`
    // as the entry point that `mod`-includes every quantity test file.
    let units_test_mod_path = Path::new("tests/units.rs");
    write_test_units_mod(units_test_mod_path, &test_modules);
    rustfmt(units_test_mod_path);

    let cross_test_path = Path::new("tests/cross_type.rs");
    let cross_test_source = generate_cross_type_tests(&cross_ops);
    std::fs::write(cross_test_path, &cross_test_source)
        .unwrap_or_else(|e| panic!("write {}: {e}", cross_test_path.display()));
    rustfmt(cross_test_path);

    println!(
        "codegen: wrote tests/units.rs + tests/units/ ({} quantity files) + tests/cross_type.rs",
        quantities.len()
    );
}

// ── Cross-type relations ──────────────────────────────────────────────────────

/// Parses `data/relations/UnitRelations.json` and returns [`CrossOp`]s for every
/// relation whose quantities were all generated.
///
/// Each curated entry `"C.Unit = A.Unit * B.Unit"` yields:
/// - `Mul<B> for A = C` and `Mul<A> for B = C`
/// - `Div<B> for C = A` and `Div<A> for C = B` (unless `-- NoInferredDivision`)
///
/// Dimensionless entries (`"1 = A.Unit * B.Unit"`) yield `A * B → T`,
/// `B * A → T`, and `1 / A = B`, `1 / B = A`.
///
/// The relation holds in the units it names. Operators work on base-unit
/// values, so the factor between those units and the base units is computed
/// exactly; it is 1 for coherent SI base units, in which case the operator is
/// a bare multiplication or division.
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

    let entries: Vec<String> = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()));

    // "Quantity.Unit" → exact to-base factor (linear units only).
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
    let op = |kind, lhs: &str, rhs: &str, output: &str, factor: &Const| CrossOp {
        kind,
        lhs: lhs.to_owned(),
        rhs: rhs.to_owned(),
        output: output.to_owned(),
        factor: (!factor.is_one()).then(|| factor.to_string()),
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

        // A * B = C  and  B * A = C  (commutative; emit both).
        ops.push(op(CrossKind::Mul, &a_qty, &b_qty, &out_qty, &k));
        if a_qty != b_qty {
            ops.push(op(CrossKind::Mul, &b_qty, &a_qty, &out_qty, &k));
        }

        // C / B = A  and  C / A = B  (unless division is suppressed).
        // Skip ops where lhs == rhs: same-type division already yields `T`.
        if !no_div {
            if out_qty != b_qty {
                ops.push(op(CrossKind::Div, &out_qty, &b_qty, &a_qty, &k_inv));
            }
            if a_qty != b_qty && out_qty != a_qty {
                ops.push(op(CrossKind::Div, &out_qty, &a_qty, &b_qty, &k_inv));
            }
        }
    }

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

// ── Module file ───────────────────────────────────────────────────────────────

/// Writes `src/unit.rs`: `unit::<quantity>` re-exports each quantity's
/// `Fixed` unit markers under a short path.
fn write_unit_markers_mod(path: &Path, modules: &[String]) {
    let mut content = String::from(
        "// Code generated by `cargo run -p codegen` \u{2014} DO NOT EDIT\n\
         //! Unit markers for [`Fixed`](crate::Fixed) integer quantities, one module per\n\
         //! quantity: `unit::length::Millimeter`, `unit::duration::Nanosecond`, \u{2026}\n\n",
    );
    for m in modules {
        content.push_str(&format!("pub use crate::units::{m}::unit as {m};\n"));
    }
    std::fs::write(path, content).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

/// Writes `src/units.rs` with `pub mod` declarations for every generated module
/// and re-exports each quantity type and its unit enum.
fn write_units_mod(path: &Path, modules: &[String], names: &[&str]) {
    let mut content =
        String::from("// Code generated by `cargo run -p codegen` \u{2014} DO NOT EDIT\n\n");

    for m in modules {
        content.push_str(&format!("pub mod {m};\n"));
    }

    content.push('\n');

    for (m, name) in modules.iter().zip(names) {
        content.push_str(&format!("pub use {m}::{{{name}, {name}Unit}};\n"));
    }

    std::fs::write(path, content).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

/// Writes `tests/units.rs` — the entry point for the `units` integration test binary.
///
/// Cargo only auto-discovers top-level `tests/*.rs` binaries; files inside
/// `tests/units/` are submodules.  Because `tests/units.rs` is the crate root
/// its `mod foo;` declarations resolve relative to `tests/` (not `tests/units/`),
/// so we use `#[path]` to point each module at the correct file.
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
fn check_fixed<U: unitsnet_rs::FixedUnit>() {
    use unitsnet_rs::{Fixed, Rounding};
    let f = Fixed::<U>::new(7);
    assert_eq!(f.to_f64(), U::to_quantity(7.0_f64));
    assert_eq!(Fixed::<U>::round_from(f.to_f64(), Rounding::NearestTiesEven), Ok(f));
    assert_eq!(Fixed::<U, i32>::round_from_count(2.5_f64, Rounding::NearestTiesEven), Ok(Fixed::new(2)));
    assert_eq!(format!("{f}"), format!("7 {}", U::ABBREVIATION));
}

/// Checks that exact integer rescaling between a `Fixed` unit and its base
/// unit agrees with float arithmetic. (Not for units related to the base by
/// a power of π: that ratio is irrational and refused at compile time.)
fn check_fixed_rescale<U: unitsnet_rs::FixedUnit>() {
    use unitsnet_rs::{Fixed, Quantity, RoundError, Rounding};
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

"#;

// ── Formatting ────────────────────────────────────────────────────────────────

/// Runs `rustfmt` on a single file so generated source always passes `cargo fmt --check`.
///
/// Failures are intentionally ignored: if `rustfmt` is unavailable the build still succeeds,
/// and CI's `cargo fmt --check` step will catch any remaining drift.
fn rustfmt(path: &Path) {
    std::process::Command::new("rustfmt")
        .arg("--edition=2021")
        .arg(path)
        .status()
        .ok();
}
