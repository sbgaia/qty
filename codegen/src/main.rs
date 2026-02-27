//! Code generator for `unitsnet-rs`.
//!
//! Reads every `data/quantities/*.json` file and writes a corresponding
//! `src/units/<quantity_snake>.rs` file, plus `src/cross_type.rs` derived
//! from `data/relations/UnitRelations.json`, and `tests/generated.rs` with
//! auto-generated integration tests for every quantity and cross-type op.
//!
//! Run with: `cargo run -p codegen`

mod formula;
mod generator;
mod parser;

use std::collections::HashSet;
use std::path::Path;

use heck::ToSnakeCase;

use generator::CrossOp;
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
    let mut quantity_names: HashSet<String> = HashSet::new();

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
        quantity_names.insert(qty.name.clone());
    }

    // Write src/units.rs — the module declaration file.
    let units_mod_path = Path::new("src/units.rs");
    write_units_mod(units_mod_path, &generated_modules);
    rustfmt(units_mod_path);
    println!("codegen: wrote src/units.rs");

    // Read curated cross-type relationships from UnitRelations.json.
    let cross_ops = parse_relations(&quantity_names);
    let cross_source = generator::generate_cross_type(&cross_ops);
    let cross_path = Path::new("src/cross_type.rs");
    std::fs::write(cross_path, &cross_source)
        .unwrap_or_else(|e| panic!("write {}: {e}", cross_path.display()));
    rustfmt(cross_path);
    println!(
        "codegen: wrote src/cross_type.rs ({} cross-type ops)",
        cross_ops.len()
    );

    // Write tests/generated.rs — integration tests for all quantities and cross-type ops.
    let tests_source = generator::generate_tests(&quantities, &cross_ops);
    let tests_path = Path::new("tests/generated.rs");
    std::fs::write(tests_path, &tests_source)
        .unwrap_or_else(|e| panic!("write {}: {e}", tests_path.display()));
    rustfmt(tests_path);
    println!(
        "codegen: wrote tests/generated.rs ({} quantities, {} cross-type ops)",
        quantities.len(),
        cross_ops.len()
    );
}

// ── Cross-type relations ──────────────────────────────────────────────────────

/// Parses `data/relations/UnitRelations.json` and returns [`CrossOp`]s for every
/// relation whose three quantity names are all present in `known`.
///
/// Each curated entry `"C.Unit = A.Unit * B.Unit"` yields:
/// - `Mul<B> for A = C` and `Mul<A> for B = C`
/// - `Div<B> for C = A` and `Div<A> for C = B` (unless `-- NoInferredDivision`)
///
/// Entries where the output is `1` or `double` (dimensionless) are skipped.
fn parse_relations(known: &HashSet<String>) -> Vec<CrossOp> {
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

    let mut ops: Vec<CrossOp> = Vec::new();

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

        // Extract the quantity name (the part before '.').
        let qty_of = |s: &str| s.split('.').next().unwrap_or("").to_owned();
        let out_qty = qty_of(out_str);
        let a_qty = qty_of(a_str);
        let b_qty = qty_of(b_str);

        // Skip dimensionless outputs.
        if out_qty == "1" || out_qty == "double" {
            continue;
        }

        // Skip if any quantity wasn't downloaded.
        if !known.contains(&out_qty) || !known.contains(&a_qty) || !known.contains(&b_qty) {
            continue;
        }

        let mul = |lhs: &str, rhs: &str| CrossOp {
            lhs: lhs.to_owned(),
            rhs: rhs.to_owned(),
            output: out_qty.clone(),
            trait_name: "Mul".to_owned(),
        };
        let div = |lhs: &str, rhs: &str, out: &str| CrossOp {
            lhs: lhs.to_owned(),
            rhs: rhs.to_owned(),
            output: out.to_owned(),
            trait_name: "Div".to_owned(),
        };

        // A * B = C  and  B * A = C  (commutative; emit both).
        ops.push(mul(&a_qty, &b_qty));
        if a_qty != b_qty {
            ops.push(mul(&b_qty, &a_qty));
        }

        // C / B = A  and  C / A = B  (unless division is suppressed).
        // Skip ops where lhs == rhs: each unit file already emits
        // `impl Div for X { type Output = f64 }` (same-type division).
        if !no_div {
            if out_qty != b_qty {
                ops.push(div(&out_qty, &b_qty, &a_qty));
            }
            if a_qty != b_qty && out_qty != a_qty {
                ops.push(div(&out_qty, &a_qty, &b_qty));
            }
        }
    }

    ops.sort_by(|a, b| {
        a.trait_name
            .cmp(&b.trait_name)
            .then(a.lhs.cmp(&b.lhs))
            .then(a.rhs.cmp(&b.rhs))
    });
    ops.dedup_by(|a, b| a.trait_name == b.trait_name && a.lhs == b.lhs && a.rhs == b.rhs);

    ops
}

// ── Module file ───────────────────────────────────────────────────────────────

/// Writes `src/units.rs` with `pub mod` declarations for every generated module
/// and re-exports all public items.
fn write_units_mod(path: &Path, modules: &[String]) {
    let mut content =
        String::from("// Code generated by `cargo run -p codegen` \u{2014} DO NOT EDIT\n\n");

    for m in modules {
        content.push_str(&format!("pub mod {m};\n"));
    }

    content.push('\n');

    for m in modules {
        content.push_str(&format!("pub use {m}::*;\n"));
    }

    std::fs::write(path, content).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

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
