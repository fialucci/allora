use crate::{error::Result, Exchange};
use std::fmt::{Debug, Formatter, Result as FmtResult};

/// A Processor transforms or acts upon an Exchange.
///
/// # Async vs Sync Feature
/// With the `async` feature enabled the `Processor` trait exposes an `async fn process`. You can
/// implement it directly for processors that perform asynchronous IO. For simple, CPU-bound or
/// header/body transformations, prefer implementing [`SyncProcessor`]; those implementations are
/// automatically adapted to `Processor` in async mode.
///
/// Without the `async` feature, `Processor` is a synchronous trait with a blocking `process`.
///
/// # Error Propagation
/// Returning an `Err(Error)` short-circuits route execution (see `Route`). Use distinct
/// variants (`Processor`, `Routing`, `Aggregation`, etc.) for clearer diagnostics.
///
/// # Implementing
/// * Implement `SyncProcessor` for synchronous logic; you get a blanket impl of `Processor`.
/// * Implement `Processor` directly (async feature) if you need `await` inside the processor.
///
/// # Example (sync)
/// ```
/// use allora::{processor::{ClosureProcessor, SyncProcessor}, Exchange, Message };
/// let p = ClosureProcessor::new(|exchange: &mut Exchange| { exchange.in_msg.set_header("t", "1"); Ok(()) });
/// let mut exchange = Exchange::new(Message::from_text("hi"));
/// p.process_sync(&mut exchange).unwrap();
/// assert_eq!(exchange.in_msg.header("t"), Some("1"));
/// ```
///
/// # Example (async custom processor)
/// ```
/// use allora::{error::Result, processor::Processor, Exchange, Message};
/// #[derive(Debug)]
/// struct Delay;
/// #[async_trait::async_trait]
/// impl Processor for Delay {
///     async fn process(&self, exchange: &mut Exchange) -> Result<()> {
///         exchange.in_msg.set_header("async", "ok");
///         Ok(())
///     }
/// }
/// let p = Delay;
/// let mut exchange = Exchange::new(Message::from_text("ping"));
/// tokio::runtime::Runtime::new().unwrap().block_on(async { p.process(&mut exchange).await.unwrap(); });
/// assert_eq!(exchange.in_msg.header("async"), Some("ok"));
/// ```
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait Processor: Send + Sync + Debug {
    async fn process(&self, exchange: &mut Exchange) -> Result<()>;
}

#[cfg(not(feature = "async"))]
pub trait Processor: Send + Sync + Debug {
    fn process(&self, exchange: &mut Exchange) -> Result<()>;
}

impl<F> Debug for ClosureProcessor<F>
where
    F: Fn(&mut Exchange) -> Result<()> + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str("ClosureProcessor{func=*closure*}")
    }
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

impl<F> SyncProcessor for ClosureProcessor<F>
where
    F: Fn(&mut Exchange) -> Result<()> + Send + Sync + 'static,
{
    fn process_sync(&self, exchange: &mut Exchange) -> Result<()> {
        (self.func)(exchange)
    }
}

#[derive()]
pub struct ClosureProcessor<F>
where
    F: Fn(&mut Exchange) -> Result<()> + Send + Sync + 'static,
{
    func: F,
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl<T> Processor for T
where
    T: SyncProcessor + Send + Sync + Debug,
{
    async fn process(&self, exchange: &mut Exchange) -> Result<()> {
        self.process_sync(exchange)
    }
}

#[cfg(not(feature = "async"))]
impl<T> Processor for T
where
    T: SyncProcessor + Send + Sync + Debug,
{
    fn process(&self, exchange: &mut Exchange) -> Result<()> {
        self.process_sync(exchange)
    }
}

/// Helper trait for synchronous processors even in async feature context.
///
/// Implement this for small, synchronous units of work. They are automatically adapted to
/// `Processor` when the async feature is active. Avoid heavy blocking IO inside `process_sync`
/// in async mode; offload that to an explicit async processor implementation.
pub trait SyncProcessor: Send + Sync + Debug {
    fn process_sync(&self, exchange: &mut Exchange) -> Result<()>;
}
/// Boxed dynamic processor type.
/// Useful for heterogeneous collections (used internally by `Route`).
pub type BoxedProcessor = Box<dyn Processor>;
