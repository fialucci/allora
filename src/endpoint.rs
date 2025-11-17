//! Endpoint abstraction: a lightweight FIFO inbox for Exchanges.
//!
//! # Purpose
//! `Endpoint` represents a minimal buffering component for messages (`Exchange`) without
//! applying routing or processing logic. It is useful for:
//! * Simple test harnesses (inject messages, assert ordering).
//! * Staging / decoupling between an inbound adapter and a downstream `Channel`.
//! * Capturing outputs in integration tests when full routing is unnecessary.
//!
//! For richer semantics (processors, correlation, queues of processed results) prefer
//! the `Channel` abstraction.
//!
//! # Sync vs Async Feature
//! When the `async` feature is enabled, you should use the async methods (`send_async`,
//! `try_receive_async`). The synchronous methods panic in async mode to avoid accidental
//! blocking calls. When `async` is disabled, only the synchronous methods are available.
//!
//! # Object Safety Note
//! The trait returns `impl Future` for async methods, which makes it not object-safe.
//! That is acceptable for current usage (direct generic or concrete types). If you need
//! dynamic dispatch (`Box<dyn Endpoint>`), refactor to use `async_trait` instead.
//!
//! # Example (unified)
//! ```
//! use allora::{ Exchange, Message};
//! use allora::endpoint::EndpointBuilder;
//! let ep = EndpointBuilder::in_out().queue().build();
//! #[cfg(feature = "async")]
//! {
//!     use allora::endpoint::Endpoint;
//!     let rt = tokio::runtime::Runtime::new().unwrap();
//!     rt.block_on(async {
//!         ep.send_async(Exchange::new(Message::from_text("A"))).await.unwrap();
//!         assert_eq!(ep.try_receive_async().await.unwrap().in_msg.body_text(), Some("A"));
//!     });
//! }
//! #[cfg(not(feature = "async"))]
//! {
//!     use allora::endpoint::Endpoint;
//!     ep.send(Exchange::new(Message::from_text("A"))).unwrap();
//!     assert_eq!(ep.try_receive().unwrap().in_msg.body_text(), Some("A"));
//! }
//! ```
//!
//! # Implementation Notes
//! `InMemoryEndpoint` is the only provided implementation, backing exchanges with an
//! in-memory queue. It uses a mutex for synchronization, and is `Send`/`Sync` safe.
//!
//! This implementation is suitable for testing and simple use cases. For more complex
//! scenarios, consider building a custom endpoint or using the `Channel` abstraction.

use crate::channel::{Channel, ChannelRef};
#[cfg(feature = "http")]
use crate::http_inbound_adapter::HttpInboundAdapter;
use crate::{error::Result, Exchange};
use std::collections::VecDeque;
use std::sync::Arc;
#[cfg(not(feature = "async"))]
use std::sync::Mutex;
use std::sync::Weak;
#[cfg(feature = "async")]
use tokio::sync::Mutex;

/// Source metadata describing origin of messages entering an endpoint.
#[derive(Clone, Debug)]
pub enum EndpointSource {
    Http {
        adapter_id: String,
        method: String,
        path: String,
    },
    Channel {
        channel_id: String,
    },
}
impl EndpointSource {
    pub(crate) fn apply_headers(&self, exchange: &mut Exchange) {
        match self {
            EndpointSource::Http {
                adapter_id,
                method,
                path,
            } => {
                if exchange.in_msg.header("source.kind").is_none() {
                    exchange.in_msg.set_header("source.kind", "http");
                }
                if exchange.in_msg.header("source.adapter_id").is_none() {
                    exchange.in_msg.set_header("source.adapter_id", adapter_id);
                }
                if exchange.in_msg.header("source.http.method").is_none() {
                    exchange.in_msg.set_header("source.http.method", method);
                }
                if exchange.in_msg.header("source.http.path").is_none() {
                    exchange.in_msg.set_header("source.http.path", path);
                }
            }
            EndpointSource::Channel { channel_id } => {
                if exchange.in_msg.header("source.kind").is_none() {
                    exchange.in_msg.set_header("source.kind", "channel");
                }
                if exchange.in_msg.header("source.channel_id").is_none() {
                    exchange.in_msg.set_header("source.channel_id", channel_id);
                }
            }
        }
    }
}

