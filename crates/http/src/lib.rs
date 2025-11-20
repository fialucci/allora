//! HTTP integration crate for Allora: Adapter DSL facade.

pub mod adapter_dsl;
pub mod http_inbound_adapter;
pub mod http_outbound_adapter;

pub use adapter_dsl::{Adapter, InboundStage, OutboundStage};
pub use http_inbound_adapter::{HttpInboundAdapter, HttpInboundBuilder, HttpServerHandle, Mep};
pub use http_outbound_adapter::{HttpOutboundAdapter, HttpOutboundAdapterBuilder};
