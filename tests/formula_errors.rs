//! The compiler errors of formulas that don't compile: each one names the
//! dimensions in SI base units, once.
//!
//! Builds `tests/formula_errors/cases.rs` against this crate with `cargo
//! check`, and checks the messages, which `qty` sets with
//! `#[diagnostic::on_unimplemented]`.

#![cfg(all(feature = "full", any(feature = "std", feature = "libm")))]

use std::path::Path;
use std::process::Command;

/// The first line of each expected error, in the order of the cases.
const EXPECTED: &[&str] = &[
    "error[E0277]: this formula gives `(kg, m<2>, s<-2>)`, not a `qty::Force`",
    "error[E0277]: this formula gives `(kg, m<-1>, s)`, not a `f64`",
    "error[E0277]: this formula gives `(kg<2>, m, s<-2>)`, not a `qty::Force`",
    "error[E0277]: can't add `(kg, m, s<-1>)` to `(kg, m<2>, s<-2>)`",
    "error[E0277]: can't subtract `(kg,)` from `(kg, m, s<-1>)`",
    "error[E0277]: the square root of `(m<3>,)` has a fractional exponent",
    "error[E0277]: the cube root of `(kg, m, s<-1>)` has a fractional exponent",
];

/// The labels under the code, which say what was expected.
const LABELS: &[&str] = &[
    "a `qty::Force` is `(kg, m, s<-2>)`",
    "a `f64` is `()`",
    "the dimensions differ",
    "a square root needs even exponents",
    "a cube root needs exponents that are multiples of 3",
];

#[test]
fn formula_errors_name_dimensions_in_base_units() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    // Outside the source tree, for the lock file and the build.
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("formula_errors");
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        format!(
            "[package]\nname = \"formula_errors\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n\
             [dependencies]\nqty = {{ path = {:?} }}\n\n[workspace]\n",
            root.display().to_string()
        ),
    )
    .unwrap();
    std::fs::copy(
        root.join("tests/formula_errors/cases.rs"),
        dir.join("src/main.rs"),
    )
    .unwrap();

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let output = Command::new(cargo)
        .args(["check", "--offline", "--quiet"])
        .arg("--manifest-path")
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", dir.join("target"))
        .env("CARGO_TERM_COLOR", "never")
        .output()
        .unwrap();
    assert!(!output.status.success(), "the cases compiled");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // One error per case, with no follow-up errors.
    let errors: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with("error["))
        .collect();
    assert_eq!(errors, EXPECTED, "\n{stderr}");
    for label in LABELS {
        assert!(stderr.contains(label), "no `{label}` in\n{stderr}");
    }
}
