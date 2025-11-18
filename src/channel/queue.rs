//! QueueChannel: buffered FIFO channel implementation.
//!
//! Provides queued message storage with optional correlation id utilities.
//! In sync mode, operations are protected by a std::sync::Mutex over a VecDeque; in async
//! mode, a Tokio Mutex guards a Vec allowing awaitable dequeue operations.
//!
//! Features:
//! * FIFO ordering
//! * Non-blocking try_receive (sync + async variants)
//! * Optional blocking receiver with timeout (sync mode)
//! * Correlation ID generation & lookup helpers via `CorrelationSupport`
//! * `async` feature for Tokio-based async support

use super::log::{log_dequeued, log_empty, log_phase, log_receive, log_send_enqueued};
use super::{Channel, CorrelationSupport, PollableChannel};
use crate::error::{Error, Result};
use crate::Exchange;
use std::any::Any;
use std::fmt::Debug;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};
#[cfg(not(feature = "async"))]
use std::{collections::VecDeque, sync::Mutex};
#[cfg(feature = "async")]
use tokio::sync::Mutex;
use tracing::trace;

#[cfg(not(feature = "async"))]
type InnerQueue = VecDeque<Exchange>;
#[cfg(feature = "async")]
type InnerQueue = Vec<Exchange>;

#[derive(Clone, Debug)]
pub struct QueueChannel {
    id: String,
    out_queue: Arc<Mutex<InnerQueue>>,
    corr_seq: Arc<AtomicU64>,
}

impl QueueChannel {
    // ========================================================================
    // Constructors (associated functions)
    // ========================================================================

