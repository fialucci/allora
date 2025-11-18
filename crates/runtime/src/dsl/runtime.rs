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
//! ```no_run
//! use allora_core::Channel;
//! use allora_runtime::build;
//! // Requires a valid allora.yml at the given path.
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
use crate::dsl::component_builders::ServiceProcessor;
use crate::Filter;
use crate::service_activator_processor::ServiceActivatorProcessor;
use std::sync::Arc;

#[derive(Debug)]
/// Aggregated runtime container for all built components (channels today, more later).
///
/// Prefer borrowing via the accessor methods (`channels()`, `channel_by_id`) for read-only
/// operations. Use `into_channels()` only when you need ownership transfer (e.g. embedding
/// channels into another structure or performing manual lifecycle management).
pub struct AlloraRuntime {
    channels: Vec<Arc<dyn Channel>>,
    filters: Vec<Filter>,
    services: Vec<ServiceProcessor>,
    service_activator_processors: Vec<ServiceActivatorProcessor>,
}

impl AlloraRuntime {
    /// Create a new runtime instance from a vector of channels.
    ///
    /// Typically, invoked internally by the DSL (`build_runtime_from_str`).
    pub fn new(channels: Vec<Box<dyn Channel>>) -> Self {
        let channels_arc = channels.into_iter().map(|c| Arc::from(c)).collect();
        Self {
            channels: channels_arc,
            filters: Vec::new(),
            services: Vec::new(),
            service_activator_processors: Vec::new(),
        }
    }
    /// Sets the filters for this runtime.
    ///
    /// Consumes the provided filters vector and assigns it to the runtime.
    pub fn with_filters(mut self, filters: Vec<Filter>) -> Self {
        self.filters = filters;
        self
    }
    /// Sets the services for this runtime.
    ///
    /// Consumes the provided services vector and assigns it to the runtime.
    pub fn with_services(mut self, services: Vec<ServiceProcessor>) -> Self {
        self.services = services;
        self
    }
    pub fn with_service_processors(mut self, proc: Vec<ServiceActivatorProcessor>) -> Self {
        self.service_activator_processors = proc;
        self
    }
    /// Borrow all channels as an iterator of &dyn Channel (zero allocation).
    pub fn channels(&self) -> impl Iterator<Item = &dyn Channel> {
        self.channels.iter().map(|c| c.as_ref())
    }
    /// Borrow underlying boxed channel slice (rarely needed).
    pub fn channels_slice(&self) -> &[Arc<dyn Channel>] {
        &self.channels
    }
    /// Borrow all filters (read-only slice).
    pub fn filters(&self) -> &[Filter] {
        &self.filters
    }
    /// Borrow all services (read-only slice).
    pub fn services(&self) -> &[ServiceProcessor] {
        &self.services
    }
    /// Consume the runtime, yielding owned channels.
    pub fn into_channels(self) -> Vec<Arc<dyn Channel>> {
        self.channels
    }
    /// Consume the runtime, yielding owned filters.
    pub fn into_filters(self) -> Vec<Filter> {
        self.filters
    }
    /// Consume the runtime, yielding owned services.
    pub fn into_services(self) -> Vec<ServiceProcessor> {
        self.services
    }
    /// Find a channel by its id; returns `None` if not present.
    ///
    /// Complexity: O(n). Optimizations (hash index) can be added later without
    /// changing this method's signature or semantics.
    pub fn channel_by_id(&self, id: &str) -> Option<&dyn Channel> {
        self.channels
            .iter()
            .find(|c| c.id() == id)
            .map(|c| c.as_ref())
    }
    /// Generic typed channel lookup: returns &T if a channel with `id` exists and downcasts to T.
    pub fn channel_typed<T: Channel + 'static>(&self, id: &str) -> Option<&T> {
        self.channels
            .iter()
            .find(|c| c.id() == id)
            .and_then(|c| c.as_any().downcast_ref::<T>())
    }
    /// Required typed channel lookup: panics with a clear message if missing or wrong type.
    pub fn channel<T: Channel + 'static>(&self, id: &str) -> &T {
        for c in &self.channels {
            if c.id() == id {
                if let Some(t) = c.as_any().downcast_ref::<T>() {
                    return t;
                } else {
                    panic!(
                        "channel '{}' exists with kind '{}' but does not match expected type '{}'",
                        id,
                        c.kind(),
                        std::any::type_name::<T>()
                    );
                }
            }
        }
        panic!(
            "channel '{}' not found (expected type '{}')",
            id,
            std::any::type_name::<T>()
        );
    }
    /// Predicate: does channel id exist and is of type T?
    pub fn channel_is<T: Channel + 'static>(&self, id: &str) -> bool {
        self.channel_typed::<T>(id).is_some()
    }
    /// Total number of channels in this runtime.
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }
    /// Total number of filters in this runtime.
    pub fn filter_count(&self) -> usize {
        self.filters.len()
    }
    /// Total number of services in this runtime.
    pub fn service_count(&self) -> usize {
        self.services.len()
    }
    /// Total number of service processors in this runtime.
    pub fn service_processor_count(&self) -> usize {
        self.service_activator_processors.len()
    }
    pub fn service_activator_processors(&self) -> &[ServiceActivatorProcessor] {
        &self.service_activator_processors
    }
    pub fn service_activator_processor_by_id(
        &self,
        id: &str,
    ) -> Option<&ServiceActivatorProcessor> {
        self.service_activator_processors
            .iter()
            .find(|p| p.id() == id)
    }
    pub fn service_activator_processor_mut_by_id(
        &mut self,
        id: &str,
    ) -> Option<&mut ServiceActivatorProcessor> {
        self.service_activator_processors
            .iter_mut()
            .find(|p| p.id() == id)
    }
}
