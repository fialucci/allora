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
//! Patterns follow Enterprise Integration Pattern semantics adapted to idiomatic Rust:
//! * Trait (`Processor`) keeps implementations lightweight.
//! * Correlation handled lazily; use `CorrelationInitializer` or call `Exchange::correlation_id()`.
//! * Each pattern lives in its own module with focused responsibilities.
//!
//! # Example: Filter and Aggregator Combined
//! ```rust
//! use allora_core::{route::Route , Exchange, Message};
//! use allora_core::patterns::{aggregator::Aggregator, correlation_initializer::CorrelationInitializer, filter::Filter};
//! let route = Route::new()
//!     .add(CorrelationInitializer::with_mirror("corr"))
//!     .add(Filter::new(|exchange: &Exchange| exchange.in_msg.body_text() == Some("keep")))
//!     .add(Aggregator::new("corr", 2))
//!     .build();
//! let mut exchange = Exchange::new(Message::from_text("keep"));
//! let rt = tokio::runtime::Runtime::new().unwrap();
//! rt.block_on(async { route.run(&mut exchange).await.unwrap(); });
//! assert!(exchange.out_msg.is_none());
//! ```
//!
//! For detailed usage, see each submodule's own documentation and tests under `tests/`.
pub mod aggregator;
pub mod content_router;
pub mod correlation_initializer;
pub mod filter;
pub mod recipient_list;
pub mod splitter;
