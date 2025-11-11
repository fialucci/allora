//! ChannelsSpec: collection of `ChannelSpec` entries under one version (schema v1).
//!
//! # Semantics
//! * Represents intent for multiple channels declared together sharing a version.
//! * Order of insertion is preserved (stable iteration order).
//! * Does not enforce uniqueness or non-empty ids (delegated to builders).
//!
//! # Construction Patterns
//! * Immutable-style chaining via `add` (returns Self for fluent building).
//! * Mutable push via `push` when incremental assembly is preferred.
//!
//! # Example
//! ```rust
//! use allora::spec::{ChannelsSpec, ChannelSpec};
//! let spec = ChannelsSpec::new(1)
//!     .add(ChannelSpec::in_memory().id("orders"))
//!     .add(ChannelSpec::in_memory().id("payments"));
//! assert_eq!(spec.channels().len(), 2);
//! ```
//!
//! # Deferred Validation
//! Structural and kind validation occur during YAML parsing (`channels_spec_yaml.rs`). Uniqueness
//! and deterministic id generation for missing ids occur during runtime build (`build_channels_from_spec`).

use crate::spec::ChannelSpec;

#[derive(Debug, Clone, Default)]
pub struct ChannelsSpec {
    version: u32,
    channels: Vec<ChannelSpec>,
}

impl ChannelsSpec {
    pub fn new(version: u32) -> Self {
        Self {
            version,
            channels: Vec::new(),
        }
    }
    pub fn add(mut self, ch: ChannelSpec) -> Self {
        self.channels.push(ch);
        self
    }
    pub fn push(&mut self, ch: ChannelSpec) {
        self.channels.push(ch);
    }
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn channels(&self) -> &[ChannelSpec] {
        &self.channels
    }
    pub fn into_channels(self) -> Vec<ChannelSpec> {
        self.channels
    }
}
