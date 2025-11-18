//! DirectChannel: immediate handoff (no internal buffering).
//!
//! Features:
//! * Zero-copy handoff (only `Exchange` clone per subscriber for isolation)
//! * Ordered subscriber invocation (registration order)
//! * Short-circuit on first error (remaining subscribers skipped)
//! * Sync and async send variants via feature flag
//! * Minimal API surface (construction + subscription + send)
//!
//! Not Provided:
//! * Internal queueing, polling APIs (use `QueueChannel` for that)
//! * Built-in correlation management (headers preserved but not generated)

use crate::channel::SubscribableChannel;
use crate::{error::Result, Channel, Exchange};
use std::any::Any;
use std::fmt::Debug;
use std::sync::Mutex;
use tracing::trace;

/// Direct, synchronous handoff channel.
///
/// No internal buffering: send immediately dispatches the exchange to all
/// subscribers in registration order. Each subscriber receives a cloned
/// Exchange.
pub struct DirectChannel {
    // Stable identifier for this channel instance.
    id: String,
    // Registered subscriber callbacks invoked on each send.
    subscribers: Mutex<Vec<Box<dyn Fn(Exchange) -> Result<()> + Send + Sync>>>,
}

impl Debug for DirectChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self.subscribers.lock().map(|v| v.len()).unwrap_or(0);
        f.debug_struct("DirectChannel")
            .field("id", &self.id)
            .field("subscriber_count", &count)
            .finish()
    }
}

impl DirectChannel {
    // ========================================================================
    // Constructors
    // ========================================================================
    /// Create a new direct channel with an auto-generated UUID-based id (`direct:<uuid>`).
    pub fn with_random_id() -> Self {
        Self {
            id: format!("direct:{}", uuid::Uuid::new_v4()),
            subscribers: Mutex::new(Vec::new()),
        }
    }
    /// Create a new direct channel with an explicit id.
    pub fn with_id<S: Into<String>>(id: S) -> Self {
        Self {
            id: id.into(),
            subscribers: Mutex::new(Vec::new()),
        }
    }
    /// Convenience: construct with optional id and an initial iterator of subscribers.
    /// Each provided closure is subscribed in iteration order.
    pub fn with_subscribers<I, F>(id: Option<&str>, subs: I) -> Self
    where
        I: IntoIterator<Item = F>,
        F: Fn(Exchange) -> Result<()> + Send + Sync + 'static,
    {
        let ch = match id {
            Some(i) => DirectChannel::with_id(i),
            None => DirectChannel::with_random_id(),
        };
        for f in subs {
            ch.subscribe(f);
        }
        ch
    }
    // ========================================================================
    // Public accessors
    // ========================================================================
    /// Returns the channel identifier.
    pub fn id(&self) -> &str {
        &self.id
    }
    // ========================================================================
    // Internal helpers
    // ========================================================================
    /// Dispatch an exchange to all subscribers. Clones the exchange once per subscriber.
    /// Short-circuits on first error.
    fn dispatch(&self, exchange: Exchange, is_async: bool) -> Result<()> {
        let subs = self.subscribers.lock().unwrap();
        trace!(target: "allora::channel", channel_id = %self.id, async = is_async, subscribers = subs.len(), in_body = ?exchange.in_msg.body_text(), "direct dispatch start");
        for (idx, sub) in subs.iter().enumerate() {
            let cloned = exchange.clone();
            trace!(target: "allora::channel", channel_id = %self.id, subscriber_index = idx, async = is_async, in_body = ?cloned.in_msg.body_text(), "direct dispatch to subscriber");
            sub(cloned)?;
        }
        Ok(())
    }
    // ========================================================================
    // Subscription management
    // ========================================================================
    /// Register a subscriber. Returns total subscriber count after insertion.
    pub fn subscribe<F>(&self, f: F) -> usize
    where
        F: Fn(Exchange) -> Result<()> + Send + Sync + 'static,
    {
        let mut subs = self.subscribers.lock().unwrap();
        subs.push(Box::new(f));
        subs.len()
    }
}

#[cfg_attr(feature = "async", async_trait::async_trait)]
impl Channel for DirectChannel {
    // --------------------------------------------------------------------
    // Identity & metadata
    // --------------------------------------------------------------------
    fn id(&self) -> &str {
        &self.id
    }
    // --------------------------------------------------------------------
    // Send operations
    // --------------------------------------------------------------------
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

impl SubscribableChannel for DirectChannel {
    fn subscribe<F>(&self, f: F) -> usize
    where
        F: Fn(Exchange) -> Result<()> + Send + Sync + 'static,
    {
        self.subscribe(f) // delegate to inherent method
    }
}
