//! Allora – Integration Patterns & Message Flow Building Blocks
//!
//! High-level, lightweight primitives for composing message-driven flows in Rust.
//! Provides channels, messages, exchanges, simple filters/patterns, plus a facade
//! (`Runtime`) for bootstrapping a runtime from a YAML configuration file.
//!
//! # Key Concepts
//! * Message – immutable payload + headers
//! * Exchange – mutable processing context (in/out message, headers, correlation)
//! * Channel – in-memory endpoint for sending/receiving `Exchange` instances
//! * Filter (pattern) – predicate over an `Exchange`
//! * Runtime – collection of declared channels & filters built from a spec
//!
//! # Features
//! Async-only architecture; HTTP adapters and serialization always enabled (no feature flags).
//!
//! # Crate Use
//! * Programmatic: build channels/filters directly via builders
//! * Declarative: provide `allora.yml` and use `Runtime::new().run()`
//!
//! # Minimal Programmatic Example
//! ```rust
//! use allora_core::{patterns::filter::Filter, Exchange, Message};
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
//! # use allora_runtime::{Runtime, Channel};
//! let rt = Runtime::new().with_config_file("./allora.yml").run()?;
//! assert!(rt.channel_by_id("inbound").is_some());
//! # Ok::<_, allora_runtime::Error>(())
//! ```
//!
//! # Building Components Directly
//! ```rust
//! use allora_runtime::{build_channel_from_str, DslFormat, Channel};
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

pub mod dsl; // new multi-format DSL facade (yaml/json/xml)
pub mod runtime;
pub mod service_activator_processor;
pub mod spec; // new specification-based builders replacing DSL façade gradually
pub use adapter::{
    ensure_correlation, Adapter, InboundAdapter, OutboundAdapter, OutboundDispatchResult,
};
pub use allora_core::channel;
pub use allora_core::endpoint;
pub use allora_core::error;
pub use allora_core::logging;
pub use allora_core::message;
pub use allora_core::patterns;
pub use allora_core::processor;
pub use allora_core::route;
pub use allora_core::service;
// Channel abstractions
pub use allora_core::Filter;
pub use allora_core::Route;
pub use allora_core::{Endpoint, EndpointBuilder, EndpointSource, InMemoryEndpoint};

// Top-level HTTP re-exports (behind the `http` feature)
pub use allora_http::{
    HttpInboundAdapter, HttpInboundBuilder, HttpOutboundAdapter, HttpOutboundAdapterBuilder,
    InboundHttpExt, Mep, OutboundHttpExt,
};
pub use channel::{
    Channel, ChannelRef, CorrelationSupport, DirectChannel, PollableChannel, QueueChannel,
    SubscribableChannel,
};
pub use dsl::runtime::{AlloraRuntime, FilterActivation, HttpOutboundAdapterActivation};
pub use dsl::{
    build, build_channel, build_channel_from_str, build_filter, build_service, DslFormat,
};
pub use error::{Error, Result};
pub use message::{Exchange, Message, Payload};
pub use processor::{BoxedProcessor, ClosureProcessor, Processor};
pub use runtime::Runtime;
pub use service::{Service, ServiceActivator};

#[derive(Clone)]
pub struct ServiceDescriptor {
    pub name: &'static str,
    pub constructor: fn() -> Arc<dyn Service>,
}
inventory::collect!(ServiceDescriptor);

pub fn all_service_descriptors() -> Vec<&'static ServiceDescriptor> {
    let mut v = Vec::new();
    for d in inventory::iter::<ServiceDescriptor> {
        v.push(d);
    }
    v
}

pub mod adapter {
    pub use allora_core::adapter::{
        ensure_correlation, Adapter, BaseAdapter, InboundAdapter, InboundStage, OutboundAdapter,
        OutboundDispatchResult, OutboundStage,
    };
    pub use allora_http::{
        HttpInboundAdapter, HttpInboundBuilder, HttpOutboundAdapter, HttpOutboundAdapterBuilder,
        InboundHttpExt, Mep, OutboundHttpExt,
    };
}

pub use inventory; // re-export for macro usage allora::inventory::submit!

use std::sync::Arc;
