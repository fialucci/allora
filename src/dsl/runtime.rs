//! AlloraRuntime: aggregate of built runtime components (extensible).
//!
//! Current contents:
//! * channels: vector of in-memory channels (single kind)
//!
//! Future extensions (not yet wired):
//! * endpoints
//! * filters / routers
//! * adapters
//! * correlation groups
//!
//! Design goals:
//! * Single return object from top-level build to avoid signature churn as components grow.
//! * Provide accessor methods with owned + borrowed variants.
//! * Keep internal storage concrete now; migrate to trait objects (`ChannelRef`) when multiple channel kinds arrive.
//!
//! Backward compatibility note removed: prefer `build()` which returns `AlloraRuntime`.
//!
//! # Overview
//! `AlloraRuntime` is the single return object from the top-level `build()` DSL facade.
//! It bundles all instantiated runtime components derived from a configuration spec.
//! Currently it only contains channels (in-memory kind); future releases will extend it
//! with endpoints, filters/routers, adapters, and correlation utilities without changing
//! the public `build()` signature.
//!
//! # Guarantees
//! * The collection of channels preserves the order they were defined in the source spec.
//! * Channel IDs are unique (enforced at build time); missing IDs receive deterministic
//!   `channel:auto.N` identifiers within the same build invocation.
//! * Lookup (`channel_by_id`) performs a linear scan; acceptable for small collections.
//!   This can be optimized later by introducing an internal index without API changes.
//!
//! # Usage Example
//! ```rust
//! use allora::{build, Channel};
//! let rt = build("tests/fixtures/allora.yml").unwrap();
//! assert!(rt.channel_by_id("inbound.orders").is_some());
//! for ch in rt.channels() { println!("id={}", ch.id()); }
//! ```
//!
//! # Future Extensions (Illustrative)
//! * `endpoints()` -> &[Endpoint]
//! * `filters()` / `routers()` -> pattern components
//! * `adapters()` -> inbound / outbound integration points
//! * `correlations()` -> tracking groups for aggregation patterns
//! These will be added as additional fields with accessor methods while keeping
//! `AlloraRuntime` construction centralized in the DSL facade.
use crate::channel::Channel;
// bring trait for id()
use crate::channel::InMemoryChannel;

#[derive(Debug)]
/// Aggregated runtime container for all built components (channels today, more later).
///
/// Prefer borrowing via the accessor methods (`channels()`, `channel_by_id`) for read-only
/// operations. Use `into_channels()` only when you need ownership transfer (e.g. embedding
/// channels into another structure or performing manual lifecycle management).
pub struct AlloraRuntime {
    channels: Vec<InMemoryChannel>,
    // future: endpoints: Vec<Endpoint>,
    // future: filters: Vec<Filter>,
}

impl AlloraRuntime {
    /// Create a new runtime instance from a vector of channels.
    ///
    /// Typically invoked internally by the DSL (`build_runtime_from_str`).
    pub fn new(channels: Vec<InMemoryChannel>) -> Self {
        Self { channels }
    }
    /// Borrow all channels (read-only slice).
    pub fn channels(&self) -> &[InMemoryChannel] {
        &self.channels
    }
    /// Consume the runtime, yielding owned channels.
    pub fn into_channels(self) -> Vec<InMemoryChannel> {
        self.channels
    }
    /// Find a channel by its id; returns `None` if not present.
    ///
    /// Complexity: O(n). Optimizations (hash index) can be added later without
    /// changing this method's signature or semantics.
    pub fn channel_by_id(&self, id: &str) -> Option<&InMemoryChannel> {
        self.channels.iter().find(|c| c.id() == id)
    }
    /// Total number of channels in this runtime.
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }
}
