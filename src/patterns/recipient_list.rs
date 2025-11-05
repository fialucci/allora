//! Recipient List pattern: fan-out a single `Exchange` sequentially to multiple processors.
//!
//! Implements the Enterprise Integration Pattern (EIP) "Recipient List". Each recipient
//! (processor) operates on the same mutable `Exchange` instance in the order added.
//!
//! # Use Cases
//! * Sequential enrichment steps (add headers, transform payload).
//! * Applying multiple independent processors where order matters (validation -> normalization -> formatting).
//! * Simple fan-out where recipients mutate the same exchange rather than produce clones.
//!
//! # Behavior
//! * Recipients execute strictly in insertion order.
//! * If a recipient returns an `Err`, processing stops and the error is propagated; remaining recipients are skipped.
//! * All recipients share and mutate the same `Exchange` (no copying or cloning performed).
//! * Last mutation wins (e.g. later recipient can overwrite `out_msg` set by earlier recipients).
//!
//! # Async vs Sync
//! Under the `async` feature the `process` method awaits each recipient's async `process` call.
//! Without it, a synchronous loop invokes `process` directly.
//!
//! # Example
//! ```
//! use allora::{patterns::recipient_list::RecipientList, processor::ClosureProcessor, route::Route, Exchange, Message};
//! let r1 = ClosureProcessor::new(|ex| { ex.in_msg.set_header("step", "1"); Ok(()) });
//! let r2 = ClosureProcessor::new(|ex| { ex.out_msg = Some(Message::from_text("done")); Ok(()) });
//! let list = RecipientList::new().add(r1).add(r2);
//! let route = Route::new().add(list).build();
//! let mut ex = Exchange::new(Message::from_text("payload"));
//! #[cfg(feature="async")] tokio::runtime::Runtime::new().unwrap().block_on(route.run(&mut ex)).unwrap();
//! #[cfg(not(feature="async"))] route.run(&mut ex).unwrap();
//! assert_eq!(ex.in_msg.header("step"), Some("1"));
//! assert_eq!(ex.out_msg.unwrap().body_text(), Some("done"));
//! ```
//!
//! # Error Short-Circuit Example
//! ```
//! use allora::{patterns::recipient_list::RecipientList, processor::ClosureProcessor, route::Route, Message, Exchange, Error, error::Result};
//! let ok = ClosureProcessor::new(|ex| { ex.in_msg.set_header("visited", "yes"); Ok(()) });
//! let fail = ClosureProcessor::new(|_| Err(Error::processor("boom"))); // stop here
//! let after = ClosureProcessor::new(|ex| { ex.in_msg.set_header("after", "should_not"); Ok(()) });
//! let list = RecipientList::new().add(ok).add(fail).add(after);
//! let route = Route::new().add(list).build();
//! let mut ex = Exchange::new(Message::from_text("payload"));
//! let res = {
//!     #[cfg(feature="async")]
//!     let r = tokio::runtime::Runtime::new().unwrap().block_on(route.run(&mut ex));
//!     #[cfg(not(feature="async"))]
//!     let r = route.run(&mut ex);
//!     r
//! };
//! assert!(res.is_err());
//! assert!(ex.in_msg.header("visited").is_some());
//! assert!(ex.in_msg.header("after").is_none());
//! ```
//!
//! # Testing Tips
//! * Verify ordering by setting incremental headers (`step=1`, `step=2`).
//! * Inject an error in the middle to assert short-circuit behavior.
//! * Confirm that later recipients can override `out_msg`.
//! * Edge case: empty `RecipientList` should be a no-op (returns `Ok(())`).
//! * Ensure that the same `Exchange` instance is passed to all recipients.

use crate::{error::Result, processor::Processor, Exchange};
use std::fmt::Debug;

#[derive(Default, Debug)]
pub struct RecipientList {
    recipients: Vec<Box<dyn Processor>>,
}

impl RecipientList {
    /// Creates a new, empty `RecipientList`.
    pub fn new() -> Self {
        Self {
            recipients: Vec::new(),
        }
    }

    /// Adds a processor to the recipient list.
    ///
    /// # Panics
    /// Panics if the processor cannot be boxed.
    pub fn add<P: Processor + 'static>(mut self, p: P) -> Self {
        self.recipients.push(Box::new(p));
        self
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl Processor for RecipientList {
    /// Processes the exchange by passing it through each recipient processor in order.
    ///
    /// # Errors
    /// Returns an error if any recipient processor returns an error.
    async fn process(&self, exchange: &mut Exchange) -> Result<()> {
        for r in &self.recipients {
            r.process(exchange).await?;
        }
        Ok(())
    }
}

#[cfg(not(feature = "async"))]
impl Processor for RecipientList {
    /// Processes the exchange by passing it through each recipient processor in order.
    ///
    /// # Errors
    /// Returns an error if any recipient processor returns an error.
    fn process(&self, exchange: &mut Exchange) -> Result<()> {
        for r in &self.recipients {
            r.process(exchange)?;
        }
        Ok(())
    }
}
