//! Splitter pattern: derives multiple logical messages from a single inbound `Exchange`.
//!
//! Implements the Enterprise Integration Pattern (EIP) "Splitter". A splitter breaks a
//! composite or aggregate message (e.g. a delimited string, a JSON array, a batch) into
//! multiple individual messages. In this minimal implementation the provided closure
//! returns a `Vec<Message>` representing the split parts.
//!
//! # Current Behavior & Limitation
//! The splitter invokes the user-supplied closure and, if the returned vector is non-empty,
//! stores ONLY the first resulting message in `exchange.out_msg`. The rest are discarded.
//! This is intentionally minimal for now. Future versions may:
//! * Emit all parts downstream via a `Vec<Message>` property.
//! * Produce cloned `Exchange`s for each part.
//! * Integrate with a downstream `RecipientList` or `Aggregator` automatically.
//!
//! # Use Cases
//! * Taking a CSV line and extracting the first column for downstream processing.
//! * Extracting primary record from a batch for quick validation.
//! * Demonstration / scaffolding before implementing full fan-out semantics.
//!
//! # Example (tokenizing text payload)
//! ```
//! use allora::{patterns::splitter::Splitter, route::Route, Message, Exchange};
//! use allora::message::Payload;
//! let splitter = Splitter::new(|ex: &Exchange| {
//!     ex.in_msg.body_text()
//!         .map(|t| t.split_whitespace().map(|w| Message::from_text(w)).collect())
//!         .unwrap_or_else(Vec::new)
//! });
//! let route = Route::new().add(splitter).build();
//! let mut ex = Exchange::new(Message::from_text("one two three"));
//! #[cfg(feature="async")] tokio::runtime::Runtime::new().unwrap().block_on(async { route.run(&mut ex).await.unwrap(); });
//! #[cfg(not(feature="async"))] route.run(&mut ex).unwrap();
//! assert_eq!(ex.out_msg.unwrap().body_text(), Some("one"));
//! ```
//!
//! # Edge Cases
//! * Empty returned vector => `out_msg` remains unchanged (None if unset).
//! * Mixed payload types allowed; only first message used when non-empty.
//! * Closure should be pure / side-effect free aside from constructing messages.
//!
//! # Testing Strategies
//! * Verify first-element extraction from multiple tokens.
//! * Ensure no `out_msg` when closure returns empty slice.
//! * Provide heterogeneous messages and confirm only first selected.

use crate::{error::Result, message::Message, processor::SyncProcessor, Exchange};
use std::fmt::{Debug, Formatter, Result as FmtResult};

pub struct Splitter<F>
where
    F: Fn(&Exchange) -> Vec<Message> + Send + Sync + 'static,
{
    func: F,
}

impl<F> Debug for Splitter<F>
where
    F: Fn(&Exchange) -> Vec<Message> + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult { f.write_str("Splitter{func=*closure*}") }
}

impl<F> Splitter<F>
where
    F: Fn(&Exchange) -> Vec<Message> + Send + Sync + 'static,
{
    pub fn new(func: F) -> Self { Self { func } }
}

impl<F> SyncProcessor for Splitter<F>
where
    F: Fn(&Exchange) -> Vec<Message> + Send + Sync + 'static,
{
    fn process_sync(&self, exchange: &mut Exchange) -> Result<()> {
        let messages = (self.func)(exchange);
        if messages.is_empty() { return Ok(()); }
        // store first message as out_msg for demonstration
        exchange.out_msg = Some(messages[0].clone());
        Ok(())
    }
}