    /// Create a new queue channel with an explicit identifier.
    pub fn with_id<S: Into<String>>(id: S) -> Self {
        Self {
            id: id.into(),
            out_queue: Arc::new(Mutex::new(InnerQueue::default())),
            corr_seq: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Create a new queue channel with a randomly generated UUID-based identifier.
    pub fn with_random_id() -> Self {
        Self::with_id(format!("queue:{}", uuid::Uuid::new_v4()))
    }

    // ========================================================================
    // Public methods
    // ========================================================================

    /// Returns the channel identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    // ========================================================================
    // Private helper methods
    // ========================================================================

    /// Generate the next correlation ID for this channel.
    pub(crate) fn next_corr_id(&self) -> String {
        let id = format!("c{}", self.corr_seq.fetch_add(1, Ordering::Relaxed));
        trace!(target: "allora::channel",channel_id = %self.id, corr_id = %id, "generated correlation id");
        id
    }

    /// Ensure the exchange has a correlation ID, reusing existing or generating new.
    pub(crate) fn ensure_correlation(&self, exchange: &mut Exchange) -> String {
        if let Some(id) = exchange.in_msg.header("corr_id") {
            let id_str = id.to_string();
            trace!(target: "allora::channel",channel_id = %self.id, corr_id = %id_str, "reusing existing corr_id");
            if exchange.in_msg.header("correlation_id").is_none() {
                exchange.in_msg.set_header("correlation_id", &id_str);
            }
            id_str
        } else {
            let id = self.next_corr_id();
            trace!(channel_id = %self.id, corr_id = %id, "assigned new corr_id");
            exchange.in_msg.set_header("corr_id", &id);
            if exchange.in_msg.header("correlation_id").is_none() {
                exchange.in_msg.set_header("correlation_id", &id);
            }
            id
        }
    }

    #[cfg(not(feature = "async"))]
    pub(crate) fn push(&self, ex: Exchange) {
        self.out_queue.lock().unwrap().push_back(ex);
    }
    #[cfg(feature = "async")]
    pub(crate) async fn push_async(&self, ex: Exchange) {
        let mut g = self.out_queue.lock().await;
        g.push(ex);
    }

    // Reusable enqueue helpers (reduce duplication in trait impls)
    #[cfg(not(feature = "async"))]
    fn enqueue_sync(&self, exchange: Exchange, corr_id: Option<&str>) -> Result<()> {
        log_send_enqueued(self.id(), &exchange, false, corr_id);
        self.push(exchange);
        Ok(())
    }
    #[cfg(feature = "async")]
    async fn enqueue_async(&self, exchange: Exchange, corr_id: Option<&str>) -> Result<()> {
        log_send_enqueued(self.id(), &exchange, true, corr_id);
        self.push_async(exchange).await;
        Ok(())
    }
}

// ============================================================================
// Trait implementations
// ============================================================================

// Channel trait - core send/receive interface
#[cfg_attr(feature = "async", async_trait::async_trait)]
impl Channel for QueueChannel {
    // --------------------------------------------------------------------
    // Identity & metadata
    // --------------------------------------------------------------------
    fn id(&self) -> &str {
        &self.id
    }
    // --------------------------------------------------------------------
    // Send operations (feature-gated)
    // --------------------------------------------------------------------
    #[cfg(not(feature = "async"))]
    fn send(&self, exchange: Exchange) -> Result<()> {
        self.enqueue_sync(exchange, None)
    }
    #[cfg(feature = "async")]
    async fn send_async(&self, exchange: Exchange) -> Result<()> {
        self.enqueue_async(exchange, None).await
    }

    fn kind(&self) -> &'static str {
        "queue"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// PollableChannel trait - queue/dequeue operations
impl PollableChannel for QueueChannel {
    // --------------------------------------------------------------------
    // Non-blocking receive (sync)
    // --------------------------------------------------------------------
    fn try_receive(&self) -> Option<Exchange> {
        #[cfg(not(feature = "async"))]
        {
            let exchange = self.out_queue.lock().unwrap().pop_front();
            match &exchange {
                Some(ex) => log_dequeued(self.id(), "try_receive", false, ex, None, None),
                None => log_empty(self.id(), "try_receive", false, None, None),
            }
            exchange
        }
        #[cfg(feature = "async")]
        {
            panic!("use try_receive_async in async mode");
        }
    }

    // --------------------------------------------------------------------
    // Non-blocking receive (async)
    // --------------------------------------------------------------------
    #[cfg(feature = "async")]
    fn try_receive_async(&self) -> impl std::future::Future<Output = Option<Exchange>> + Send {
        let this = self.clone();
        async move {
            let mut g = this.out_queue.lock().await;
            if g.is_empty() {
                log_empty(this.id(), "try_receive_async", true, Some(g.len()), None);
                return None;
            }
            let exchange = g.remove(0);
            log_dequeued(
                this.id(),
                "try_receive_async",
                true,
                &exchange,
                Some(g.len()),
                None,
            );
            Some(exchange)
        }
    }

    // --------------------------------------------------------------------
    // Blocking receive (sync only)
    // --------------------------------------------------------------------
    fn receive_blocking(&self, timeout: Option<Duration>) -> Option<Exchange> {
        log_phase(
            self.id(),
            "receive_blocking",
            "start",
            false,
            None,
            None,
            timeout,
        );
        let start = Instant::now();
        let mut attempts = 0u32;
        loop {
            if let Some(ex) = self.try_receive() {
                log_dequeued(self.id(), "receive_blocking", false, &ex, None, None);
                return Some(ex);
            }
            attempts += 1;
            if let Some(t) = timeout {
                if start.elapsed() >= t {
                    log_phase(
                        self.id(),
                        "receive_blocking",
                        "timeout",
                        false,
                        Some(attempts),
                        Some(&start),
                        Some(t),
                    );
                    return None;
                }
            }
            sleep(Duration::from_millis(5));
        }
    }
}

// CorrelationSupport trait - request/reply correlation patterns
impl CorrelationSupport for QueueChannel {
    // --------------------------------------------------------------------
    // Correlated send
    // --------------------------------------------------------------------
    fn send_with_correlation(&self, mut exchange: Exchange) -> Result<String> {
        let id_val = self.ensure_correlation(&mut exchange);
        #[cfg(not(feature = "async"))]
        {
            self.enqueue_sync(exchange, Some(&id_val))?;
        }
        #[cfg(feature = "async")]
        {
            let rt = tokio::runtime::Runtime::new().map_err(|e| Error::other(e.to_string()))?;
            rt.block_on(self.enqueue_async(exchange, Some(&id_val)))?;
        }
        Ok(id_val)
    }

    // --------------------------------------------------------------------
    // Correlated receive (sync)
    // --------------------------------------------------------------------
    fn receive_by_correlation(&self, corr_id: &str) -> Option<Exchange> {
        #[cfg(not(feature = "async"))]
        {
            let mut g = self.out_queue.lock().unwrap();
            if let Some(pos) = g
                .iter()
                .position(|e| e.in_msg.header("corr_id") == Some(corr_id))
            {
                let ex = g.remove(pos);
                log_receive(
                    self.id(),
                    "receive_by_correlation",
                    "dequeued",
                    false,
                    Some(&ex),
                    Some(g.len()),
                    Some(corr_id),
                    None,
                    None,
                    None,
                );
                return Some(ex);
            }
            log_receive(
                self.id(),
                "receive_by_correlation",
                "empty",
                false,
                None,
                Some(g.len()),
                Some(corr_id),
                None,
                None,
                None,
            );
            None
        }
        #[cfg(feature = "async")]
        {
            panic!("use receive_by_correlation_async for corr_id={}", corr_id);
        }
    }

    // --------------------------------------------------------------------
    // Correlated receive (async)
    // --------------------------------------------------------------------
    #[cfg(feature = "async")]
    fn receive_by_correlation_async(
        &self,
        corr_id: &str,
    ) -> impl std::future::Future<Output = Option<Exchange>> + Send {
        let corr = corr_id.to_string();
        let this = self.clone();
        async move {
            let mut g = this.out_queue.lock().await;
            if let Some(pos) = g
                .iter()
                .position(|e| e.in_msg.header("corr_id") == Some(&corr))
            {
                let ex = g.remove(pos);
                log_receive(
                    this.id(),
                    "receive_by_correlation_async",
                    "dequeued",
                    true,
                    Some(&ex),
                    Some(g.len()),
                    Some(&corr),
                    None,
                    None,
                    None,
                );
                return Some(ex);
            }
            log_receive(
                this.id(),
                "receive_by_correlation_async",
                "empty",
                true,
                None,
                Some(g.len()),
                Some(&corr),
                None,
                None,
                None,
            );
            None
        }
    }

    // --------------------------------------------------------------------
    // Await correlation (polling with timeout)
    // --------------------------------------------------------------------
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
