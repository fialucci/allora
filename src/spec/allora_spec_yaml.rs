//! YAML parser for AlloraSpec (top-level, v1) reusing ChannelsSpec parser logic.
//!
//! # Responsibilities
//! * Validate root `version` (must equal 1).
//! * Ensure `channels` sequence exists.
//! * Delegate channel entry parsing to `ChannelsSpecYamlParser` (structural + kind validation).
//!
//! # Error Cases
//! * Missing or non-integer `version` -> `Error::Serialization`.
//! * Unsupported version (not 1) -> `Error::Serialization`.
//! * Missing `channels` or non-sequence -> `Error::Serialization`.
//! * Individual channel mapping errors / kind mismatches surfaced from delegated parser.
//!
//! # Design Notes
//! The nested parser is reused by synthesizing an intermediate YAML Mapping containing only
//! `version` and `channels`. This avoids code duplication and keeps channel collection logic
//! centralized.
//!
//! # Future Extensions
//! Additional top-level sections (e.g. `endpoints`, `filters`) can be added by retrieving their
//! YAML values and invoking corresponding spec parsers before constructing `AlloraSpec`.

use crate::error::{Error, Result};
use crate::spec::{
    allora_spec::AlloraSpec, channels_spec_yaml::ChannelsSpecYamlParser, ChannelsSpec,
};
use serde_yaml::Value as YamlValue;

pub struct AlloraSpecYamlParser;

impl AlloraSpecYamlParser {
    pub fn parse_value(yaml: &YamlValue) -> Result<AlloraSpec> {
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
        let channels_root = yaml
            .get("channels")
            .ok_or_else(|| Error::serialization("missing 'channels'"))?;
        // Synthesize a YAML mapping with 'version' and 'channels' to reuse ChannelsSpecYamlParser,
        // which expects this structure. This avoids code duplication and centralizes channel parsing logic.
        if !channels_root.is_sequence() {
            return Err(Error::serialization("'channels' must be a sequence"));
        }
        // Reconstruct a YAML value containing version + channels for reuse of ChannelsSpecYamlParser
        let mut obj = serde_yaml::Mapping::new();
        obj.insert(
            serde_yaml::Value::String("version".into()),
            serde_yaml::Value::Number(serde_yaml::Number::from(1)),
        );
        obj.insert(
            serde_yaml::Value::String("channels".into()),
            channels_root.clone(),
        );
        let synthesized = serde_yaml::Value::Mapping(obj);
        let channels_spec: ChannelsSpec = ChannelsSpecYamlParser::parse_value(&synthesized)?;
        Ok(AlloraSpec::new(v as u32, channels_spec))
    }
    pub fn parse_str(raw: &str) -> Result<AlloraSpec> {
        let val: YamlValue = serde_yaml::from_str(raw)
            .map_err(|e| Error::serialization(format!("yaml parse error: {e}")))?;
        Self::parse_value(&val)
    }
}