/// A trait representing a message endpoint for sending and receiving [`Exchange`] objects.
pub trait Endpoint: Send + Sync {
    /// Stable identifier for this endpoint instance.
    fn id(&self) -> &str;
    /// Enqueue an Exchange (synchronous mode). Panics if called under `async` feature.
    fn send(&self, exchange: Exchange) -> Result<()>;
    /// Non-blocking attempt to dequeue next Exchange (synchronous mode). Panics if async feature enabled.
    fn try_receive(&self) -> Option<Exchange>;
    #[cfg(feature = "async")]
    /// Async enqueue of an Exchange.
    fn send_async(
        &self,
        exchange: Exchange,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    #[cfg(feature = "async")]
    /// Async non-blocking dequeue of next Exchange.
    fn try_receive_async(&self) -> impl std::future::Future<Output = Option<Exchange>> + Send;
}

/// Staged builder root for endpoints.
pub struct EndpointBuilder;
impl EndpointBuilder {
    pub fn in_out() -> InOutStage {
        InOutStage
    }
    pub fn in_only() -> InOnlyStage {
        InOnlyStage
    }
}
pub struct InOutStage;
pub struct InOnlyStage;
impl InOutStage {
    pub fn queue(self) -> InOutQueueEndpointBuilder {
        InOutQueueEndpointBuilder {
            id: None,
            source: None,
            wire: None,
            channel: None,
        }
    }
}
impl InOnlyStage {
    pub fn queue(self) -> InOnlyInMemoryEndpointBuilder {
        InOnlyInMemoryEndpointBuilder {
            id: None,
            source: None,
        }
    }
}
/// Builder for in-out (send + receive) in-memory endpoint.
#[allow(dead_code)]
enum DeferredWire {
    #[cfg(feature = "http")]
    Http {
        adapter: Weak<HttpInboundAdapter>,
        method: String,
        path: String,
    },
    #[allow(dead_code)]
    Channel { channel: Weak<dyn Channel> },
}
pub struct InOutQueueEndpointBuilder {
    id: Option<String>,
    source: Option<EndpointSource>,
    wire: Option<DeferredWire>,
    channel: Option<ChannelRef>,
}
impl InOutQueueEndpointBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
    pub fn channel(mut self, ch: ChannelRef) -> Self {
        self.channel = Some(ch);
        self
    }
    #[cfg(feature = "http")]
    pub fn source_http(
        mut self,
        adapter: &Arc<HttpInboundAdapter>,
        method: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        let m = method.into();
        let p = path.into();
        assert!(
            self.channel.is_some(),
            "endpoint channel must be set before source_http"
        );
        self.source = Some(EndpointSource::Http {
            adapter_id: adapter.id().to_string(),
            method: m.clone(),
            path: p.clone(),
        });
        self.wire = Some(DeferredWire::Http {
            adapter: Arc::downgrade(adapter),
            method: m,
            path: p,
        });
        self
    }
    pub fn source_channel<T: Channel + 'static>(mut self, channel: &Arc<T>) -> Self {
        self.source = Some(EndpointSource::Channel {
            channel_id: channel.id().to_string(),
        });
        // store as trait object
        let obj: ChannelRef = Arc::clone(channel) as ChannelRef;
        self.channel = Some(obj);
        self
    }
    pub fn build(self) -> Arc<InMemoryEndpoint> {
        let ep = match self.id {
            Some(id) => Arc::new(InMemoryEndpoint::with_id_and_source(
                id,
                self.source.clone(),
                self.channel.clone(),
            )),
            None => Arc::new(InMemoryEndpoint::new_with_source(
                self.source.clone(),
                self.channel.clone(),
            )),
        };
        if let Some(w) = self.wire {
            match w {
                DeferredWire::Channel { channel: _ } => {
                    // skip channel wiring for in-out endpoints (not supported)
                }
                #[cfg(feature = "http")]
                DeferredWire::Http {
                    adapter,
                    method,
                    path,
                } => {
                    if let Some(ad) = adapter.upgrade() {
                        ad.register_endpoint(&method, &path, Arc::downgrade(&ep));
                    }
                }
            }
        }
        ep
    }
}
/// Builder for in-only (send only) in-memory endpoint.
pub struct InOnlyInMemoryEndpointBuilder {
    id: Option<String>,
    source: Option<EndpointSource>,
}
impl InOnlyInMemoryEndpointBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
    #[cfg(feature = "http")]
    pub fn source_http(
        mut self,
        adapter: &Arc<HttpInboundAdapter>,
        method: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        let m = method.into();
        let p = path.into();
        self.source = Some(EndpointSource::Http {
            adapter_id: adapter.id().to_string(),
            method: m.clone(),
            path: p.clone(),
        });
        // no registration for in-only endpoints
        self
    }
    pub fn source_channel<T: Channel + 'static>(mut self, channel: &Arc<T>) -> Self {
        self.source = Some(EndpointSource::Channel {
            channel_id: channel.id().to_string(),
        });
        self
    }
    pub fn build(self) -> Arc<InMemoryInOnlyEndpoint> {
        let id = self
            .id
            .unwrap_or_else(|| format!("endpoint:{}", uuid::Uuid::new_v4()));
        let ep = Arc::new(InMemoryInOnlyEndpoint {
            id,
            inner: std::sync::Arc::new(Mutex::new(VecDeque::new())),
            source: self.source,
        });
        ep
    }
}

