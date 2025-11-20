//! HttpInboundAdapterSpec: specification for a single HTTP inbound adapter binding an HTTP endpoint
//! to messaging channels.
//!
//! # Purpose
//! Represents an HTTP server endpoint definition that publishes inbound requests as messages onto a
//! `request_channel` and optionally consumes replies from a `reply_channel` to form synchronous HTTP responses.
//!
//! # Field Semantics
//! * `id` (optional): Explicit adapter identifier for reference / diagnostics; may be auto-generated
//!   by a future collection builder (`http-inbound-adapter:auto.N`).
//! * `host` (required, non-empty): Interface or hostname to bind (e.g. `0.0.0.0`, `127.0.0.1`).
//! * `port` (required, 1-65535): TCP port to listen on.
//! * `path` (required, non-empty, leading `/`): Exact path segment for the endpoint.
//! * `methods` (required, non-empty list): Allowed HTTP methods (subset of standard verbs).
//! * `request_channel` (required, non-empty): Channel id receiving inbound requests as messages.
//! * `reply_channel` (optional, non-empty if present): Channel id for reply messages correlated to HTTP responses.
//!
//! # Construction Helpers
//! * `new(host, port, path, methods, request_channel)` – minimal spec (no id / reply_channel).
//! * `with_id(id, host, port, path, methods, request_channel)` – explicit id.
//! * `with_reply(host, port, path, methods, request_channel, reply_channel)` – reply channel only.
//! * `with_id_reply(id, host, port, path, methods, request_channel, reply_channel)` – id + reply.
//!
//! # Example
//! ```rust
//! use allora_runtime::spec::HttpInboundAdapterSpec;
//! let spec = HttpInboundAdapterSpec::new("127.0.0.1", 8080, "/receive", vec!["POST".into()], "inbound.receive");
//! assert_eq!(spec.host(), "127.0.0.1");
//! assert_eq!(spec.port(), 8080);
//! assert_eq!(spec.path(), "/receive");
//! assert_eq!(spec.methods(), &["POST"]);
//! assert_eq!(spec.request_channel(), "inbound.receive");
//! assert!(spec.reply_channel().is_none());
//! ```

#[derive(Debug, Clone)]
pub struct HttpInboundAdapterSpec {
    id: Option<String>,
    host: String,
    port: u16,
    path: String,
    methods: Vec<String>,
    request_channel: String,
    reply_channel: Option<String>,
}

impl HttpInboundAdapterSpec {
    pub fn new<H: Into<String>, P: Into<String>, RC: Into<String>>(
        host: H,
        port: u16,
        path: P,
        methods: Vec<String>,
        request_channel: RC,
    ) -> Self {
        Self {
            id: None,
            host: host.into(),
            port,
            path: path.into(),
            methods,
            request_channel: request_channel.into(),
            reply_channel: None,
        }
    }
    pub fn with_id<ID: Into<String>, H: Into<String>, P: Into<String>, RC: Into<String>>(
        id: ID,
        host: H,
        port: u16,
        path: P,
        methods: Vec<String>,
        request_channel: RC,
    ) -> Self {
        Self {
            id: Some(id.into()),
            host: host.into(),
            port,
            path: path.into(),
            methods,
            request_channel: request_channel.into(),
            reply_channel: None,
        }
    }
    pub fn with_reply<H: Into<String>, P: Into<String>, RC: Into<String>, R: Into<String>>(
        host: H,
        port: u16,
        path: P,
        methods: Vec<String>,
        request_channel: RC,
        reply_channel: R,
    ) -> Self {
        Self {
            id: None,
            host: host.into(),
            port,
            path: path.into(),
            methods,
            request_channel: request_channel.into(),
            reply_channel: Some(reply_channel.into()),
        }
    }
    pub fn with_id_reply<
        ID: Into<String>,
        H: Into<String>,
        P: Into<String>,
        RC: Into<String>,
        R: Into<String>,
    >(
        id: ID,
        host: H,
        port: u16,
        path: P,
        methods: Vec<String>,
        request_channel: RC,
        reply_channel: R,
    ) -> Self {
        Self {
            id: Some(id.into()),
            host: host.into(),
            port,
            path: path.into(),
            methods,
            request_channel: request_channel.into(),
            reply_channel: Some(reply_channel.into()),
        }
    }
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }
    pub fn host(&self) -> &str {
        &self.host
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn methods(&self) -> &[String] {
        &self.methods
    }
    pub fn request_channel(&self) -> &str {
        &self.request_channel
    }
    pub fn reply_channel(&self) -> Option<&str> {
        self.reply_channel.as_deref()
    }
    pub fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}
