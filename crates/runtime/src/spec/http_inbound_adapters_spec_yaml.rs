//! YAML parser for HttpInboundAdaptersSpec (collection v1).
//! Expects structure defined in `schema/v1/http-inbound-adapters.schema.yml`.
//! Delegates per-entry parsing to `HttpInboundAdapterSpecYamlParser` and focuses on sequence + version validation.
//!
//! # Responsibilities
//! * Validate root `version` (must equal 1).
//! * Ensure `http-inbound-adapters` is a non-empty YAML sequence.
//! * Delegate each item mapping to existing single parser by synthesizing a temporary document.
//! * Preserve order of entries.
//!
//! # Error Cases
//! * Missing `http-inbound-adapters` key.
//! * Non-sequence `http-inbound-adapters` value.
//! * Empty sequence (minItems=1).
//! * Non-mapping item inside sequence.
//! * Any error bubbled from `HttpInboundAdapterSpecYamlParser`.
//!
//! # Example YAML
//! ```yaml
//! version: 1
//! http-inbound-adapters:
//!   - id: http.receiveGateway
//!     host: 127.0.0.1
//!     port: 8080
//!     path: /receiveGateway
//!     methods: [ POST ]
//!     request-channel: receiveChannel
//!     reply-channel: replyChannel
//!   - host: 0.0.0.0
//!     port: 8081
//!     path: /orders
//!     methods: [ POST, GET ]
//!     request-channel: inbound.orders
//! ```

use crate::error::{Error, Result};
use crate::spec::version::validate_version;
use crate::spec::{HttpInboundAdapterSpecYamlParser, HttpInboundAdaptersSpec};
use serde_yaml::Value as YamlValue;

pub struct HttpInboundAdaptersSpecYamlParser;

impl HttpInboundAdaptersSpecYamlParser {
    pub fn parse_value(yaml: &YamlValue) -> Result<HttpInboundAdaptersSpec> {
        let v = validate_version(yaml)?;
        let root = yaml
            .get("http-inbound-adapters")
            .ok_or_else(|| Error::serialization("missing 'http-inbound-adapters'"))?;
        if !root.is_sequence() {
            return Err(Error::serialization(
                "'http-inbound-adapters' must be a sequence",
            ));
        }
        let seq = root.as_sequence().unwrap();
        if seq.is_empty() {
            return Err(Error::serialization(
                "'http-inbound-adapters' sequence must not be empty (minItems=1)",
            ));
        }
        let mut spec = HttpInboundAdaptersSpec::new(v);
        for item in seq {
            if !item.is_mapping() {
                return Err(Error::serialization(
                    "http-inbound-adapter entry must be a mapping",
                ));
            }
            let mut obj = serde_yaml::Mapping::new();
            obj.insert(
                serde_yaml::Value::String("version".into()),
                serde_yaml::Value::Number(serde_yaml::Number::from(v)),
            );
            obj.insert(
                serde_yaml::Value::String("http-inbound-adapter".into()),
                item.clone(),
            );
            let synthesized = YamlValue::Mapping(obj);
            let single = HttpInboundAdapterSpecYamlParser::parse_value(&synthesized)?;
            spec.push(single);
        }
        Ok(spec)
    }
    pub fn parse_str(raw: &str) -> Result<HttpInboundAdaptersSpec> {
        let val: YamlValue = serde_yaml::from_str(raw)
            .map_err(|e| Error::serialization(format!("yaml parse error: {e}")))?;
        Self::parse_value(&val)
    }
}
