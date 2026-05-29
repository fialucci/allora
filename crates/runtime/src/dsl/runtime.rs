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
use crate::service_activator_processor::ServiceActivatorProcessor;
use crate::Filter;
use crate::HttpInboundAdapter;
use crate::HttpOutboundAdapter;
use std::sync::Arc;

#[derive(Debug)]
/// Aggregated runtime container for all built components (channels today, more later).
///
/// Prefer borrowing via the accessor methods (`channels()`, `channel_by_id`) for read-only
/// operations. Use `into_channels()` only when you need ownership transfer (e.g. embedding
/// channels into another structure or performing manual lifecycle management).
pub struct AlloraRuntime {
    channels: Vec<Arc<dyn Channel>>,
    filters: Vec<FilterActivation>,
    services: Vec<ServiceProcessor>,
    service_activator_processors: Vec<ServiceActivatorProcessor>,
    http_inbound_adapters: Vec<HttpInboundAdapter>,
    http_outbound_adapters: Vec<HttpOutboundAdapterActivation>,
}

/// Pairs a [`Filter`] predicate with the channel routing metadata from
/// the [`FilterSpec`] it was built from.
///
/// A bare `Filter` only knows how to evaluate its predicate
/// (`accepts(&exchange)`); it has no notion of "where to consume from" or
/// "where to forward to." `FilterActivation` carries that wiring intent
/// alongside the predicate so the runtime can subscribe each filter to
/// its inbound channel and dispatch accepted exchanges to its outbound
/// channel — the same pattern `ServiceActivatorProcessor` uses for
/// services.
///
/// Constructed by [`build_filters_from_spec`](crate::dsl::component_builders::build_filters_from_spec)
/// from a parsed [`FiltersSpec`] and stored on the runtime via
/// [`with_filters`](AlloraRuntime::with_filters). The runtime calls
/// [`wire_filters`](crate::wire_filters) during startup to subscribe
/// each activation to its channels.
///
/// When `to` is `None` the filter is treated as **predicate-only** —
/// kept in the runtime for callers that hold the `AlloraRuntime` and
/// invoke `filter.accepts(...)` directly, but not auto-wired.
pub struct FilterActivation {
    filter: std::sync::Arc<Filter>,
    from: String,
    to: Option<String>,
    id: String,
}

impl std::fmt::Debug for FilterActivation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilterActivation")
            .field("id", &self.id)
            .field("from", &self.from)
            .field("to", &self.to)
            .finish()
    }
}

impl FilterActivation {
    /// Build a new activation from a `Filter` (typically the output of
    /// `Filter::from_apl_with_id`) and its routing metadata.
    pub fn new(
        filter: Filter,
        from: impl Into<String>,
        to: Option<String>,
        id: impl Into<String>,
    ) -> Self {
        Self {
            filter: std::sync::Arc::new(filter),
            from: from.into(),
            to,
            id: id.into(),
        }
    }

    /// The wrapped filter predicate (shared via `Arc` so the runtime can
    /// hand a clone to its subscription closure).
    pub fn filter(&self) -> &std::sync::Arc<Filter> {
        &self.filter
    }

    /// Inbound channel ID the runtime subscribes the filter to.
    pub fn from(&self) -> &str {
        &self.from
    }

    /// Outbound channel ID the runtime dispatches accepted exchanges to.
    /// `None` means the filter is predicate-only (not auto-wired by the
    /// runtime; available via the runtime accessor for external use).
    pub fn to(&self) -> Option<&str> {
        self.to.as_deref()
    }

