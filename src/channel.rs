///! Channel abstractions: dispatching, queuing, and request-reply correlation.
///!
///! # Overview
///! A `Channel` is a lightweight conduit that applies a `Route` (ordered `Processor`s)
///! to incoming `Exchange`s. This file defines:
///! * [`Channel`] – core dispatch interface (sync or async depending on feature flags).
///! * [`OutboundQueue`] – extension trait for retrieving processed Exchanges (queue semantics).
///! * [`CorrelationSupport`] – extension trait for request/reply style correlation.
///! * [`InMemoryChannel`] – default in-process implementation with optional queuing & correlation.
///!
///! # Sync vs Async
///! With the `async` feature enabled:
///! * Use `dispatch_async` for non-blocking operation.
///! * A convenience synchronous wrapper `dispatch` is provided (spins a temporary runtime).
///! Without `async`, only synchronous `dispatch` exists.
///!
///! # Queue Semantics
///! Channels implementing [`OutboundQueue`] push processed Exchanges onto an internal queue.
///! This lets callers poll for route results after fire-and-forget `send` calls.
///!
///! # Correlation Semantics
///! [`CorrelationSupport::send_with_correlation`] guarantees a correlation id header. For historical
///! compatibility it sets `corr_id`; documentation recommends migrating to `correlation_id`.
///! The implementation now mirrors the value into `correlation_id` when generating a new id.
///!
///! # Example (basic dispatch)
///! ```
///! use allora::{route::Route, processor::ClosureProcessor, Message, Exchange, InMemoryChannel};
///! let route = Route::new().add(ClosureProcessor::new(|ex| { ex.out_msg = Some(Message::from_text("ok")); Ok(()) })).build();
///! let channel = InMemoryChannel::new(route);
///! let ex = Exchange::new(Message::from_text("ping"));
///! let processed = channel.dispatch(ex).unwrap();
///! assert_eq!(processed.out_msg.unwrap().body_text(), Some("ok"));
///! ```
///!
///! # Example (request/reply with correlation)
///! ```
///! use allora::{route::Route, processor::ClosureProcessor, Message, Exchange, InMemoryChannel, CorrelationSupport};
///! let route = Route::new().add(ClosureProcessor::new(|ex| { ex.out_msg = Some(Message::from_text("reply")); Ok(()) })).build();
///! let channel = InMemoryChannel::new(route);
///! let corr_id = channel.send_with_correlation(Exchange::new(Message::from_text("ask"))).unwrap();
///! let reply = channel.receive_by_correlation(&corr_id).unwrap();
///! assert_eq!(reply.out_msg.unwrap().body_text(), Some("reply"));
///! ```
///
use crate::error::Error;
use crate::{error::Result, route::Route, Exchange};
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


/// Core Channel interface: dispatches Exchanges through an associated Route.
///
/// Implementors may add capabilities via the extension traits:
/// * [`OutboundQueue`] for retrieving processed results.
/// * [`CorrelationSupport`] for request-reply flows.
///
/// # Error Handling
/// Errors from processors propagate directly. If partial output was produced before an error,
/// that `Exchange` instance is returned only if the implementation chooses (InMemoryChannel does not).
#[cfg_attr(feature = "async", async_trait)]
pub trait Channel: Send + Sync + Debug {
    /// Process (mutate) an `Exchange` and return the final state (sync variant, non-`async` feature).
    #[cfg(not(feature = "async"))]
    fn dispatch(&self, exchange: Exchange) -> Result<Exchange>;
    /// Asynchronous dispatch variant under the `async` feature.
    #[cfg(feature = "async")]
    async fn dispatch_async(&self, exchange: Exchange) -> Result<Exchange>;
    /// Convenience synchronous wrapper for async environments (spawns a temporary runtime).
    #[cfg(feature = "async")]
    fn dispatch(&self, exchange: Exchange) -> Result<Exchange> {
        let rt = Runtime::new().map_err(|e| Error::other(e.to_string()))?;
        rt.block_on(self.dispatch_async(exchange))
    }
    /// Fire-and-forget style send; default implementation delegates to `dispatch` and discards returned Exchange.
    fn send(&self, exchange: Exchange) -> Result<()> {
        let _ = self.dispatch(exchange)?;
        Ok(())
    }
}

