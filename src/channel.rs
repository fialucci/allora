//! Channel module – lightweight message pipes.
//!
//! Provides abstractions for enqueuing and retrieving `Exchange` instances. Channels
//! do not perform transformation; they simply hold messages. Processing (filters,
//! processors, routing) occurs before sending into a channel.
//!
//! # Components
//! * [`Channel`] – core interface (sync or async send based on `async` feature).
//! * [`OutboundQueue`] – dequeue/receive operations (non-blocking + blocking convenience).
//! * [`CorrelationSupport`] – helper methods for request/reply style correlation IDs.
//! * [`DirectChannel`] – default direct handoff implementation (no internal queue).
//! * [`InMemoryChannel`] – buffered queue implementation (FIFO, supports correlation & dequeue).
//! * `ChannelBuilder` – staged builder for constructing in-memory channels.
//!
//! # Basic (Sync) Example
//! ```no_run
//! # use allora::{Message, Exchange};
//! # use allora::channel::{Channel, ChannelBuilder, OutboundQueue};
//! let ch = ChannelBuilder::point_to_point().in_memory().id("demo").build();
//! ch.send(Exchange::new(Message::from_text("ping"))).unwrap();
//! let exchange = ch.try_receive().unwrap();
//! assert_eq!(exchange.in_msg.body_text(), Some("ping"));
//! ```
//!
//! # Basic (Async) Example
//! ```no_run
//! # #[cfg(feature = "async")]
//! # {
//! # use allora::{Exchange, Message};
//! # use allora::channel::{Channel, ChannelBuilder, OutboundQueue};
//! let ch = ChannelBuilder::point_to_point().in_memory().id("async-demo").build();
//! let rt = tokio::runtime::Runtime::new().unwrap();
//! rt.block_on(async {
//!     ch.send_async(Exchange::new(Message::from_text("ping"))).await.unwrap();
//!     let received = ch.try_receive_async().await.unwrap();
//!     assert_eq!(received.in_msg.body_text(), Some("ping"));
//! });
//! # }
//! ```
//!
//! # Correlation Example (Sync)
//! ```no_run
//! # use allora::{Message, Exchange};
//! # use allora::channel::{ChannelBuilder, CorrelationSupport, OutboundQueue};
//! let ch = ChannelBuilder::point_to_point().in_memory().build();
//! let corr_id = ch.send_with_correlation(Exchange::new(Message::from_text("req"))).unwrap();
//! let exchange = ch.receive_by_correlation(&corr_id).unwrap();
//! assert_eq!(exchange.in_msg.body_text(), Some("req"));
//! ```
//!
//! # Correlation Example (Async)
//! ```no_run
//! # #[cfg(feature = "async")]
//! # {
//! # use allora::{Exchange, Message};
//! # use allora::channel::{ChannelBuilder, CorrelationSupport, OutboundQueue};
//! let ch = ChannelBuilder::point_to_point().in_memory().build();
//! let corr_id = ch.send_with_correlation(Exchange::new(Message::from_text("req"))).unwrap();
//! let exchange = tokio::runtime::Runtime::new().unwrap().block_on(async { ch.receive_by_correlation_async(&corr_id).await.unwrap() });
//! assert_eq!(exchange.in_msg.body_text(), Some("req"));
//! # }
//! ```
#[allow(dead_code)]
const _CHANNEL_DOC_EXAMPLE: () = ();

use crate::error::Error;
use crate::route::Route;
use crate::{error::Result, Exchange};
#[cfg(feature = "async")]
use async_trait::async_trait;
use std::any::Any;
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
use tracing::{debug, trace};

