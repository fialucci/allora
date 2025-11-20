//! Content-Based Router pattern: route an `Exchange` to one of several processors based on a header value.
//!
//! This implements the classic Enterprise Integration Pattern (EIP) "Content-Based Router".
//! It examines a specific message header and selects a processor whose configured value matches.
//!
//! # Why It Exists
//! In message-driven integrations you often need to send different message types (or business events)
//! through distinct processing flows. Instead of embedding conditional logic inside processors or
//! scattering `if/else` routes, a dedicated router centralizes this decision, improving readability
//! and testability.
//!
//! # Why In `patterns` Folder
//! The `patterns` module groups canonical EIP building blocks (filter, splitter, aggregator, etc.).
//! `ContentBasedRouter` is one of those core patterns, so it lives alongside them for discoverability
//! and conceptual cohesion rather than in a generic utilities namespace.
//!
//! # Behavior
//! * Looks up a header (configured via `new(header_name)`).
//! * If the header matches a registered route value (added via `when(value, processor)`), the associated
//!   processor is invoked.
//! * If no route matches, returns a `Routing` error (`Error::Routing`).
//! * Exactly one processor is invoked (first matching key lookup).
//!
//! # Async vs Sync
//! The router itself is lightweight; it defers to the selected processor. Under the `async` feature
//! its `process` method is async and awaits the downstream processor; without it, synchronous dispatch occurs.
//!
//! # Example (basic usage)
//! ```rust
//! use allora_core::{patterns::content_router::ContentBasedRouter, processor::ClosureProcessor, route::Route, Exchange, Message};
//! let hi = ClosureProcessor::new(|exchange| { exchange.out_msg = Some(Message::from_text("hi route")); Ok(()) });
//! let bye = ClosureProcessor::new(|exchange| { exchange.out_msg = Some(Message::from_text("bye route")); Ok(()) });
//! let router = ContentBasedRouter::new("kind").when("hi", Box::new(hi)).when("bye", Box::new(bye));
//! let route = Route::new().add(router).build();
//! let mut exchange = Exchange::new(Message::from_text("payload"));
//! exchange.in_msg.set_header("kind", "hi");
//! let rt = tokio::runtime::Runtime::new().unwrap();
//! rt.block_on(async { route.run(&mut exchange).await.unwrap(); });
//! assert_eq!(exchange.out_msg.unwrap().body_text(), Some("hi route"));
//! ```
//!
//! # Error Handling
//! No match: `Error::Routing("no matching route")`.
//! Downstream processor errors propagate unchanged.
//!
//! # Testing Strategies
//! * Provide several `when` routes, assert correct one chosen.
//! * Assert error when header missing.
//! * Use processors that mutate headers to verify only one is executed.
//!
//! # Future Extensions
//! * Predicate-based routing (closures) instead of plain equality.
//! * Default / fallback processor.
//! * Support for wildcards / pattern matching.
//! * Metrics counters (per route hit counts).
use crate::{
    error::{Error, Result},
    processor::Processor,
    Exchange,
};
use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Debug)]
pub struct ContentBasedRouter {
    routes: HashMap<String, Box<dyn Processor>>,
    header: String,
}

impl ContentBasedRouter {
    /// Create a new [`ContentBasedRouter`] instance, specifying the header name to inspect for routing.
    /// Example: `ContentBasedRouter::new("x-my-header")`.
    ///
    /// # Panics
    /// Panics if the header name is empty.
    pub fn new<H: Into<String>>(header: H) -> Self {
        let h = header.into();
        assert!(!h.is_empty(), "header name must not be empty");
        Self {
            routes: HashMap::new(),
            header: h,
        }
    }

    /// Add a route to the router, specifying a header value and the processor to handle messages with
    /// that value.
    /// Example: `.when("value1", Box::new(my_processor))`.
    ///
    /// # Panics
    /// Panics if the value is empty.
    pub fn when<V: Into<String>>(mut self, value: V, proc: Box<dyn Processor>) -> Self {
        let v = value.into();
        assert!(!v.is_empty(), "route value must not be empty");
        self.routes.insert(v, proc);
        self
    }

    /// Internal method to select the appropriate processor based on the exchange's message header.
    /// Returns `None` if no matching processor is found.
    fn select(&self, exchange: &Exchange) -> Option<&Box<dyn Processor>> {
        exchange
            .in_msg
            .header(&self.header)
            .and_then(|val| self.routes.get(val))
    }
}

#[async_trait::async_trait]
impl Processor for ContentBasedRouter {
    /// Process the exchange by delegating to the selected processor based on the content-based routing.
    /// Returns an error if no matching route is found.
    async fn process(&self, exchange: &mut Exchange) -> Result<()> {
        if let Some(p) = self.select(exchange) {
            p.process(exchange).await
        } else {
            Err(Error::Routing("no matching route".into()))
        }
    }
}
