//! HTTP Inbound Adapter: bridges incoming HTTP requests into Allora `Exchange`s.
//! (see module docs above for overview; trimmed here for brevity)
//!
//! # Overview
//! Translates inbound HTTP requests into `Exchange`s with a `Message` payload, then enqueues them
//! on a configured channel. Optional endpoints (method/path registrations) can be attached for
//! direct processing or routing augmentation.
//!
//! # Configuration (Builder)
//! * `id` (optional) – stable identifier; auto-generated from socket if omitted.
//! * `host` / `port` – listening address.
//! * `base_path` – URL path prefix for registrations (e.g. `/api`).
//! * `channel` – target ChannelRef (mandatory).
//! * `mep` – message exchange pattern (request/reply vs fire-and-forget).
//! * `register` / `register_any` – attach endpoints for specific method/path pairs.
//!
//! # Message Exchange Patterns
//! * `InOut` – awaits downstream processing and echoes (or uses first endpoint mutation) in the HTTP response.
//! * `InOnly202` – returns HTTP 202 immediately; processing continues asynchronously.
//!
//! Notes:
//! * Async-only (Tokio) – no sync variant.
//! * HTTP adapter always available (no feature flags).
//! * Correlation header ensured automatically.
//!
//! # Example (Builder)
//! ```rust
//! use allora_core::adapter::Adapter;
//! use allora_core::channel::QueueChannel;
//! use allora_http::adapter_dsl::InboundHttpExt; // bring .http() into scope
//! let channel = QueueChannel::with_id("http-pipe");
//! let adapter = Adapter::inbound()
//!     .http()
//!     .host("127.0.0.1")
//!     .port(0)
//!     .channel(std::sync::Arc::new(channel))
//!     .in_only_202()
//!     .build();
//! use allora_http::http_inbound_adapter::Mep;
//! assert_eq!(adapter.mep(), Mep::InOnly202);
//! ```

use allora_core::adapter::{ensure_correlation, BaseAdapter, InboundAdapter};
use allora_core::channel::{ChannelRef, QueueChannel};
// for downcast when polling reply_channel
use allora_core::endpoint::{EndpointSource, InMemoryEndpoint};
use allora_core::error::Result;
use allora_core::{Exchange, Message, Payload};
use async_trait::async_trait;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, Version};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex, Weak};
use std::task::{Context, Poll};
use tracing::{debug, error, info, trace};

// Reply to polling configuration (seconds to wait for a reply-channel before falling back, and interval between polls)
const REPLY_TIMEOUT_SECS: u64 = 3;
const REPLY_POLL_INTERVAL_MILLIS: u64 = 50;

/// Message Exchange Pattern for HTTP inbound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mep {
    /// Request/Reply: wait for the route to complete and return its response body (legacy behavior).
    InOut,
    /// Fire-and-forget: return 202 immediately and dispatch the Exchange in the background.
    InOnly202,
}
impl Default for Mep {
    fn default() -> Self {
        Mep::InOut
    }
}

#[derive(Clone, Debug)]
pub struct HttpInboundAdapter {
    id: String,
    addr: SocketAddr,
    base_path: String,
    channel: ChannelRef,
    mep: Mep,
    reply_channel: Option<ChannelRef>,
    routes: Arc<Mutex<HashMap<(String, String), Vec<Weak<InMemoryEndpoint>>>>>,
}

pub struct HttpServerHandle {
    join: tokio::task::JoinHandle<Result<()>>,
}

impl HttpServerHandle {
    pub async fn wait(self) -> Result<()> {
        self.join
            .await
            .unwrap_or_else(|e| Err(allora_core::error::Error::other(e.to_string())))
    }
    pub fn abort(self) {
        self.join.abort();
    }
}

