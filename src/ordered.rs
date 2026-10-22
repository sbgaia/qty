//! Total ordering for float quantities.

use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};

use crate::{Float, Quantity, ToFloat};

/// A float quantity with a total order, equality and hash, for use as a key
/// in `HashMap`, `HashSet`, `BTreeMap` or `BTreeSet`, or with `sort()`.
///
/// Values are compared with [`f64::total_cmp`] after turning `-0.0` into
/// `0.0` and every NaN into one NaN. So `0.0` equals `-0.0`, as with `==`,
/// and NaN is a valid key that sorts after everything else.
///
/// To merge nearly equal measurements into one key, round them to integers
/// instead, with `round_from` (or `Fixed::round_from` to a suitable unit,
/// with the `fixed` feature).
///
/// ```rust
/// use std::collections::BTreeMap;
/// use qty::*;
///
/// let mut by_length = BTreeMap::new();
/// by_length.insert(Ordered(Length::from_meters(2.0)), "b");
/// by_length.insert(Ordered(Length::from_centimeters(50.0)), "a");
/// let names: Vec<_> = by_length.values().copied().collect();
/// assert_eq!(names, ["a", "b"]);
/// ```
#[derive(Debug, Clone, Copy, Default)]
#[repr(transparent)]
pub struct Ordered<Q>(pub Q);

impl<Q: Quantity> Ordered<Q>
where
    Q::Value: Float,
{
    #[inline(always)]
    fn canonical(&self) -> Q::Value {
        let v = self.0.as_base();
        let zero = <Q::Value as crate::Number>::from_f64(0.0);
        #[allow(clippy::eq_op)]
        if v != v {
            <Q::Value as crate::Number>::from_f64(f64::NAN)
        } else if v == zero {
            zero
        } else {
            v
        }
    }
}

impl<Q: Quantity> PartialEq for Ordered<Q>
where
    Q::Value: Float,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl<Q: Quantity> Eq for Ordered<Q> where Q::Value: Float {}
impl<Q: Quantity> PartialOrd for Ordered<Q>
where
    Q::Value: Float,
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<Q: Quantity> Ord for Ordered<Q>
where
    Q::Value: Float,
{
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.canonical().total_cmp(&other.canonical())
    }
}
impl<Q: Quantity> Hash for Ordered<Q>
where
    Q::Value: Float,
{
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.canonical().to_f64().to_bits().hash(state);
    }
}
impl<Q: fmt::Display> fmt::Display for Ordered<Q> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl<Q> From<Q> for Ordered<Q> {
    #[inline(always)]
    fn from(q: Q) -> Self {
        Ordered(q)
    }
}
