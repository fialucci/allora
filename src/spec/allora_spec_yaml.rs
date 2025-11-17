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

use super::version::validate_version;
use crate::error::{Error, Result};
use crate::spec::{
    allora_spec::AlloraSpec, channels_spec_yaml::ChannelsSpecYamlParser,
    filters_spec_yaml::FiltersSpecYamlParser, ChannelsSpec, FiltersSpec, ServiceActivatorsSpec,
    ServiceSpecYamlParser,
};
use serde_yaml::Value as YamlValue;

pub struct AlloraSpecYamlParser;

impl AlloraSpecYamlParser {
    pub fn parse_value(yaml: &YamlValue) -> Result<AlloraSpec> {
        let v = validate_version(yaml)?; // shared validation
        let channels_root = yaml
            .get("channels")
            .ok_or_else(|| Error::serialization("missing 'channels'"))?;
        if !channels_root.is_sequence() {
            return Err(Error::serialization("'channels' must be a sequence"));
        }
        // optional filters
        let filters_root = yaml.get("filters");
        // optional services
        let services_root = yaml.get("service-activators");
        // Synthesize mapping for channel parser reuse
        let mut obj = serde_yaml::Mapping::new();
        obj.insert(
            serde_yaml::Value::String("version".into()),
            serde_yaml::Value::Number(serde_yaml::Number::from(v)),
        );
        obj.insert(
            serde_yaml::Value::String("channels".into()),
            channels_root.clone(),
        );
        let synthesized = serde_yaml::Value::Mapping(obj);
        let channels_spec: ChannelsSpec = ChannelsSpecYamlParser::parse_value(&synthesized)?;
        let mut all = AlloraSpec::new(v, channels_spec);
        if let Some(fr) = filters_root {
            if !fr.is_sequence() {
                return Err(Error::serialization("'filters' must be a sequence"));
            }
            // Build mapping for filters parser reuse
            let mut fobj = serde_yaml::Mapping::new();
            fobj.insert(
                serde_yaml::Value::String("version".into()),
                serde_yaml::Value::Number(serde_yaml::Number::from(v)),
            );
            fobj.insert(serde_yaml::Value::String("filters".into()), fr.clone());
            let fsynth = serde_yaml::Value::Mapping(fobj);
            let filters_spec: FiltersSpec = FiltersSpecYamlParser::parse_value(&fsynth)?;
            all = all.with_filters(filters_spec);
        }
        if let Some(sr) = services_root {
            if !sr.is_sequence() {
                return Err(Error::serialization(
                    "'service-activators' must be a sequence",
                ));
            }
            let seq = sr.as_sequence().unwrap();
            if seq.is_empty() {
                return Err(Error::serialization(
                    "'service-activators' sequence must not be empty",
                ));
            }
            let mut services_spec = ServiceActivatorsSpec::new(v);
            for item in seq {
                if !item.is_mapping() {
                    return Err(Error::serialization(
                        "service-activator entry must be a mapping",
                    ));
                }
                // synthesize a single service document for an existing parser
                let mut obj = serde_yaml::Mapping::new();
                obj.insert(
                    serde_yaml::Value::String("version".into()),
                    serde_yaml::Value::Number(serde_yaml::Number::from(v)),
                );
                obj.insert(
                    serde_yaml::Value::String("service-activator".into()),
                    item.clone(),
                );
                let synthesized = serde_yaml::Value::Mapping(obj);
                let svc = ServiceSpecYamlParser::parse_value(&synthesized)?;
                services_spec.push(svc);
            }
            all = all.with_services(services_spec);
        }
        Ok(all)
    }
    pub fn parse_str(raw: &str) -> Result<AlloraSpec> {
        let val: YamlValue = serde_yaml::from_str(raw)
            .map_err(|e| Error::serialization(format!("yaml parse error: {e}")))?;
        Self::parse_value(&val)
    }
}
