//! `src/arith_table.rs`: the generic `*` and `/` of every quantity in
//! formulas (see `src/arith.rs`).
//!
//! The output of `a * b` and `a / b` is, in order:
//!
//! 1. the relation's output, for the relations of `src/cross_type.rs`;
//! 2. the quantity the result's dimensions pick, from [`PREFERRED`] when
//!    several quantities share them;
//! 3. an `Expr`.
//!
//! The relations that agree with the dimensions need nothing in the table.
//! The others are listed per pair of operands.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::generator::{cfg_for, in_formulas, point_delta, CrossKind, CrossOp};
use crate::parser::{Dimensions, QuantityDef};
use crate::si;

/// The quantity the dimensions pick when several quantities have them.
/// Every group of quantities with the same dimensions needs one here.
pub const PREFERRED: &[&str] = &[
    "Area",
    "Density",
    "ElectricConductance",
    "ElectricResistance",
    "Energy",
    "Force",
    "ForceChangeRate",
    "ForcePerLength",
    "Frequency",
    "HeatFlux",
    "Luminance",
    "LuminousIntensity",
    "Power",
    "PowerDensity",
    "Pressure",
    "SpecificEnergy",
    "Speed",
];

/// The order in which the lookup reads the exponents: the rarely nonzero
/// ones first, which keeps the lookup small. Indices into [`Dimensions`]
/// (length, mass, time, current, temperature, amount, luminous intensity).
const ORDER: [usize; 7] = [6, 4, 5, 3, 1, 2, 0];
/// The type parameters of `Dim` in [`ORDER`].
const PARAMS: [&str; 7] = ["J", "H", "N", "I", "M", "T", "L"];

/// A quantity with the generic operators.
struct Operand<'a> {
    name: &'a str,
    dimensions: Dimensions,
}

/// Whether `qty` has the generic operators: an amount that works in
/// formulas.
pub fn has_arith(qty: &QuantityDef, all: &[QuantityDef]) -> bool {
    point_delta(&qty.name).is_none() && in_formulas(qty, all)
}

/// Whether `op` is one of the products and quotients that the generic
/// operators compute, rather than an impl of its own in `cross_type.rs`.
pub fn is_generic(op: &CrossOp, all: &[QuantityDef]) -> bool {
    let arith = |name: &str| all.iter().any(|q| q.name == name && has_arith(q, all));
    matches!(
        op.kind,
        CrossKind::Mul | CrossKind::Div | CrossKind::MulScalar
    ) && arith(&op.lhs)
        && arith(&op.rhs)
}

