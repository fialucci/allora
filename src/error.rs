//! Error types and result alias for Allora.
//!
//! # Overview
//! The library uses a single enum [`Error`] to classify failures across
//! processors, routing, aggregation, serialization, and miscellaneous cases.
//! A crate-wide `Result<T>` type alias simplifies return signatures.
//!
//! # Goals
//! * Lightweight – no deep error type hierarchy.
//! * Human-friendly `Display` messages via `thiserror`.
//! * Extensible – new variants can be added in a non-breaking way (add to end).
//!
//! # When to Use Each Variant
//! * `Processor` – logic inside a processor failed (validation, transformation).
//! * `Routing` – route selection / content-based routing issues (no match, invalid path).
//! * `Aggregation` – aggregator state or completion failures.
//! * `Serialization` – encoding/decoding payload/header data.
//! * `Other` – fallback / miscellaneous; prefer creating a dedicated variant when patterns emerge.
//!
//! # Adding New Variants
//! Add at the end of the enum to avoid changing numeric discriminants (though they are
//! not relied upon externally). Provide a clear `#[error(...)]` message and (optionally)
//! a convenience constructor method.
//!
//! # Example
//! ```
//! use allora::{Result, Error};
//! fn do_work(ok: bool) -> Result<()> {
//!     if ok { Ok(()) } else { Err(Error::processor("failed logic")) }
//! }
//! assert!(do_work(true).is_ok());
//! assert!(matches!(do_work(false), Err(Error::Processor(_))));
//! ```

use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("processor error: {0}")]
    /// Errors originating from processor logic (validation, transformation, enrichment).
    Processor(String),
    #[error("routing error: {0}")]
    /// Routing / selection failures (no content-based router match, invalid endpoint path).
    Routing(String),
    #[error("aggregation error: {0}")]
    /// Aggregation pattern related failures (missing bucket, premature completion, etc.).
    Aggregation(String),
    #[error("serialization error: {0}")]
    /// (De)serialization failures for payloads or headers (JSON, bytes conversion, etc.).
    Serialization(String),
    #[error("unknown error: {0}")]
    /// Catch-all variant for uncategorized errors; prefer creating a dedicated variant over time.
    Other(String),
}

impl Error {
    /// Convenience: create an `Other` error.
    pub fn other<S: Into<String>>(s: S) -> Self {
        Self::Other(s.into())
    }
    /// Convenience: create a `Processor` error.
    pub fn processor<S: Into<String>>(s: S) -> Self {
        Self::Processor(s.into())
    }
    /// Convenience: create a `Routing` error.
    pub fn routing<S: Into<String>>(s: S) -> Self {
        Self::Routing(s.into())
    }
    /// Convenience: create an `Aggregation` error.
    pub fn aggregation<S: Into<String>>(s: S) -> Self {
        Self::Aggregation(s.into())
    }
    /// Convenience: create a `Serialization` error.
    pub fn serialization<S: Into<String>>(s: S) -> Self {
        Self::Serialization(s.into())
    }
}
