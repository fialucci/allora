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
//! use allora::{endpoint::InMemoryEndpoint, Message, Exchange};
//! let ep = InMemoryEndpoint::new();
//! #[cfg(feature="async")]
//! {
//!     use allora::endpoint::Endpoint;
//!     let rt = tokio::runtime::Runtime::new().unwrap();
//!     rt.block_on(async {
//!         ep.send_async(Exchange::new(Message::from_text("A"))).await.unwrap();
//!         assert_eq!(ep.try_receive_async().await.unwrap().in_msg.body_text(), Some("A"));
//!     });
//! }
//! #[cfg(not(feature="async"))]
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

use crate::{error::Result, Exchange};
use std::collections::VecDeque;
use std::sync::Arc;
#[cfg(not(feature = "async"))]
use std::sync::Mutex;
#[cfg(feature = "async")]
use tokio::sync::Mutex;

/// A trait representing a message endpoint for sending and receiving [`Exchange`] objects.
pub trait Endpoint: Send + Sync {
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

/// An in-memory FIFO endpoint for quick testing.
///
/// Internally uses a `VecDeque` protected by a mutex (`std::sync::Mutex` or
/// `tokio::sync::Mutex` for async mode). No backpressure, size limits, or correlation
/// semantics are provided—this is intentionally minimal.
#[derive(Default)]
pub struct InMemoryEndpoint {
    inner: Arc<Mutex<VecDeque<Exchange>>>,
}

impl InMemoryEndpoint {
    /// Create a new empty endpoint.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
}

impl Endpoint for InMemoryEndpoint {
    fn send(&self, _exchange: Exchange) -> Result<()> {
        #[cfg(not(feature = "async"))]
        {
            let mut guard = self.inner.lock().unwrap();
            guard.push_back(_exchange);
            Ok(())
        }
        #[cfg(feature = "async")]
        // This function should only be called from async context when async feature is enabled
        {
            panic!("send should not be called in async mode; use send_async instead");
        }
    }
    #[cfg(feature = "async")]
    async fn send_async(&self, exchange: Exchange) -> Result<()> {
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