/// The source of `src/arith_table.rs`.
pub fn generate_arith_table(all: &[QuantityDef], ops: &[CrossOp], max_exponent: i32) -> String {
    let operands: Vec<Operand> = all
        .iter()
        .filter(|q| has_arith(q, all))
        .map(|q| {
            // The operators multiply and divide base values as they are.
            assert!(
                si::base_unit_size(q, all).is_none_or(|size| size.is_one()),
                "codegen: {} has operators, so its base unit must be coherent SI; \
                 list it in parser::COHERENT_BASE_UNITS",
                q.name
            );
            Operand {
                name: &q.name,
                dimensions: q.dimensions,
            }
        })
        .collect();
    let by_name: BTreeMap<&str, &Operand> = operands.iter().map(|o| (o.name, o)).collect();

    // The quantity for each set of dimensions.
    let mut by_dimensions: BTreeMap<Dimensions, Vec<&str>> = BTreeMap::new();
    for o in &operands {
        if o.dimensions != [0; 7] {
            by_dimensions.entry(o.dimensions).or_default().push(o.name);
        }
    }
    let canonical: BTreeMap<Dimensions, &str> = by_dimensions
        .iter()
        .map(|(d, names)| {
            let name = match names.as_slice() {
                [name] => *name,
                _ => {
                    let preferred: Vec<&str> = names
                        .iter()
                        .copied()
                        .filter(|n| PREFERRED.contains(n))
                        .collect();
                    match preferred.as_slice() {
                        [name] => *name,
                        _ => panic!(
                            "codegen: {names:?} have the same dimensions; list exactly one of them \
                             in arith::PREFERRED"
                        ),
                    }
                }
            };
            (*d, name)
        })
        .collect();

    // Relations whose output isn't what the dimensions pick.
    let mut overrides: BTreeMap<(&str, bool, &str), (&str, Option<&str>)> = BTreeMap::new();
    for op in ops.iter().filter(|op| is_generic(op, all)) {
        let mul = !matches!(op.kind, CrossKind::Div);
        let (a, b) = (by_name[op.lhs.as_str()], by_name[op.rhs.as_str()]);
        let d = combine(a.dimensions, b.dimensions, mul);
        let picked = if d == [0; 7] {
            ""
        } else {
            canonical.get(&d).copied().unwrap_or("")
        };
        if picked != op.output {
            assert!(
                !op.output.is_empty(),
                "codegen: {} {} {} is dimensionless, but its dimensions pick {picked}",
                op.lhs,
                if mul { "*" } else { "/" },
                op.rhs
            );
            overrides.insert(
                (op.lhs.as_str(), mul, op.rhs.as_str()),
                (op.output.as_str(), op.factor.as_deref()),
            );
        }
    }
    let lhs_groups = |mul: bool| -> BTreeSet<&str> {
        overrides
            .keys()
            .filter(|k| k.1 == mul)
            .map(|k| k.0)
            .collect()
    };
    let rhs_groups = |mul: bool| -> BTreeSet<&str> {
        overrides
            .keys()
            .filter(|k| k.1 == mul)
            .map(|k| k.2)
            .collect()
    };
    let (lhs_mul, lhs_div) = (lhs_groups(true), lhs_groups(false));
    let (rhs_mul, rhs_div) = (rhs_groups(true), rhs_groups(false));

    let mut out = String::with_capacity(1 << 20);
    w(
        &mut out,
        "// Code generated by `cargo run -p codegen` \u{2014} DO NOT EDIT",
    );
    w(
        &mut out,
        "//! The tables of the generic operators: see `arith.rs`.",
    );
    w(&mut out, "");
    w(&mut out, "use super::*;");
    w(
        &mut out,
        "use crate::formula::{Dim, Dims, DivDim, Expr, MulDim, E};",
    );
    w(&mut out, "use crate::units::*;");
    w(&mut out, "");

    emit_lookup(&mut out, &canonical, max_exponent);
    emit_groups(&mut out, &lhs_mul, &lhs_div, &rhs_mul, &rhs_div, &overrides);

    for o in &operands {
        let group = |set: &BTreeSet<&str>| {
            if set.contains(o.name) {
                format!("R{}", o.name)
            } else {
                "NoGroup".to_owned()
            }
        };
        let lhs = if lhs_mul.contains(o.name) || lhs_div.contains(o.name) {
            format!("L{}", o.name)
        } else {
            "NoGroup".to_owned()
        };
        emit_operand(&mut out, o, &group(&rhs_mul), &group(&rhs_div), &lhs);
    }
    out
}

fn w(out: &mut String, line: &str) {
    out.push_str(line);
    out.push('\n');
}

fn combine(a: Dimensions, b: Dimensions, mul: bool) -> Dimensions {
    std::array::from_fn(|i| if mul { a[i] + b[i] } else { a[i] - b[i] })
}

