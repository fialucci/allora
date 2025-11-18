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

use super::log::{log_dequeued, log_empty, log_receive, log_send_enqueued};
use super::{Channel, CorrelationSupport, PollableChannel};
use crate::error::Result;
use crate::Exchange;
use std::any::Any;
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::sync::Mutex;
use tracing::trace;

type InnerQueue = VecDeque<Exchange>;

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

    pub(crate) async fn push(&self, ex: Exchange) {
        let mut g = self.out_queue.lock().await;
        g.push_back(ex);
    }

    // Reusable enqueue helpers (reduce duplication in trait impls)
    async fn enqueue(&self, exchange: Exchange, corr_id: Option<&str>) -> Result<()> {
        log_send_enqueued(self.id(), &exchange, true, corr_id);
        self.push(exchange).await;
        Ok(())
    }
}

// ============================================================================
// Trait implementations
// ============================================================================

// Channel trait - core send/receive interface
#[async_trait::async_trait]
impl Channel for QueueChannel {
    // --------------------------------------------------------------------
    // Identity & metadata
    // --------------------------------------------------------------------
    fn id(&self) -> &str {
        &self.id
    }
    // --------------------------------------------------------------------
    // Send operations
    // --------------------------------------------------------------------
    async fn send(&self, exchange: Exchange) -> Result<()> {
        self.enqueue(exchange, None).await
    }

    fn kind(&self) -> &'static str {
        "queue"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// PollableChannel trait - queue/dequeue operations
#[async_trait::async_trait]
impl PollableChannel for QueueChannel {
    // --------------------------------------------------------------------
    // Receive
    // --------------------------------------------------------------------
    async fn try_receive(&self) -> Option<Exchange> {
        let mut g = self.out_queue.lock().await;
        if g.is_empty() {
            log_empty(self.id(), "try_receive", true, Some(g.len()), None);
            return None;
        }
        let exchange = g.pop_front().unwrap();
        log_dequeued(
            self.id(),
            "try_receive",
            true,
            &exchange,
            Some(g.len()),
            None,
        );
        Some(exchange)
    }
}

// CorrelationSupport trait - request/reply correlation patterns
#[async_trait::async_trait]
impl CorrelationSupport for QueueChannel {
    // --------------------------------------------------------------------
    // Correlated send
    // --------------------------------------------------------------------
    async fn send_with_correlation(&self, mut exchange: Exchange) -> Result<String> {
        let id_val = self.ensure_correlation(&mut exchange);
        self.enqueue(exchange, Some(&id_val)).await?;
        Ok(id_val)
    }

    // --------------------------------------------------------------------
    // Correlated receive
    // --------------------------------------------------------------------
    async fn receive_by_correlation(&self, corr_id: &str) -> Option<Exchange> {
        let cid = corr_id.to_string();
        let mut g = self.out_queue.lock().await;
        if let Some(pos) = g
            .iter()
            .position(|e| e.in_msg.header("corr_id") == Some(&cid))
        {
            let ex = g.remove(pos).unwrap();
            log_receive(
                self.id(),
                "receive_by_correlation",
                "dequeued",
                true,
                Some(&ex),
                Some(g.len()),
                Some(&cid),
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
            true,
            None,
            Some(g.len()),
            Some(&cid),
            None,
            None,
            None,
        );
        None
    }
}
