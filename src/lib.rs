//! Allora – Integration Patterns & Message Flow Building Blocks
//!
//! High-level, lightweight primitives for composing message-driven flows in Rust.
//! Provides channels, messages, exchanges, simple filters/patterns, plus a facade
//! (`Allora`) for bootstrapping a runtime from a YAML configuration file.
//!
//! # Key Concepts
//! * Message – immutable payload + headers
//! * Exchange – mutable processing context (in/out message, headers, correlation)
//! * Channel – in-memory endpoint for sending/receiving `Exchange` instances
//! * Filter (pattern) – predicate over an `Exchange`
//! * Runtime – collection of declared channels & filters built from a spec
//!
//! # Features
//! * `async` (default) – async channel operations
//! * `http` – optional HTTP adapters
//! * `serde` always on for (de)serialization
//!
//! # Crate Use
//! * Programmatic: build channels/filters directly via builders
//! * Declarative: provide `allora.yml` and use `Allora::new().run()`
//!
//! # Minimal Programmatic Example
//! ```rust
//! use allora::{Exchange, Message, patterns::filter::Filter};
//! let mut exchange = Exchange::new(Message::from_text("ping"));
//! let f = Filter::new(|e: &Exchange| e.in_msg.body_text() == Some("ping"));
//! assert!(f.accepts(&exchange));
//! ```
//!
//! # Minimal YAML Channel Spec
//! ```yaml
//! version: 1
//! channels:
//!   - kind: direct
//!     id: inbound
//!   - kind: direct
//!     id: outbound
//! ```
//! Build from file with:
//! ```no_run
//! # use allora::{Allora, Channel};
//! let rt = Allora::new().with_config_file("./allora.yml").run()?;
//! assert!(rt.channel_by_id("inbound").is_some());
//! # Ok::<_, allora::Error>(())
//! ```
//!
//! # Building Components Directly
//! ```rust
//! use allora::{build_channel_from_str, DslFormat, Channel};
//! let raw = "version: 1\nchannel:\n  kind: direct\n  id: demo";
//! let ch = build_channel_from_str(raw, DslFormat::Yaml).unwrap();
//! assert_eq!(ch.id(), "demo");
//! ```
//!
//! # Errors
//! All builder and facade operations surface failures via `Error`.
//! Use `Result<T, Error>` and propagate with `?`.
//!
//! # License
//! MIT OR Apache-2.0.
//!
//! # Stability & Versioning
//! * Parsers enforce `version`.
//! * New versions add new parser modules.

pub mod adapter; // generic inbound adapter abstractions (file adapter.rs)
pub mod allora;
#[path = "channel/mod.rs"]
pub mod channel;
pub mod dsl; // new multi-format DSL facade (yaml/json/xml)
pub mod endpoint;
pub mod error;
#[cfg(feature = "http")]
pub mod http_inbound_adapter;
#[cfg(feature = "http")]
pub mod http_outbound_adapter;
pub mod logging;
pub mod message;
pub mod patterns;
pub mod processor;
pub mod route; // YAML DSL support (channel schema v1)
pub mod service;
pub mod service_activator_processor;
pub mod spec; // new specification-based builders replacing DSL façade gradually

// Channel abstractions
pub use adapter::{
    ensure_correlation, Adapter, InboundAdapter, OutboundAdapter, OutboundDispatchResult,
};
pub use allora::Allora;
pub use allora_macros::service;
pub use channel::{Channel, ChannelRef, DirectChannel, QueueChannel};
pub use dsl::runtime::AlloraRuntime;
pub use dsl::{
    build, build_channel, build_channel_from_str, build_filter, build_service, DslFormat,
};
pub use endpoint::{Endpoint, InMemoryEndpoint};
pub use error::{Error, Result};
#[cfg(feature = "http")]
pub use http_inbound_adapter::{HttpInboundAdapter, Mep};
#[cfg(feature = "http")]
pub use http_outbound_adapter::HttpOutboundAdapter;
pub use message::{Exchange, Message, Payload};
pub use patterns::filter::Filter;
pub use processor::{BoxedProcessor, ClosureProcessor, Processor, SyncProcessor};
pub use service::{Service, ServiceActivator};

#[derive(Clone)]
pub struct ServiceDescriptor {
    pub name: &'static str,
    pub constructor: fn() -> Arc<dyn SyncProcessor>,
}
inventory::collect!(ServiceDescriptor);
use inventory;
use std::sync::Arc;
pub fn all_service_descriptors() -> Vec<&'static ServiceDescriptor> {
    let mut v = Vec::new();
    for d in inventory::iter::<ServiceDescriptor> {
        v.push(d);
    }
    v
}
