//! YAML parser for ChannelsSpec (v1 collection).
//!
//! # Responsibilities
//! * Validate `version` integer (must equal 1).
//! * Ensure `channels` is a sequence of mapping entries.
//! * For each channel entry: require `kind`, validate its string value.
//! * Extract optional `id` (may be absent or empty -> empty triggers error; absent allowed).
//! * Construct a `ChannelsSpec` preserving order; uniqueness not enforced here.
//!
//! # Errors
//! * Missing / non-integer version.
//! * Unsupported version (not 1).
//! * Missing `channels` or non-sequence value.
//! * Non-mapping channel entry.
//! * Missing `kind` or non-string kind.
//! * Unsupported kind variant.
//! * Empty `id` string.
//!
//! # Schema Fragment (Informal)
//! ```yaml
//! version: 1
//! channels:
//!   - kind: direct
//!     id: optional-string
//! ```
//!
//! # Deferral of Uniqueness
//! Duplicate ids are permitted at parse time; builders (`build_channels_from_spec`) enforce
//! uniqueness and generate deterministic auto IDs for missing ids.
//!
//! # Future Extensions
//! Additional fields (e.g. QoS, durability) can be added per entry; preserve strict rejection of
//! unknown root properties by performing validation before spec construction.

use super::version::validate_version;
use crate::error::{Error, Result};
use crate::spec::channels_spec::ChannelsSpec;
use crate::spec::{ChannelKindSpec, ChannelSpec};
use serde_yaml::Value as YamlValue;

pub struct ChannelsSpecYamlParser;

impl ChannelsSpecYamlParser {
    pub fn parse_value(yaml: &YamlValue) -> Result<ChannelsSpec> {
        let v = validate_version(yaml)?; // shared validation
        let channels_val = yaml
            .get("channels")
            .ok_or_else(|| Error::serialization("missing 'channels'"))?;
        if !channels_val.is_sequence() {
            return Err(Error::serialization("'channels' must be a sequence"));
        }
        let mut spec = ChannelsSpec::new(v);
        for item in channels_val.as_sequence().unwrap() {
            if !item.is_mapping() {
                return Err(Error::serialization("channel entry must be a mapping"));
            }
            let kind_spec = if let Some(kind_val) = item.get("kind") {
                let kind_str = kind_val
                    .as_str()
                    .ok_or_else(|| Error::serialization("channel.kind must be string"))?;
                match kind_str {
                    "direct" => ChannelKindSpec::Direct,
                    "in_memory" => ChannelKindSpec::InMemory,
                    other => {
                        return Err(Error::serialization(format!(
                            "unsupported channel.kind '{other}'"
                        )))
                    }
                }
            } else {
                tracing::info!("channel.kind missing in entry; defaulting to 'direct'");
                ChannelKindSpec::Direct
            };
            let id_opt = item
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if let Some(ref id) = id_opt {
                if id.is_empty() {
                    return Err(Error::serialization("channel.id must not be empty"));
                }
            }
            spec.push(ChannelSpec {
                id: id_opt,
                kind: kind_spec,
            });
        }
        Ok(spec)
    }
    pub fn parse_str(raw: &str) -> Result<ChannelsSpec> {
        let val: YamlValue = serde_yaml::from_str(raw)
            .map_err(|e| Error::serialization(format!("yaml parse error: {e}")))?;
        Self::parse_value(&val)
    }
}
