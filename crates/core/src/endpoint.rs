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
//! # Object Safety Note
//! The trait returns `impl Future` for async methods, which makes it not object-safe.
//! That is acceptable for current usage (direct generic or concrete types). If you need
//! dynamic dispatch (`Box<dyn Endpoint>`), refactor to use `async_trait` instead.
//!
//! # Example
//! ```rust
//! use allora_core::{Exchange, Message};
//! use allora_core::endpoint::{Endpoint, EndpointBuilder};
//! let ep = EndpointBuilder::in_out().queue().build();
//! let rt = tokio::runtime::Runtime::new().unwrap();
//! rt.block_on(async {
//!     ep.send(Exchange::new(Message::from_text("A"))).await.unwrap();
//!     let received = ep.try_receive().await.unwrap();
//!     assert_eq!(received.in_msg.body_text(), Some("A"));
//! });
//! ```

use crate::channel::{Channel, ChannelRef};
use crate::{error::Result, Exchange};
use std::collections::VecDeque;
use std::sync::Arc;
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
    pub fn apply_headers(&self, exchange: &mut Exchange) {
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
    /// Enqueue of an Exchange.
    fn send(&self, exchange: Exchange) -> impl std::future::Future<Output = Result<()>> + Send;
    /// Non-blocking dequeue of next Exchange.
    fn try_receive(&self) -> impl std::future::Future<Output = Option<Exchange>> + Send;
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
pub struct InOutQueueEndpointBuilder {
    id: Option<String>,
    source: Option<EndpointSource>,
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

    /// Directly set a source metadata descriptor.
    pub fn source(mut self, src: EndpointSource) -> Self {
        self.source = Some(src);
        self
    }
    #[cfg(feature = "http")]
    pub fn source_http(
        self,
        _adapter: &Arc<()>,
        _method: impl Into<String>,
        _path: impl Into<String>,
    ) -> Self {
        self
    }
    pub fn source_channel<T: Channel + 'static>(mut self, channel: &Arc<T>) -> Self {
        self.source = Some(EndpointSource::Channel {
            channel_id: channel.id().to_string(),
        });
        // implicit unsize coercion Arc<T> -> Arc<dyn Channel>
        self.channel = Some(channel.clone());
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
        self,
        _adapter: &Arc<()>,
        _method: impl Into<String>,
        _path: impl Into<String>,
    ) -> Self {
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
}

impl Endpoint for InMemoryEndpoint {
    fn id(&self) -> &str {
        &self.id
    }
    fn send(&self, mut exchange: Exchange) -> impl std::future::Future<Output = Result<()>> + Send {
        async move {
            if let Some(src) = &self.source {
                src.apply_headers(&mut exchange);
            }
            let mut guard = self.inner.lock().await;
            guard.push_back(exchange);
            Ok(())
        }
    }
    fn try_receive(&self) -> impl std::future::Future<Output = Option<Exchange>> + Send {
        async move {
            let mut guard = self.inner.lock().await;
            guard.pop_front()
        }
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
    fn send(&self, mut exchange: Exchange) -> impl std::future::Future<Output = Result<()>> + Send {
        async move {
            if let Some(src) = &self.source {
                src.apply_headers(&mut exchange);
            }
            let mut g = self.inner.lock().await;
            g.push_back(exchange);
            Ok(())
        }
    }
    fn try_receive(&self) -> impl std::future::Future<Output = Option<Exchange>> + Send {
        async move { None }
    }
}
