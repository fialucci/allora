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
//! * Each parser validates an integer `version` field (currently must equal 1).
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
//! # Specification Modules
//!
//! ## AlloraSpec
//! The top-level specification aggregate. Currently only supports channel specifications, but
//! designed for future extension to include endpoints, filters, and adapters.
//!
//! ## ChannelSpec
//! Represents the intent for a single channel, including its type (kind) and identifier. Used for
//! both programmatic specification and as a data model for parsed YAML configurations.
//!
//! ## ChannelsSpec
//! A collection of channel specifications that share the same version. Facilitates versioned
//! management of multiple channel specs.
//!
//! # YAML Parser Modules
//! Each spec type has a corresponding YAML parser module (e.g., `channel_spec_yaml.rs`) that
//! translates parsed YAML values into the respective spec data models. These parsers also handle
//! structural validation and version checks to ensure compatibility and correctness of the
//! specifications.
//!
//! # DSL Builders
//! Located in `dsl/component_builders.rs`, these builders take the spec data models and handle
//! the runtime instantiation logic, such as assigning unique IDs to channels and enforcing
//! uniqueness constraints. They serve as the bridge between the declarative spec definitions and
//! the imperative runtime environment.
//!
//! # Facade
//! The facade layer, accessible through `dsl/mod.rs`, provides a simplified interface for
//! building runtime components from specifications. It handles format inference and offers a
//! unified `build()` function that returns the constructed `AlloraRuntime` instance.
//!
//! # Goals
//! The primary goals of this architecture are to provide a clear separation between the various
//! stages of specification parsing, validation, and runtime instantiation, enable programmatic
//! specification construction without the need for YAML, and allow for non-breaking extensions
//! of the system to accommodate new component types in the future.
//!
//! # Versioning Strategy
//! The versioning strategy involves validating an integer `version` field in the specifications,
//! with breaking changes leading to the introduction of new versioned parser modules, while
//! maintaining older parsers for backward compatibility.
//!
//! # Uniqueness & IDs
//! Uniqueness and ID management is handled by the builders, which enforce uniqueness constraints
//! and generate deterministic auto IDs for channel specifications where necessary.
//!
//! # Example
//! An example demonstrating both programmatic specification and YAML parsing is provided to
//! illustrate the usage and capabilities of the specification module.
pub mod allora_spec;
pub mod allora_spec_yaml;
pub mod channel_spec;
pub mod channel_spec_yaml;
pub mod channels_spec;
pub mod channels_spec_yaml;

pub use allora_spec::AlloraSpec;
pub use allora_spec_yaml::AlloraSpecYamlParser;
pub use channel_spec::{ChannelKindSpec, ChannelSpec};
pub use channel_spec_yaml::ChannelSpecYamlParser;
pub use channels_spec::ChannelsSpec;
pub use channels_spec_yaml::ChannelsSpecYamlParser;
