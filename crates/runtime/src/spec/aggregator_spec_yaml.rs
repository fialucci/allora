//! YAML parser for [`AggregatorSpec`] (v1).
//!
//! # Accepted Shape (Informal)
//! ```yaml
//! version: 1
//! aggregator:
//!   id: finality_quorum                # optional, non-empty if present
//!   correlation_header: block_hash     # required, non-empty
//!   completion: chain.validator_quorum # required, non-empty (registry name)
//!   strategy: allora.emit_signal       # optional, non-empty if present
//!   store: chain.persistent_finality   # optional, non-empty if present
//! ```
//!
//! # Validation Responsibilities
//! * Enforce presence + string type + non-empty strings for `correlation_header` and `completion`.
//! * Enforce non-empty `id` / `strategy` / `store` if present.
//! * Validate top-level `version` via shared `validate_version` (must equal 1).
//! * Reject non-mapping `aggregator` root.
//! * Defer uniqueness of `id` to the collection builder.
//!
//! # Non-Goals
//! * Verifying that referenced registry names resolve — handled at build time.

use crate::error::{Error, Result};
use crate::spec::version::validate_version;
use crate::spec::AggregatorSpec;
use serde_yaml::Value as YamlValue;

pub struct AggregatorSpecYamlParser;

impl AggregatorSpecYamlParser {
    pub fn parse_value(yaml: &YamlValue) -> Result<AggregatorSpec> {
        let _v = validate_version(yaml)?;
        let root = yaml
            .get("aggregator")
            .ok_or_else(|| Error::serialization("missing 'aggregator'"))?;
        if !root.is_mapping() {
            return Err(Error::serialization("'aggregator' must be a mapping"));
        }
        let header = required_non_empty_str(root, "correlation_header")?;
        let completion = required_non_empty_str(root, "completion")?;
        let id = optional_non_empty_str(root, "id")?;
        let strategy = optional_non_empty_str(root, "strategy")?;
        let store = optional_non_empty_str(root, "store")?;

        let mut spec = match id {
            Some(id) => AggregatorSpec::with_id(id, header, completion),
            None => AggregatorSpec::new(header, completion),
        };
        if let Some(s) = strategy {
            spec = spec.set_strategy(s);
        }
        if let Some(s) = store {
            spec = spec.set_store(s);
        }
        Ok(spec)
    }

    pub fn parse_str(raw: &str) -> Result<AggregatorSpec> {
        let val: YamlValue = serde_yaml::from_str(raw)
            .map_err(|e| Error::serialization(format!("yaml parse error: {e}")))?;
        Self::parse_value(&val)
    }
}

fn required_non_empty_str(root: &YamlValue, field: &str) -> Result<String> {
    let val = root
        .get(field)
        .ok_or_else(|| Error::serialization(format!("aggregator.{field} required")))?;
    let s = val
        .as_str()
        .ok_or_else(|| Error::serialization(format!("aggregator.{field} must be string")))?;
    if s.is_empty() {
        return Err(Error::serialization(format!(
            "aggregator.{field} must not be empty"
        )));
    }
    Ok(s.to_string())
}

fn optional_non_empty_str(root: &YamlValue, field: &str) -> Result<Option<String>> {
    let Some(val) = root.get(field) else {
        return Ok(None);
    };
    // Permit explicit `null` to mean "absent".
    if val.is_null() {
        return Ok(None);
    }
    let s = val
        .as_str()
        .ok_or_else(|| Error::serialization(format!("aggregator.{field} must be string")))?;
    if s.is_empty() {
        return Err(Error::serialization(format!(
            "aggregator.{field} must not be empty"
        )));
    }
    Ok(Some(s.to_string()))
}
