//! Spec module: programmatic component specification data models and format parsers.
//!
//! Provides strongly typed specification builders ("Spec") for components. Parsing of external
//! configuration (YAML today) is isolated in `*_spec_yaml.rs` modules that yield these pure data
//! structs. Runtime instantiation is handled separately by DSL builders (`dsl/component_builders.rs`).
//!
//! # Current Specs
//! * `ChannelSpec` / `ChannelKindSpec` – single channel intent.
//! * `ChannelsSpec` – collection of channel specs sharing a version.
//! * `AlloraSpec` – top-level aggregate (currently only channels; future: endpoints, filters, adapters).
//!
//! # Layering
//! 1. Spec data models (`*_spec.rs`) – no parsing, no IO, no instantiation.
//! 2. Parsers (`*_spec_yaml.rs`) – YAML Value -> Spec (+ structural validation, version checks).
//! 3. DSL builders (`dsl/component_builders.rs`) – Spec -> runtime (assign ids, enforce uniqueness).
//! 4. Facade (`dsl/mod.rs`) – format inference + `build()` returning `AlloraRuntime`.
//!
//! # Goals
//! * Clear separation between parsing, specification, and instantiation.
//! * Programmatic construction (e.g. `ChannelSpec::in_memory().id("orders")`) without YAML.
//! * Non-breaking extension path for new components (add new spec + parser; do not alter existing semantics).
//!
//! # Versioning Strategy
//! * Each parser validates an integer `version` field (currently must equal 1) via shared helper `version::validate_version`.
//! * Breaking changes introduce new versioned parser modules (e.g. `channel_spec_yaml_v2.rs`).
//! * Older parsers remain for backward compatibility.
//!
//! # Uniqueness & IDs
//! * Parsers allow duplicate / missing `id` values (structural validation only).
//! * Builders enforce uniqueness (error on duplicates) and generate deterministic auto IDs (`channel:auto.N`) for missing channel ids in multi-builds; single channel builds use a UUID-based id.
//!
//! # Example (Programmatic + YAML)
//! ```rust
//! use allora::{build_channel_from_str, spec::ChannelSpec, Channel, DslFormat};
//! // Programmatic spec
//! let spec = ChannelSpec::in_memory().id("prog-demo");
//! assert_eq!(spec.channel_id(), Some("prog-demo"));
//! // YAML parsed via facade (no need to construct spec manually)
//! let raw = "version: 1\nchannel:\n  kind: in_memory\n  id: parsed-demo";
//! let chan = build_channel_from_str(raw, DslFormat::Yaml).unwrap();
//! assert_eq!(chan.id(), "parsed-demo");
//! ```
//!
//! # Specification Modules
//! * `AlloraSpec`: top-level aggregate; future extension point.
//! * `ChannelSpec`: single channel intent.
//! * `ChannelsSpec`: ordered collection of channel specs sharing a version.
//!
//! # YAML Parser Modules
//! Each spec type has a corresponding YAML parser module translating YAML values into the spec.
//! Parsers perform structural + version validation; they defer uniqueness to builders.
//!
//! # DSL Builders
//! Builders instantiate runtime components from specs (ID generation, uniqueness checks).
//!
//! # Facade
//! `dsl/mod.rs` exposes user-facing build APIs (`build()`, `build_channel()`), performing format inference.
//!
//! This documentation intentionally avoids redundancy; for architecture-wide details see crate root docs.
pub mod allora_spec;
pub mod allora_spec_yaml;
pub mod channel_spec;
pub mod channel_spec_yaml;
pub mod channels_spec;
pub mod channels_spec_yaml;
pub mod filter_spec;
pub mod filter_spec_yaml;
pub mod filters_spec;
pub mod filters_spec_yaml;
pub mod version;

pub use allora_spec::AlloraSpec;
pub use allora_spec_yaml::AlloraSpecYamlParser;
pub use channel_spec::{ChannelKindSpec, ChannelSpec};
pub use channel_spec_yaml::ChannelSpecYamlParser;
pub use channels_spec::ChannelsSpec;
pub use channels_spec_yaml::ChannelsSpecYamlParser;
pub use filter_spec::FilterSpec;
pub use filter_spec_yaml::FilterSpecYamlParser;
pub use filters_spec::FiltersSpec;
pub use filters_spec_yaml::FiltersSpecYamlParser;
