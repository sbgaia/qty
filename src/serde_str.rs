//! (De)serialize quantities as text with a unit, such as `"30 s"` or
//! `"1.5 km"`, for configuration files and other human-edited data. Use it
//! with `#[serde(with = "unitsnet_rs::serde_str")]`.
//!
//! Plain `serde` stores a quantity as its bare base-unit number (`30.0`),
//! which is compact but leaves the unit implicit. With this module:
//!
//! * **reading** accepts any unit and any spelling the quantity parses —
//!   abbreviations (`"0.5 min"`) and names (`"2 minutes"`);
//! * **writing** uses the base unit at full precision (`"30 s"`), so values
//!   round-trip exactly.
//!
//! It works for every type that displays and parses: float quantities,
//! [`Fixed`](crate::Fixed) counts (`"12 mm"`) and unit enums. For optional
//! fields use [`option`].
//!
//! ```rust
//! use serde::{Deserialize, Serialize};
//! use unitsnet_rs::*;
//! use unitsnet_rs::unit::length::Millimeter;
//!
//! #[derive(Serialize, Deserialize, Debug, PartialEq)]
//! struct Config {
//!     #[serde(with = "unitsnet_rs::serde_str")]
//!     timeout: Duration,
//!     #[serde(with = "unitsnet_rs::serde_str")]
//!     gap: Fixed<Millimeter>,
//!     #[serde(with = "unitsnet_rs::serde_str::option", default)]
//!     max_speed: Option<Speed>,
//! }
//!
//! let config: Config =
//!     serde_json::from_str(r#"{ "timeout": "2 minutes", "gap": "12 mm", "max_speed": "36 km/h" }"#).unwrap();
//! assert_eq!(config.timeout, Duration::from_seconds(120.0));
//! assert_eq!(config.max_speed, Some(Speed::from_meters_per_second(10.0)));
//! assert_eq!(
//!     serde_json::to_string(&config).unwrap(),
//!     r#"{"timeout":"120 s","gap":"12 mm","max_speed":"10 m/s"}"#
//! );
//!
//! let err = serde_json::from_str::<Config>(r#"{ "timeout": "2 parsecs", "gap": "1 mm" }"#).unwrap_err();
//! assert!(err.to_string().contains("unknown unit"));
//! ```
//!
//! Available with the `serde` feature.

use core::fmt;
use core::marker::PhantomData;
use core::str::FromStr;

use serde::de::{self, Deserializer, Visitor};
use serde::Serializer;

use crate::ParseError;

/// Serializes `value` as text: its `Display` form, e.g. `"1.5 m"`.
pub fn serialize<Q: fmt::Display, S: Serializer>(
    value: &Q,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_str(value)
}

/// Deserializes text with a unit, e.g. `"1.5 km"` or `"1.5 kilometres"`.
pub fn deserialize<'de, Q, D>(deserializer: D) -> Result<Q, D::Error>
where
    Q: FromStr<Err = ParseError>,
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(TextVisitor(PhantomData))
}

struct TextVisitor<Q>(PhantomData<Q>);

impl<Q: FromStr<Err = ParseError>> Visitor<'_> for TextVisitor<Q> {
    type Value = Q;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a number with a unit, like \"1.5 km\"")
    }

    fn visit_str<E: de::Error>(self, s: &str) -> Result<Q, E> {
        s.parse()
            .map_err(|e: ParseError| E::custom(format_args!("{e} in {s:?}")))
    }
}

/// The same for `Option<…>` fields: `None` is `null` (or a missing field,
/// with `#[serde(default)]`).
pub mod option {
    use super::*;
    use serde::{Deserialize, Serialize};

    /// Serializes `Some(value)` as text and `None` as `null`.
    pub fn serialize<Q: fmt::Display, S: Serializer>(
        value: &Option<Q>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(q) => serializer.serialize_some(&AsText(q)),
            None => serializer.serialize_none(),
        }
    }

    /// Deserializes text with a unit, or `null`.
    pub fn deserialize<'de, Q, D>(deserializer: D) -> Result<Option<Q>, D::Error>
    where
        Q: FromStr<Err = ParseError>,
        D: Deserializer<'de>,
    {
        Ok(Option::<FromText<Q>>::deserialize(deserializer)?.map(|t| t.0))
    }

    struct AsText<'a, Q>(&'a Q);

    impl<Q: fmt::Display> Serialize for AsText<'_, Q> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            super::serialize(self.0, serializer)
        }
    }

    struct FromText<Q>(Q);

    impl<'de, Q: FromStr<Err = ParseError>> Deserialize<'de> for FromText<Q> {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            super::deserialize(deserializer).map(FromText)
        }
    }
}