fn log_send_enqueued(channel_id: &str, exchange: &Exchange, is_async: bool, corr_id: Option<&str>) {
    trace!(channel_id=%channel_id, async=%is_async, corr_id=?corr_id, in_body=?exchange.in_msg.body_text(), "send enqueued");
}
fn log_receive(
    channel_id: &str,
    kind: &'static str, // try_receive | try_receive_async | receive_by_correlation | receive_by_correlation_async | receive_blocking
    phase: &'static str, // empty | dequeued | start | timeout | received
    is_async: bool,
    exchange: Option<&Exchange>,
    queue_size: Option<usize>,
    corr_id: Option<&str>,
    attempts: Option<u32>,
    elapsed_ms: Option<u128>,
    timeout_ms: Option<u128>,
) {
    trace!(
        channel_id=%channel_id,
        kind=%kind,
        phase=%phase,
        async=%is_async,
        queue_size=?queue_size,
        corr_id=?corr_id,
        attempts=?attempts,
        elapsed_ms=?elapsed_ms,
        timeout_ms=?timeout_ms,
        in_body=?exchange.and_then(|e| e.in_msg.body_text()),
        out_body=?exchange.and_then(|e| e.out_msg.as_ref().and_then(|m| m.body_text())),
        "receive dequeued"
    );
}

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
    /// Implementation kind identifier (lowercase). Default "unknown" for third-party channels until overridden.
    fn kind(&self) -> &'static str {
        "unknown"
    }
    /// Downcast helper for callers needing concrete type behavior.
    fn as_any(&self) -> &dyn Any; // object-safe downcast hook
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

/// Direct, synchronous handoff channel.
///
/// Semantics:
/// * No internal queue / buffering – `send` immediately invokes all subscribers in registration order.
/// * Temporal coupling – sender blocks until all subscribers finish (or an error short-circuits).
/// * Error handling – first subscriber returning `Err` stops dispatch; subsequent subscribers are skipped.
/// * Each subscriber receives a cloned `Exchange` (mutations are isolated per subscriber).
///
/// For decoupling / polling semantics use [`InMemoryChannel`].
pub struct DirectChannel {
    id: String,
    subscribers: std::sync::Mutex<Vec<Box<dyn Fn(Exchange) -> Result<()> + Send + Sync>>>,
}

impl std::fmt::Debug for DirectChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self.subscribers.lock().map(|v| v.len()).unwrap_or(0);
        f.debug_struct("DirectChannel")
            .field("id", &self.id)
            .field("subscribers", &count)
            .finish()
    }
}

impl DirectChannel {
    /// Create a new direct channel with auto-generated id `direct:<uuid>`.
    pub fn new() -> Self {
        Self {
            id: format!("direct:{}", uuid::Uuid::new_v4()),
            subscribers: std::sync::Mutex::new(Vec::new()),
        }
    }
    pub(crate) fn with_id<S: Into<String>>(id: S) -> Self {
        Self {
            id: id.into(),
            subscribers: std::sync::Mutex::new(Vec::new()),
        }
    }
    /// Subscribe a closure. Returns total subscriber count after registration.
    pub fn subscribe<F>(&self, f: F) -> usize
    where
        F: Fn(Exchange) -> Result<()> + Send + Sync + 'static,
    {
        let mut subs = self.subscribers.lock().unwrap();
        subs.push(Box::new(f));
        subs.len()
    }
    /// Internal helper: accept an already boxed subscriber (used by builder to avoid re-wrapping).
    fn subscribe_box(&self, boxed: Box<dyn Fn(Exchange) -> Result<()> + Send + Sync>) -> usize {
        let mut subs = self.subscribers.lock().unwrap();
        subs.push(boxed);
        subs.len()
    }
    fn dispatch(&self, exchange: Exchange, is_async: bool) -> Result<()> {
        let subs = self.subscribers.lock().unwrap();
        trace!(channel_id=%self.id, async=%is_async, subscribers=%subs.len(), in_body=?exchange.in_msg.body_text(), "direct dispatch start");
        for (idx, sub) in subs.iter().enumerate() {
            let cloned = exchange.clone();
            trace!(channel_id=%self.id, subscriber_index=idx, async=%is_async, in_body=?cloned.in_msg.body_text(), "direct dispatch to subscriber");
            sub(cloned)?;
        }
        Ok(())
    }
}

