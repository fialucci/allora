// crates/http/src/endpoint_http_ext.rs

use std::sync::{Arc, Weak};
use allora_core::{
    endpoint::{EndpointSource, InOutQueueEndpointBuilder, InMemoryEndpoint},
};
use crate::http_inbound_adapter::HttpInboundAdapter;

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
    fn attach_http(self: &Arc<Self>, adapter: &HttpInboundAdapter, method: &str, path: &str)
                   -> &Self;
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