/// `Canon` for `Dim`: a tree with one level per exponent, whose leaves are
/// the quantities.
fn emit_lookup(out: &mut String, canonical: &BTreeMap<Dimensions, &str>, max_exponent: i32) {
    w(
        out,
        "/// The lookup from dimensions to quantities starts here.",
    );
    w(out, "#[derive(Debug)]");
    w(out, "pub struct Root;");
    let mut nodes: BTreeMap<Vec<i32>, String> = BTreeMap::new();
    nodes.insert(Vec::new(), "Root".to_owned());
    let mut leaves: Vec<(Vec<i32>, Option<&str>)> = canonical
        .iter()
        .map(|(d, q)| (ORDER.iter().map(|&i| d[i]).collect(), Some(*q)))
        .collect();
    // Dimensionless: a plain number.
    leaves.push((vec![0; 7], None));
    leaves.sort();
    for (key, _) in &leaves {
        for len in 1..=key.len() {
            let prefix = key[..len].to_vec();
            if !nodes.contains_key(&prefix) {
                let name = format!("N{}", nodes.len());
                writeln!(out, "#[doc(hidden)]\n#[derive(Debug)]\npub struct {name};").unwrap();
                nodes.insert(prefix, name);
            }
        }
    }
    for (prefix, node) in &nodes {
        if prefix.len() == 7 {
            continue;
        }
        for e in -max_exponent..=max_exponent {
            let mut child = prefix.clone();
            child.push(e);
            let next = nodes.get(&child).map_or("Unnamed", String::as_str);
            writeln!(
                out,
                "impl Step<E<{e}>> for {node} {{ type Next = {next}; }}"
            )
            .unwrap();
        }
    }
    for (key, quantity) in &leaves {
        let node = &nodes[key];
        match quantity {
            Some(q) => {
                let cfg = cfg_for(&[q]);
                if !cfg.is_empty() {
                    w(out, &cfg);
                }
                writeln!(out, "impl<D, V> Leaf<D, V> for {node} {{ type Out = {q}<V>; #[inline(always)] fn make(value: V) -> {q}<V> {{ {q}::from_base(value) }} }}").unwrap();
                // With the quantity's feature off, the dimensions stay an `Expr`.
                if let Some(condition) = cfg.strip_prefix("#[cfg(").and_then(|c| c.strip_suffix(")]")) {
                    w(out, &format!("#[cfg(not({condition}))]"));
                    writeln!(out, "impl<D, V> Leaf<D, V> for {node} {{ type Out = Expr<D, V>; #[inline(always)] fn make(value: V) -> Expr<D, V> {{ Expr::new(value) }} }}").unwrap();
                }
            }
            None => writeln!(out, "impl<D, V> Leaf<D, V> for {node} {{ type Out = V; #[inline(always)] fn make(value: V) -> V {{ value }} }}").unwrap(),
        }
    }
    let mut chain = "Root".to_owned();
    let mut bounds = Vec::new();
    for p in PARAMS {
        bounds.push(format!("{chain}: Step<{p}>"));
        chain = format!("<{chain} as Step<{p}>>::Next");
    }
    bounds.push(format!("{chain}: Leaf<Self, V>"));
    writeln!(
        out,
        "impl<L, M, T, I, H, N, J, V> Canon<V> for Dim<L, M, T, I, H, N, J>\nwhere\n    {},\n{{\n    type Out = <{chain} as Leaf<Self, V>>::Out;\n    #[inline(always)]\n    fn make(value: V) -> Self::Out {{\n        <{chain} as Leaf<Self, V>>::make(value)\n    }}\n}}",
        bounds.join(",\n    ")
    )
    .unwrap();
}

/// The markers of the quantities in relations whose output isn't what the
/// dimensions pick, `Pick` for every pair of them, and `Select` for each
/// relation.
fn emit_groups(
    out: &mut String,
    lhs_mul: &BTreeSet<&str>,
    lhs_div: &BTreeSet<&str>,
    rhs_mul: &BTreeSet<&str>,
    rhs_div: &BTreeSet<&str>,
    overrides: &BTreeMap<(&str, bool, &str), (&str, Option<&str>)>,
) {
    let lhs: BTreeSet<&str> = lhs_mul.union(lhs_div).copied().collect();
    let rhs: BTreeSet<&str> = rhs_mul.union(rhs_div).copied().collect();
    for a in &lhs {
        writeln!(out, "#[doc(hidden)]\n#[derive(Debug)]\npub struct L{a};").unwrap();
    }
    for b in &rhs {
        writeln!(out, "#[doc(hidden)]\n#[derive(Debug)]\npub struct R{b};").unwrap();
    }
    for a in &lhs {
        for (mul, lhs_set, rhs_set) in [(true, lhs_mul, rhs_mul), (false, lhs_div, rhs_div)] {
            let op = if mul { "MulOp" } else { "DivOp" };
            let kind = |g: &str| {
                if mul {
                    format!("Quantity<{g}, G>")
                } else {
                    format!("Quantity<G, {g}>")
                }
            };
            if !lhs_set.contains(a) {
                writeln!(out, "impl<GM, GD> Pick<Quantity<GM, GD>, {op}> for L{a} {{ type Sel = ByDimensions; }}").unwrap();
                continue;
            }
            writeln!(
                out,
                "impl<G> Pick<{}, {op}> for L{a} {{ type Sel = ByDimensions; }}",
                kind("NoGroup")
            )
            .unwrap();
            for b in rhs_set {
                let sel = match overrides.get(&(*a, mul, *b)) {
                    Some(_) => relation_name(a, mul, b),
                    None => "ByDimensions".to_owned(),
                };
                writeln!(
                    out,
                    "impl<G> Pick<{}, {op}> for L{a} {{ type Sel = {sel}; }}",
                    kind(&format!("R{b}"))
                )
                .unwrap();
            }
        }
    }
    for (&(a, mul, b), &(c, factor)) in overrides {
        let name = relation_name(a, mul, b);
        let (op, op_type, op_trait) = if mul {
            ("*", "MulOp", "Mul")
        } else {
            ("/", "DivOp", "Div")
        };
        let (bound, value) = match factor {
            None => (
                format!("core::ops::{op_trait}<Output = V>"),
                format!("a {op} b"),
            ),
            Some(k) => (
                "crate::Number".to_owned(),
                format!("a {op} b * V::from_f64({k})"),
            ),
        };
        writeln!(out, "#[doc(hidden)]\n#[derive(Debug)]\npub struct {name};").unwrap();
        let cfg = cfg_for(&[a, b, c]);
        if !cfg.is_empty() {
            w(out, &cfg);
        }
        writeln!(
            out,
            "impl<V: {bound}, D> Select<{a}<V>, {b}<V>, {op_type}, D> for {name} {{\n    type Out = {c}<V>;\n    #[inline(always)]\n    fn apply(a: V, b: V) -> {c}<V> {{\n        {c}::from_base({value})\n    }}\n}}"
        )
        .unwrap();
    }
}

