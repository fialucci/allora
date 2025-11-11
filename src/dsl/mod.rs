//! DSL Facade: multi-format (YAML today; JSON/XML forthcoming) configuration entry points.
//!
//! This module orchestrates conversion of external textual configuration into runtime
//! components by coordinating three layers:
//! 1. Parsers (in `spec/`): format-specific translation into strongly typed specs.
//! 2. Builders (in `dsl/component_builders.rs`): spec -> concrete runtime objects.
//! 3. Facade (this file): public ergonomic API + format inference.
//!
//! # Goals
//! * Provide a minimal, stable surface (`build`, `build_channel`, `build_channel_from_str`, `build_filter`, `DslFormat`).
//! * Keep parsing & instantiation decoupled so additional formats/components add minimal code.
//! * Fail fast with clear, categorized errors (`Error::Serialization` vs `Error::Other`).
//!
//! # Supported Components (v1)
//! * Channel (InMemory kind)
//! * Filter (single filter spec via `build_filter` AND aggregated when present in top-level `allora.yml` into `AlloraRuntime`)
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
//! # Building a Single Filter
//! ```rust
//! use allora::build_filter;
//! let f = build_filter("tests/fixtures/filter.yml").unwrap();
//! // apply using f.accepts(exchange)
//! ```
//!
//! # Building Multiple Filters (collection spec)
//! Parse with `FiltersSpecYamlParser` then call `build_filters_from_spec` (internal builder). IDs are auto-generated when absent.
//! ```rust
//! use allora::spec::FiltersSpecYamlParser;
//! use allora::dsl::component_builders::build_filters_from_spec;
//! let raw = std::fs::read_to_string("tests/fixtures/filters.yml").unwrap();
//! let spec = FiltersSpecYamlParser::parse_str(&raw).unwrap();
//! let filters = build_filters_from_spec(spec).unwrap();
//! assert!(!filters.is_empty());
//! ```
//!
//! # Building the Full Runtime (AlloraRuntime)
//! Use `build()` when you want all declared components from top-level `allora.yml`:
//! ```rust
//! use allora::{build, Channel, Filter};
//! let rt = build("tests/fixtures/allora.yml").unwrap();
//! assert!(rt.channel_by_id("inbound.orders").is_some());
//! assert!(rt.filters().len() >= 1); // filters aggregated when declared
//! ```
//! Runtime accessors now: `channels()`, `channel_by_id()`, `channel_count()`, `filters()`, `filter_count()`, plus ownership via `into_channels()` / `into_filters()`.
//!
//! # Access Patterns
//! * Borrow: `rt.channels()`, `rt.filters()`
//! * Lookup: `rt.channel_by_id("inbound.orders")`
//! * Counts: `rt.channel_count()`, `rt.filter_count()`
//! * Consume: `rt.into_channels()`, `rt.into_filters()`
//!
//! # Error Semantics
//! * `Error::Other` – I/O failures (e.g. unreadable file path)
//! * `Error::Serialization` – structural issues (missing fields / invalid values / unsupported version / unsupported format)
//!
//! # Extension Guide (Adding New Components)
//! 1. Define data model spec (`*_spec.rs`)
//! 2. Add parser (`*_spec_yaml.rs`) validating version + fields
//! 3. Extend `AlloraSpec` to hold new spec collection (e.g. filters, endpoints)
//! 4. Add builder in `component_builders.rs`
//! 5. Augment `build_runtime_from_str` to assemble new runtime objects
//! 6. Add accessors on `AlloraRuntime`
//!
//! # Format Addition (JSON Outline)
//! * Introduce `*_json_parser` implementing `parse_str`
//! * Branch in `build_*_from_str` and `build_runtime_from_str`
//! * Reuse existing builders (format-agnostic)
//!
//! # Testing Strategy
//! * Parser edge cases near parser modules (`*_spec_yaml.rs`)
//! * Builder invariants in dedicated tests (channels, filters)
//! * Facade behavior & runtime aggregation in runtime-focused tests
//!
//! # Versioning
//! * Each spec parser validates an explicit integer `version`
//! * Breaking changes add parallel parser modules (`*_v2_yaml.rs`) preserving old behavior
//!
//! # Internal Helpers (Non-Public)
//! * `build_runtime_from_str` – dispatcher from raw text + format (now aggregates filters when present)
//! * `build_filter_from_str` / `build_filters_from_str` – kept private
//!
//! # Roadmap
//! * Multiple channel kinds (kafka, amqp)
//! * Additional pattern components (routers, splitters) aggregated in runtime
//! * Endpoints & adapters (HTTP, file, custom transport)
//! * JSON/XML DSL formats
//! * Expanded expression language (parentheses, negation, path navigation)
//!
//! This documentation focuses on architecture & extensibility; see component modules for specifics.

use crate::{
    channel::InMemoryChannel,
    error::{Error, Result},
    patterns::filter::Filter,
    spec::{
        AlloraSpecYamlParser, ChannelSpecYamlParser, FilterSpecYamlParser, FiltersSpecYamlParser,
    },
};
use component_builders::{
    build_channel_from_spec, build_channels_from_spec, build_filter_from_spec,
    build_filters_from_spec,
};
use std::path::Path;

pub mod component_builders;
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

/// Build filter from raw string + specified format.
fn build_filter_from_str(raw: &str, format: DslFormat) -> Result<Filter> {
    match format {
        DslFormat::Yaml => {
            let spec = FilterSpecYamlParser::parse_str(raw)?;
            build_filter_from_spec(spec)
        }
        DslFormat::Json => Err(Error::serialization("json format not yet supported")),
        DslFormat::Xml => Err(Error::serialization("xml format not yet supported")),
    }
}

/// Convenience: build filter from a file path (auto-detect format via extension).
pub fn build_filter(path: impl AsRef<Path>) -> Result<Filter> {
    let path_ref = path.as_ref();
    let raw =
        std::fs::read_to_string(path_ref).map_err(|e| Error::other(format!("read error: {e}")))?;
    let format = DslFormat::from_path(path_ref)
        .ok_or_else(|| Error::serialization("cannot infer DSL format from extension"))?;
    build_filter_from_str(&raw, format)
}

/// Build filters from raw string + format (collection form) - private helper.
fn build_filters_from_str(raw: &str, format: DslFormat) -> Result<Vec<Filter>> {
    match format {
        DslFormat::Yaml => {
            let spec = FiltersSpecYamlParser::parse_str(raw)?;
            build_filters_from_spec(spec)
        }
        _ => Err(Error::serialization("filters: format not yet supported")),
    }
}

/// Internal helper: build full runtime from raw + format.
fn build_runtime_from_str(raw: &str, format: DslFormat) -> Result<AlloraRuntime> {
    match format {
        DslFormat::Yaml => {
            let top = AlloraSpecYamlParser::parse_str(raw)?;
            let channels = build_channels_from_spec(top.into_channels_spec())?;
            let mut rt = AlloraRuntime::new(channels);
            if let Some(fspec) = top.filters_spec() {
                let filters = build_filters_from_spec(fspec.clone())?;
                rt = rt.with_filters(filters);
            }
            Ok(rt)
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
