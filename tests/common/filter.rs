//! Common test helpers for filter-related tests.
//! Centralizes small utility functions to avoid duplication across multiple
//! filter DSL test files.
//!
//! Functions:
//! * `apply` – run a filter against a synthetic text body, returning the predicate result.
//!
//! Keeping helpers here makes future additions (e.g. building an Exchange with headers) available
//! in one place.

use allora::{patterns::filter::Filter, Exchange, Message};

/// Apply a `Filter` to a new `Exchange` built from a text body.
#[allow(dead_code)]
pub fn apply(filter: &Filter, body: &str) -> bool {
    let ex = Exchange::new(Message::from_text(body));
    filter.accepts(&ex)
}
