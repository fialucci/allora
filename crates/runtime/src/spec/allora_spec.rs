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
//! use allora_runtime::spec::{AlloraSpec, ChannelsSpec, ChannelSpec};
//! // Programmatic construction
//! let channels = ChannelsSpec::new(1).add(ChannelSpec::queue().id("orders"));
//! let all = AlloraSpec::new(1, channels);
//! assert_eq!(all.version(), 1);
//! assert_eq!(all.channels_spec().channels().len(), 1);
//! ```
//!

use crate::spec::{ChannelsSpec, FiltersSpec, ServiceActivatorsSpec, HttpInboundAdaptersSpec};

#[derive(Debug, Clone)]
pub struct AlloraSpec {
    version: u32,
    channels: ChannelsSpec,
    filters: Option<FiltersSpec>,
    services: Option<ServiceActivatorsSpec>,
    http_inbound_adapters: Option<HttpInboundAdaptersSpec>,
}

impl AlloraSpec {
    pub fn new(version: u32, channels: ChannelsSpec) -> Self {
        Self {
            version,
            channels,
            filters: None,
            services: None,
            http_inbound_adapters: None,
        }
    }
    /// Construct with an optional filters collection in a single call (ergonomic alternative to chaining `with_filters`).
    /// When `filters` is `None`, behaves the same as `new`.
    pub fn new_with_filters(
        version: u32,
        channels: ChannelsSpec,
        filters: Option<FiltersSpec>,
    ) -> Self {
        Self {
            version,
            channels,
            filters,
            services: None,
            http_inbound_adapters: None,
        }
    }
    pub fn with_filters(mut self, filters: FiltersSpec) -> Self {
        self.filters = Some(filters);
        self
    }
    pub fn with_services(mut self, services: ServiceActivatorsSpec) -> Self {
        self.services = Some(services);
        self
    }
    pub fn with_http_inbound_adapters(mut self, adapters: HttpInboundAdaptersSpec) -> Self {
        self.http_inbound_adapters = Some(adapters);
        self
    }
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn channels_spec(&self) -> &ChannelsSpec {
        &self.channels
    }
    pub fn filters_spec(&self) -> Option<&FiltersSpec> {
        self.filters.as_ref()
    }
    pub fn services_spec(&self) -> Option<&ServiceActivatorsSpec> {
        self.services.as_ref()
    }
    pub fn http_inbound_adapters_spec(&self) -> Option<&HttpInboundAdaptersSpec> {
        self.http_inbound_adapters.as_ref()
    }
    pub fn into_channels_spec(self) -> ChannelsSpec {
        self.channels
    }
    pub fn into_filters_spec(self) -> Option<FiltersSpec> {
        self.filters
    }
    pub fn into_services_spec(self) -> Option<ServiceActivatorsSpec> {
        self.services
    }
    pub fn into_http_inbound_adapters_spec(self) -> Option<HttpInboundAdaptersSpec> {
        self.http_inbound_adapters
    }
}
