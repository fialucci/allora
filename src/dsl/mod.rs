//! DSL Facade: multi-format (YAML today; JSON/XML forthcoming) configuration entry points.
//!
//! This module orchestrates conversion of external textual configuration into runtime
//! components by coordinating three layers:
//! 1. Parsers (in `spec/`): format-specific translation into strongly typed specs.
//! 2. Builders (in `dsl/component_builders.rs`): spec -> concrete runtime objects.
//! 3. Facade (this file): public ergonomic API + format inference.
//!
//! # Goals
//! * Provide a minimal, stable surface (`build`, `build_channel`, `build_channel_from_str`, `DslFormat`).
//! * Keep parsing & instantiation decoupled so additional formats/components add minimal code.
//! * Fail fast with clear, categorized errors (`Error::Serialization` vs `Error::Other`).
//!
//! # Supported Components (v1)
//! * Channel (InMemory kind)
//!
//! # Supported Formats
//! * YAML (`DslFormat::Yaml`)
//! * JSON / XML reserved (emit explicit unsupported errors)
//!
//! # Building a Single Channel
//! ```rust
//! use allora::{build_channel, Channel};
//! let ch = build_channel("tests/fixtures/channel.yml").unwrap();
//! println!("channel id={}", ch.id());
//! ```
//!
//! # Building the Full Runtime (AlloraRuntime)
//! Use `build()` when you want all declared components from a top-level `allora.yml`:
//! ```rust
//! use allora::{build, Channel};
//! let rt = build("tests/fixtures/allora.yml").unwrap();
//! for ch in rt.channels() { println!("channel={}", ch.id()); }
//! assert_eq!(rt.channel_count(), 3);
//! ```
//! This returns an `AlloraRuntime` aggregate. Future versions will add `endpoints()`, `filters()`, etc.
//!
//! # Access Patterns
//! * Borrow: `rt.channels()`
//! * Lookup: `rt.channel_by_id("inbound.orders")`
//! * Consume: `rt.into_channels()` (yields `Vec<InMemoryChannel>`)
//!
//! # Error Semantics
//! * `Error::Other` – I/O failures (e.g. unreadable file path)
//! * `Error::Serialization` – structural issues (missing fields / invalid values / unsupported version / unsupported format)
//!
//! # Extension Guide (Adding New Components)
//! 1. Define data model spec (`*_spec.rs`)
//! 2. Add parser (`*_spec_yaml.rs`) validating version + fields
//! 3. Extend `AlloraSpec` to hold new spec collection
//! 4. Add builder in `component_builders.rs`
//! 5. Augment `build_runtime_from_str` to assemble new runtime objects
//! 6. Add accessors on `AlloraRuntime`
//!
//! # Format Addition (JSON Example Outline)
//! * Introduce `ChannelSpecJsonParser` implementing `parse_str` from JSON text
//! * Add branch in `build_channel_from_str` / `build_runtime_from_str` for `DslFormat::Json`
//! * Reuse existing builders (format-agnostic)
//!
//! # Testing Strategy
//! * Parser edge cases near parser modules (`*_spec_yaml.rs`)
//! * Builder invariants in `tests/channels.rs` / `tests/channels_spec.rs`
//! * Facade behavior & runtime aggregation in `tests/dsl_runtime.rs` & `tests/allora_spec.rs`
//!
//! # Versioning
//! * Each spec parser validates an explicit integer `version`
//! * New incompatible changes introduce parallel parser modules (`*_v2_yaml.rs`) preserving old behavior
//!
//! # Internal Helpers (Non-Public)
//! * `build_runtime_from_str` – core dispatcher from raw text + format
//! * Channel-only parser path reused for runtime build
//!
//! # Future Roadmap (Illustrative)
//! * Multiple channel kinds (e.g. `kafka`, `amqp`) -> extend `ChannelKindSpec` & builder dispatch
//! * Endpoints (HTTP, File) & adapters -> additional spec + builder sets
//! * Filters & routers -> expression parsing + pattern instantiation prior to channel send
//!
//! This documentation intentionally focuses on architecture & extensibility rather than reiterating
//! implementation details already present in source.

use crate::{
    channel::InMemoryChannel,
    error::{Error, Result},
    spec::{AlloraSpecYamlParser, ChannelSpecYamlParser},
};
use std::path::Path;

pub mod component_builders;
use component_builders::{build_channel_from_spec, build_channels_from_spec};
pub mod runtime;
use runtime::AlloraRuntime;

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

/// Internal helper: build full runtime from raw + format.
fn build_runtime_from_str(raw: &str, format: DslFormat) -> Result<AlloraRuntime> {
    match format {
        DslFormat::Yaml => {
            let top = AlloraSpecYamlParser::parse_str(raw)?;
            let channels_spec = top.channels_spec(); // borrow rather than consume
            let channels = build_channels_from_spec(channels_spec.clone())?; // clone spec for build
            Ok(AlloraRuntime::new(channels))
        }
        DslFormat::Json => Err(Error::serialization("json format not yet supported")),
        DslFormat::Xml => Err(Error::serialization("xml format not yet supported")),
    }
}

/// Public: build full runtime from a file path (future: endpoints, filters, etc.).
pub fn build(path: impl AsRef<Path>) -> Result<AlloraRuntime> {
    let path_ref = path.as_ref();
    let raw =
        std::fs::read_to_string(path_ref).map_err(|e| Error::other(format!("read error: {e}")))?;
    let format = DslFormat::from_path(path_ref)
        .ok_or_else(|| Error::serialization("cannot infer DSL format from extension"))?;
    build_runtime_from_str(&raw, format)
}
