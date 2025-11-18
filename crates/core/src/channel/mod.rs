//! Channel module – lightweight in-process message pipes.
//!
//! Provides abstractions for enqueuing and handing off `Exchange` instances between processing
//! stages. Channels do not transform messages; they only move them. Transformation, routing,
//! and filtering happen before (on the way in) or after (on receipt) via other crate components.
//!
//! # Implementations
//! * [`DirectChannel`] – immediate fan‑out to registered subscribers (no buffering).
//! * [`QueueChannel`] – FIFO buffered channel supporting dequeue & correlation lookup.
//!
//! # Extension Traits
//! * [`SubscribableChannel`] – register subscriber callbacks (DirectChannel).
//! * [`PollableChannel`] – dequeue / blocking receive operations (QueueChannel).
//! * [`CorrelationSupport`] – correlation id helpers (QueueChannel only).
//!
//! # Construction
//! Channels expose explicit and random id constructors:
//! * `DirectChannel::with_id("id")`, `DirectChannel::with_random_id()`
//! * `QueueChannel::with_id("id")`, `QueueChannel::with_random_id()`
//! Or use helpers: [`new_queue()`], [`new_queue_with_id("id")`].
//!
//! # Examples
//!
//! ## DirectChannel
//! ```rust
//! use allora_core::{channel::DirectChannel, Channel, Exchange, Message};
//! let dc = DirectChannel::with_random_id();
//! dc.subscribe(|ex| { assert_eq!(ex.in_msg.body_text(), Some("ping")); Ok(()) });
//! let rt = tokio::runtime::Runtime::new().unwrap();
//! rt.block_on(async { dc.send(Exchange::new(Message::from_text("ping"))).await.unwrap(); });
//! ```
//!
//! ## QueueChannel
//! ```rust
//! use allora_core::{channel::{QueueChannel, PollableChannel, Channel}, Exchange, Message};
//! let ch = QueueChannel::with_id("demo");
//! let rt = tokio::runtime::Runtime::new().unwrap();
//! rt.block_on(async {
//!     ch.send(Exchange::new(Message::from_text("ping"))).await.unwrap();
//!     let ex = ch.try_receive().await.unwrap();
//!     assert_eq!(ex.in_msg.body_text(), Some("ping"));
//! });
//! ```
//!
//! ## Correlation
//! ```rust
//! use allora_core::{channel::{QueueChannel, CorrelationSupport}, Exchange, Message};
//! let ch = QueueChannel::with_random_id();
//! let rt = tokio::runtime::Runtime::new().unwrap();
//! rt.block_on(async {
//!     let cid = ch.send_with_correlation(Exchange::new(Message::from_text("req"))).await.unwrap();
//!     let ex_opt = ch.receive_by_correlation(&cid).await;
//!     let ex = ex_opt.unwrap();
//!     assert_eq!(ex.in_msg.body_text(), Some("req"));
//! });
//! ```
//!
//! ## Notes
//! * DirectChannel does not implement `PollableChannel` or `CorrelationSupport`.
//! * QueueChannel guarantees FIFO order for normal dequeue operations.
//! * Correlation lookup removes the matched exchange from the internal queue.

// ============================================================================
// Module declarations
// ============================================================================
mod direct;
mod log; // internal logging utilities
mod queue;

// ============================================================================
// Public exports
// ============================================================================
pub use direct::DirectChannel;
pub use queue::QueueChannel;

// ============================================================================
// Imports
// ============================================================================
use crate::{error::Result, Exchange};
use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;

// ============================================================================
// Core Traits
// ============================================================================

/// Core channel trait providing send capabilities and metadata.
///
/// Implementors provide either sync or async send depending on the `async` feature.
#[async_trait::async_trait]
pub trait Channel: Send + Sync + Debug {
    fn id(&self) -> &str;
    async fn send(&self, exchange: Exchange) -> Result<()>;
    fn kind(&self) -> &'static str {
        "unknown"
    }
    fn as_any(&self) -> &dyn Any;
}

// ============================================================================
// Extension Traits
// ============================================================================

/// Register-and-fanout extension trait (DirectChannel).
pub trait SubscribableChannel: Channel {
    fn subscribe<F>(&self, f: F) -> usize
    where
        F: Fn(Exchange) -> Result<()> + Send + Sync + 'static;
}
/// Dequeue extension (QueueChannel).
#[async_trait::async_trait]
pub trait PollableChannel: Channel {
    async fn try_receive(&self) -> Option<Exchange>;
}
/// Correlation lookup extension (QueueChannel only).
#[async_trait::async_trait]
pub trait CorrelationSupport: Channel {
    async fn send_with_correlation(&self, exchange: Exchange) -> Result<String>;
    async fn receive_by_correlation(&self, corr_id: &str) -> Option<Exchange>;
}

// ============================================================================
// Type aliases
// ============================================================================

/// Type alias for a trait object reference to any Channel implementation.
pub type ChannelRef = Arc<dyn Channel>;

// ============================================================================
// Convenience constructors
// ============================================================================
/// Create a new queue channel with auto-generated id (`queue:<uuid>`).
pub fn new_queue() -> ChannelRef {
    Arc::new(QueueChannel::with_random_id())
}
/// Create a new queue channel with explicit id.
pub fn new_queue_with_id(id: impl Into<String>) -> ChannelRef {
    Arc::new(QueueChannel::with_id(id))
}