impl std::future::Future for HttpServerHandle {
    type Output = Result<()>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = unsafe { self.map_unchecked_mut(|s| &mut s.join) };
        match inner.poll(cx) {
            Poll::Ready(r) => Poll::Ready(
                r.unwrap_or_else(|e| Err(allora_core::error::Error::other(e.to_string()))),
            ),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl HttpInboundAdapter {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
    pub fn base_path(&self) -> &str {
        &self.base_path
    }
    pub fn mep(&self) -> Mep {
        self.mep
    }
    /// Direct constructor bypassing the staged DSL builder.
    /// Use this when you want to instantiate the adapter programmatically without chaining
    /// `Adapter::inbound().http()...` calls. `id` is optional; if `None`, an id of the form
    /// `http-inbound:<host>:<port>` is synthesized (matching the builder's auto pattern).
    pub fn new(
        host: impl Into<String>,
        port: u16,
        base_path: impl Into<String>,
        channel: ChannelRef,
        reply_channel: Option<ChannelRef>,
        mep: Mep,
        id: Option<String>,
    ) -> Self {
        let host_str = host.into();
        let addr: SocketAddr = format!("{}:{}", host_str, port)
            .parse()
            .expect("invalid socket addr");
        let base = {
            let b = base_path.into();
            if b.is_empty() {
                "/".to_string()
            } else {
                b
            }
        };
        let id_final = id.unwrap_or_else(|| format!("http-inbound:{}", addr));
        trace!(adapter.id=%id_final, host=%host_str, port=%port, base_path=%base, mep=?mep, "constructing HttpInboundAdapter (direct)");
        HttpInboundAdapter {
            id: id_final,
            addr,
            base_path: base,
            channel,
            mep,
            reply_channel,
            routes: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    /// Convenience: construct InOut adapter (request/reply pattern).
    pub fn new_in_out(
        host: impl Into<String>,
        port: u16,
        base_path: impl Into<String>,
        channel: ChannelRef,
        reply_channel: Option<ChannelRef>,
        id: Option<String>,
    ) -> Self {
        Self::new(
            host,
            port,
            base_path,
            channel,
            reply_channel,
            Mep::InOut,
            id,
        )
    }
    /// Convenience: construct InOnly202 adapter (fire-and-forget pattern).
    pub fn new_in_only_202(
        host: impl Into<String>,
        port: u16,
        base_path: impl Into<String>,
        channel: ChannelRef,
        id: Option<String>,
    ) -> Self {
        Self::new(host, port, base_path, channel, None, Mep::InOnly202, id)
    }
    // Builder entry via staged pattern: Adapter::inbound().http() returns builder
}

pub struct HttpInboundBuilder {
    id: Option<String>,
    host: String,
    port: u16,
    base_path: String,
    channel: Option<ChannelRef>,
    mep: Mep,
    reply_channel: Option<ChannelRef>,
    registrations: Vec<(String, String, Arc<InMemoryEndpoint>)>,
}
impl HttpInboundBuilder {
    pub(crate) fn new() -> Self {
        Self {
            id: None,
            host: String::new(),
            port: 0,
            base_path: String::new(),
            channel: None,
            mep: Mep::InOut,
            reply_channel: None,
            registrations: Vec::new(),
        }
    }
    /// Register an endpoint for a specific HTTP method and path. Path relative to base_path.
    pub fn register(mut self, method: &str, path: &str, endpoint: Arc<InMemoryEndpoint>) -> Self {
        let norm = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };
        self.registrations
            .push((method.to_ascii_uppercase(), norm, endpoint));
        self
    }
    /// Convenience: register endpoint for all methods (stored as ANY wildcard).
    pub fn register_any(self, path: &str, endpoint: Arc<InMemoryEndpoint>) -> Self {
        self.register("ANY", path, endpoint)
    }
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
    pub fn base_path(mut self, path: impl Into<String>) -> Self {
        self.base_path = path.into();
        self
    }
    pub fn channel(mut self, ch: ChannelRef) -> Self {
        self.channel = Some(ch);
        self
    }
    pub fn reply_channel(mut self, ch: ChannelRef) -> Self {
        self.reply_channel = Some(ch);
        self
    }

    /// Set the message exchange pattern. Default is `InOut`.
    pub fn mep(mut self, mep: Mep) -> Self {
        self.mep = mep;
        self
    }

    /// Convenience: respond 202 immediately and dispatch in background.
    pub fn in_only_202(self) -> Self {
        self.mep(Mep::InOnly202)
    }

    pub fn build(self) -> HttpInboundAdapter {
        let addr: SocketAddr = format!("{}:{}", self.host, self.port)
            .parse()
            .expect("invalid socket addr");
        let id = self.id.unwrap_or_else(|| format!("http-inbound:{}", addr));
        let base_path = if self.base_path.is_empty() { "/".to_string() } else { self.base_path };
        let channel = self.channel.expect("channel must be set on HttpInboundBuilder before build()");
        let effective_mep = if self.reply_channel.is_some() { Mep::InOut } else { self.mep };
        let adapter = HttpInboundAdapter {
            id: id.clone(),
            addr,
            base_path: base_path.clone(),
            channel,
            mep: effective_mep,
            reply_channel: self.reply_channel.clone(),
            routes: Arc::new(Mutex::new(HashMap::new())),
        };
        info!(adapter.id=%adapter.id, addr=%adapter.addr, base_path=%adapter.base_path, mep=?adapter.mep, reply_channel=adapter.reply_channel.is_some(), "HttpInboundAdapter built via builder");
        for (method, path, ep) in self.registrations.into_iter() {
            adapter.register_endpoint(&method, &path, Arc::downgrade(&ep));
        }
        adapter
    }
}

impl BaseAdapter for HttpInboundAdapter {
    fn id(&self) -> &str {
        &self.id
    }
}

#[async_trait]
impl InboundAdapter for HttpInboundAdapter {
    async fn run(&self) -> Result<()> {
        self.serve().await
    }
}

fn normalize_path<'a>(base: &'a str, full: &'a str) -> &'a str {
    if base == "/" {
        return full;
    }
    match full.strip_prefix(base) {
        Some(p) if p.is_empty() => "/",
        Some(p) => {
            if p.starts_with('/') {
                p
            } else {
                "/"
            }
        }
        None => full,
    }
}

fn http_version_str(v: Version) -> &'static str {
    match v {
        Version::HTTP_09 => "0.9",
        Version::HTTP_10 => "1.0",
        Version::HTTP_11 => "1.1",
        Version::HTTP_2 => "2.0",
        Version::HTTP_3 => "3.0",
        _ => "unknown",
    }
}

async fn adapt_request(
    adapter_id: String,
    channel: ChannelRef,
    reply_channel: Option<ChannelRef>,
    req: Request<Body>,
    base_path: String,
    mep: Mep,
    routes: Arc<Mutex<HashMap<(String, String), Vec<Weak<InMemoryEndpoint>>>>>,
) -> Result<Response<Body>> {
    let method = req.method().clone();
    let path_full = req.uri().path().to_string();
    let path_norm = normalize_path(&base_path, &path_full).to_string();
    let query = req.uri().query().unwrap_or("").to_string();
    let version = http_version_str(req.version()).to_string();
    // Extract headers before consuming body.
    let mut content_type = None::<String>;
    let headers_clone: Vec<(String, String)> = req
        .headers()
        .iter()
        .filter_map(|(name, val)| {
            val.to_str()
                .ok()
                .map(|s| (name.as_str().to_ascii_lowercase(), s.to_string()))
        })
        .collect();
    if let Some(ct) = headers_clone
        .iter()
        .find(|(k, _)| k == "content-type")
        .map(|(_, v)| v.clone())
    {
        content_type = Some(ct);
    }
    // Consume body afterwards.
    let body_bytes = hyper::body::to_bytes(req.into_body())
        .await
        .map_err(|e| allora_core::error::Error::other(e.to_string()))?;

    // Build Message with raw bytes payload (no lossy conversion).
    let mut msg = if let Ok(txt) = std::str::from_utf8(&body_bytes) {
        Message::from_text(txt)
    } else {
        Message::new(Payload::Bytes(body_bytes.to_vec()))
    };
    msg.set_header("http.method", method.as_str());
    msg.set_header("http.path", &path_norm);
    if !query.is_empty() {
        msg.set_header("http.query", &query);
    }
    msg.set_header("http.version", &version);
    for (k, v) in headers_clone.iter() {
        let key = format!("http.header.{}", k);
        msg.set_header(&key, v);
    }
    if let Some(ct) = content_type {
        msg.set_header("http.content_type", &ct);
    }
    if let Ok(txt) = std::str::from_utf8(&body_bytes) {
        msg.set_header("http.body_text", txt);
    }

    // Prepare Exchange with correlation id.
    let mut exchange = Exchange::new(msg);
    ensure_correlation(&mut exchange);
    debug!(adapter.id=%adapter_id, corr_id=?exchange.in_msg.header("corr_id"), "correlation ensured for inbound exchange");
    match mep {
        Mep::InOut => {
            let key_exact = (method.as_str().to_ascii_uppercase(), path_norm.clone());
            let key_any = ("ANY".to_string(), path_norm.clone());
            let mut endpoints: Vec<Weak<InMemoryEndpoint>> = Vec::new();
            if let Ok(map) = routes.lock() {
                if let Some(list) = map.get(&key_exact) {
                    endpoints.extend(list.iter().cloned());
                }
                if let Some(list) = map.get(&key_any) {
                    endpoints.extend(list.iter().cloned());
                }
            }
            if !endpoints.is_empty() {
                debug!(adapter.id=%adapter_id, endpoints.count=endpoints.len(), path=%path_norm, "matched in-memory endpoints");
                let mut response_body: Option<String> = None;
                for weak_ep in endpoints.iter() {
                    if let Some(ep) = weak_ep.upgrade() {
                        if let Some(ch_ref) = ep.channel() {
                            let mut ex_clone = exchange.clone();
                            EndpointSource::Http {
                                adapter_id: adapter_id.clone(),
                                method: method.as_str().to_string(),
                                path: path_norm.clone(),
                            }
                            .apply_headers(&mut ex_clone);
                            ch_ref.send(ex_clone.clone()).await?;
                            trace!(adapter.id=%adapter_id, endpoint.channel=%ch_ref.id(), method=%method, path=%path_norm, "dispatched exchange to endpoint channel");
                            if response_body.is_none() {
                                response_body = ex_clone.in_msg.body_text().map(|s| s.to_string());
                            }
                        }
                    } else {
                        trace!(adapter.id=%adapter_id, method=%method, path=%path_norm, "skipping stale endpoint");
                    }
                }
                let body_final = response_body.unwrap_or_else(|| String::new());
                return Ok(Response::new(Body::from(body_final)));
            }
            // Fallback: enqueue on primary channel only, then optionally wait for reply_channel.
            trace!(adapter.id=%adapter_id, channel.id=?channel.id(), mep=?mep, "no endpoints matched; sending to primary channel");
            channel.send(exchange.clone()).await?;
            if let Some(rc) = reply_channel {
                // Downcast to a pollable queue channel.
                if let Some(qc) = rc.as_any().downcast_ref::<QueueChannel>() {
                    use allora_core::PollableChannel;
                    let start = std::time::Instant::now();
                    while start.elapsed() < std::time::Duration::from_secs(REPLY_TIMEOUT_SECS) {
                        if let Some(ex_reply) = qc.try_receive().await {
                            let body = ex_reply
                                .out_msg
                                .as_ref()
                                .and_then(|m| m.body_text())
                                .or_else(|| ex_reply.in_msg.body_text())
                                .unwrap_or("");
                            return Ok(Response::new(Body::from(body.to_string())));
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(
                            REPLY_POLL_INTERVAL_MILLIS,
                        ))
                        .await;
                    }
                    trace!(adapter.id=%adapter_id, "reply-channel timeout; returning original inbound body");
                } else {
                    trace!(adapter.id=%adapter_id, "reply-channel present but not queue/pollable; skipping reply wait");
                }
            }
            let response_body = exchange
                .in_msg
                .body_text()
                .map(|s| s.to_string())
                .unwrap_or_else(|| String::from_utf8_lossy(&body_bytes).to_string());
            Ok(Response::new(Body::from(response_body)))
        }
        Mep::InOnly202 => {
            trace!(adapter.id=%adapter_id, channel.id=?channel.id(), "IN_ONLY_202 mode: spawning background send");
            // Fire-and-forget: dispatch in the background and ack now.
            let ch = channel.clone();
            tokio::spawn(async move {
                let _ = ch.send(exchange).await;
            });
            Ok(Response::builder()
                .status(202)
                .body(Body::from("ok"))
                .unwrap())
        }
    }
}

impl HttpInboundAdapter {
    pub fn register_endpoint(&self, method: &str, path: &str, ep: Weak<InMemoryEndpoint>) {
        let key = (method.to_ascii_uppercase(), path.to_string());
        let mut map = self.routes.lock().unwrap();
        map.entry(key).or_insert_with(Vec::new).push(ep);
    }
    pub fn register_endpoint_any(&self, path: &str, ep: Weak<InMemoryEndpoint>) {
        for m in [
            "GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD", "ANY",
        ] {
            self.register_endpoint(m, path, ep.clone());
        }
    }
    pub async fn serve(&self) -> Result<()> {
        let channel = self.channel.clone();
        let reply_channel = self.reply_channel.clone();
        let base = self.base_path.clone();
        let mep = self.mep;
        let adapter_id = self.id.clone();
        let routes_arc = self.routes.clone();
        let make = make_service_fn(move |_conn| {
            let channel_clone = channel.clone();
            let base_clone = base.clone();
            let adapter_id_clone = adapter_id.clone();
            let routes_ref = routes_arc.clone();
            let reply_channel_outer = reply_channel.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                    let c = channel_clone.clone();
                    let b = base_clone.clone();
                    let r = routes_ref.clone();
                    let a = adapter_id_clone.clone();
                    let rc = reply_channel_outer.clone();
                    async move {
                        match adapt_request(a, c, rc, req, b, mep, r).await {
                            Ok(resp) => Ok::<_, hyper::Error>(resp),
                            Err(e) => {
                                error!(error=%e, "request handling failed");
                                Ok(Response::builder()
                                    .status(500)
                                    .body(Body::from("internal error"))
                                    .unwrap())
                            }
                        }
                    }
                }))
            }
        });
        info!(address=%self.addr, mep=?self.mep, "starting HTTP inbound adapter (continuous)");
        Server::bind(&self.addr)
            .serve(make)
            .await
            .map_err(|e| allora_core::error::Error::other(e.to_string()))?;
        Ok(())
    }
    pub async fn run_once(self) -> Result<()> {
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let channel = self.channel.clone();
        let reply_channel = self.reply_channel.clone();
        let base = self.base_path.clone();
        let mep = self.mep;
        let adapter_id = self.id.clone();
        let routes_arc = self.routes.clone();
        let shutdown_flag = Arc::new(Mutex::new(Some(tx)));
        let make = make_service_fn(move |_conn| {
            let channel_clone = channel.clone();
            let base_clone = base.clone();
            let adapter_id_clone = adapter_id.clone();
            let routes_ref = routes_arc.clone();
            let reply_channel_outer = reply_channel.clone();
            let shutdown_inner = shutdown_flag.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                    let c = channel_clone.clone();
                    let b = base_clone.clone();
                    let r = routes_ref.clone();
                    let a = adapter_id_clone.clone();
                    let rc = reply_channel_outer.clone();
                    let shutdown_local = shutdown_inner.clone();
                    async move {
                        let result = adapt_request(a, c, rc, req, b, mep, r).await;
                        if let Some(sender) = shutdown_local.lock().unwrap().take() {
                            let _ = sender.send(());
                        }
                        match result {
                            Ok(resp) => Ok::<_, hyper::Error>(resp),
                            Err(e) => {
                                error!(error=%e, "request handling failed (run_once)");
                                Ok(Response::builder()
                                    .status(500)
                                    .body(Body::from("internal error"))
                                    .unwrap())
                            }
                        }
                    }
                }))
            }
        });
        info!(address=%self.addr, mep=?self.mep, "starting HTTP inbound adapter (single request)");
        Server::bind(&self.addr)
            .serve(make)
            .with_graceful_shutdown(async {
                let _ = rx.await;
            })
            .await
            .map_err(|e| allora_core::error::Error::other(e.to_string()))?;
        Ok(())
    }
    pub fn spawn_once(self) -> HttpServerHandle {
        HttpServerHandle {
            join: tokio::spawn(async move { self.run_once().await }),
        }
    }
    pub fn spawn_serve(self) -> HttpServerHandle {
        HttpServerHandle {
            join: tokio::spawn(async move { self.serve().await }),
        }
    }
}
