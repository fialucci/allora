//! Integration Patterns module: collection of Enterprise Integration Patterns (EIP) primitives.
//!
//! # Provided Patterns
//! * `filter` – Conditional acceptance/rejection of an `Exchange` (`Filter`).
//! * `content_router` – Content-based routing using predicates to select a downstream processor (`ContentBasedRouter`).
//! * `splitter` – Split a single inbound message into multiple outbound messages (`Splitter`).
//! * `aggregator` – Aggregate correlated messages until a completion condition is met (`Aggregator`).
//! * `recipient_list` – Fan-out to a dynamic list of processors (`RecipientList`).
//! * `correlation_initializer` – Ensure a `correlation_id` (and optional mirror header) early in a route.
//!
//! # Design Notes
//! Patterns follow Camel / Spring Integration semantics but are adapted to idiomatic Rust:
//! * Traits (`Processor`, `SyncProcessor`) keep implementations lightweight.
//! * Correlation handled lazily; use `CorrelationInitializer` or call `Exchange::correlation_id()`.
//! * Each pattern lives in its own module with focused responsibilities.
//!
//! # Example: Filter and Aggregator Combined
//! ```
//! use allora::{route::Route, processor::ClosureProcessor, Message, Exchange};
//! use allora::patterns::{filter::Filter, aggregator::Aggregator, correlation_initializer::CorrelationInitializer};
//! // Correlation first
//! let route = Route::new()
//!     .add(CorrelationInitializer::with_mirror("corr"))
//!     .add(Filter::new(|ex: &Exchange| ex.in_msg.body_text() == Some("keep")))
//!     .add(Aggregator::new("corr", 2)) // simple size completion
//!     .build();
//! let mut ex = Exchange::new(Message::from_text("keep"));
//! #[cfg(feature="async")]
//! tokio::runtime::Runtime::new().unwrap().block_on(async { route.run(&mut ex).await.unwrap(); });
//! #[cfg(not(feature="async"))]
//! route.run(&mut ex).unwrap();
//! // After first message aggregator not complete yet, so no out_msg
//! assert!(ex.out_msg.is_none());
//! ```
//!
//! For detailed usage, see each submodule's own documentation and tests under `tests/`.
pub mod aggregator;
pub mod content_router;
pub mod correlation_initializer;
pub mod filter;
pub mod recipient_list;
pub mod splitter;
