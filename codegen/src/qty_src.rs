//! qty's own unit-expression parser, compiled into the generator, so that
//! each quantity's base unit is written out exactly as qty reads it and the
//! library doesn't parse it again at compile time.
//!
//! `unit_expr.rs` and `info.rs` take `Dimensions`, `ParseError` and
//! `Quantity` from their parent module; the last two are stand-ins here.

#![allow(dead_code, unexpected_cfgs)]

#[path = "../../src/info.rs"]
mod info;
#[path = "../../src/unit_expr.rs"]
mod unit_expr;

pub use info::{Dimensions, QuantityInfo};
pub use unit_expr::UnitExpr;

/// The parser's error. qty's has more variants, which the parser doesn't use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    UnknownUnit,
}

/// What `UnitExpr::factor_to_base` reads from a quantity.
pub trait Quantity {
    const BASE_UNIT_EXPR: Option<UnitExpr>;
    const INFO: QuantityInfo;
}
