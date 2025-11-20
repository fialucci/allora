//! ServiceActivatorsSpec: collection of ServiceActivatorSpec entries (v1) sharing a version.
//! Mirrors FiltersSpec semantics for ordered aggregation and deferred uniqueness enforcement.
//!
//! # Responsibilities
//! * Preserve declaration order (Vec internally).
//! * Carry a single `version` validated by parser (future parser TBD).
//! * Allow optional / duplicate service activator ids; uniqueness & auto-id handled in builder.
//!
//! # Auto-ID Strategy (Builder Level)
//! * Missing `service-activator.id` values assigned deterministic `service:auto.N` IDs starting at 1.
//! * Duplicate explicit ids trigger a build error.
//!
//! # Field Semantics
//! * `ref_name` is a reference identifier matching the `#[service(name=...)]` macro parameter,
//!   NOT a file path. It links the YAML configuration to the registered service implementation.
//!
//! # Example (programmatic construction)
//! ```rust
//! use allora_runtime::spec::{ServiceActivatorSpec, ServiceActivatorsSpec};
//! let spec = ServiceActivatorsSpec::new(1)
//!     .add(ServiceActivatorSpec::with_id("svc.hello", "hello_world", "inbound.a", "outbound.b"))
//!     .add(ServiceActivatorSpec::new("worker_service", "inbound.b", "outbound.c"));
//! assert_eq!(spec.services_activators().len(), 2);
//! assert_eq!(spec.services_activators()[0].id(), Some("svc.hello"));
//! assert!(spec.services_activators()[1].id().is_none());
//! ```

use crate::spec::ServiceActivatorSpec;

#[derive(Debug, Clone)]
pub struct ServiceActivatorsSpec {
    version: u32,
    services: Vec<ServiceActivatorSpec>,
}

impl ServiceActivatorsSpec {
    pub fn new(version: u32) -> Self {
        Self {
            version,
            services: Vec::new(),
        }
    }
    pub fn add(mut self, s: ServiceActivatorSpec) -> Self {
        self.services.push(s);
        self
    }
    pub fn push(&mut self, s: ServiceActivatorSpec) {
        self.services.push(s);
    }
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn services_activators(&self) -> &[ServiceActivatorSpec] {
        &self.services
    }
    pub fn into_service_activators(self) -> Vec<ServiceActivatorSpec> {
        self.services
    }
}