    /// Stable identifier (matches the `FilterSpec`'s `id`, or a
    /// deterministic `filter:auto.N` if the spec omitted it).
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// Pairs an [`HttpOutboundAdapter`] with the channel routing metadata
/// from the [`HttpOutboundAdapterSpec`] it was built from.
///
/// Mirrors [`FilterActivation`] for the http-outbound side. When the
/// spec had a `from:` field, the runtime subscribes a closure on that
/// inbound channel that:
///
/// 1. Calls `adapter.dispatch(&exchange)`.
/// 2. If the spec also had `to:`, transforms the exchange (response
///    body → `in_msg.payload`; status code + acknowledged → header
///    metadata) and forwards it to the outbound channel.
/// 3. If `to:` is `None`, the dispatch is fire-and-forget — result
///    logged at `debug!`, message dropped.
///
/// When `from:` is `None` the activation is **static-only** — the
/// adapter lives on the runtime for application code that calls
/// `.dispatch()` directly (the legacy http-outbound pattern). The
/// runtime does not auto-wire it.
pub struct HttpOutboundAdapterActivation {
    adapter: std::sync::Arc<HttpOutboundAdapter>,
    from: Option<String>,
    to: Option<String>,
    id: String,
}

impl std::fmt::Debug for HttpOutboundAdapterActivation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpOutboundAdapterActivation")
            .field("id", &self.id)
            .field("from", &self.from)
            .field("to", &self.to)
            .finish()
    }
}

impl HttpOutboundAdapterActivation {
    /// Build a new activation from a built `HttpOutboundAdapter` and its
    /// routing metadata.
    pub fn new(
        adapter: HttpOutboundAdapter,
        from: Option<String>,
        to: Option<String>,
        id: impl Into<String>,
    ) -> Self {
        Self {
            adapter: std::sync::Arc::new(adapter),
            from,
            to,
            id: id.into(),
        }
    }

    /// The wrapped HTTP outbound adapter (shared via `Arc` so the
    /// runtime can hand a clone to its subscription closure).
    pub fn adapter(&self) -> &std::sync::Arc<HttpOutboundAdapter> {
        &self.adapter
    }

    /// Inbound channel ID the runtime subscribes the adapter to.
    /// `None` means the activation is static-only — accessible via
    /// `AlloraRuntime::http_outbound_adapters()` but not auto-wired.
    pub fn from(&self) -> Option<&str> {
        self.from.as_deref()
    }

    /// Outbound channel ID for the post-dispatch exchange. Ignored
    /// when `from` is `None`. When `from` is set and `to` is `None`,
    /// dispatch is fire-and-forget.
    pub fn to(&self) -> Option<&str> {
        self.to.as_deref()
    }

    /// Stable identifier (matches the `HttpOutboundAdapterSpec`'s
    /// `id`, or `http-outbound:<host>:<port>` from the builder default
    /// when the spec omitted it).
    pub fn id(&self) -> &str {
        &self.id
    }
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
            http_inbound_adapters: Vec::new(),
            http_outbound_adapters: Vec::new(),
        }
    }
    /// Sets the filter activations for this runtime.
    ///
    /// Consumes the provided activations vector and assigns it to the
    /// runtime. The runtime's `wire_filters` step then subscribes each
    /// activation with a non-`None` `to:` to its inbound channel.
    pub fn with_filters(mut self, filters: Vec<FilterActivation>) -> Self {
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
    /// Sets the HTTP inbound adapters for this runtime.
    ///
    /// Consumes the provided adapters vector and assigns it to the runtime.
    pub fn with_http_inbound_adapters(mut self, adapters: Vec<HttpInboundAdapter>) -> Self {
        self.http_inbound_adapters = adapters;
        self
    }
    pub fn with_http_outbound_adapters(
        mut self,
        adapters: Vec<HttpOutboundAdapterActivation>,
    ) -> Self {
        self.http_outbound_adapters = adapters;
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
    /// Borrow all filter activations (read-only slice).
    pub fn filters(&self) -> &[FilterActivation] {
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
    /// Consume the runtime, yielding owned filter activations.
    pub fn into_filters(self) -> Vec<FilterActivation> {
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
    /// Return a cloned Arc<dyn Channel> by id if it exists (helper for builders needing ownership).
    pub fn channel_ref_by_id(&self, id: &str) -> Option<Arc<dyn Channel>> {
        self.channels
            .iter()
            .find(|c| c.id() == id)
            .map(|c| Arc::clone(c))
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
    pub fn http_inbound_adapters(&self) -> &[HttpInboundAdapter] {
        &self.http_inbound_adapters
    }
    pub fn http_inbound_adapter_count(&self) -> usize {
        self.http_inbound_adapters.len()
    }
    pub fn http_outbound_adapters(&self) -> &[HttpOutboundAdapterActivation] {
        &self.http_outbound_adapters
    }
    pub fn http_outbound_adapter_count(&self) -> usize {
        self.http_outbound_adapters.len()
    }
}
