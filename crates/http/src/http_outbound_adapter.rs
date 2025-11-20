//! HTTP Outbound Adapter: dispatches an `Exchange` to an external HTTP endpoint.
//!
//! # Overview
//! `HttpOutboundAdapter` implements [`OutboundAdapter`], taking the (optionally transformed)
//! message from an `Exchange` and POSTing (default) it to a configured remote endpoint.
//! A builder is provided for ergonomic configuration.
//!
//! # Selection of Message
//! By default the adapter prefers `exchange.out_msg` (if present) falling back to `exchange.in_msg`.
//! This can be inverted via `use_out_msg(false)` to always send the inbound message.
//!
//! # Builder Fields
//! * id (optional) – stable identifier; auto-generated UUID if omitted.
//! * host (required) – remote host or IP.
//! * port (required) – remote TCP port.
//! * base_path (optional) – leading path (default "/").
//! * path (optional) – trailing path segment appended to base_path.
//! * method (optional) – HTTP method (default POST).
//! * use_out_msg (optional) – whether to prioritize `out_msg` (default true).
//!
//! # Error Semantics
//! Network / protocol errors map to `Error::other`. Non-success HTTP status still returns
//! `OutboundDispatchResult` (acknowledged = status.is_success()).
//!
//! # Example
//! ```no_run
//! use allora_core::{adapter::OutboundAdapter, Exchange, Message};
//! use allora_http::HttpOutboundAdapter;
//! # async fn demo() {
//! let adapter = HttpOutboundAdapter::builder()
//!     .id("http-public")
//!     .host("0.0.0.0")
//!     .port(8080)
//!     .base_path("/")
//!     .build().unwrap();
//! let mut exchange = Exchange::new(Message::from_text("ping"));
//! #[cfg(feature = "async")]
//! let _res = adapter.dispatch(&exchange).await.unwrap();
//! # }
//! ```
//!
//! # Future Extensions
//! * Header propagation (copy selected message headers to HTTP request).
//! * Custom serialization strategies (content-type aware).
//! * Retry / backoff policies & circuit breaking.
//! * Metrics emission (latency, status codes).

use allora_core::adapter::{BaseAdapter, OutboundAdapter, OutboundDispatchResult};
use allora_core::{
    error::{Error, Result},
    Exchange, Payload,
};
use async_trait::async_trait;
use hyper::client::HttpConnector;
use hyper::{Body, Client, Method, Request};
use std::fmt::Debug;

#[derive(Clone)]
pub struct HttpOutboundAdapter {
    id: String,
    host: String,
    port: u16,
    base_path: String,
    path: Option<String>,
    method: Method,
    use_out_msg: bool,
    client: Client<HttpConnector>,
}

impl Debug for HttpOutboundAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpOutboundAdapter")
            .field("id", &self.id)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("base_path", &self.base_path)
            .field("path", &self.path)
            .field("method", &self.method.as_str())
            .field("use_out_msg", &self.use_out_msg)
            .finish()
    }
}

impl HttpOutboundAdapter {
    pub fn builder() -> HttpOutboundAdapterBuilder {
        HttpOutboundAdapterBuilder::default()
    }
    fn full_url(&self) -> String {
        let mut base = self.base_path.clone();
        if !base.starts_with('/') {
            base.insert(0, '/');
        }
        if base.len() > 1 && base.ends_with('/') {
            base.pop();
        }
        let mut full = base;
        if let Some(p) = &self.path {
            if !p.is_empty() {
                if !p.starts_with('/') {
                    full.push('/');
                }
                full.push_str(p);
            }
        }
        format!("http://{}:{}{}", self.host, self.port, full)
    }
}

impl BaseAdapter for HttpOutboundAdapter {
    fn id(&self) -> &str {
        &self.id
    }
}

#[derive(Default)]
pub struct HttpOutboundAdapterBuilder {
    id: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    base_path: Option<String>,
    path: Option<String>,
    method: Option<Method>,
    use_out_msg: Option<bool>,
}

impl HttpOutboundAdapterBuilder {
    pub fn id(mut self, v: impl Into<String>) -> Self {
        self.id = Some(v.into());
        self
    }
    pub fn host(mut self, v: impl Into<String>) -> Self {
        self.host = Some(v.into());
        self
    }
    pub fn port(mut self, v: u16) -> Self {
        self.port = Some(v);
        self
    }
    pub fn base_path(mut self, v: impl Into<String>) -> Self {
        self.base_path = Some(v.into());
        self
    }
    pub fn path(mut self, v: impl Into<String>) -> Self {
        self.path = Some(v.into());
        self
    }
    pub fn method(mut self, m: Method) -> Self {
        self.method = Some(m);
        self
    }
    pub fn use_out_msg(mut self, flag: bool) -> Self {
        self.use_out_msg = Some(flag);
        self
    }
    pub fn build(self) -> Result<HttpOutboundAdapter> {
        let host = self.host.ok_or_else(|| Error::other("host required"))?;
        let port = self.port.ok_or_else(|| Error::other("port required"))?;
        let id = self.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let base_path = self.base_path.unwrap_or_else(|| "/".to_string());
        let method = self.method.unwrap_or(Method::POST);
        Ok(HttpOutboundAdapter {
            id,
            host,
            port,
            base_path,
            path: self.path,
            method,
            use_out_msg: self.use_out_msg.unwrap_or(true),
            client: Client::new(),
        })
    }
}

#[async_trait]
impl OutboundAdapter for HttpOutboundAdapter {
    async fn dispatch(&self, exchange: &Exchange) -> Result<OutboundDispatchResult> {
        let msg_ref = if self.use_out_msg {
            exchange.out_msg.as_ref().unwrap_or(&exchange.in_msg)
        } else {
            &exchange.in_msg
        };
        let bytes: Vec<u8> = match &msg_ref.payload {
            Payload::Text(s) => s.clone().into_bytes(),
            Payload::Bytes(b) => b.clone(),
            Payload::Json(v) => {
                serde_json::to_vec(v).map_err(|e| Error::serialization(e.to_string()))?
            }
            Payload::Empty => Vec::new(),
        };
        let req = Request::builder()
            .method(self.method.clone())
            .uri(self.full_url())
            .header("Content-Type", "application/octet-stream")
            .body(Body::from(bytes))
            .map_err(|e| Error::other(e.to_string()))?;
        let resp = self
            .client
            .request(req)
            .await
            .map_err(|e| Error::other(e.to_string()))?;
        let status = resp.status();
        Ok(OutboundDispatchResult {
            acknowledged: status.is_success(),
            message: Some(format!("HTTP {}", status)),
        })
    }
}
