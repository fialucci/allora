//! YAML parser for HttpInboundAdapterSpec (v1).
//! Expects structure defined in `schema/v1/http-inbound-adapter.schema.yml`.
//! Performs structural validation: required non-empty `host`, `path`, `methods` (non-empty list),
//! `request-channel`; numeric range for `port`; optional non-empty `id`, `reply-channel`.
//!
//! # Accepted Shape (Informal)
//! ```yaml
//! version: 1
//! http-inbound-adapter:
//!   id: http.receiveGateway          # optional
//!   host: 127.0.0.1                  # required
//!   port: 8080                       # required (1-65535)
//!   path: /receiveGateway            # required (starts with '/')
//!   methods: [ POST ]                # required, non-empty list, allowed verbs
//!   request-channel: receiveChannel  # required
//!   reply-channel: replyChannel      # optional
//! ```
//!
//! # Validation Responsibilities
//! * Validate top-level `version` (must equal 1).
//! * Ensure mapping exists under `http-inbound-adapter`.
//! * Enforce required keys and non-empty string semantics.
//! * Enforce port integer & range (1-65535).
//! * Enforce methods is non-empty sequence of allowed verbs.
//! * Enforce path starts with '/'.
//! * Defer uniqueness of `id` to future builder / collection.
//!
//! # Errors
//! * Missing fields -> `Error::Serialization` with descriptive messages.
//! * Wrong types -> `Error::Serialization` (e.g. non-string method).
//! * Unsupported method -> `Error::Serialization`.
//! * Out-of-range port -> `Error::Serialization`.
//!
//! # Non-Goals
//! * Network binding validation (performed at runtime instantiation).
//! * Channel existence checks.
//! * Reply correlation semantics.
//!
//! See collection parser for aggregation logic: `HttpInboundAdaptersSpecYamlParser`.

use crate::error::{Error, Result};
use crate::spec::version::validate_version;
use crate::spec::HttpInboundAdapterSpec;
use serde_yaml::Value as YamlValue;

pub struct HttpInboundAdapterSpecYamlParser;

impl HttpInboundAdapterSpecYamlParser {
    pub fn parse_value(yaml: &YamlValue) -> Result<HttpInboundAdapterSpec> {
        let _v = validate_version(yaml)?;
        let root = yaml
            .get("http-inbound-adapter")
            .ok_or_else(|| Error::serialization("missing 'http-inbound-adapter'"))?;
        if !root.is_mapping() {
            return Err(Error::serialization(
                "'http-inbound-adapter' must be a mapping",
            ));
        }
        // host
        let host_val = root
            .get("host")
            .ok_or_else(|| Error::serialization("http-inbound-adapter.host required"))?;
        let host_str = host_val
            .as_str()
            .ok_or_else(|| Error::serialization("http-inbound-adapter.host must be string"))?;
        if host_str.is_empty() {
            return Err(Error::serialization(
                "http-inbound-adapter.host must not be empty",
            ));
        }
        // port
        let port_val = root
            .get("port")
            .ok_or_else(|| Error::serialization("http-inbound-adapter.port required"))?;
        let port_num = port_val
            .as_u64()
            .ok_or_else(|| Error::serialization("http-inbound-adapter.port must be integer"))?;
        if port_num == 0 || port_num > 65535 {
            return Err(Error::serialization(
                "http-inbound-adapter.port out of range (1-65535)",
            ));
        }
        let port_u16 = port_num as u16;
        // path
        let path_val = root
            .get("path")
            .ok_or_else(|| Error::serialization("http-inbound-adapter.path required"))?;
        let path_str = path_val
            .as_str()
            .ok_or_else(|| Error::serialization("http-inbound-adapter.path must be string"))?;
        if path_str.is_empty() {
            return Err(Error::serialization(
                "http-inbound-adapter.path must not be empty",
            ));
        }
        if !path_str.starts_with('/') {
            return Err(Error::serialization(
                "http-inbound-adapter.path must start with '/'",
            ));
        }
        // methods
        let methods_val = root
            .get("methods")
            .ok_or_else(|| Error::serialization("http-inbound-adapter.methods required"))?;
        if !methods_val.is_sequence() {
            return Err(Error::serialization(
                "http-inbound-adapter.methods must be a sequence",
            ));
        }
        let seq = methods_val.as_sequence().unwrap();
        if seq.is_empty() {
            return Err(Error::serialization(
                "http-inbound-adapter.methods sequence must not be empty",
            ));
        }
        const ALLOWED: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD"];
        let mut methods: Vec<String> = Vec::with_capacity(seq.len());
        for m in seq {
            let ms = m.as_str().ok_or_else(|| {
                Error::serialization("http-inbound-adapter.method must be string")
            })?;
            if !ALLOWED.contains(&ms) {
                return Err(Error::serialization(format!(
                    "unsupported http-inbound-adapter.method '{ms}'"
                )));
            }
            methods.push(ms.to_string());
        }
        // request-channel
        let req_val = root
            .get("request-channel")
            .ok_or_else(|| Error::serialization("http-inbound-adapter.request-channel required"))?;
        let req_str = req_val.as_str().ok_or_else(|| {
            Error::serialization("http-inbound-adapter.request-channel must be string")
        })?;
        if req_str.is_empty() {
            return Err(Error::serialization(
                "http-inbound-adapter.request-channel must not be empty",
            ));
        }
        // reply-channel (optional)
        let reply_opt = root
            .get("reply-channel")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        if let Some(ref rc) = reply_opt {
            if rc.is_empty() {
                return Err(Error::serialization(
                    "http-inbound-adapter.reply-channel must not be empty",
                ));
            }
        }
        // id (optional)
        let id_opt = root
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        if let Some(ref idv) = id_opt {
            if idv.is_empty() {
                return Err(Error::serialization(
                    "http-inbound-adapter.id must not be empty",
                ));
            }
        }
        Ok(match (id_opt, reply_opt) {
            (Some(id), Some(reply)) => HttpInboundAdapterSpec::with_id_reply(
                id, host_str, port_u16, path_str, methods, req_str, reply,
            ),
            (Some(id), None) => {
                HttpInboundAdapterSpec::with_id(id, host_str, port_u16, path_str, methods, req_str)
            }
            (None, Some(reply)) => HttpInboundAdapterSpec::with_reply(
                host_str, port_u16, path_str, methods, req_str, reply,
            ),
            (None, None) => {
                HttpInboundAdapterSpec::new(host_str, port_u16, path_str, methods, req_str)
            }
        })
    }
    pub fn parse_str(raw: &str) -> Result<HttpInboundAdapterSpec> {
        let val: YamlValue = serde_yaml::from_str(raw)
            .map_err(|e| Error::serialization(format!("yaml parse error: {e}")))?;
        Self::parse_value(&val)
    }
}
