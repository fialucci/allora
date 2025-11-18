// --- Core re-exports -------------------------------------------------

pub use allora_core::{
    adapter::{
        ensure_correlation, BaseAdapter, InboundAdapter, OutboundAdapter, OutboundDispatchResult,
    },
    channel::{
        Channel, ChannelRef, CorrelationSupport, DirectChannel, PollableChannel, QueueChannel,
        SubscribableChannel,
    },
    endpoint::{
        Endpoint, EndpointBuilder, EndpointSource, InMemoryEndpoint, InMemoryInOnlyEndpoint,
    },
    error::{Error, Result},
    message::{Exchange, Message, Payload},
    // ...other patterns as you add them
    processor::{ClosureProcessor, Processor},
    service::Service,
};

// --- Runtime facade ---------------------------------------------------

#[cfg(feature = "runtime")]
pub use allora_runtime::{
    dsl::build, // low-level: build(&Path) -> Result<AlloraRuntime>
    runtime::Runtime,
};

// --- Macros -----------------------------------------------------------

pub use allora_macros::service;

// --- Service Descriptors ----------------------------------------------

// Re-export ServiceDescriptor and descriptor accessors from runtime (single source of truth).
pub use allora_runtime::inventory;
pub use allora_runtime::{all_service_descriptors, ServiceDescriptor};
