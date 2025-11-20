//! HttpInboundAdaptersSpec: collection of HttpInboundAdapterSpec entries sharing a single version.
//!
//! Mirrors ServiceActivatorsSpec / FiltersSpec semantics for ordered aggregation, preserving
//! declaration order and deferring uniqueness / auto-id concerns to builder logic.
//!
//! # Responsibilities
//! * Store version (validated by YAML parser).
//! * Preserve adapter declaration order (Vec internally).
//! * Allow optional / duplicate ids (uniqueness enforced later).
//!
//! # Auto-ID Strategy (Builder Level - FUTURE)
//! * Missing `http-inbound-adapter.id` values assigned deterministic `http-inbound-adapter:auto.N`.
//! * Duplicate explicit ids -> build error.
//!
//! # Example
//! ```rust
//! use allora_runtime::spec::{HttpInboundAdapterSpec, HttpInboundAdaptersSpec};
//! let spec = HttpInboundAdaptersSpec::new(1)
//!     .add(HttpInboundAdapterSpec::with_id("http.recv", "0.0.0.0", 8080, "/recv", vec!["POST".into()], "inbound.recv"))
//!     .add(HttpInboundAdapterSpec::new("127.0.0.1", 8081, "/health", vec!["GET".into()], "inbound.health"));
//! assert_eq!(spec.adapters().len(), 2);
//! assert_eq!(spec.adapters()[0].id(), Some("http.recv"));
//! assert!(spec.adapters()[1].id().is_none());
//! ```

use crate::spec::HttpInboundAdapterSpec;

#[derive(Debug, Clone)]
pub struct HttpInboundAdaptersSpec {
    version: u32,
    adapters: Vec<HttpInboundAdapterSpec>,
}

impl HttpInboundAdaptersSpec {
    pub fn new(version: u32) -> Self {
        Self {
            version,
            adapters: Vec::new(),
        }
    }
    pub fn add(mut self, a: HttpInboundAdapterSpec) -> Self {
        self.adapters.push(a);
        self
    }
    pub fn push(&mut self, a: HttpInboundAdapterSpec) {
        self.adapters.push(a);
    }
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn adapters(&self) -> &[HttpInboundAdapterSpec] {
        &self.adapters
    }
    pub fn into_adapters(self) -> Vec<HttpInboundAdapterSpec> {
        self.adapters
    }
}