/// Extension trait for channels that maintain an outbound (processed) Exchange queue.
/// Enables asynchronous decoupling: caller submits work then polls / blocks for completion.
pub trait OutboundQueue: Send + Sync + Debug {
    /// Non-blocking attempt to retrieve next processed Exchange (sync mode only).
    fn try_receive(&self) -> Option<Exchange>;
    /// Async non-blocking attempt (only available with `async` feature).
    #[cfg(feature = "async")]
    fn try_receive_async(&self) -> impl std::future::Future<Output = Option<Exchange>> + Send;
    /// Blocking retrieval with optional timeout (polling sleep strategy in current implementation).
    fn receive_blocking(&self, timeout: Option<Duration>) -> Option<Exchange>;
}

/// Extension trait for channels offering request-reply correlation semantics.
/// The default implementation in [`InMemoryChannel`] uses a monotonic sequence to generate ids.
pub trait CorrelationSupport: Send + Sync + Debug {
    /// Send an Exchange, injecting a new correlation id if absent, returning the id.
    fn send_with_correlation(&self, exchange: Exchange) -> Result<String>;
    /// Attempt to retrieve a processed Exchange matching the provided correlation id.
    fn receive_by_correlation(&self, corr_id: &str) -> Option<Exchange>;
    /// Async variant of retrieval (only with `async` feature).
    #[cfg(feature = "async")]
    fn receive_by_correlation_async(
        &self,
        corr_id: &str,
    ) -> impl std::future::Future<Output = Option<Exchange>> + Send;
    /// Block until matching Exchange arrives or timeout elapses.
    fn await_correlation(&self, corr_id: &str, timeout: Option<Duration>) -> Option<Exchange>;
}

/// In-memory Channel implementation supporting routing, outbound queueing and simple
/// correlation for request-reply style flows.
///
/// # Correlation Header
/// Generates `corr_id` and mirrors it into `correlation_id` for convergence toward a unified header.
/// Existing code relying on `corr_id` remains compatible.
#[derive(Clone, Debug)]
pub struct InMemoryChannel {
    route: Arc<Route>,
    #[cfg(not(feature = "async"))]
    out_queue: Arc<Mutex<VecDeque<Exchange>>>,
    #[cfg(feature = "async")]
    out_queue: Arc<Mutex<Vec<Exchange>>>,
    corr_seq: Arc<AtomicU64>,
}

impl InMemoryChannel {
    /// Construct a new in-memory channel wrapping the provided `Route`.
    pub fn new(route: Route) -> Self {
        Self {
            route: Arc::new(route),
            #[cfg(not(feature = "async"))]
            out_queue: Arc::new(Mutex::new(VecDeque::new())),
            #[cfg(feature = "async")]
            out_queue: Arc::new(Mutex::new(Vec::new())),
            corr_seq: Arc::new(AtomicU64::new(1)),
        }
    }
    /// Generate the next correlation id (internal monotonic sequence: `c1`, `c2`, ...).
    fn next_corr_id(&self) -> String {
        format!("c{}", self.corr_seq.fetch_add(1, Ordering::Relaxed))
    }
    /// Ensure the given `Exchange` has a correlation id. Returns the id.
    /// Sets both `corr_id` and (if absent) `correlation_id` for forward compatibility.
    fn ensure_correlation(&self, ex: &mut Exchange) -> String {
        let existing = ex.in_msg.header("corr_id").map(|s| s.to_string());
        if let Some(id) = existing {
            if ex.in_msg.header("correlation_id").is_none() {
                ex.in_msg.set_header("correlation_id", &id);
            }
            id
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
            let mut g = self.out_queue.lock().unwrap();
            g.push_back(_ex);
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
    #[cfg(not(feature = "async"))]
    fn dispatch(&self, mut exchange: Exchange) -> Result<Exchange> {
        self.route.run(&mut exchange)?;
        self.push_out(exchange.clone());
        Ok(exchange)
    }
    #[cfg(feature = "async")]
    async fn dispatch_async(&self, mut exchange: Exchange) -> Result<Exchange> {
        self.route.run(&mut exchange).await?;
        self.push_out_async(exchange.clone()).await;
        Ok(exchange)
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
            panic!("try_receive should not be called in async mode; use try_receive_async instead");
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
        let _ = self.dispatch(exchange)?; // processed added to out_queue
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
            panic!("receive_by_correlation should not be called in async mode; use receive_by_correlation_async instead");
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

/// Type alias for the current default channel implementation. This maintains a simple
/// migration path; user code can depend on `DefaultChannel` today and later switch.
pub type DefaultChannel = InMemoryChannel;
pub type ChannelRef = std::sync::Arc<dyn Channel>; // ergonomic alias for trait object
