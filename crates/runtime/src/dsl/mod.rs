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
//! * Service (single service spec via `build_service`; aggregation forthcoming when services spec collection is added)
//!
//! # Supported Formats
//! * YAML (`DslFormat::Yaml`)
//! * JSON / XML reserved (emit explicit unsupported errors)
//!
//! # Building a Single Channel
//! ```no_run
//! use allora_core::Channel;
//! use allora_runtime::build_channel;
//! let ch = build_channel("tests/fixtures/channel.yml").unwrap();
//! println!("channel id={}", ch.id());
//! ```
//!
//! # Building a Single Filter
//! ```no_run
//! use allora_runtime::build_filter;
//! // Requires tests/fixtures/filter.yml present.
//! let f = build_filter("tests/fixtures/filter.yml").unwrap();
//! // apply using f.accepts(exchange)
//! ```
//!
//! # Building a Single Service
//! ```no_run
//! use allora_runtime::build_service;
//! // Requires tests/fixtures/service.yml present.
//! let svc = build_service("tests/fixtures/service.yml").unwrap();
//! // invoke via svc.process_sync(&mut exchange)
//! ```
//!
//! # Building Multiple Filters (collection spec)
//! ```no_run
//! use allora_runtime::spec::FiltersSpecYamlParser;
//! use allora_runtime::dsl::component_builders::build_filters_from_spec;
//! let raw = std::fs::read_to_string("tests/fixtures/filters.yml").unwrap();
//! let spec = FiltersSpecYamlParser::parse_str(&raw).unwrap();
//! let filters = build_filters_from_spec(spec).unwrap();
//! assert!(!filters.is_empty());
//! ```
//!
//! # Building the Full Runtime (AlloraRuntime)
//! ```no_run
//! use allora_runtime::{build, Channel, Filter};
//! let rt = build("tests/fixtures/allora.yml").unwrap();
//! assert!(rt.channel_by_id("inbound.orders").is_some());
//! assert!(rt.filters().len() >= 1);
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
//! * `build_filter_from_str` – kept private
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
    service_activator_processor::ServiceActivatorProcessor,
    spec::{
        AlloraSpecYamlParser, ChannelSpecYamlParser, FilterSpecYamlParser, ServiceSpecYamlParser,
    },
    Channel, Error, Filter, Result,
};
use component_builders::{
    build_channel_from_spec, build_channels_from_spec, build_filter_from_spec,
    build_filters_from_spec, build_http_inbound_adapters_from_spec,
    build_http_outbound_adapters_from_spec, build_service_from_spec,
};
use std::path::Path;

pub mod component_builders;
pub mod pattern_registry;
pub mod runtime;
pub use pattern_registry::{
    PatternRegistry, STRATEGY_CONCAT_TEXT, STRATEGY_EMIT_SIGNAL, STRATEGY_JSON_ARRAY,
};
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
pub fn build_channel_from_str(raw: &str, format: DslFormat) -> Result<Box<dyn Channel>> {
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
pub fn build_channel(path: impl AsRef<Path>) -> Result<Box<dyn Channel>> {
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

/// Build service from raw string + specified format.
fn build_service_from_str(
    raw: &str,
    format: DslFormat,
) -> Result<component_builders::ServiceProcessor> {
    match format {
        DslFormat::Yaml => {
            let spec = ServiceSpecYamlParser::parse_str(raw)?;
            build_service_from_spec(spec)
        }
        DslFormat::Json => Err(Error::serialization("json format not yet supported")),
        DslFormat::Xml => Err(Error::serialization("xml format not yet supported")),
    }
}

/// Convenience: build service from a file path (auto-detect format via extension).
pub fn build_service(path: impl AsRef<Path>) -> Result<component_builders::ServiceProcessor> {
    let path_ref = path.as_ref();
    let raw =
        std::fs::read_to_string(path_ref).map_err(|e| Error::other(format!("read error: {e}")))?;
    let format = DslFormat::from_path(path_ref)
        .ok_or_else(|| Error::serialization("cannot infer DSL format from extension"))?;
    build_service_from_str(&raw, format)
}

/// Internal helper: build full runtime from raw + format.
///
/// `pub(crate)` so unit tests in `crate::runtime` can build an
/// `AlloraRuntime` from an inline YAML string without writing a temp
/// file.
pub(crate) fn build_runtime_from_str(raw: &str, format: DslFormat) -> Result<AlloraRuntime> {
    match format {
        DslFormat::Yaml => {
            let top = AlloraSpecYamlParser::parse_str(raw)?;
            let filters_spec = top.filters_spec().cloned();
            let services_spec = top.services_spec().cloned();
            let http_inbound_spec = top.http_inbound_adapters_spec().cloned();
            let http_outbound_spec = top.http_outbound_adapters_spec().cloned();
            let channels_spec = top.into_channels_spec();
            let channels = build_channels_from_spec(channels_spec)?;
            let mut rt = AlloraRuntime::new(channels);
            if let Some(fspec) = filters_spec {
                let filters = build_filters_from_spec(fspec)?;
                rt = rt.with_filters(filters);
            }
            if let Some(sspec) = services_spec {
                let services =
                    component_builders::build_service_activators_from_spec(sspec.clone())?;
                rt = rt.with_services(services);
                // Build activator processors from original specs (clone services spec entries)
                let mut procs = Vec::new();
                for s in sspec.services_activators() {
                    procs.push(ServiceActivatorProcessor::new(s.clone()));
                }
                rt = rt.with_service_processors(procs);
            }
            if let Some(hspec) = http_inbound_spec {
                let lookup = |id: &str| rt.channel_ref_by_id(id);
                let adapters = build_http_inbound_adapters_from_spec(hspec, &lookup)?;
                rt = rt.with_http_inbound_adapters(adapters);
            }
            if let Some(ospec) = http_outbound_spec {
                let outbound = build_http_outbound_adapters_from_spec(ospec)?;
                rt = rt.with_http_outbound_adapters(outbound);
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
