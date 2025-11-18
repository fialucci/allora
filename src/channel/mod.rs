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
//! ## DirectChannel (Sync)
//! ```no_run
//! # #[cfg(not(feature = "async"))]
//! # {
//! # use allora::{Exchange, Message};
//! # use allora::DirectChannel;
//! let dc = DirectChannel::with_random_id();
//! dc.subscribe(|ex| { assert_eq!(ex.in_msg.body_text(), Some("ping")); Ok(()) });
//! dc.send(Exchange::new(Message::from_text("ping"))).unwrap();
//! # }
//! ```
//!
//! ## DirectChannel (Async)
//! ```no_run
//! # #[cfg(feature = "async")]
//! # {
//! # use allora::{Exchange, Message, Channel};
//! # use allora::DirectChannel;
//! let dc = DirectChannel::with_id("direct-demo");
//! dc.subscribe(|ex| { assert_eq!(ex.in_msg.body_text(), Some("pong")); Ok(()) });
//! let rt = tokio::runtime::Runtime::new().unwrap();
//! rt.block_on(async { dc.send_async(Exchange::new(Message::from_text("pong"))).await.unwrap(); });
//! # }
//! ```
//!
//! ## QueueChannel (Sync)
//! ```no_run
//! # #[cfg(not(feature = "async"))]
//! # {
//! # use allora::{Exchange, Message};
//! # use allora::QueueChannel;
//! # use allora::channel::PollableChannel;
//! let ch = QueueChannel::with_id("demo");
//! ch.send(Exchange::new(Message::from_text("ping"))).unwrap();
//! let ex = ch.try_receive().unwrap();
//! assert_eq!(ex.in_msg.body_text(), Some("ping"));
//! # }
//! ```
//!
//! ## QueueChannel (Async)
//! ```no_run
//! # #[cfg(feature = "async")]
//! # {
//! # use allora::{Exchange, Message, Channel};
//! # use allora::QueueChannel;
//! # use allora::channel::PollableChannel;
//! let ch = QueueChannel::with_id("async-demo");
//! let rt = tokio::runtime::Runtime::new().unwrap();
//! rt.block_on(async {
//!     ch.send_async(Exchange::new(Message::from_text("ping"))).await.unwrap();
//!     let received = ch.try_receive_async().await.unwrap();
//!     assert_eq!(received.in_msg.body_text(), Some("ping"));
//! });
//! # }
//! ```
//!
//! ## Correlation (Sync)
//! ```no_run
//! # use allora::{Exchange, Message};
//! # use allora::QueueChannel;
//! # use allora::channel::CorrelationSupport;
//! let ch = QueueChannel::with_random_id();
//! let cid = ch.send_with_correlation(Exchange::new(Message::from_text("req"))).unwrap();
//! let ex = ch.receive_by_correlation(&cid).unwrap();
//! assert_eq!(ex.in_msg.body_text(), Some("req"));
//! ```
//!
//! ## Correlation (Async)
//! ```no_run
//! # #[cfg(feature = "async")]
//! # {
//! # use allora::{Exchange, Message};
//! # use allora::QueueChannel;
//! # use allora::channel::CorrelationSupport;
//! let ch = QueueChannel::with_random_id();
//! let cid = ch.send_with_correlation(Exchange::new(Message::from_text("req"))).unwrap();
//! let ex = tokio::runtime::Runtime::new().unwrap().block_on(async { ch.receive_by_correlation_async(&cid).await.unwrap() });
//! assert_eq!(ex.in_msg.body_text(), Some("req"));
//! # }
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
#[cfg(feature = "async")]
use async_trait::async_trait;
use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// Core Traits
// ============================================================================

/// Core channel trait providing send capabilities and metadata.
///
/// Implementors provide either sync or async send depending on the `async` feature.
#[cfg_attr(feature = "async", async_trait)]
pub trait Channel: Send + Sync + Debug {
    fn id(&self) -> &str;
    #[cfg(not(feature = "async"))]
    fn send(&self, exchange: Exchange) -> Result<()>;
    #[cfg(feature = "async")]
    async fn send_async(&self, exchange: Exchange) -> Result<()>;
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
pub trait PollableChannel: Channel {
    fn try_receive(&self) -> Option<Exchange>;
    #[cfg(feature = "async")]
    fn try_receive_async(&self) -> impl std::future::Future<Output = Option<Exchange>> + Send;
    fn receive_blocking(&self, timeout: Option<Duration>) -> Option<Exchange>;
}
/// Correlation lookup extension (QueueChannel only).
pub trait CorrelationSupport: Channel {
    fn send_with_correlation(&self, exchange: Exchange) -> Result<String>;
    fn receive_by_correlation(&self, corr_id: &str) -> Option<Exchange>;
    #[cfg(feature = "async")]
    fn receive_by_correlation_async(
        &self,
        corr_id: &str,
    ) -> impl std::future::Future<Output = Option<Exchange>> + Send;
    fn await_correlation(&self, corr_id: &str, timeout: Option<Duration>) -> Option<Exchange>;
}

// ============================================================================
// Type aliases
// ============================================================================

/// Type alias for a trait object reference to any Channel implementation.
pub type ChannelRef = Arc<dyn Channel>;

// ============================================================================
// Deprecated traits (compatibility)
// ============================================================================

/// Deprecated: Use `PollableChannel` instead.
///
/// This trait is maintained for backward compatibility but will be removed in a future version.
#[deprecated(since = "0.1.0", note = "Use PollableChannel instead")]
pub trait OutboundQueue: Send + Sync + Debug {
    fn try_receive(&self) -> Option<Exchange>;
    #[cfg(feature = "async")]
    fn try_receive_async(&self) -> impl std::future::Future<Output = Option<Exchange>> + Send;
    fn receive_blocking(&self, timeout: Option<std::time::Duration>) -> Option<Exchange>;
}

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
