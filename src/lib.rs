//! Allora: Rust-native Enterprise Integration Patterns library.
//!
//! Provides building blocks (Message, Exchange, Endpoint, Processor) and common EIP patterns
//! (Filter, Content-Based Router, Splitter, Aggregator) for high-performance, idiomatic Rust flows.
//!
//! # Installation
//! ```toml
//! [dependencies]
//! allora = { version = "0.1", features = ["async", "http"] }
//! ```
//!
//! # Features
//! * `async` (default) – async channel ops & adapter runtimes
//! * `http` – HTTP inbound/outbound adapters
//! * Serde always enabled
//!
//! # Quick Start
//! ```rust
//! use allora::{patterns::filter::Filter, Exchange, Message};
//! let mut ex = Exchange::new(Message::from_text("hello"));
//! let filter = Filter::new(|e: &Exchange| e.in_msg.body_text() == Some("hello"));
//! assert!(filter.accepts(&ex));
//! ```
//!
//! # Architecture
//! Specs -> Parsers -> DSL -> Builders -> Runtime Patterns
//! 1. Specs: intent (no IO)
//! 2. Parsers: YAML -> Spec
//! 3. DSL: unified build APIs
//! 4. Builders: Spec -> runtime component
//! 5. Patterns: processing primitives
//!
//! # Channel YAML (v1)
//! ```yaml
//! version: 1
//! channel:
//!   kind: in_memory
//!   id: my-channel    # optional
//! ```
//!
//! # Mapping
//! * Filter => patterns::filter::Filter
//! * Content Router => patterns::content_router::ContentRouter
//! * Splitter => patterns::splitter::Splitter
//! * Aggregator => patterns::aggregator::Aggregator
//!
//! # DSL Example
//! ```rust
//! use allora::{build_channel_from_str, DslFormat, Channel};
//! let raw = "version: 1\nchannel:\n  kind: in_memory\n  id: demo";
//! let ch = build_channel_from_str(raw, DslFormat::Yaml).unwrap();
//! assert_eq!(ch.id(), "demo");
//! ```
//!
//! # Stability & Versioning
//! * Parsers enforce `version`.
//! * New versions add new parser modules.
//!
//! # License: MIT OR Apache-2.0
//!
//! # Full Runtime Build (AlloraRuntime)
//! If you have a top-level `allora.yml` describing multiple channels:
//! ```yaml
//! version: 1
//! channels:
//!   - kind: in_memory
//!     id: inbound.orders
//!   - kind: in_memory
//!     id: processed.orders
//! ```
//! Build all declared components (currently channels) with:
//! ```rust
//! use allora::{build, Channel};
//! let rt = build("tests/fixtures/allora.yml").unwrap();
//! assert!(rt.channel_by_id("inbound.orders").is_some());
//! assert_eq!(rt.channel_count(), 3); // depending on your fixture
//! ```
//!
//! # Programmatic vs YAML Specs
//! You can construct specs directly:
//! ```rust
//! use allora::spec::ChannelSpec;
//! let spec = ChannelSpec::in_memory().id("direct");
//! // Then feed into builder: allora::dsl::component_builders::build_channel_from_spec(spec)
//! ```
//! or rely on YAML parsing via `build_channel_from_str` / `build()`.
//!
//! # ID Strategy
//! * Single channel without id -> UUID (via builder)
//! * Multi-channel (top-level build) missing ids -> deterministic `channel:auto.N`
//! * Duplicate ids -> build-time error (`duplicate channel.id`)
//!
//! # Roadmap (Indicative)
//! * Additional channel kinds (kafka, amqp)
//! * Endpoint / Adapter specs
//! * Filter & Router DSL (expression + structured forms)
//! * JSON / XML format parsers
//! * Pluggable id generation strategy
//!
//! # Contributing
//! Keep additions layered: Spec -> Parser -> Builder -> Facade -> Pattern. Avoid embedding parsing logic in builders.

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
pub use dsl::runtime::AlloraRuntime;
pub use dsl::{build, build_channel, build_channel_from_str, DslFormat};
pub use endpoint::{Endpoint, InMemoryEndpoint};
pub use error::{Error, Result};
#[cfg(feature = "http")]
pub use http_inbound_adapter::{HttpInboundAdapter, Mep};
#[cfg(feature = "http")]
pub use http_outbound_adapter::HttpOutboundAdapter;
pub use message::{Exchange, Message, Payload};
pub use processor::{BoxedProcessor, ClosureProcessor, Processor, SyncProcessor};
pub use spec::{ChannelKindSpec, ChannelSpec};
