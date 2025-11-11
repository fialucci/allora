///! Channel abstractions: queuing and request-reply correlation (pure pipe – no Route processing).
///!
///! # Overview
///! A `Channel` is now a lightweight pipe: it accepts `Exchange`s (optionally assigns correlation)
///! and stores them for retrieval. Transformation logic (filters/processors) lives outside
///! in a `Route`, executed before sending to the channel.
///!
///! This file defines:
///! * [`Channel`] – core pipe interface (sync / async send depending on feature flags).
///! * [`OutboundQueue`] – extension trait for retrieving queued Exchanges.
///! * [`CorrelationSupport`] – extension trait for request/reply style correlation helpers.
///! * [`InMemoryChannel`] – default in-process implementation with queue & correlation.
///!
///! # Example (pipe usage)
///! ```
///! use allora::{channel::{ChannelBuilder, OutboundQueue}, Message, Exchange};
///! let channel = ChannelBuilder::point_to_point().in_memory().id("chan-1").build();
///! channel.send(Exchange::new(Message::from_text("ping"))).unwrap();
///! let ex = channel.try_receive().unwrap();
///! assert_eq!(ex.in_msg.body_text(), Some("ping"));
///! ```
///!
///! # Example (processing outside channel)
///! ```
///! use allora::{route::Route, processor::ClosureProcessor, Message, Exchange, channel::{ChannelBuilder, OutboundQueue}};
///! let route = Route::new().add(ClosureProcessor::new(|ex| { ex.out_msg = Some(Message::from_text("done")); Ok(()) })).build();
///! let mut ex = Exchange::new(Message::from_text("start"));
///! route.run(&mut ex).unwrap(); // apply transformations
///! let channel = ChannelBuilder::point_to_point().in_memory().build();
///! channel.send(ex.clone()).unwrap();
///! let stored = channel.try_receive().unwrap();
///! assert_eq!(stored.out_msg.unwrap().body_text(), Some("done"));
///! ```
#[allow(dead_code)]
const _CHANNEL_DOC_EXAMPLE: () = ();

use crate::error::Error;
use crate::route::Route;
use crate::{error::Result, Exchange};
#[cfg(feature = "async")]
use async_trait::async_trait;
#[cfg(not(feature = "async"))]
use std::collections::VecDeque;
use std::fmt::Debug;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
#[cfg(not(feature = "async"))]
use std::sync::Mutex;
use std::thread::sleep;
use std::time::{Duration, Instant};
#[cfg(feature = "async")]
use tokio::runtime::Runtime;
#[cfg(feature = "async")]
use tokio::sync::Mutex;

/// Core Channel interface: pipe for sending Exchanges (no internal processing).
#[cfg_attr(feature = "async", async_trait)]
pub trait Channel: Send + Sync + Debug {
    /// Stable identifier for this channel instance.
    fn id(&self) -> &str;
    /// Enqueue an `Exchange` (sync variant when `async` feature disabled).
    #[cfg(not(feature = "async"))]
    fn send(&self, exchange: Exchange) -> Result<()>;
    /// Async enqueue variant.
    #[cfg(feature = "async")]
    async fn send_async(&self, exchange: Exchange) -> Result<()>;
    /// Convenience sync wrapper for async mode.
    #[cfg(feature = "async")]
    fn send(&self, exchange: Exchange) -> Result<()> {
        let rt = Runtime::new().map_err(|e| Error::other(e.to_string()))?;
        rt.block_on(self.send_async(exchange))
    }
}

/// Extension trait for channels that maintain an outbound (queued) Exchange list.
pub trait OutboundQueue: Send + Sync + Debug {
    fn try_receive(&self) -> Option<Exchange>;
    #[cfg(feature = "async")]
    fn try_receive_async(&self) -> impl std::future::Future<Output = Option<Exchange>> + Send;
    fn receive_blocking(&self, timeout: Option<Duration>) -> Option<Exchange>;
}

