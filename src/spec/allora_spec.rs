//! AlloraSpec: top-level configuration spec aggregating inner component specs (currently channels only).
//!
//! # Responsibilities
//! * Capture version once for all nested component collections.
//! * Provide structured access to nested specs (`channels_spec`).
//! * Act as stable expansion point for future components (endpoints, filters, adapters) without
//!   changing the external DSL facade return type (`AlloraRuntime`).
//!
//! # Not Responsible For
//! * Parsing (handled by `AlloraSpecYamlParser`).
//! * Runtime instantiation (handled by `dsl/component_builders.rs` via `build_channels_from_spec`).
//! * Validation beyond version presence (nested parsers validate their own structures).
//!
//! # Future Extensions
//! Additional fields (e.g. `endpoints: EndpointsSpec`, `filters: FiltersSpec`) can be added while
//! preserving existing methods. Accessor naming should stay consistent (`*_spec()` for borrowing,
//! `into_*_spec()` for ownership transfer).
//!
//! # Example
//! ```rust
//! use allora::spec::{AlloraSpec, ChannelsSpec, ChannelSpec};
//! // Programmatic construction
//! let channels = ChannelsSpec::new(1).add(ChannelSpec::in_memory().id("orders"));
//! let all = AlloraSpec::new(1, channels);
//! assert_eq!(all.version(), 1);
//! assert_eq!(all.channels_spec().channels().len(), 1);
//! ```
//!

use crate::spec::ChannelsSpec;

#[derive(Debug, Clone)]
pub struct AlloraSpec {
    version: u32,
    channels: ChannelsSpec,
}

impl AlloraSpec {
    pub fn new(version: u32, channels: ChannelsSpec) -> Self {
        Self { version, channels }
    }
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn channels_spec(&self) -> &ChannelsSpec {
        &self.channels
    }
    pub fn into_channels_spec(self) -> ChannelsSpec {
        self.channels
    }
}
