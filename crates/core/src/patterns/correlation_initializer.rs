use crate::{error::Result, processor::Processor, Exchange};
use std::fmt::{Debug, Formatter, Result as FmtResult};

/// A convenience processor that ensures a correlation id is present on the inbound
/// message and (optionally) mirrors it to a configurable header name for legacy
/// compatibility.
///
/// This is useful to place at the start of a `Route` so downstream processors
/// (splitters, aggregators, recipient lists, request/reply patterns) can assume
/// that a stable `correlation_id` header exists. It reduces boilerplate and the
/// chance of forgetting to set one, especially in heterogeneous pipelines.
///
/// # Behavior
/// * If `correlation_id` header exists, it is left untouched.
/// * Otherwise a new UUID v4 is generated and stored under `correlation_id`.
/// * If `mirror_header` is set, that header key is also set to the same value.
///
/// # Example
/// ```rust
/// use allora_core::{patterns::correlation_initializer::CorrelationInitializer, processor::ClosureProcessor, route::Route, Exchange, Message};
/// let route = Route::new()
///     .add(CorrelationInitializer::default())
///     .add(ClosureProcessor::new(|exchange| { exchange.out_msg = Some(Message::from_text("done")); Ok(()) }))
///     .build();
/// let mut exchange = Exchange::new(Message::from_text("hi"));
/// let rt = tokio::runtime::Runtime::new().unwrap();
/// rt.block_on(async { route.run(&mut exchange).await.unwrap(); });
/// assert!(exchange.in_msg.header("correlation_id").is_some());
/// ```
#[derive(Clone, Default)]
pub struct CorrelationInitializer {
    mirror_header: Option<String>,
}

impl CorrelationInitializer {
    /// Create with optional mirror header (e.g. legacy systems expect `corr`).
    pub fn with_mirror<H: Into<String>>(header: H) -> Self {
        Self {
            mirror_header: Some(header.into()),
        }
    }
}

impl Debug for CorrelationInitializer {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("CorrelationInitializer")
            .field("mirror_header", &self.mirror_header)
            .finish()
    }
}

#[async_trait::async_trait]
impl Processor for CorrelationInitializer {
    async fn process(&self, exchange: &mut Exchange) -> Result<()> {
        if exchange.in_msg.header("correlation_id").is_none() {
            let cid = uuid::Uuid::new_v4().to_string();
            exchange.in_msg.set_header("correlation_id", &cid);
            if let Some(mh) = &self.mirror_header {
                exchange.in_msg.set_header(mh, &cid);
            }
        } else if let Some(mh) = &self.mirror_header {
            if exchange.in_msg.header(mh).is_none() {
                if let Some(cid_val) = exchange
                    .in_msg
                    .header("correlation_id")
                    .map(|s| s.to_string())
                {
                    exchange.in_msg.set_header(mh, &cid_val);
                }
            }
        }
        Ok(())
    }
}
