//! YAML parser/loader for ChannelSpec (schema v1).
//! Focus: translate a YAML document into a validated ChannelSpec; no component instantiation.
//!
//! # Responsibilities
//! * Validate top-level `version` (currently must be `1`).
//! * Extract and validate `channel` mapping (kind + optional id).
//! * Enforce non-empty id if present; leave id generation to builders when absent.
//! * Return a `ChannelSpec` without performing instantiation.
//!
//! # YAML Schema (v1)
//! ```yaml
//! version: 1
//! channel:
//!   kind: in_memory   # required, enum
//!   id: my-channel    # optional
//! ```
//!
//! # Errors
//! * Missing fields -> `Error::Serialization` with descriptive message.
//! * Type mismatches (non-integer version, non-string kind) -> `Error::Serialization`.
//! * Unsupported kind / version -> `Error::Serialization`.
//! * Malformed YAML -> `Error::Serialization("yaml parse error: ...")` via `parse_str`.
//!
//! # Example
//! ```rust
//! use allora::spec::{ChannelSpecYamlParser, ChannelSpec};
//! let raw = "version: 1\nchannel:\n  kind: in_memory\n  id: parser-demo";
//! let spec = ChannelSpecYamlParser::parse_str(raw).unwrap();
//! assert_eq!(spec.channel_id(), Some("parser-demo"));
//! ```
//!
//! # Extending to New Versions
//! Introduce `ChannelSpecYamlParserV2` (keeping V1 intact) and dispatch based on detected `version`.
//! Avoid breaking changes in existing parsers; prefer additive fields or new versioned parsers.
//!
//! # Future Formats
//! Parallel parsers (e.g. `ChannelSpecJsonParser`) should replicate the same validation
//! semantics and return `ChannelSpec` for uniform builder consumption.
use super::{ChannelKindSpec, ChannelSpec};
use crate::error::{Error, Result};
use serde_yaml::Value as YamlValue;

/// Parses YAML into a ChannelSpec (v1 schema).
pub struct ChannelSpecYamlParser;

impl ChannelSpecYamlParser {
    /// Internal helper: validate presence of required top-level / nested keys.
    fn validate_required(yaml: &YamlValue) -> Result<()> {
        // version key present?
        if yaml.get("version").is_none() {
            return Err(Error::serialization("missing 'version'"));
        }
        // enforce root additionalProperties: false
        if let Some(root_map) = yaml.as_mapping() {
            for (k, _) in root_map.iter() {
                if let Some(key_str) = k.as_str() {
                    if key_str != "version" && key_str != "channel" {
                        return Err(Error::serialization(format!(
                            "unknown root property '{key_str}'"
                        )));
                    }
                }
            }
        }
        let channel = yaml
            .get("channel")
            .ok_or_else(|| Error::serialization("missing 'channel' section"))?;
        if !channel.is_mapping() {
            return Err(Error::serialization("'channel' must be a mapping"));
        }
        // enforce channel.additionalProperties: false
        if let Some(ch_map) = channel.as_mapping() {
            for (k, _) in ch_map.iter() {
                if let Some(key_str) = k.as_str() {
                    if key_str != "kind" && key_str != "id" {
                        return Err(Error::serialization(format!(
                            "unknown channel property '{key_str}'"
                        )));
                    }
                }
            }
        }
        if channel.get("kind").is_none() {
            return Err(Error::serialization("channel.kind required"));
        }
        Ok(())
    }
    /// Parse an already loaded YAML Value into a ChannelSpec.
    pub fn parse_value(yaml: &YamlValue) -> Result<ChannelSpec> {
        Self::validate_required(yaml)?; // early structural presence check
        let version = yaml
            .get("version")
            .ok_or_else(|| Error::serialization("missing 'version'"))?;
        if !version.is_i64() && !version.is_u64() {
            return Err(Error::serialization("'version' must be integer"));
        }
        let v = version
            .as_i64()
            .unwrap_or(version.as_u64().unwrap_or(0) as i64);
        if v != 1 {
            return Err(Error::serialization("unsupported version (expected 1)"));
        }
        let channel = yaml
            .get("channel")
            .ok_or_else(|| Error::serialization("missing 'channel' section"))?;
        if !channel.is_mapping() {
            return Err(Error::serialization("'channel' must be a mapping"));
        }
        let kind = channel
            .get("kind")
            .ok_or_else(|| Error::serialization("channel.kind required"))?;
        let kind_str = kind
            .as_str()
            .ok_or_else(|| Error::serialization("channel.kind must be string"))?;
        if kind_str != "in_memory" {
            return Err(Error::serialization(format!(
                "unsupported channel.kind '{kind_str}'"
            )));
        }
        let id_opt = channel
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        if let Some(ref id) = id_opt {
            if id.is_empty() {
                return Err(Error::serialization("channel.id must not be empty"));
            }
        }
        Ok(ChannelSpec {
            id: id_opt,
            kind: ChannelKindSpec::InMemory,
        })
    }
    /// Convenience: parse raw YAML string directly (one-shot). Suitable for tests or embedding.
    pub fn parse_str(raw: &str) -> Result<ChannelSpec> {
        let val: YamlValue = serde_yaml::from_str(raw)
            .map_err(|e| Error::serialization(format!("yaml parse error: {e}")))?;
        Self::parse_value(&val)
    }
}

#[deprecated(note = "Use ChannelSpecYamlParser::parse_value or parse_str")]
pub fn build_channel_from_yaml(yaml: &YamlValue) -> Result<ChannelSpec> {
    ChannelSpecYamlParser::parse_value(yaml)
}

#[deprecated(note = "Use ChannelSpecYamlParser instead of DslChannelSpecYamlBuilder")]
pub type DslChannelSpecYamlBuilder = ChannelSpecYamlParser;
