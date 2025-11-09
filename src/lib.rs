//! Allora: Rust-native Enterprise Integration Patterns library.
//!
//! This crate provides building blocks (Message, Exchange, Endpoint, Processor)
//! and common EIP pattern implementations (Filter, Content-Based Router, Splitter, Aggregator, etc.)
//! for constructing integration flows that are idiomatic to Rust.
//!
//! Tagline: Connect everything into one continuous, high-performance flow.
//!
//! # Installation
//! Add to your `Cargo.toml`:
//! ```toml
//! [dependencies]
//! allora = { version = "0.1", features = ["async", "serde"] }
//! # To enable HTTP adapters:
//! # allora = { version = "0.1", features = ["async", "serde", "http"] }
//! ```
//!
//! # Optional Features
//! * `async` (default) – asynchronous channel operations & adapter runtimes.
//! * `http` – HTTP inbound/outbound adapters.
//! * `serde` – (de)serialization helpers for payloads.
//!
//! # Quick Start
//! ```rust
//! use allora::{patterns::filter::Filter, Exchange, Message};
//! let msg = Message::from_text("hello");
//! let mut ex = Exchange::new(msg);
//! let filter = Filter::new(|e: &Exchange| e.in_msg.body_text().map(|t| t == "hello").unwrap_or(false));
//! assert!(filter.accepts(&ex));
//! ```
//!
//! # Architecture Overview
//! * Specs (`spec/`) define component intent (no IO).
//! * Parsers (`spec/*_yaml.rs`) turn format documents into Specs.
//! * DSL (`dsl/`) provides user-facing build APIs across formats.
//! * Builders (`dsl/component_builders.rs`) instantiate runtime objects.
//! * Runtime patterns (`patterns/`, `processor/`, `channel/`) implement EIP constructs.
//!
//! # Spec vs DSL
//! * Spec: Programmatic, strongly typed (e.g. `ChannelSpec::in_memory().id("orders")`).
//! * DSL: Textual configuration translation (YAML today). Calls parser -> spec -> builder.
//! * Builders: Format-agnostic; consume specs only.
//!
//! # Channel YAML Schema (v1)
//! ```yaml
//! version: 1
//! channel:
//!   kind: in_memory   # required
//!   id: my-channel    # optional
//! ```
//!
//! # EIP Mapping Examples
//! * Filter -> `patterns::filter::Filter`
//! * Content-Based Router -> `patterns::content_router::ContentRouter`
//! * Splitter -> `patterns::splitter::Splitter`
//! * Aggregator -> `patterns::aggregator::Aggregator`
//!
//! # End-to-End (YAML DSL -> Channel)
//! ```rust
//! use allora::{build_channel_from_str, DslFormat, Channel};
//! let raw = "version: 1\nchannel:\n  kind: in_memory\n  id: demo-e2e";
//! let ch = build_channel_from_str(raw, DslFormat::Yaml).unwrap();
//! assert_eq!(ch.id(), "demo-e2e");
//! ```
//!
//! # Stability & Versioning
//! * Spec versioning enforced in parsers (reject unsupported versions).
//! * Additive changes favored; breaking changes introduce new versioned parsers.
//!
//! # License
//! MIT OR Apache-2.0 at your option.
//!
//! # Features
//! * `async` (default): Enables asynchronous processing using Tokio and async-trait.
//! * `serde`: Enables (de)serialization support for message payloads.
//!
//! # Quick Start
//! ```rust
//! use allora::{patterns::filter::Filter, Exchange, Message};
//! let msg = Message::from_text("hello");
//! let mut ex = Exchange::new(msg);
//! let filter = Filter::new(|e: &Exchange| e.in_msg.body_text().map(|t| t == "hello").unwrap_or(false));
//! assert!(filter.accepts(&ex));
//! ```
//!
//! # Architecture Overview
//! * Specs (`spec/`) define component intent (no IO).
//! * Parsers (`spec/*_yaml.rs`) turn format documents into Specs.
//! * DSL (`dsl/`) provides user-facing build APIs across formats.
//! * Builders (`dsl/component_builders.rs`) instantiate runtime objects.
//! * Runtime patterns (`patterns/`, `processor/`, `channel/`) implement EIP constructs.
//!
//! # EIP Mapping Examples
//! * Filter -> `patterns::filter::Filter`
//! * Content-Based Router -> `patterns::content_router::ContentRouter`
//! * Splitter -> `patterns::splitter::Splitter`
//! * Aggregator -> `patterns::aggregator::Aggregator`
//!
//! # End-to-End (YAML DSL -> Channel)
//! ```rust
//! use allora::{build_channel_from_str, DslFormat, Channel};
//! let raw = "version: 1\nchannel:\n  kind: in_memory\n  id: demo-e2e";
//! let ch = build_channel_from_str(raw, DslFormat::Yaml).unwrap();
//! assert_eq!(ch.id(), "demo-e2e");
//! ```
//!
//! # Feature Flags
//! * `async` – async channel operations & adapter runtimes.
//! * `http` – HTTP inbound/outbound adapters.
//! * `serde` – (de)serialization helpers for payloads.
//!
//! # Stability & Versioning
//! * Spec versioning enforced in parsers (reject unsupported versions).
//! * Additive changes favored; breaking changes require new version-specific parser modules.

pub mod adapter; // generic inbound adapter abstractions (file adapter.rs)
pub mod channel;
pub mod dsl; // new multi-format DSL facade (yaml/json/xml)
pub mod endpoint;
pub mod error;
#[cfg(feature = "http")]
pub mod http_inbound_adapter;
#[cfg(feature = "http")]
pub mod http_outbound_adapter;
pub mod message;
pub mod patterns;
pub mod processor;
pub mod route; // YAML DSL support (channel schema v1)
pub mod spec; // new specification-based builders replacing DSL façade gradually

// Channel abstractions
pub use adapter::{
    ensure_correlation, Adapter, InboundAdapter, OutboundAdapter, OutboundDispatchResult,
};
pub use channel::{
    Channel, ChannelRef, CorrelationSupport, DefaultChannel, InMemoryChannel, OutboundQueue,
};
pub use dsl::{build_channel, build_channel_from_str, DslFormat};
pub use endpoint::{Endpoint, InMemoryEndpoint};
pub use error::{Error, Result};
#[cfg(feature = "http")]
pub use http_inbound_adapter::HttpInboundAdapter;
#[cfg(feature = "http")]
pub use http_outbound_adapter::HttpOutboundAdapter;
pub use message::{Exchange, Message, Payload};
pub use processor::{BoxedProcessor, ClosureProcessor, Processor, SyncProcessor};
pub use spec::{ChannelKindSpec, ChannelSpec};
