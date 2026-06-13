//! HTTP Outbound Adapter: dispatches an `Exchange` to an external HTTP(S) endpoint.
//!
//! # Overview
//! `HttpOutboundAdapter` implements [`OutboundAdapter`], taking the (optionally transformed)
//! message from an `Exchange` and POSTing (default) it to a configured remote endpoint.
//! A builder is provided for ergonomic configuration.
//!
//! As of 0.0.9 the adapter is configured by a single `url` (string parsed into
//! [`url::Url`]) rather than the previous `host` + `port` + `base-path` triple,
//! and the underlying HTTP client is [`reqwest::Client`] — which transparently
//! supports both `http://` and `https://` schemes via `rustls`.
//!
//! # Selection of Message
//! By default the adapter prefers `exchange.out_msg` (if present) falling back to `exchange.in_msg`.
//! This can be inverted via `use_out_msg(false)` to always send the inbound message.
//!
//! # Builder Fields
//! * id (optional) – stable identifier; auto-generated UUID if omitted.
//! * url (required) – full target URL including scheme (e.g. `https://api.example.com/submit`).
//! * method (optional) – HTTP method (default POST).
//! * use_out_msg (optional) – whether to prioritize `out_msg` (default true).
//!
//! # Error Semantics
//! Network / protocol errors map to `Error::other`. Non-success HTTP status still returns
//! `OutboundDispatchResult` (acknowledged = status.is_success()).
//!
//! # Observability (0.0.10+)
//! Every failure path in [`HttpOutboundAdapter::dispatch`] emits a structured
//! `tracing::warn!` event so operators can diagnose silent dispatch problems
//! in logs (e.g. CloudWatch). The four failure classes are:
//!
//! 1. **Serialization** of a `Payload::Json` body fails.
//! 2. **Transport/connection** error from `reqwest` (DNS, TCP, TLS, …).
//! 3. **Body read** failure after a response was received.
//! 4. **Non-success HTTP status** (any non-2xx); the first 512 chars of the
//!    response body are included as `body_preview`.
//!
//! Successful dispatches are intentionally silent — the happy path runs at
//! high frequency and `info!` here would flood log sinks. The dispatch
//! contract (return type and channel-propagation semantics) is unchanged;
//! the new log calls are purely additive.
//!
//! # Example
//! ```no_run
//! use allora_core::{adapter::OutboundAdapter, Exchange, Message};
//! use allora_http::HttpOutboundAdapter;
//! # async fn demo() {
//! let adapter = HttpOutboundAdapter::builder()
//!     .id("http-public")
//!     .url("https://api.example.com/submit")
//!     .build().unwrap();
//! let mut exchange = Exchange::new(Message::from_text("ping"));
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
use reqwest::{Client, Method};
use std::fmt::Debug;

#[derive(Clone)]
pub struct HttpOutboundAdapter {
    id: String,
    /// Parsed, validated target URL. Cached at construction; the per-dispatch
    /// hot path just clones this `Url` and hands it to `reqwest`.
    url: url::Url,
    method: Method,
    use_out_msg: bool,
    client: Client,
}

impl Debug for HttpOutboundAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpOutboundAdapter")
            .field("id", &self.id)
            .field("url", &self.url.as_str())
            .field("method", &self.method.as_str())
            .field("use_out_msg", &self.use_out_msg)
            .finish()
    }
}

impl HttpOutboundAdapter {
    pub fn builder() -> HttpOutboundAdapterBuilder {
        HttpOutboundAdapterBuilder::default()
    }
    /// Configured target URL (parsed). Useful for tests and structured logging.
    pub fn url(&self) -> &url::Url {
        &self.url
    }
    /// Configured HTTP method (default POST).
    pub fn method(&self) -> &Method {
        &self.method
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
    url: Option<String>,
    method: Option<Method>,
    use_out_msg: Option<bool>,
    /// Test-only escape hatch — see the `dangerous_accept_invalid_certs`
    /// setter. Off by default. **Do not enable in production.**
    dangerous_accept_invalid_certs: bool,
}

impl HttpOutboundAdapterBuilder {
    pub fn id(mut self, v: impl Into<String>) -> Self {
        self.id = Some(v.into());
        self
    }
    /// Set the full target URL (must include scheme; either `http://` or `https://`).
    pub fn url(mut self, v: impl Into<String>) -> Self {
        self.url = Some(v.into());
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
    /// Disable TLS certificate validation on the underlying `reqwest::Client`.
    ///
    /// **Intended for tests only.** Enabling this against a real endpoint
    /// defeats TLS — an active network attacker can read or modify all
    /// traffic. Production configuration loaded from YAML never sets this
    /// flag; it has no corresponding spec field.
    pub fn dangerous_accept_invalid_certs(mut self, flag: bool) -> Self {
        self.dangerous_accept_invalid_certs = flag;
        self
    }
    pub fn build(self) -> Result<HttpOutboundAdapter> {
        let raw_url = self.url.ok_or_else(|| Error::other("url required"))?;
        let parsed = url::Url::parse(&raw_url)
            .map_err(|e| Error::other(format!("invalid url '{raw_url}': {e}")))?;
        match parsed.scheme() {
            "http" | "https" => {}
            other => {
                return Err(Error::other(format!(
                    "unsupported url scheme '{other}' (expected http or https)"
                )));
            }
        }
        let id = self.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let method = self.method.unwrap_or(Method::POST);
        let client = Client::builder()
            .danger_accept_invalid_certs(self.dangerous_accept_invalid_certs)
            .build()
            .map_err(|e| Error::other(format!("failed to build http client: {e}")))?;
        Ok(HttpOutboundAdapter {
            id,
            url: parsed,
            method,
            use_out_msg: self.use_out_msg.unwrap_or(true),
            client,
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
            Payload::Json(v) => match serde_json::to_vec(v) {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!(
                        url = %self.url.as_str(),
                        error = %e,
                        "HttpOutboundAdapter: failed to serialize JSON payload"
                    );
                    return Err(Error::serialization(e.to_string()));
                }
            },
            Payload::Empty => Vec::new(),
        };
        let resp = match self
            .client
            .request(self.method.clone(), self.url.clone())
            .header("Content-Type", "application/octet-stream")
            .body(bytes)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(
                    url = %self.url.as_str(),
                    method = %self.method,
                    error = %e,
                    "HttpOutboundAdapter: transport error during dispatch"
                );
                return Err(Error::other(e.to_string()));
            }
        };
        let status = resp.status();
        let body_string = match resp.text().await {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(
                    url = %self.url.as_str(),
                    method = %self.method,
                    status = status.as_u16(),
                    error = %e,
                    "HttpOutboundAdapter: failed to read response body"
                );
                return Err(Error::other(e.to_string()));
            }
        };
        if !status.is_success() {
            // Char-aware truncation: byte slicing here would panic on a
            // multi-byte UTF-8 boundary if the server returns non-ASCII.
            let body_preview: String = body_string.chars().take(512).collect();
            tracing::warn!(
                url = %self.url.as_str(),
                method = %self.method,
                status = status.as_u16(),
                body_preview = %body_preview,
                "HttpOutboundAdapter: non-success HTTP status"
            );
        }
        Ok(OutboundDispatchResult {
            acknowledged: status.is_success(),
            message: Some(format!("HTTP {}", status)),
            status_code: Some(status.as_u16()),
            body: Some(body_string),
        })
    }
}
