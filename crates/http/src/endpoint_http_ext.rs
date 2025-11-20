//! HTTP Endpoint Extension Traits
//!
//! This module provides ergonomic extensions for integrating core endpoint builders and
//! concrete in-memory endpoints (`InMemoryEndpoint`) with an `HttpInboundAdapter`.
//! It separates concerns into two layers:
//!
//! 1. Builder-Level Extension (`HttpInOutEndpointBuilderExt`) – augments
//!    `InOutQueueEndpointBuilder` with a `source_http(...)` method so you can declaratively
//!    tag an endpoint as originating from a specific HTTP method + path on a given adapter.
//!    This records the intent (adapter id, method, path) in the endpoint's source metadata.
//!
//! 2. Endpoint-Level Extension (`HttpEndpointExt`) – supplies `attach_http(...)` and
//!    `attach_http_any(...)` to register a built `InMemoryEndpoint` with an adapter after the
//!    builder completes. Registration uses a `Weak` reference (via `Arc::downgrade`) to avoid
//!    reference cycles between adapter and endpoint while still permitting adapter-driven
//!    dispatch.
//!
//! # When to Use Each
//! * Use `source_http` during route / topology construction for clarity and for later
//!   introspection of endpoint provenance (e.g., generating diagrams, logs, or health
//!   summaries).
//! * Use `attach_http` right after building the endpoint to activate HTTP routing. This keeps
//!   side-effects (adapter mutation) out of the pure builder phase.
//!
//! # Example
//! ```rust
//! use std::sync::Arc;
//! use allora_core::endpoint::{InOutQueueEndpointBuilder, EndpointSource};
//! use allora_http::{HttpInboundAdapter, HttpEndpointExt, HttpInOutEndpointBuilderExt};
//! # fn build(adapter: Arc<HttpInboundAdapter>) {
//! let builder = InOutQueueEndpointBuilder::new("in.queue", "out.queue")
//!     .source_http(&adapter, "POST", "/submit");
//! let endpoint = Arc::new(builder.build().expect("endpoint"));
//! // Register with the adapter so inbound requests dispatch to this endpoint.
//! endpoint.attach_http(&adapter, "POST", "/submit");
//! # }
//! ```
//!
//! # Design Notes
//! * Separation of source annotation (pure metadata) and registration (side-effect) eases
//!   testing and allows inspection of intended HTTP mappings before performing adapter
//!   mutation.
//! * Weak endpoint references prevent memory leaks if adapters retain many endpoints.
//! * The `attach_http_any` convenience permits handling multiple HTTP verbs for the same
//!   path without multiple explicit registrations (adapter can interpret "ANY").
//! * These extensions deliberately avoid adding lifetime or generic complexity to keep
//!   ergonomics straightforward for application code.
//!
//! # Future Extensions
//! * Automatic bulk registration from builder metadata.
//! * Middleware hooks (auth / rate limit) on `attach_http`.
//! * Metrics counters for registered endpoints.
//! * Support for path parameters & pattern matching at registration time.

use crate::http_inbound_adapter::HttpInboundAdapter;
use allora_core::endpoint::{EndpointSource, InMemoryEndpoint, InOutQueueEndpointBuilder};
use std::sync::Arc;

pub trait HttpInOutEndpointBuilderExt {
    fn source_http(
        self,
        adapter: &Arc<HttpInboundAdapter>,
        method: impl Into<String>,
        path: impl Into<String>,
    ) -> Self;
}

impl HttpInOutEndpointBuilderExt for InOutQueueEndpointBuilder {
    fn source_http(
        mut self,
        adapter: &Arc<HttpInboundAdapter>,
        method: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        let m = method.into();
        let p = path.into();

        self = self.source(EndpointSource::Http {
            adapter_id: adapter.id().to_string(),
            method: m.clone(),
            path: p.clone(),
        });

        // optional: store wiring info in the builder via a separate field
        // or provide a separate `wire_http_endpoint` function that you call after build.

        self
    }
}

// Extension on the built endpoint:
pub trait HttpEndpointExt {
    fn attach_http(
        self: &Arc<Self>,
        adapter: &HttpInboundAdapter,
        method: &str,
        path: &str,
    ) -> &Self;
    fn attach_http_any(self: &Arc<Self>, adapter: &HttpInboundAdapter, path: &str) -> &Self;
}

impl HttpEndpointExt for InMemoryEndpoint {
    fn attach_http(
        self: &Arc<Self>,
        adapter: &HttpInboundAdapter,
        method: &str,
        path: &str,
    ) -> &Self {
        adapter.register_endpoint(method, path, Arc::downgrade(self));
        self
    }

    fn attach_http_any(self: &Arc<Self>, adapter: &HttpInboundAdapter, path: &str) -> &Self {
        adapter.register_endpoint("ANY", path, Arc::downgrade(self));
        self
    }
}
