//! Allora Core – foundational message and processing types (multi-crate split).
//!
//! This crate defines the fundamental primitives used by higher-level crates:
//! * Message / Payload
//! * Exchange
//! * Error / Result
//! * Processor trait (+ ClosureProcessor helper)
//! * Service & ServiceActivator traits
//! * Channel traits (Channel, PollableChannel, SubscribableChannel, CorrelationSupport)
//!
//! Concrete channel implementations (DirectChannel, QueueChannel) remain in the parent crate
//! until migration completes. This core crate intentionally avoids depending on higher-level
//! components (adapters, endpoints, patterns) to prevent circular dependencies.
//!
//! NOTE: The #[service] macro currently targets the root `allora` crate (`allora::` path) and
//! should not be invoked inside this core crate until the macro is generalized. Thus we do NOT
//! re-export the macro from here.

pub mod adapter;
pub mod channel;
pub mod endpoint;
pub mod error;
pub mod logging;
pub mod message;
pub mod patterns;
pub mod processor;
pub mod route;
pub mod service;

pub use channel::{Channel, ChannelRef, CorrelationSupport, PollableChannel, SubscribableChannel};
pub use endpoint::{Endpoint, EndpointBuilder, EndpointSource, InMemoryEndpoint};
pub use error::{Error, Result};
pub use logging::{init_from_dir as core_init_logging, LoggingSettings};
pub use message::{Exchange, Message, Payload};
#[cfg(feature = "filter")]
pub use patterns::filter::Filter as CoreFilter;
pub use processor::{BoxedProcessor, ClosureProcessor, Processor};
pub use route::Route;
pub use service::{Service, ServiceActivator};
