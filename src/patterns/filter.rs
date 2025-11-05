//! Filter pattern: conditionally allow an `Exchange` to continue through a Route based on a predicate.
//!
//! Implements the Enterprise Integration Pattern (EIP) "Message Filter". A filter evaluates
//! a boolean predicate against the current `Exchange`; if it returns `true` processing continues,
//! otherwise the route is short‑circuited with a `Routing` error.
//!
//! # Why Use a Filter
//! * Declarative separation of routing conditions from processing logic.
//! * Early rejection of messages not relevant to a downstream flow (performance & clarity).
//! * Composable with other patterns (place after a correlation initializer, before a splitter, etc.).
//!
//! # Behavior
//! * Predicate receives an immutable reference to the `Exchange`.
//! * On `true`, `process_sync` returns `Ok(())` and the route proceeds.
//! * On `false`, returns `Err(Error::Routing(..))` with a customizable message.
//! * No mutation is performed by the filter itself.
//!
//! # Async Feature
//! When the `async` feature is enabled, `Filter` still implements `SyncProcessor`; it is
//! automatically adapted to the async `Processor` trait (no internal `await` needed).
//!
//! # Examples
//! Basic usage (works in sync or async mode):
//! ```
//! use allora::{patterns::filter::Filter, route::Route, Message, Exchange};
//! let route = Route::new().add(Filter::new(|ex| ex.in_msg.body_text() == Some("KEEP"))).build();
//! let mut ex = Exchange::new(Message::from_text("KEEP"));
//! #[cfg(feature="async")] tokio::runtime::Runtime::new().unwrap().block_on(async { route.run(&mut ex).await.unwrap(); });
//! #[cfg(not(feature="async"))] route.run(&mut ex).unwrap();
//! ```
//!
//! Custom error message when predicate fails:
//! ```
//! use allora::{patterns::filter::Filter, route::Route, Error, Exchange, Message};
//! let route = Route::new().add(Filter::with_error(|ex| ex.in_msg.body_text()==Some("X"), "not X" )).build();
//! let mut ex = Exchange::new(Message::from_text("Y"));
//! #[cfg(feature="async")]
//! let res = tokio::runtime::Runtime::new().unwrap().block_on(route.run(&mut ex));
//! #[cfg(not(feature="async"))]
//! let res = route.run(&mut ex);
//! assert!(matches!(res, Err(Error::Routing(msg)) if msg=="not X"));
//! ```
//!
//! # Testing Tips
//! * Use different payloads / headers to assert accept vs reject behavior.
//! * Chain filters to model compound logic (`Route::new().add(f1).add(f2)`).
//! * Provide a distinctive error message to distinguish which filter rejected.
//! * Inspect the `Exchange` state before and after the filter to ensure expected behavior.
//! * Leverage async test features to simulate real-world usage.

use crate::{error::Result, processor::SyncProcessor, Exchange};
use std::fmt::{Debug, Formatter, Result as FmtResult};

pub type Predicate = Box<dyn Fn(&Exchange) -> bool + Send + Sync + 'static>;

pub struct Filter {
    predicate: Predicate,
    error_message: Option<String>,
}

impl Debug for Filter {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str("Filter{predicate=*closure*}")
    }
}

impl Filter {
    pub fn new<P>(p: P) -> Self
    where
        P: Fn(&Exchange) -> bool + Send + Sync + 'static,
    {
        Self {
            predicate: Box::new(p),
            error_message: None,
        }
    }
    /// Create a filter with a custom rejection error message.
    pub fn with_error<P, S>(p: P, msg: S) -> Self
    where
        P: Fn(&Exchange) -> bool + Send + Sync + 'static,
        S: Into<String>,
    {
        Self {
            predicate: Box::new(p),
            error_message: Some(msg.into()),
        }
    }
    pub fn accepts(&self, exchange: &Exchange) -> bool {
        (self.predicate)(exchange)
    }
}

impl SyncProcessor for Filter {
    fn process_sync(&self, exchange: &mut Exchange) -> Result<()> {
        if self.accepts(exchange) {
            Ok(())
        } else {
            let em = self.error_message.as_deref().unwrap_or("filtered out");
            Err(crate::error::Error::Routing(em.into()))
        }
    }
}