/// An in-memory FIFO endpoint for quick testing.
///
/// Internally uses a `VecDeque` protected by a mutex (`std::sync::Mutex` or
/// `tokio::sync::Mutex` for async mode). No backpressure, size limits, or correlation
/// semantics are provided—this is intentionally minimal.
#[derive(Clone, Default)]
pub struct InMemoryEndpoint {
    id: String,
    inner: Arc<Mutex<VecDeque<Exchange>>>,
    source: Option<EndpointSource>,
    channel: Option<ChannelRef>,
}

impl InMemoryEndpoint {
    /// Create a new empty endpoint (crate-private; use builder).
    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Self {
            id: format!("endpoint:{}", uuid::Uuid::new_v4()),
            inner: Arc::new(Mutex::new(VecDeque::new())),
            source: None,
            channel: None,
        }
    }
    pub(crate) fn new_with_source(
        source: Option<EndpointSource>,
        channel: Option<ChannelRef>,
    ) -> Self {
        Self {
            id: format!("endpoint:{}", uuid::Uuid::new_v4()),
            inner: Arc::new(Mutex::new(VecDeque::new())),
            source,
            channel,
        }
    }
    /// Create a new empty endpoint with a custom ID (crate-private; use builder).
    #[allow(dead_code)]
    pub(crate) fn with_id<S: Into<String>>(id: S) -> Self {
        Self {
            id: id.into(),
            inner: Arc::new(Mutex::new(VecDeque::new())),
            source: None,
            channel: None,
        }
    }
    #[allow(dead_code)]
    pub(crate) fn with_id_and_source<S: Into<String>>(
        id: S,
        source: Option<EndpointSource>,
        channel: Option<ChannelRef>,
    ) -> Self {
        Self {
            id: id.into(),
            inner: Arc::new(Mutex::new(VecDeque::new())),
            source,
            channel,
        }
    }
    /// Get the ID of the endpoint.
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn source(&self) -> Option<&EndpointSource> {
        self.source.as_ref()
    }
    pub fn channel(&self) -> Option<&ChannelRef> {
        self.channel.as_ref()
    }
    #[allow(dead_code)]
    pub(crate) fn enqueue_with_source(&self, mut _exchange: Exchange, _src: EndpointSource) {
        #[cfg(not(feature = "async"))]
        {
            _src.apply_headers(&mut _exchange);
            if let Ok(mut g) = self.inner.lock() {
                g.push_back(_exchange);
            }
        }
        #[cfg(feature = "async")]
        {
            panic!("watcher enqueue not supported in async mode yet");
        }
    }
}

