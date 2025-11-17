//! YAML parser for ServiceActivatorSpec (v1).
//! Expects structure defined in `schema/v1/service-activator.schema.yml`.
//! Performs structural validation: required non-empty `ref-name`, `from`, `to`; optional non-empty `id`.
//!
//! # Accepted Shape (Informal)
//! ```yaml
//! version: 1
//! service-activator:
//!   id: hello_world          # optional, non-empty if present
//!   ref-name: hello_world     # required, non-empty reference identifier (matches #[service(name=..)])
//!   from: inbound.orders      # required, non-empty
//!   to: vetted.orders         # required, non-empty
//! ```
//!
//! # Validation Responsibilities
//! * Validate top-level `version` via shared `validate_version` (must equal 1).
//! * Ensure `service-activator` mapping exists and contains required keys.
//! * Enforce non-empty strings for all present fields.
//! * Defer uniqueness of `id` to future collection builder.
//!
//! # Non-Goals
//! * Enforcing naming patterns or file path semantics for `ref-name` (it is a logical identifier).
//! * Channel existence checks (handled by orchestration layer later).
//!
//! # Errors
//! * Missing or wrong-typed fields -> `Error::Serialization` with descriptive messages.
//! * Unsupported version -> `Error::Serialization`.

use crate::error::{Error, Result};
use crate::spec::version::validate_version;
use crate::spec::ServiceActivatorSpec;
use serde_yaml::Value as YamlValue;

pub struct ServiceSpecYamlParser;

impl ServiceSpecYamlParser {
    pub fn parse_value(yaml: &YamlValue) -> Result<ServiceActivatorSpec> {
        let _v = validate_version(yaml)?;
        let service_root = yaml
            .get("service-activator")
            .ok_or_else(|| Error::serialization("missing 'service-activator'"))?;
        if !service_root.is_mapping() {
            return Err(Error::serialization(
                "'service-activator' must be a mapping",
            ));
        }
        let name_val = service_root
            .get("ref-name")
            .ok_or_else(|| Error::serialization("service-activator.ref-name required"))?;
        let name_str = name_val
            .as_str()
            .ok_or_else(|| Error::serialization("service-activator.ref-name must be string"))?;
        if name_str.is_empty() {
            return Err(Error::serialization(
                "service-activator.ref-name must not be empty",
            ));
        }
        let from_val = service_root
            .get("from")
            .ok_or_else(|| Error::serialization("service-activator.from required"))?;
        let from_str = from_val
            .as_str()
            .ok_or_else(|| Error::serialization("service-activator.from must be string"))?;
        if from_str.is_empty() {
            return Err(Error::serialization(
                "service-activator.from must not be empty",
            ));
        }
        let to_val = service_root
            .get("to")
            .ok_or_else(|| Error::serialization("service-activator.to required"))?;
        let to_str = to_val
            .as_str()
            .ok_or_else(|| Error::serialization("service-activator.to must be string"))?;
        if to_str.is_empty() {
            return Err(Error::serialization(
                "service-activator.to must not be empty",
            ));
        }
        let id_opt = service_root
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        if let Some(ref idv) = id_opt {
            if idv.is_empty() {
                return Err(Error::serialization(
                    "service-activator.id must not be empty",
                ));
            }
        }
        Ok(match id_opt {
            Some(id) => ServiceActivatorSpec::with_id(id, name_str, from_str, to_str),
            None => ServiceActivatorSpec::new(name_str, from_str, to_str),
        })
    }
    pub fn parse_str(raw: &str) -> Result<ServiceActivatorSpec> {
        let val: YamlValue = serde_yaml::from_str(raw)
            .map_err(|e| Error::serialization(format!("yaml parse error: {e}")))?;
        Self::parse_value(&val)
    }
}
