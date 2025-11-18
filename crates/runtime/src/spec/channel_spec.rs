//! ChannelSpec: immutable, format-agnostic description of a Channel (pure pipe).
//!
//! # Purpose
//! Represents the minimal set of attributes required to instantiate a Channel.
//! Parsing (YAML / future JSON / XML) is delegated to format-specific parsers in
//! `channel_spec_yaml.rs` (and future siblings). Instantiation is handled by DSL builders.
//!
//! # Invariants
//! * `id` is optional; if omitted a runtime builder will generate a unique id.
//! * If `id` is present it must not be the empty string (builders enforce).
//! * `kind` enumerates supported implementations; currently only `queue`.
//!
//! # Adding New Channel Kinds
//! 1. Extend `ChannelKindSpec` enum (e.g. `Kafka`, `Redis`).
//! 2. Add validation + mapping logic inside each format parser.
//! 3. Update the component builder to match on the new variant.
//! 4. Provide user-facing documentation and tests (spec + builder).
//!
//! # Programmatic Example
//! ```rust
//! use allora_runtime::spec::ChannelSpec;
//! let spec = ChannelSpec::queue().id("orders-events");
//! assert_eq!(spec.channel_id(), Some("orders-events"));
//! ```
//!
//! Combined Channel specification & YAML translator (v1, pure pipe).
//! This file merges previous channel_dsl.rs (YAML DSL) and channel_spec.rs (programmatic spec).
//! See top-level docs in spec/mod.rs for overarching concepts.

use serde::Deserialize;

/// Programmatic Channel specification (pure pipe).
#[derive(Debug, Clone, Default)]
pub struct ChannelSpec {
    pub(crate) id: Option<String>,
    pub(crate) kind: ChannelKindSpec,
}

impl ChannelSpec {
    pub fn direct() -> Self {
        Self {
            id: None,
            kind: ChannelKindSpec::Direct,
        }
    }
    pub fn queue() -> Self {
        Self {
            id: None,
            kind: ChannelKindSpec::Queue,
        }
    }
    /// Set the channel identifier (required by some builders, ignored by others).
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
    /// Return the channel implementation kind captured by this spec.
    pub fn kind(&self) -> ChannelKindSpec {
        self.kind
    }
    /// Get the channel identifier, if one was set.
    pub fn channel_id(&self) -> Option<&str> {
        self.id.as_deref()
    }
}

/// Supported channel kinds (extend as more implementations are introduced).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelKindSpec {
    Direct,
    Queue,
}

impl Default for ChannelKindSpec {
    fn default() -> Self {
        ChannelKindSpec::Direct
    }
}
