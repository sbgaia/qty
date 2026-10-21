// Formulas that don't compile, for `tests/formula_errors.rs`. Each function
// is one case, which must give exactly one error.

#![allow(unused)]

use qty::constants::STANDARD_GRAVITY;
use qty::formula::Expr;
use qty::*;

fn wrong_target(m: Mass, v: Speed) {
    let f: Force = formula!(0.5 * m * v * v);
}

fn wrong_plain_number(m: Mass, v: Speed) {
    let x: f64 = formula!(m / v);
}

fn wrong_into(m: Mass, v: Speed) {
    let w: Force = (m * STANDARD_GRAVITY * v).into();
}

fn add(m: Mass, v: Speed) {
    let e: Energy = formula!(m * v * v + m * v);
}

fn subtract(m: Mass, v: Speed) {
    let p = expr!(m * v) - expr!(m);
}

fn square_root(volume: Volume) {
    let side: Length = formula!(volume.sqrt());
}

fn cube_root(m: Mass, v: Speed) {
    let a: Length = formula!((m * v).cbrt());
}

fn main() {}
