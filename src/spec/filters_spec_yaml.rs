//! YAML parser for FiltersSpec (collection v1).
//! Delegates per-entry parsing to `FilterSpecYamlParser` and focuses on sequence + version validation.
//!
//! # Responsibilities
//! * Validate root `version` (must equal 1) using shared helper.
//! * Ensure `filters` is a YAML sequence; each item is a mapping.
//! * Delegate each item to `FilterSpecYamlParser::parse_value` (thus reusing single-filter validation logic).
//! * Preserve order of entries for deterministic downstream processing.
//!
//! # Non-Responsibilities
//! * Uniqueness / auto-id generation (handled in builder `build_filters_from_spec`).
//! * Cross-reference verification (e.g. ensuring `from` channel exists).
//! * Expression semantic analysis beyond non-empty string.
//!
//! # Error Cases
//! * Missing `filters` key or non-sequence value.
//! * Non-mapping item inside the sequence.
//! * Any error bubbled from `FilterSpecYamlParser` (invalid fields / empty strings / version mismatch).
//!
//! # Example YAML
//! ```yaml
//! version: 1
//! filters:
//!   - id: filt.orders
//!     from: inbound.orders
//!     to: vetted.orders
//!     when: body.contains("KEEP") && exists(header("Trace-Id"))
//!   - from: inbound.audit
//!     when: header("Audit-Flag") == "true" && exists(header("Audit-Flag"))
//! ```
//!

use crate::error::{Error, Result};
use crate::spec::filters_spec::FiltersSpec;
use crate::spec::version::validate_version;
use crate::spec::FilterSpecYamlParser;
use serde_yaml::Value as YamlValue;

pub struct FiltersSpecYamlParser;

impl FiltersSpecYamlParser {
    pub fn parse_value(yaml: &YamlValue) -> Result<FiltersSpec> {
        let v = validate_version(yaml)?;
        let filters_val = yaml
            .get("filters")
            .ok_or_else(|| Error::serialization("missing 'filters'"))?;
        if !filters_val.is_sequence() {
            return Err(Error::serialization("'filters' must be a sequence"));
        }
        let mut spec = FiltersSpec::new(v);
        for item in filters_val.as_sequence().unwrap() {
            if !item.is_mapping() {
                return Err(Error::serialization("filter entry must be a mapping"));
            }
            // Synthesize single filter doc for existing parser
            let mut obj = serde_yaml::Mapping::new();
            obj.insert(
                serde_yaml::Value::String("version".into()),
                serde_yaml::Value::Number(serde_yaml::Number::from(v)),
            );
            obj.insert(serde_yaml::Value::String("filter".into()), item.clone());
            let synthesized = YamlValue::Mapping(obj);
            let single = FilterSpecYamlParser::parse_value(&synthesized)?;
            spec.push(single);
        }
        Ok(spec)
    }
    pub fn parse_str(raw: &str) -> Result<FiltersSpec> {
        let val: YamlValue = serde_yaml::from_str(raw)
            .map_err(|e| Error::serialization(format!("yaml parse error: {e}")))?;
        Self::parse_value(&val)
    }
}
