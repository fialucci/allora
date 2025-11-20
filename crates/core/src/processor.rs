use crate::{error::Result, Exchange};
use std::fmt::{Debug, Formatter, Result as FmtResult};

/// A Processor transforms an Exchange (async-only model).
///
/// # Semantics
/// * Single async `process` method; must be non-blocking (offload heavy CPU or blocking IO).
/// * Return `Ok(())` to continue route execution; return `Err(Error)` to short-circuit.
/// * Mutate `exchange.in_msg`, set `exchange.out_msg`, or add headers as needed; avoid global state.
///
/// # Error Propagation
/// Use specific `Error` variants for clarity (`Error::Processor`, `Error::Routing`, etc.).
///
/// # Example
/// ```rust
/// use allora_core::{error::Result, processor::Processor, Exchange, Message};
/// #[derive(Debug)] struct Upper;
/// #[async_trait::async_trait]
/// impl Processor for Upper {
///     async fn process(&self, exchange: &mut Exchange) -> Result<()> {
///         if let Some(body) = exchange.in_msg.body_text() {
///             exchange.out_msg = Some(Message::from_text(body.to_uppercase()));
///         }
///         Ok(())
///     }
/// }
/// let p = Upper;
/// let mut ex = Exchange::new(Message::from_text("hi"));
/// tokio::runtime::Runtime::new().unwrap().block_on(async { p.process(&mut ex).await.unwrap(); });
/// assert_eq!(ex.out_msg.unwrap().body_text(), Some("HI"));
/// ```
#[async_trait::async_trait]
pub trait Processor: Send + Sync + Debug {
    async fn process(&self, exchange: &mut Exchange) -> Result<()>;
}

#[derive()]
pub struct ClosureProcessor<F>
where
    F: Fn(&mut Exchange) -> Result<()> + Send + Sync + 'static,
{
    func: F,
}

impl<F> ClosureProcessor<F>
where
    F: Fn(&mut Exchange) -> Result<()> + Send + Sync + 'static,
{
    pub fn new(func: F) -> Self {
        Self { func }
    }
    /// Convenience helper identical to `new` for readability in fluent route construction.
    pub fn closure(func: F) -> Self {
        Self { func }
    }
}

#[async_trait::async_trait]
impl<F> Processor for ClosureProcessor<F>
where
    F: Fn(&mut Exchange) -> Result<()> + Send + Sync + 'static,
{
    async fn process(&self, exchange: &mut Exchange) -> Result<()> {
        (self.func)(exchange)
    }
}

impl<F> Debug for ClosureProcessor<F>
where
    F: Fn(&mut Exchange) -> Result<()> + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str("ClosureProcessor{func=*closure*}")
    }
}

/// Boxed dynamic processor type.
/// Useful for heterogeneous collections (used internally by `Route`).
pub type BoxedProcessor = Box<dyn Processor>;