impl Endpoint for InMemoryEndpoint {
    fn id(&self) -> &str {
        &self.id
    }
    fn send(&self, mut exchange: Exchange) -> Result<()> {
        if let Some(src) = &self.source {
            src.apply_headers(&mut exchange);
        }
        #[cfg(not(feature = "async"))]
        {
            let mut guard = self.inner.lock().unwrap();
            guard.push_back(exchange);
            Ok(())
        }
        #[cfg(feature = "async")]
        // This function should only be called from async context when async feature is enabled
        {
            panic!("send should not be called in async mode; use send_async instead");
        }
    }
    #[cfg(feature = "async")]
    async fn send_async(&self, mut exchange: Exchange) -> Result<()> {
        if let Some(src) = &self.source {
            src.apply_headers(&mut exchange);
        }
        let mut guard = self.inner.lock().await;
        guard.push_back(exchange);
        Ok(())
    }
    fn try_receive(&self) -> Option<Exchange> {
        #[cfg(not(feature = "async"))]
        {
            let mut guard = self.inner.lock().unwrap();
            guard.pop_front()
        }
        #[cfg(feature = "async")]
        // This function should only be called from async context when async feature is enabled
        {
            panic!("try_receive should not be called in async mode; use try_receive_async instead");
        }
    }
    #[cfg(feature = "async")]
    async fn try_receive_async(&self) -> Option<Exchange> {
        let mut guard = self.inner.lock().await;
        guard.pop_front()
    }
}
#[cfg(feature = "http")]
impl InMemoryEndpoint {
    pub fn attach_http(
        self: &Arc<Self>,
        adapter: &HttpInboundAdapter,
        method: &str,
        path: &str,
    ) -> &Self {
        adapter.register_endpoint(method, path, Arc::downgrade(self));
        self
    }
    pub fn attach_http_any(self: &Arc<Self>, adapter: &HttpInboundAdapter, path: &str) -> &Self {
        adapter.register_endpoint("ANY", path, Arc::downgrade(self));
        self
    }
}

/// In-only endpoint: supports sending but not receiving (try_receive returns None).
#[derive(Clone, Default)]
pub struct InMemoryInOnlyEndpoint {
    id: String,
    inner: Arc<Mutex<VecDeque<Exchange>>>,
    source: Option<EndpointSource>,
}
impl InMemoryInOnlyEndpoint {
    pub fn id(&self) -> &str {
        &self.id
    }
}
impl Endpoint for InMemoryInOnlyEndpoint {
    fn id(&self) -> &str {
        &self.id
    }
    fn send(&self, mut exchange: Exchange) -> Result<()> {
        if let Some(src) = &self.source {
            src.apply_headers(&mut exchange);
        }
        #[cfg(not(feature = "async"))]
        {
            let mut g = self.inner.lock().unwrap();
            g.push_back(exchange);
            Ok(())
        }
        #[cfg(feature = "async")]
        {
            panic!("send should not be called in async mode; use send_async instead")
        }
    }
    #[cfg(feature = "async")]
    async fn send_async(&self, mut exchange: Exchange) -> Result<()> {
        if let Some(src) = &self.source {
            src.apply_headers(&mut exchange);
        }
        let mut g = self.inner.lock().await;
        g.push_back(exchange);
        Ok(())
    }
    fn try_receive(&self) -> Option<Exchange> {
        None
    }
    #[cfg(feature = "async")]
    async fn try_receive_async(&self) -> Option<Exchange> {
        None
    }
}
