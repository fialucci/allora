use crate::{error::Result, processor::Processor, Exchange};

/// A `Route` represents an ordered pipeline of `Processor` implementations applied to
/// an `Exchange`. Each processor can mutate the `Exchange` (e.g. enrich headers,
/// transform the payload, set `out_msg`, assign correlation identifiers, etc.).
///
/// # Design Goals
/// * Simple, minimal abstraction over a Vec of boxed processors.
/// * Works in both synchronous and asynchronous modes (feature `async`).
/// * Deterministic ordering: processors are executed in the order they were added.
/// * Uniform error propagation: the first processor returning an error short-circuits the route.
/// * Extensible: callers can wrap `Route::run` to add cross-cutting concerns (metrics, tracing, retry).
///
/// # When to Use
/// Use a `Route` anytime you need to compose a series of message transformations / routing logic.
/// It mirrors concepts from EIP frameworks where a route is an assembly of steps.
///
/// # Correlation & Message IDs
/// If your processors rely on correlation IDs (e.g. aggregation), have an early processor call
/// `exchange.in_msg.ensure_correlation_id()` to guarantee one is present. `Message` already
/// auto-generates a `message_id` header on creation.
///
/// # Examples
/// Unified example:
/// ```rust
/// use allora_core::{processor::ClosureProcessor, route::Route, Exchange, Message};
/// let route = Route::new().add(ClosureProcessor::new(|exchange| { exchange.out_msg = Some(Message::from_text("done")); Ok(()) })).build();
/// let mut exchange = Exchange::new(Message::from_text("hi"));
/// tokio::runtime::Runtime::new().unwrap().block_on(async { route.run(&mut exchange).await.unwrap(); });
/// assert_eq!(exchange.out_msg.unwrap().body_text(), Some("done"));
/// ```
///
/// # Error Handling
/// If any processor returns an `Err`, processing stops immediately and the error is returned to
/// the caller. Downstream processors are not executed.
///
/// # Future Extensions (Ideas)
/// * Conditional routing (e.g. only execute processor if a header matches).
/// * Branch / fork processors returning multiple Exchanges.
/// * Built-in metrics / tracing instrumentation wrapper.
/// * Middleware style before / after hooks.
///
/// Create routes via the builder pattern: `Route::new().add(...).add(...).build()`.
#[derive(Default, Debug)]
pub struct Route {
    processors: Vec<Box<dyn Processor>>,
}

impl Route {
    /// Create an empty route.
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }
    /// Add a processor to the route. Processors execute in insertion order.
    pub fn add<P: Processor + 'static>(mut self, p: P) -> Self {
        self.processors.push(Box::new(p));
        self
    }
    /// Finalize the route (currently a no-op, kept for API symmetry / future extension).
    pub fn build(self) -> Self {
        self
    }
    /// Convenience: create a route whose first processor ensures a correlation id.
    /// Optionally mirror the correlation id into an additional header name.
    /// Example:
    /// ```rust
    /// use allora_core::{route::Route, processor::ClosureProcessor, Message, Exchange};
    /// let route = Route::with_correlation(None)
    ///     .add(ClosureProcessor::new(|exchange| { exchange.out_msg = Some(Message::from_text("pong")); Ok(()) }))
    ///     .build();
    /// let mut exchange = Exchange::new(Message::from_text("ping"));
    /// tokio::runtime::Runtime::new().unwrap().block_on(async { route.run(&mut exchange).await.unwrap(); });
    /// assert!(exchange.in_msg.header("correlation_id").is_some());
    /// ```
    pub fn with_correlation(mirror_header: Option<&str>) -> Self {
        use crate::patterns::correlation_initializer::CorrelationInitializer;
        let init = match mirror_header {
            Some(h) => CorrelationInitializer::with_mirror(h),
            None => CorrelationInitializer::default(),
        };
        Self {
            processors: vec![Box::new(init)],
        }
    }
    /// Run the route asynchronously over a mutable `Exchange`.
    pub async fn run(&self, exchange: &mut Exchange) -> Result<()> {
        for p in &self.processors {
            p.process(exchange).await?;
        }
        Ok(())
    }
}
