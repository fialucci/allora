//! Aggregator pattern: collects messages sharing a correlation header until a configured
//! completion size is reached, then emits a concatenated outbound text message.
//!
//! # Behavior
//! * Messages are grouped by the value of a specified correlation header (e.g. "corr").
//! * When the number of messages in a group reaches `completion_size`, all messages are removed
//!   from the internal store and concatenated (text-only) into `Exchange.out_msg`.
//! * Only groups where every message has a UTF-8 text payload (`Payload::Text`) are aggregated.
//!   Mixed payload kinds (bytes, JSON, empty) or non-text groups are ignored (no outbound message).
//! * Once a group completes it is removed, allowing subsequent batches with the same key.
//!
//! # Limitations
//! * Non-text payloads are ignored for aggregation; bytes-only or JSON-only groups will not produce
//!   an `out_msg`. Future enhancements could support bytes joining or JSON array creation.
//! * Concurrency relies on a single `Mutex<HashMap<..>>`; high contention scenarios may warrant
//!   a sharded map or lock-free structure.
//! * No time-based or predicate-based completion—only size threshold is supported.
//!
//! # Example
//! ```
//! use allora::{patterns::aggregator::Aggregator, route::Route, Message, Exchange};
//! // Build a route with an aggregator expecting 2 messages per group
//! let route = Route::new().add(Aggregator::new("corr", 2)).build();
//! // First message (group not complete yet)
//! let mut ex1 = Exchange::new(Message::from_text("A"));
//! ex1.in_msg.set_header("corr", "grp");
//! #[cfg(feature = "async")]
//! tokio::runtime::Runtime::new().unwrap().block_on(async { route.run(&mut ex1).await.unwrap(); });
//! #[cfg(not(feature = "async"))]
//! route.run(&mut ex1).unwrap();
//! assert!(ex1.out_msg.is_none());
//! // Second message completes group => out_msg becomes "AB"
//! let mut ex2 = Exchange::new(Message::from_text("B"));
//! ex2.in_msg.set_header("corr", "grp");
//! #[cfg(feature = "async")]
//! tokio::runtime::Runtime::new().unwrap().block_on(async { route.run(&mut ex2).await.unwrap(); });
//! #[cfg(not(feature = "async"))]
//! route.run(&mut ex2).unwrap();
//! assert_eq!(ex2.out_msg.unwrap().body_text(), Some("AB"));
//! ```

use crate::{error::Result, message::Message, processor::SyncProcessor, Exchange};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct Aggregator {
    store: Arc<Mutex<HashMap<String, Vec<Message>>>>,
    correlation_header: String,
    completion_size: usize,
}

impl Aggregator {
    /// Create a new `Aggregator`.
    ///
    /// * `correlation_header` – header key used to group messages (e.g. "corr" or "correlation_id").
    /// * `completion_size` – number of messages required before aggregation triggers.
    pub fn new<H: Into<String>>(correlation_header: H, completion_size: usize) -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
            correlation_header: correlation_header.into(),
            completion_size,
        }
    }

    /// Clear all stored partial groups. Intended for test isolation; not usually needed in production.
    pub fn clear_store(&self) {
        let mut guard = self.store.lock().unwrap();
        guard.clear();
    }

    fn add(&self, exchange: &Exchange) -> Option<Vec<Message>> {
        let key_opt = exchange.in_msg.header(&self.correlation_header);
        let key = match key_opt {
            Some(k) => k,
            None => return None,
        };
        let mut guard = self.store.lock().unwrap();
        let bucket = guard.entry(key.to_string()).or_default();
        bucket.push(exchange.in_msg.clone());
        if bucket.len() >= self.completion_size {
            let completed = bucket.clone();
            guard.remove(key);
            Some(completed)
        } else {
            None
        }
    }
}

impl SyncProcessor for Aggregator {
    /// Process the incoming exchange:
    /// * Attempt to add the inbound message to its correlation bucket.
    /// * If bucket size reaches `completion_size` and all payloads are text, set `out_msg` to concatenated text.
    fn process_sync(&self, exchange: &mut Exchange) -> Result<()> {
        if let Some(completed) = self.add(exchange) {
            let all_text = completed.iter().all(|m| m.body_text().is_some());
            if all_text {
                let concat = completed
                    .iter()
                    .map(|m| m.body_text().unwrap())
                    .collect::<String>();
                exchange.out_msg = Some(Message::from_text(concat));
            }
        }
        Ok(())
    }
}
