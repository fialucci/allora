//! Spec module: programmatic and YAML-backed component specifications.
//!
//! Provides strongly typed specification builders ("Spec") for components.
//! Initially supports Channels; future additions (EndpointSpec, AdapterSpec) can
//! follow the same pattern.
//!
//! # Goals
//! * Separate internal DSL translation from public specification building.
//! * Enable programmatic spec construction without YAML.
//! * Provide clear, future-proof types (`ChannelSpec`, etc.) over `*Dsl` suffix.
//!
//! # Public Exports
//! * `ChannelSpec` / `ChannelKind` – programmatic specification types.
//! * `build_channel_spec_from_yaml_value` – translate parsed YAML value to `InMemoryChannel`.
//! * `build_channel_spec_from_path` – convenience file path builder.
//!
//! # Layering
//! * `channel_spec.rs` – pure data model (no parsing / IO / instantiation).
//! * `channel_spec_yaml.rs` – format parser (YAML -> ChannelSpec).
//! * `dsl/component_builders.rs` – spec -> runtime component construction.
//!
//! # Adding a New Component Spec (Endpoint)
//! 1. Create `endpoint_spec.rs` for the data model.
//! 2. Add `endpoint_spec_yaml.rs` for YAML parsing.
//! 3. Extend DSL builders with `build_endpoint_from_spec`.
//!
//! # Versioning Strategy
//! * Each parser validates `version` and rejects unsupported ones.
//! * Introduce new parser modules (`*_v2`) rather than mutating existing behavior.
//! * Maintain backwards compatibility by leaving older parsers intact.
//!
//! # Example
//! ```rust
//! use allora::{spec::ChannelSpec, build_channel_from_str, DslFormat, Channel};
//! // Programmatic spec
//! let spec = ChannelSpec::in_memory().id("prog-demo");
//! assert_eq!(spec.channel_id(), Some("prog-demo"));
//! // Parsed via DSL facade
//! let raw = "version: 1\nchannel:\n  kind: in_memory\n  id: parsed-demo";
//! let chan = build_channel_from_str(raw, DslFormat::Yaml).unwrap();
//! assert_eq!(chan.id(), "parsed-demo");
//! ```
pub mod channel_spec;
pub mod channel_spec_yaml;

pub use channel_spec::{ChannelKindSpec, ChannelSpec};
pub use channel_spec_yaml::ChannelSpecYamlParser;
