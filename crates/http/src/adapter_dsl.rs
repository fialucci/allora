use crate::{HttpInboundBuilder, HttpOutboundAdapterBuilder};
pub(crate) use allora_core::adapter::{InboundStage, OutboundStage};

/// Extension trait adding `.http()` to the core `InboundStage`.
pub trait InboundHttpExt {
    /// Begin building an HTTP inbound adapter (server) from the core adapter DSL.
    fn http(self) -> HttpInboundBuilder;
}

impl InboundHttpExt for InboundStage {
    fn http(self) -> HttpInboundBuilder {
        HttpInboundBuilder::new()
    }
}

/// Extension trait adding `.http()` to the core `OutboundStage`.
pub trait OutboundHttpExt {
    /// Begin building an HTTP outbound adapter (client) from the core adapter DSL.
    fn http(self) -> HttpOutboundAdapterBuilder;
}

impl OutboundHttpExt for OutboundStage {
    fn http(self) -> HttpOutboundAdapterBuilder {
        HttpOutboundAdapterBuilder::default()
    }
}
