//! YAML parser for FilterSpec (v1).
//! Expects structure defined in `schema/v1/filter.schema.yml`.
//! Performs structural validation only (no expression grammar validation yet) and now
//! recognizes optional `id` for uniqueness enforcement at build time.
//!
//! # Accepted Shape (Informal)
//! ```yaml
//! version: 1
//! filter:
//!   id: optional-id          # optional, non-empty if present
//!   from: inbound.orders     # required, non-empty
//!   to: vetted.orders        # optional
//!   when: body.contains("KEEP") && exists(header("Trace-Id"))  # required, non-empty
//! ```
//!
//! # Validation Responsibilities
//! * Enforce presence & string type for `from` and `when`.
//! * Enforce non-empty strings for `from`, `when`, `to`, `id` (when present).
//! * Validate top-level `version` via shared `validate_version` (must equal 1).
//! * Reject non-mapping `filter` root.
//! * Defer uniqueness of `id` to collection builder (`build_filters_from_spec`).
//!
//! # Non-Goals
//! * Expression syntax validation beyond non-empty string (left to runtime evaluation logic).
//! * Referenced channel existence (handled by orchestration layer during routing configuration).

use crate::error::{Error, Result};
use crate::spec::version::validate_version;
use crate::spec::FilterSpec;
use serde_yaml::Value as YamlValue;

pub struct FilterSpecYamlParser;

impl FilterSpecYamlParser {
    pub fn parse_value(yaml: &YamlValue) -> Result<FilterSpec> {
        let _v = validate_version(yaml)?; // version currently unused beyond validation
        let filter_root = yaml
            .get("filter")
            .ok_or_else(|| Error::serialization("missing 'filter'"))?;
        if !filter_root.is_mapping() {
            return Err(Error::serialization("'filter' must be a mapping"));
        }
        let from_val = filter_root
            .get("from")
            .ok_or_else(|| Error::serialization("filter.from required"))?;
        let from_str = from_val
            .as_str()
            .ok_or_else(|| Error::serialization("filter.from must be string"))?;
        if from_str.is_empty() {
            return Err(Error::serialization("filter.from must not be empty"));
        }
        let when_val = filter_root
            .get("when")
            .ok_or_else(|| Error::serialization("filter.when required"))?;
        let when_str = when_val
            .as_str()
            .ok_or_else(|| Error::serialization("filter.when must be string"))?;
        if when_str.is_empty() {
            return Err(Error::serialization("filter.when must not be empty"));
        }
        let to_opt = filter_root
            .get("to")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        if let Some(ref to) = to_opt {
            if to.is_empty() {
                return Err(Error::serialization("filter.to must not be empty"));
            }
        }
        let id_opt = filter_root
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        if let Some(ref idv) = id_opt {
            if idv.is_empty() {
                return Err(Error::serialization("filter.id must not be empty"));
            }
        }
        Ok(match (id_opt, to_opt) {
            (Some(id), Some(to)) => FilterSpec::with_id_to(id, from_str, to, when_str),
            (Some(id), None) => FilterSpec::with_id(id, from_str, when_str),
            (None, Some(to)) => FilterSpec::with_to(from_str, to, when_str),
            (None, None) => FilterSpec::new(from_str, when_str),
        })
    }
    pub fn parse_str(raw: &str) -> Result<FilterSpec> {
        let val: YamlValue = serde_yaml::from_str(raw)
            .map_err(|e| Error::serialization(format!("yaml parse error: {e}")))?;
        Self::parse_value(&val)
    }
}
