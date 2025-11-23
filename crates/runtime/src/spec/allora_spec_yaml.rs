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
    filters_spec_yaml::FiltersSpecYamlParser, ChannelsSpec, FiltersSpec, HttpInboundAdaptersSpec,
    HttpInboundAdaptersSpecYamlParser, HttpOutboundAdaptersSpec,
    HttpOutboundAdaptersSpecYamlParser, ServiceActivatorsSpec, ServiceSpecYamlParser,
};
use serde_yaml::Value as YamlValue;

pub struct AlloraSpecYamlParser;

impl AlloraSpecYamlParser {
    pub fn parse_value(yaml: &YamlValue) -> Result<AlloraSpec> {
        let v = validate_version(yaml)?; // shared validation
        let channels_root_opt = yaml.get("channels");
        // optional channels now; if missing -> empty collection
        let mut all = if let Some(ch_root) = channels_root_opt {
            if !ch_root.is_sequence() {
                return Err(Error::serialization("'channels' must be a sequence"));
            }
            let mut obj = serde_yaml::Mapping::new();
            obj.insert(
                serde_yaml::Value::String("version".into()),
                serde_yaml::Value::Number(serde_yaml::Number::from(v)),
            );
            obj.insert(
                serde_yaml::Value::String("channels".into()),
                ch_root.clone(),
            );
            let synthesized = serde_yaml::Value::Mapping(obj);
            let channels_spec: ChannelsSpec = ChannelsSpecYamlParser::parse_value(&synthesized)?;
            AlloraSpec::new(v, channels_spec)
        } else {
            AlloraSpec::new(v, ChannelsSpec::new(v))
        };
        // optional filters
        let filters_root = yaml.get("filters");
        // optional services
        let services_root = yaml.get("service-activators");
        // optional http inbound adapters
        let http_inbound_root = yaml.get("http-inbound-adapters");
        // optional http outbound adapters
        let http_outbound_root = yaml.get("http-outbound-adapters");
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
        if let Some(hr) = http_inbound_root {
            if !hr.is_sequence() {
                return Err(Error::serialization(
                    "'http-inbound-adapters' must be a sequence",
                ));
            }
            if hr.as_sequence().unwrap().is_empty() {
                return Err(Error::serialization(
                    "'http-inbound-adapters' sequence must not be empty",
                ));
            }
            // synthesize mapping for collection parser
            let mut hobj = serde_yaml::Mapping::new();
            hobj.insert(
                serde_yaml::Value::String("version".into()),
                serde_yaml::Value::Number(serde_yaml::Number::from(v)),
            );
            hobj.insert(
                serde_yaml::Value::String("http-inbound-adapters".into()),
                hr.clone(),
            );
            let hsynth = serde_yaml::Value::Mapping(hobj);
            let adapters_spec: HttpInboundAdaptersSpec =
                HttpInboundAdaptersSpecYamlParser::parse_value(&hsynth)?;
            all = all.with_http_inbound_adapters(adapters_spec);
        }
        if let Some(oroot) = http_outbound_root {
            if !oroot.is_sequence() {
                return Err(Error::serialization(
                    "'http-outbound-adapters' must be a sequence",
                ));
            }
            if oroot.as_sequence().unwrap().is_empty() {
                return Err(Error::serialization(
                    "'http-outbound-adapters' sequence must not be empty",
                ));
            }
            let mut oobj = serde_yaml::Mapping::new();
            oobj.insert(
                serde_yaml::Value::String("version".into()),
                serde_yaml::Value::Number(serde_yaml::Number::from(v)),
            );
            oobj.insert(
                serde_yaml::Value::String("http-outbound-adapters".into()),
                oroot.clone(),
            );
            let osynth = serde_yaml::Value::Mapping(oobj);
            let outbound_spec: HttpOutboundAdaptersSpec =
                HttpOutboundAdaptersSpecYamlParser::parse_value(&osynth)?;
            all = all.with_http_outbound_adapters(outbound_spec);
        }
        Ok(all)
    }
    pub fn parse_str(raw: &str) -> Result<AlloraSpec> {
        let val: YamlValue = serde_yaml::from_str(raw)
            .map_err(|e| Error::serialization(format!("yaml parse error: {e}")))?;
        Self::parse_value(&val)
    }
}