#[cfg_attr(feature = "async", async_trait)]
impl Channel for DirectChannel {
    fn id(&self) -> &str {
        &self.id
    }
    #[cfg(not(feature = "async"))]
    fn send(&self, exchange: Exchange) -> Result<()> {
        self.dispatch(exchange, false)
    }
    #[cfg(feature = "async")]
    async fn send_async(&self, exchange: Exchange) -> Result<()> {
        self.dispatch(exchange, true)
    }
    fn kind(&self) -> &'static str {
        "direct"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Builder for DirectChannel supporting optional id and initial subscribers.
pub struct DirectChannelBuilder {
    id: Option<String>,
    subscribers: Vec<Box<dyn Fn(Exchange) -> Result<()> + Send + Sync>>,
}
impl DirectChannelBuilder {
    pub fn new() -> Self {
        Self {
            id: None,
            subscribers: Vec::new(),
        }
    }
    /// Set an explicit id for the channel (skips auto-generated `direct:<uuid>`).
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
    /// Register an initial subscriber closure to be invoked on each dispatch.
    pub fn subscriber<F>(mut self, f: F) -> Self
    where
        F: Fn(Exchange) -> Result<()> + Send + Sync + 'static,
    {
        self.subscribers.push(Box::new(f));
        self
    }
    /// Finalize the builder returning a constructed DirectChannel.
    pub fn build(self) -> DirectChannel {
        let ch = match self.id {
            Some(id) => DirectChannel::with_id(id),
            None => DirectChannel::new(),
        };
        for s in self.subscribers {
            ch.subscribe_box(s);
        }
        ch
    }
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
    reported_kind: &'static str, // allows distinguishing 'direct' vs 'in_memory' config
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
            reported_kind: "in_memory",
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
            reported_kind: "in_memory",
        }
    }
    fn next_corr_id(&self) -> String {
        let id = format!("c{}", self.corr_seq.fetch_add(1, Ordering::Relaxed));
        trace!(channel_id=%self.id, corr_id=%id, "generated correlation id");
        id
    }
    fn ensure_correlation(&self, exchange: &mut Exchange) -> String {
        if let Some(id) = exchange.in_msg.header("corr_id") {
            let id_str = id.to_string();
            trace!(channel_id=%self.id, corr_id=%id_str, "reusing existing corr_id");
            if exchange.in_msg.header("correlation_id").is_none() {
                exchange.in_msg.set_header("correlation_id", &id_str);
            }
            id_str
        } else {
            let id = self.next_corr_id();
            trace!(channel_id=%self.id, corr_id=%id, "assigned new corr_id");
            exchange.in_msg.set_header("corr_id", &id);
            if exchange.in_msg.header("correlation_id").is_none() {
                exchange.in_msg.set_header("correlation_id", &id);
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
    async fn push_out_async(&self, exchange: Exchange) {
        let mut g = self.out_queue.lock().await;
        g.push(exchange);
    }
}

#[cfg_attr(feature = "async", async_trait)]
impl Channel for InMemoryChannel {
    fn id(&self) -> &str {
        &self.id
    }
    #[cfg(not(feature = "async"))]
    fn send(&self, exchange: Exchange) -> Result<()> {
        log_send_enqueued(self.id(), &exchange, false, None);
        self.push_out(exchange);
        Ok(())
    }
    #[cfg(feature = "async")]
    async fn send_async(&self, exchange: Exchange) -> Result<()> {
        log_send_enqueued(self.id(), &exchange, true, None);
        self.push_out_async(exchange).await;
        Ok(())
    }
    fn kind(&self) -> &'static str {
        self.reported_kind
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl OutboundQueue for InMemoryChannel {
    fn try_receive(&self) -> Option<Exchange> {
        #[cfg(not(feature = "async"))]
        {
            let exchange = self.out_queue.lock().unwrap().pop_front();
            if let Some(ref e) = exchange {
                log_receive(
                    self.id(),
                    "try_receive",
                    "dequeued",
                    false,
                    Some(e),
                    None,
                    None,
                    None,
                    None,
                    None,
                );
            } else {
                log_receive(
                    self.id(),
                    "try_receive",
                    "empty",
                    false,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
            }
            exchange
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
            log_receive(
                self.id(),
                "try_receive_async",
                "empty",
                true,
                None,
                Some(g.len()),
                None,
                None,
                None,
                None,
            );
            None
        } else {
            let exchange = g.remove(0);
            log_receive(
                self.id(),
                "try_receive_async",
                "dequeued",
                true,
                Some(&exchange),
                Some(g.len()),
                None,
                None,
                None,
                None,
            );
            Some(exchange)
        }
    }

    fn receive_blocking(&self, timeout: Option<Duration>) -> Option<Exchange> {
        log_receive(
            self.id(),
            "receive_blocking",
            "start",
            false,
            None,
            None,
            None,
            None,
            None,
            timeout.map(|t| t.as_millis()),
        );
        let start = Instant::now();
        let mut attempts = 0u32;
        loop {
            if let Some(exchange) = self.try_receive() {
                log_receive(
                    self.id(),
                    "receive_blocking",
                    "received",
                    false,
                    Some(&exchange),
                    None,
                    None,
                    Some(attempts),
                    Some(start.elapsed().as_millis()),
                    timeout.map(|t| t.as_millis()),
                );
                return Some(exchange);
            }
            attempts += 1;
            if let Some(t) = timeout {
                if start.elapsed() >= t {
                    log_receive(
                        self.id(),
                        "receive_blocking",
                        "timeout",
                        false,
                        None,
                        None,
                        None,
                        Some(attempts),
                        Some(start.elapsed().as_millis()),
                        Some(t.as_millis()),
                    );
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
        #[cfg(not(feature = "async"))]
        {
            log_send_enqueued(self.id(), &exchange, false, Some(&id));
            self.push_out(exchange);
        }
        #[cfg(feature = "async")]
        {
            log_send_enqueued(self.id(), &exchange, true, Some(&id));
            let rt = Runtime::new().map_err(|e| Error::other(e.to_string()))?;
            rt.block_on(self.push_out_async(exchange));
        }
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
                let exchange = g.remove(pos);
                log_receive(
                    self.id(),
                    "receive_by_correlation",
                    "dequeued",
                    false,
                    Some(&exchange),
                    Some(g.len()),
                    Some(_corr_id),
                    None,
                    None,
                    None,
                );
                return Some(exchange);
            }
            log_receive(
                self.id(),
                "receive_by_correlation",
                "empty",
                false,
                None,
                Some(g.len()),
                Some(_corr_id),
                None,
                None,
                None,
            );
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
            let exchange = g.remove(pos);
            log_receive(
                self.id(),
                "receive_by_correlation_async",
                "dequeued",
                true,
                Some(&exchange),
                Some(g.len()),
                Some(corr_id),
                None,
                None,
                None,
            );
            return Some(exchange);
        }
        log_receive(
            self.id(),
            "receive_by_correlation_async",
            "empty",
            true,
            None,
            Some(g.len()),
            Some(corr_id),
            None,
            None,
            None,
        );
        None
    }

    fn await_correlation(&self, corr_id: &str, timeout: Option<Duration>) -> Option<Exchange> {
        let start = Instant::now();
        loop {
            if let Some(exchange) = self.receive_by_correlation(corr_id) {
                return Some(exchange);
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

pub type ChannelRef = Arc<dyn Channel>;
pub type DefaultChannel = DirectChannel;
// Transitional alias for previous default queue-based channel.
pub type QueueChannel = InMemoryChannel;

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
    pub fn direct(self) -> DirectChannelBuilder {
        DirectChannelBuilder::new()
    }
}
pub struct InMemoryChannelBuilder {
    id: Option<String>,
}
impl InMemoryChannelBuilder {
    /// Assign a stable identifier (skips auto-generated `channel:<uuid>`).
    pub fn id(mut self, id: impl Into<String>) -> Self {
        let id_str = id.into();
        debug!(target: "allora::builder::channel", builder_id = %id_str, "InMemoryChannelBuilder: id set");
        self.id = Some(id_str);
        self
    }
    /// Backward compatibility: accept a Route during transition to pure pipe channels; ignored.
    pub fn route(self, _route: Route) -> Self {
        debug!(target: "allora::builder::channel", "InMemoryChannelBuilder: route() called (ignored in pure pipe mode)");
        self
    }
    /// Build the channel, generating an id if not provided.
    pub fn build(self) -> InMemoryChannel {
        debug!(target: "allora::builder::channel", builder_has_id = %self.id.is_some(), "InMemoryChannelBuilder: building channel");
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
/// #[cfg(not(feature = "async"))]
/// let exchange = ch.try_receive().unwrap();
/// #[cfg(feature = "async")]
/// let exchange = tokio::runtime::Runtime::new().unwrap().block_on(async { ch.try_receive_async().await.unwrap() });
/// assert_eq!(exchange.in_msg.body_text(), Some("data"));
/// ```
#[allow(dead_code)]
const _STAGED_BUILDER_EXAMPLE: () = ();
