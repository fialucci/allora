//! DSL Facade: multi-format (YAML today; JSON/XML forthcoming) configuration entry points.
//!
//! This module orchestrates conversion of external textual configuration into runtime
//! components by coordinating three layers:
//! 1. Parsers (in `spec/`): format-specific translation into strongly typed specs.
//! 2. Builders (in `dsl/component_builders.rs`): spec -> concrete runtime objects.
//! 3. Facade (this file): public ergonomic API + format inference.
//!
//! # Goals
//! * Provide a minimal, stable surface (`build_channel`, `build_channel_from_str`, `DslFormat`).
//! * Keep parsing & instantiation decoupled so additional formats/components add minimal code.
//! * Fail fast with clear, categorized errors (`Error::Serialization` vs `Error::Other`).
//!
//! # Supported Components
//! * Channel (InMemory kind) – more components (Endpoint, Adapter) will follow the same pattern.
//!
//! # Supported Formats
//! * YAML (`DslFormat::Yaml`).
//! * JSON / XML are reserved (return explicit unsupported errors until implemented).
//!
//! # Usage
//! Build from a file (extension-based format inference):
//! ```rust
//! use allora::{build_channel, Channel};
//! // Assume `channel.yml` contains a valid v1 YAML spec.
//! let chan = build_channel("tests/fixtures/channel.yml").unwrap();
//! assert!(!chan.id().is_empty());
//! ```
//!
//! Build from an in-memory YAML string:
//! ```rust
//! use allora::{build_channel_from_str, DslFormat, Channel};
//! let raw = "version: 1\nchannel:\n  kind: in_memory\n  id: demo";
//! let chan = build_channel_from_str(raw, DslFormat::Yaml).unwrap();
//! assert_eq!(chan.id(), "demo");
//! ```
//!
//! Unsupported format (JSON for now):
//! ```rust
//! use allora::{build_channel_from_str, DslFormat, Error};
//! let raw_json = "{ \"version\":1, \"channel\": { \"kind\": \"in_memory\", \"id\": \"demo\" } }";
//! match build_channel_from_str(raw_json, DslFormat::Json) {
//!     Err(Error::Serialization(msg)) => assert!(msg.contains("not yet supported")),
//!     _ => panic!("expected serialization error for unsupported JSON"),
//! }
//! ```
//!
//! # Error Semantics
//! * `Error::Other` – I/O failures (e.g. unreadable file).
//! * `Error::Serialization` – structural issues (missing fields, invalid values, unsupported formats).
//!
//! # Extension Guide
//! To add JSON support:
//! 1. Implement `ChannelSpecJsonParser` (translate JSON Value -> `ChannelSpec`).
//! 2. Add `Json` branch handling in `build_channel_from_str` calling the new parser.
//! 3. Keep builder logic untouched (format-agnostic).
//!
//! To add a new component (Endpoint):
//! 1. Create `EndpointSpec` + parser(s) under `spec/`.
//! 2. Add `build_endpoint_from_spec` to `component_builders.rs`.
//! 3. Expose facade functions here (`build_endpoint`, `build_endpoint_from_str`).
//!
//! # Testing Strategy
//! * Parser edge cases tested near parser modules.
//! * Builder invariants (auto/empty id) covered in `tests/dsl_component_builders.rs`.
//! * Facade inference & unsupported format behavior covered in `tests/dsl_api.rs`.
//!
//! # Versioning
//! YAML spec version checked explicitly; incompatible versions yield a fast `unsupported version` error.
//! New versions should introduce parallel parser modules without breaking existing consumers.

use crate::{
    channel::InMemoryChannel,
    error::{Error, Result},
    spec::ChannelSpecYamlParser,
};
use std::path::Path;

pub mod component_builders;
use component_builders::build_channel_from_spec;

/// Supported DSL input formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DslFormat {
    Yaml,
    Json,
    Xml,
}

impl DslFormat {
    /// Infer format from file extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
        {
            Some(ref ext) if ext == "yml" || ext == "yaml" => Some(DslFormat::Yaml),
            Some(ref ext) if ext == "json" => Some(DslFormat::Json),
            Some(ref ext) if ext == "xml" => Some(DslFormat::Xml),
            _ => None,
        }
    }
}

/// Build channel from raw string + specified format.
pub fn build_channel_from_str(raw: &str, format: DslFormat) -> Result<InMemoryChannel> {
    match format {
        DslFormat::Yaml => {
            let spec = ChannelSpecYamlParser::parse_str(raw)?;
            build_channel_from_spec(spec)
        }
        DslFormat::Json => Err(Error::serialization("json format not yet supported")),
        DslFormat::Xml => Err(Error::serialization("xml format not yet supported")),
    }
}

/// Convenience: build channel from a file path (auto-detect format via extension).
pub fn build_channel(path: impl AsRef<Path>) -> Result<InMemoryChannel> {
    let path_ref = path.as_ref();
    let raw =
        std::fs::read_to_string(path_ref).map_err(|e| Error::other(format!("read error: {e}")))?;
    let format = DslFormat::from_path(path_ref)
        .ok_or_else(|| Error::serialization("cannot infer DSL format from extension"))?;
    build_channel_from_str(&raw, format)
}
