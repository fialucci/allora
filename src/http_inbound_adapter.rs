#![cfg(feature = "http")]
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
//! * `InOut` – waits for downstream processing (legacy synchronous echo).
//! * `InOnly202` – returns HTTP 202 immediately; processing continues asynchronously.
//!
//! # Example (Builder)
//! ```rust
//! use allora::{channel::{ChannelBuilder, Channel}, http_inbound_adapter::{HttpInboundAdapter, Mep}};
//! use allora::adapter::Adapter;
//! # #[cfg(feature="http")] {
//! let channel = ChannelBuilder::point_to_point().in_memory().id("http-pipe").build();
//! let adapter = Adapter::inbound()
//!     .http()
//!     .host("127.0.0.1")
//!     .port(0)
//!     .channel(std::sync::Arc::new(channel))
//!     .in_only_202()
//!     .build();
//! assert_eq!(adapter.mep(), Mep::InOnly202);
//! }
//! ```

use crate::adapter::{ensure_correlation, BaseAdapter, InboundAdapter};
use crate::endpoint::EndpointSource;
use crate::endpoint::InMemoryEndpoint;
use crate::{channel::ChannelRef, error::Result, Exchange, Message, Payload};
use async_trait::async_trait;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, Version};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex, Weak};
use std::task::{Context, Poll};
use tracing::{error, info};

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
    routes: Arc<Mutex<HashMap<(String, String), Vec<Weak<InMemoryEndpoint>>>>>,
}

pub struct HttpServerHandle {
    join: tokio::task::JoinHandle<Result<()>>,
}

impl HttpServerHandle {
    pub async fn wait(self) -> Result<()> {
        self.join
            .await
            .unwrap_or_else(|e| Err(crate::error::Error::other(e.to_string())))
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
            Poll::Ready(r) => {
                Poll::Ready(r.unwrap_or_else(|e| Err(crate::error::Error::other(e.to_string()))))
            }
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
    // Builder entry via staged pattern: Adapter::inbound().http() returns builder
}

pub struct HttpInboundBuilder {
    id: Option<String>,
    host: String,
    port: u16,
    base_path: String,
    channel: Option<ChannelRef>,
    mep: Mep,
    registrations: Vec<(
        String,
        String,
        std::sync::Arc<crate::endpoint::InMemoryEndpoint>,
    )>,
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
            registrations: Vec::new(),
        }
    }
    /// Register an endpoint for a specific HTTP method and path. Path relative to base_path.
    pub fn register(
        mut self,
        method: &str,
        path: &str,
        endpoint: std::sync::Arc<crate::endpoint::InMemoryEndpoint>,
    ) -> Self {
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
    pub fn register_any(
        self,
        path: &str,
        endpoint: std::sync::Arc<crate::endpoint::InMemoryEndpoint>,
    ) -> Self {
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
        let base_path = if self.base_path.is_empty() {
            "/".to_string()
        } else {
            self.base_path
        };
        let channel = self
            .channel
            .expect("channel must be set on HttpInboundBuilder before build()");
        let adapter = HttpInboundAdapter {
            id,
            addr,
            base_path,
            channel,
            mep: self.mep,
            routes: Arc::new(Mutex::new(HashMap::new())),
        };
        // register endpoints (ANY method) after constructing adapter
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
        .map_err(|e| crate::error::Error::other(e.to_string()))?;

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
    let mut ex = Exchange::new(msg);
    ensure_correlation(&mut ex);

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
                let mut response_body: Option<String> = None;
                let mut remove_indices: Vec<usize> = Vec::new();
                for (idx, weak_ep) in endpoints.iter().enumerate() {
                    if let Some(ep) = weak_ep.upgrade() {
                        if let Some(ch_ref) = ep.channel() {
                            let mut ex_clone = ex.clone();
                            EndpointSource::Http {
                                adapter_id: adapter_id.clone(),
                                method: method.as_str().to_string(),
                                path: path_norm.clone(),
                            }
                            .apply_headers(&mut ex_clone);
                            #[cfg(feature = "async")]
                            {
                                ch_ref.send_async(ex_clone).await?;
                            }
                            #[cfg(not(feature = "async"))]
                            {
                                ch_ref.send(ex_clone)?;
                            }
                            if response_body.is_none() {
                                response_body = ex.in_msg.body_text().map(|s| s.to_string());
                            }
                        }
                    } else {
                        remove_indices.push(idx);
                    }
                }
                let body_final = response_body.unwrap_or_else(|| String::new());
                return Ok(Response::new(Body::from(body_final)));
            }
            // Fallback: enqueue on primary channel only.
            #[cfg(feature = "async")]
            {
                channel.send_async(ex.clone()).await?;
            }
            #[cfg(not(feature = "async"))]
            {
                channel.send(ex.clone())?;
            }
            let response_body = ex
                .in_msg
                .body_text()
                .map(|s| s.to_string())
                .unwrap_or_else(|| String::from_utf8_lossy(&body_bytes).to_string());
            Ok(Response::new(Body::from(response_body)))
        }
        Mep::InOnly202 => {
            // Fire-and-forget: dispatch in the background and ack now.
            let ch = channel.clone();
            #[cfg(feature = "async")]
            tokio::spawn(async move {
                let _ = ch.send_async(ex).await;
            });
            #[cfg(not(feature = "async"))]
            {
                // If no async feature, still try to dispatch synchronously in a thread.
                let _ = std::thread::spawn(move || {
                    let _ = ch.send(ex);
                });
            }
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
    pub async fn serve(&self) -> Result<()> {
        let channel = self.channel.clone();
        let base = self.base_path.clone();
        let mep = self.mep;
        let adapter_id = self.id.clone();
        let make = make_service_fn(move |_conn| {
            let channel_clone = channel.clone();
            let base_clone = base.clone();
            let routes_ref = self.routes.clone();
            let adapter_id_clone = adapter_id.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                    let c = channel_clone.clone();
                    let b = base_clone.clone();
                    let r = routes_ref.clone();
                    let a = adapter_id_clone.clone();
                    async move {
                        match adapt_request(a, c, req, b, mep, r).await {
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
            .map_err(|e| crate::error::Error::other(e.to_string()))?;
        Ok(())
    }
    pub async fn run_once(self) -> Result<()> {
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let tx_guard = Arc::new(Mutex::new(Some(tx)));
        let channel = self.channel.clone();
        let base = self.base_path.clone();
        let mep = self.mep;
        let adapter_id = self.id.clone();
        let make = make_service_fn(move |_conn| {
            let channel_clone = channel.clone();
            let tx_clone = tx_guard.clone();
            let base_clone = base.clone();
            let routes_ref = self.routes.clone();
            let adapter_id_clone = adapter_id.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                    let c = channel_clone.clone();
                    let tx_local = tx_clone.clone();
                    let b = base_clone.clone();
                    let r = routes_ref.clone();
                    let a = adapter_id_clone.clone();
                    async move {
                        let result = adapt_request(a.clone(), c, req, b, mep, r).await;
                        match result {
                            Ok(resp) => {
                                if let Some(sender) = tx_local.lock().unwrap().take() {
                                    let _ = sender.send(());
                                }
                                Ok::<_, hyper::Error>(resp)
                            }
                            Err(e) => {
                                error!(error=%e, "request handling failed");
                                if let Some(sender) = tx_local.lock().unwrap().take() {
                                    let _ = sender.send(());
                                }
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
            .map_err(|e| crate::error::Error::other(e.to_string()))?;
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