fn relation_name(a: &str, mul: bool, b: &str) -> String {
    format!("Rel{a}{}{b}", if mul { "Times" } else { "Per" })
}

/// `Arith`, `Mul` and `Div` for one quantity.
fn emit_operand(out: &mut String, o: &Operand, mul_group: &str, div_group: &str, lhs: &str) {
    let name = o.name;
    let dims = o
        .dimensions
        .iter()
        .map(i32::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let cfg = cfg_for(&[name]);
    let cfg_line = if cfg.is_empty() {
        String::new()
    } else {
        format!("{cfg}\n")
    };
    writeln!(
        out,
        "{cfg_line}impl<V> Arith for {name}<V> {{
    type Value = V;
    type Kind = Quantity<{mul_group}, {div_group}>;
    type Dim = Dims<{dims}>;
    #[inline(always)]
    fn into_value(self) -> V {{
        self.into_base()
    }}
    #[inline(always)]
    fn from_value(value: V) -> Self {{
        {name}::from_base(value)
    }}
}}"
    )
    .unwrap();
    for (op_trait, method, op, dim) in [
        ("Mul", "mul", "MulOp", "MulDim"),
        ("Div", "div", "DivOp", "DivDim"),
    ] {
        let result = format!("<Dims<{dims}> as {dim}<R::Dim>>::Out");
        // Without a group, the kind of `R` selects directly.
        let (pick, pick_bound) = if lhs == "NoGroup" {
            ("R::Kind".to_owned(), String::new())
        } else {
            (
                format!("<{lhs} as Pick<R::Kind, {op}>>::Sel"),
                format!("\n    {lhs}: Pick<R::Kind, {op}>,"),
            )
        };
        let select = format!("<{pick} as Select<{name}<V>, R, {op}, {result}>>");
        writeln!(
            out,
            "{cfg_line}impl<V, R> core::ops::{op_trait}<R> for {name}<V>
where
    R: Arith<Value = V>,
    Dims<{dims}>: {dim}<R::Dim>,{pick_bound}
    {pick}: Select<{name}<V>, R, {op}, {result}>,
{{
    type Output = {select}::Out;
    #[inline(always)]
    fn {method}(self, rhs: R) -> Self::Output {{
        {select}::apply(self.into_base(), rhs.into_value())
    }}
}}"
        )
        .unwrap();
        // `expr op quantity`.
        let result = format!("<D as {dim}<Dims<{dims}>>>::Out");
        let select = format!("<ByDimensions as Select<Expr<D, V>, {name}<V>, {op}, {result}>>");
        writeln!(
            out,
            "{cfg_line}impl<D, V> core::ops::{op_trait}<{name}<V>> for Expr<D, V>
where
    D: {dim}<Dims<{dims}>>,
    ByDimensions: Select<Expr<D, V>, {name}<V>, {op}, {result}>,
{{
    type Output = {select}::Out;
    #[inline(always)]
    fn {method}(self, rhs: {name}<V>) -> Self::Output {{
        {select}::apply(self.value(), rhs.into_base())
    }}
}}"
        )
        .unwrap();
    }
}