/// Extension trait for correlation helpers on channels.
pub trait CorrelationSupport: Send + Sync + Debug {
    fn send_with_correlation(&self, exchange: Exchange) -> Result<String>;
    fn receive_by_correlation(&self, corr_id: &str) -> Option<Exchange>;
    #[cfg(feature = "async")]
    fn receive_by_correlation_async(
        &self,
        corr_id: &str,
    ) -> impl std::future::Future<Output = Option<Exchange>> + Send;
    fn await_correlation(&self, corr_id: &str, timeout: Option<Duration>) -> Option<Exchange>;
}

/// Optional introspection for channels (implementation kind).
pub trait ChannelInfo {
    /// Returns a stable lowercase identifier for the channel implementation kind.
    fn kind(&self) -> &'static str;
}

/// In-memory pipe implementation.
#[derive(Clone, Debug)]
pub struct InMemoryChannel {
    id: String,
    #[cfg(not(feature = "async"))]
    out_queue: Arc<Mutex<VecDeque<Exchange>>>,
    #[cfg(feature = "async")]
    out_queue: Arc<Mutex<Vec<Exchange>>>,
    corr_seq: Arc<AtomicU64>,
}

impl InMemoryChannel {
    pub(crate) fn new() -> Self {
        Self {
            id: format!("channel:{}", uuid::Uuid::new_v4()),
            #[cfg(not(feature = "async"))]
            out_queue: Arc::new(Mutex::new(VecDeque::new())),
            #[cfg(feature = "async")]
            out_queue: Arc::new(Mutex::new(Vec::new())),
            corr_seq: Arc::new(AtomicU64::new(1)),
        }
    }
    pub(crate) fn with_id<S: Into<String>>(id: S) -> Self {
        Self {
            id: id.into(),
            #[cfg(not(feature = "async"))]
            out_queue: Arc::new(Mutex::new(VecDeque::new())),
            #[cfg(feature = "async")]
            out_queue: Arc::new(Mutex::new(Vec::new())),
            corr_seq: Arc::new(AtomicU64::new(1)),
        }
    }
    fn next_corr_id(&self) -> String {
        format!("c{}", self.corr_seq.fetch_add(1, Ordering::Relaxed))
    }
    fn ensure_correlation(&self, ex: &mut Exchange) -> String {
        if let Some(id) = ex.in_msg.header("corr_id") {
            let id_str = id.to_string();
            if ex.in_msg.header("correlation_id").is_none() {
                ex.in_msg.set_header("correlation_id", &id_str);
            }
            id_str
        } else {
            let id = self.next_corr_id();
            ex.in_msg.set_header("corr_id", &id);
            if ex.in_msg.header("correlation_id").is_none() {
                ex.in_msg.set_header("correlation_id", &id);
            }
            id
        }
    }
    #[allow(dead_code)]
    fn push_out(&self, _ex: Exchange) {
        #[cfg(not(feature = "async"))]
        {
            self.out_queue.lock().unwrap().push_back(_ex);
        }
        #[cfg(feature = "async")]
        {
            panic!("push_out should not be called in async mode; use push_out_async instead");
        }
    }
    #[cfg(feature = "async")]
    async fn push_out_async(&self, ex: Exchange) {
        let mut g = self.out_queue.lock().await;
        g.push(ex);
    }
}

#[cfg_attr(feature = "async", async_trait)]
impl Channel for InMemoryChannel {
    fn id(&self) -> &str {
        &self.id
    }
    #[cfg(not(feature = "async"))]
    fn send(&self, exchange: Exchange) -> Result<()> {
        self.push_out(exchange);
        Ok(())
    }
    #[cfg(feature = "async")]
    async fn send_async(&self, exchange: Exchange) -> Result<()> {
        self.push_out_async(exchange).await;
        Ok(())
    }
}

impl OutboundQueue for InMemoryChannel {
    fn try_receive(&self) -> Option<Exchange> {
        #[cfg(not(feature = "async"))]
        {
            self.out_queue.lock().unwrap().pop_front()
        }
        #[cfg(feature = "async")]
        {
            panic!("try_receive should not be called in async mode; use try_receive_async")
        }
    }
    #[cfg(feature = "async")]
    async fn try_receive_async(&self) -> Option<Exchange> {
        let mut g = self.out_queue.lock().await;
        if g.is_empty() {
            None
        } else {
            Some(g.remove(0))
        }
    }

