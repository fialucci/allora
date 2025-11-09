//! DSL runtime component builders: instantiate runtime components from validated specs.
//! This module will host builders for multiple component types (Channel, Endpoint, Adapter, etc.).
//! Each builder converts a spec (format-agnostic, already validated) into a concrete runtime type.
//!
//! # Purpose
//! Bridge the gap between a format-specific parsed spec (e.g. YAML -> `ChannelSpec`) and the
//! actual runtime component (e.g. `InMemoryChannel`). Parsing & validation happen elsewhere
//! (under `spec/` parsers). Builders assume the spec is structurally valid and focus solely on
//! instantiation and enforcing runtime constraints (like non-empty IDs).
//!
//! # Design Principles
//! * One builder per component type (not per serialization format).
//! * Builders accept only strongly typed specs (no raw YAML/JSON here).
//! * Fail fast on remaining invariants (e.g. empty string ID) that are easier to check post-parse.
//! * Keep side-effects minimal: no I/O, no global state modifications.
//!
//! # Usage Example
//! ```rust
//! use allora::spec::ChannelSpec;
//! use allora::dsl::component_builders::build_channel_from_spec;
//! use allora::Channel; // bring trait into scope for channel.id()
//! let spec = ChannelSpec::in_memory().id("example-channel");
//! let channel = build_channel_from_spec(spec).unwrap();
//! assert_eq!(channel.id(), "example-channel");
//! ```
//!
//! # Auto-generated IDs
//! If the `ChannelSpec` omits an ID, the underlying `ChannelBuilder` creates an auto-generated
//! identifier (prefixed with `channel:`). Tests ensure uniqueness at a practical level.
//!
//! # Extending
//! To add support for a new component type (e.g. Endpoint):
//! 1. Define `EndpointSpec` under `spec/endpoint/`.
//! 2. Add a format parser (e.g. `EndpointSpecYamlParser`).
//! 3. Add a builder here: `pub fn build_endpoint_from_spec(spec: EndpointSpec) -> Result<InMemoryEndpoint>`.
//! 4. Expose a DSL facade entry point akin to `build_channel_from_str`.
//!
//! # Error Semantics
//! * Returns `Error::Serialization` for invariant violations (currently only empty ID).
//! * Propagates any future construction failures as `Error::Other` where appropriate.
//!
//! # Future Improvements
//! * Centralize shared build-time checks (e.g. ID normalization) in utility functions.
//! * Introduce traits (`ComponentSpec`, `ComponentBuilder`) for generic dispatch.
//! * Metrics/telemetry hooks (opt-in) before/after instantiation.

use crate::{
    channel::{ChannelBuilder, InMemoryChannel},
    error::{Error, Result},
    spec::{ChannelKindSpec, ChannelSpec},
};

/// Build a concrete channel from a validated `ChannelSpec`.
///
/// # Invariants
/// * `spec.kind()` must refer to a supported channel kind (currently only `InMemory`).
/// * If `spec.channel_id()` is Some("") this returns `Error::Serialization`.
/// * If `spec.channel_id()` is `None`, an auto-generated ID is assigned.
///
/// # Errors
/// * `Error::Serialization` – empty ID string.
///
/// # Examples
/// ```rust
/// use allora::spec::ChannelSpec;
/// use allora::dsl::component_builders::build_channel_from_spec;
/// use allora::Channel; // trait import for id()
/// let spec = ChannelSpec::in_memory().id("chan-A");
/// let channel = build_channel_from_spec(spec).unwrap();
/// assert_eq!(channel.id(), "chan-A");
/// ```
pub fn build_channel_from_spec(spec: ChannelSpec) -> Result<InMemoryChannel> {
    match spec.kind() {
        ChannelKindSpec::InMemory => {
            let builder = ChannelBuilder::point_to_point().in_memory();
            Ok(match spec.channel_id() {
                Some("") => return Err(Error::serialization("channel.id must not be empty")),
                Some(id) => builder.id(id).build(),
                None => builder.build(),
            })
        }
    }
}
