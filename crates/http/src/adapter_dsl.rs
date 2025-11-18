use crate::{HttpInboundBuilder, HttpOutboundAdapterBuilder};

pub struct Adapter;

pub struct InboundStage;
pub struct OutboundStage;

impl Adapter {
    pub fn inbound() -> InboundStage {
        InboundStage
    }
    pub fn outbound() -> OutboundStage {
        OutboundStage
    }
}

impl InboundStage {
    pub fn http(self) -> HttpInboundBuilder {
        HttpInboundBuilder::new()
    }
}

impl OutboundStage {
    pub fn http(self) -> HttpOutboundAdapterBuilder {
        HttpOutboundAdapterBuilder::default()
    }
}