    fn receive_blocking(&self, timeout: Option<Duration>) -> Option<Exchange> {
        let start = Instant::now();
        loop {
            if let Some(ex) = self.try_receive() {
                return Some(ex);
            }
            if let Some(t) = timeout {
                if start.elapsed() >= t {
                    return None;
                }
            }
            sleep(Duration::from_millis(5));
        }
    }
}

impl CorrelationSupport for InMemoryChannel {
    fn send_with_correlation(&self, mut exchange: Exchange) -> Result<String> {
        let id = self.ensure_correlation(&mut exchange);
        self.send(exchange)?;
        Ok(id)
    }

    fn receive_by_correlation(&self, _corr_id: &str) -> Option<Exchange> {
        #[cfg(not(feature = "async"))]
        {
            let mut g = self.out_queue.lock().unwrap();
            if let Some(pos) = g
                .iter()
                .position(|e| e.in_msg.header("corr_id") == Some(_corr_id))
            {
                return g.remove(pos);
            }
            None
        }
        #[cfg(feature = "async")]
        {
            panic!("receive_by_correlation should not be called in async mode; use receive_by_correlation_async")
        }
    }
    #[cfg(feature = "async")]
    async fn receive_by_correlation_async(&self, corr_id: &str) -> Option<Exchange> {
        let mut g = self.out_queue.lock().await;
        if let Some(pos) = g
            .iter()
            .position(|e| e.in_msg.header("corr_id") == Some(corr_id))
        {
            Some(g.remove(pos))
        } else {
            None
        }
    }

    fn await_correlation(&self, corr_id: &str, timeout: Option<Duration>) -> Option<Exchange> {
        let start = Instant::now();
        loop {
            if let Some(ex) = self.receive_by_correlation(corr_id) {
                return Some(ex);
            }
            if let Some(t) = timeout {
                if start.elapsed() >= t {
                    return None;
                }
            }
            sleep(Duration::from_millis(5));
        }
    }
}

impl ChannelInfo for InMemoryChannel {
    fn kind(&self) -> &'static str {
        "in_memory"
    }
}

/// Staged builder root (pattern-only for now). Channels are pure pipes; builder selects kind & optional id.
pub struct ChannelBuilder;
impl ChannelBuilder {
    /// Begin building a point-to-point channel (currently only in-memory implementation).
    pub fn point_to_point() -> PointToPointStage {
        PointToPointStage
    }
}
pub struct PointToPointStage;
impl PointToPointStage {
    pub fn in_memory(self) -> InMemoryChannelBuilder {
        InMemoryChannelBuilder { id: None }
    }
}
pub struct InMemoryChannelBuilder {
    id: Option<String>,
}
impl InMemoryChannelBuilder {
    /// Assign a stable identifier (skips auto-generated `channel:<uuid>`).
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
    /// Backward compatibility: accept a Route during transition to pure pipe channels; ignored.
    pub fn route(self, _route: Route) -> Self {
        self
    }
    /// Build the channel, generating an id if not provided.
    pub fn build(self) -> InMemoryChannel {
        match self.id {
            Some(id) => InMemoryChannel::with_id(id),
            None => InMemoryChannel::new(),
        }
    }
}
/// Example (staged builder)
/// ```
/// use allora::channel::{ChannelBuilder, Channel, OutboundQueue};
/// use allora::{Message, Exchange};
/// let ch = ChannelBuilder::point_to_point().in_memory().id("pipe").build();
/// ch.send(Exchange::new(Message::from_text("data"))).unwrap();
/// #[cfg(not(feature="async"))]
/// let ex = ch.try_receive().unwrap();
/// #[cfg(feature="async")]
/// let ex = tokio::runtime::Runtime::new().unwrap().block_on(async { ch.try_receive_async().await.unwrap() });
/// assert_eq!(ex.in_msg.body_text(), Some("data"));
/// ```
#[allow(dead_code)]
const _STAGED_BUILDER_EXAMPLE: () = ();

pub type ChannelRef = Arc<dyn Channel>;
pub type DefaultChannel = InMemoryChannel;
