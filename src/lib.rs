//! Allora: Rust-native Enterprise Integration Patterns library.
//!
//! This crate provides building blocks (Message, Exchange, Endpoint, Processor)
//! and common EIP pattern implementations (Filter, Content-Based Router, Splitter, Aggregator, etc.)
//! for constructing integration flows that are idiomatic to Rust.
//!
//! Tagline: An Open Source integration framework that helps you connect everything into one continuous, high-performance flow.
//!
//! # Features
//! * `async` (default): Enables asynchronous processing using Tokio and async-trait.
//! * `serde`: Enables (de)serialization support for message payloads.
//!
//! # Quick Start
//! ```rust
//! use allora::{Message, Exchange, patterns::filter::Filter};
//! let msg = Message::from_text("hello");
//! let mut ex = Exchange::new(msg);
//! let filter = Filter::new(|e: &Exchange| e.in_msg.body_text().map(|t| t == "hello").unwrap_or(false));
//! assert!(filter.accepts(&ex));
//! ```

pub mod adapter; // generic inbound adapter abstractions (file adapter.rs)
pub mod channel;
pub mod endpoint;
pub mod error;
#[cfg(feature = "http")]
pub mod http_inbound_adapter;
#[cfg(feature = "http")]
pub mod http_outbound_adapter;
pub mod message;
pub mod patterns;
pub mod processor;
pub mod route;

// Channel abstractions
pub use adapter::{
    ensure_correlation, Adapter, InboundAdapter, OutboundAdapter, OutboundDispatchResult,
};
pub use channel::{
    Channel, ChannelRef, CorrelationSupport, DefaultChannel, InMemoryChannel, OutboundQueue,
};
pub use endpoint::{Endpoint, InMemoryEndpoint};
pub use error::{Error, Result};
#[cfg(feature = "http")]
pub use http_inbound_adapter::HttpInboundAdapter;
#[cfg(feature = "http")]
pub use http_outbound_adapter::HttpOutboundAdapter;
pub use message::{Exchange, Message, Payload};
pub use processor::{BoxedProcessor, ClosureProcessor, Processor, SyncProcessor};
