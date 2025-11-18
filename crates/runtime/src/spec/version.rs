//! Shared YAML spec version validation helper.
//! Ensures presence of integer `version` field and equality to expected version.
//! Centralizes error message strings to keep parser modules consistent.
//!
//! # Purpose
//! Single source of truth for spec version checks so all parsers emit identical
//! error messages and evolve coherently when new versions are introduced.
//!
//! # Usage
//! ```rust,ignore
//! use serde_yaml::Value;
//! // Inside crate: use spec::version::validate_version
//! let v: Value = serde_yaml::from_str("version: 1").unwrap();
//! let ver = allora::spec::version::validate_version(&v).unwrap();
//! assert_eq!(ver, 1);
//! ```
//!
//! # Error Semantics
//! * Missing field -> `missing 'version'`
//! * Non-integer type -> `'version' must be integer`
//! * Unsupported value -> `unsupported version (expected 1)`
//!
//! # Extension Strategy
//! For a future v2 schema, prefer adding `validate_version_v2` or making this
//! function accept an `expected: u32` parameter. Keep old helper available to
//! avoid breaking existing parsers. Example:
//! ```ignore
//! pub(crate) fn validate_version_expected(yaml: &Value, expected: u32) -> Result<u32> { /* ... */ }
//! ```
//!
//! # Design Notes
//! * Returns `u32` (small range, explicit cast from i64): avoids accidental
//!   negative values after validation.
//! * Does not attempt coercion (e.g. strings); strict typing keeps configs clean.
//! * Performs only version validation; structural validation remains in each
//!   parser to keep responsibilities localized.
use crate::error::{Error, Result};
use serde_yaml::Value as YamlValue;

/// Validate that `yaml` contains an integer `version` equal to 1.
///
/// # Errors
/// * Missing field -> `"missing 'version'"`
/// * Non-integer -> `"'version' must be integer"`
/// * Unsupported value -> `"unsupported version (expected 1)"`
pub(crate) fn validate_version(yaml: &YamlValue) -> Result<u32> {
    let version_val = yaml
        .get("version")
        .ok_or_else(|| Error::serialization("missing 'version'"))?;
    if !version_val.is_i64() && !version_val.is_u64() {
        return Err(Error::serialization("'version' must be integer"));
    }
    let v = version_val
        .as_i64()
        .unwrap_or(version_val.as_u64().unwrap_or(0) as i64);
    if v != 1 {
        return Err(Error::serialization("unsupported version (expected 1)"));
    }
    Ok(v as u32)
}
