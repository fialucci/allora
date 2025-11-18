//! ServiceActivatorSpec: specification for a single processing service (activator) binding logic between channels.
//!
//! # Purpose
//! Represents a unit of executable logic (a processor) wired between an inbound (`from`) and
//! outbound (`to`) channel. The `ref_name` field is a symbolic reference matching the
//! `#[service(name=...)]` macro parameter used to locate the registered implementation (not a file path).
//!
//! # Field Semantics
//! * `id` (optional): Explicit identifier for referencing/diagnostics. Missing ids may be assigned
//!   deterministically by a future collection builder (`service:auto.N`).
//! * `ref_name` (required, non-empty): Reference identifier matching the service macro name; links YAML spec to runtime descriptor.
//! * `from` (required, non-empty): Inbound channel id the service consumes messages from.
//! * `to` (required, non-empty): Outbound channel id the service publishes processed messages to.
//!
//! # Uniqueness & Validation
//! * Parser enforces presence and non-empty strings for required fields; uniqueness of `id` deferred.
//! * Validation does not enforce any naming pattern for `ref_name` beyond non-empty.
//!
//! # Runtime Mapping
//! `ServiceActivatorSpec` does not itself execute code; builders transform the spec into a runtime
//! processor abstraction matched against inventory descriptors submitted by the macro.
//!
//! # Construction Helpers
//! * `new(ref_name, from, to)` – minimal spec without id.
//! * `with_id(id, ref_name, from, to)` – explicit id.
//! * `id()` / `ref_name()` / `from()` / `to()` – accessors.
//! * `set_id(id)` – internal builder use for auto-id assignment.
//!
//! # Example
//! ```rust
//! use allora_runtime::spec::ServiceActivatorSpec;
//! let spec = ServiceActivatorSpec::new("hello_world", "inbound.orders", "vetted.orders");
//! assert_eq!(spec.ref_name(), "hello_world");
//! assert_eq!(spec.from(), "inbound.orders");
//! assert_eq!(spec.to(), "vetted.orders");
//! ```

#[derive(Debug, Clone)]
pub struct ServiceActivatorSpec {
    id: Option<String>,
    ref_name: String,
    from: String,
    to: String,
}

impl ServiceActivatorSpec {
    pub fn new<I: Into<String>, F: Into<String>, T: Into<String>>(
        ref_name: I,
        from: F,
        to: T,
    ) -> Self {
        Self {
            id: None,
            ref_name: ref_name.into(),
            from: from.into(),
            to: to.into(),
        }
    }
    pub fn with_id<ID: Into<String>, I: Into<String>, F: Into<String>, T: Into<String>>(
        id: ID,
        ref_name: I,
        from: F,
        to: T,
    ) -> Self {
        Self {
            id: Some(id.into()),
            ref_name: ref_name.into(),
            from: from.into(),
            to: to.into(),
        }
    }
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }
    pub fn ref_name(&self) -> &str {
        &self.ref_name
    }
    pub fn from(&self) -> &str {
        &self.from
    }
    pub fn to(&self) -> &str {
        &self.to
    }
    pub fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}
