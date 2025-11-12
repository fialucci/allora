//! Filter: evaluates a boolean predicate over an `Exchange` to decide if routing continues.
//!
//! Implements the Enterprise Integration Pattern (EIP) "Message Filter".
//! If the predicate returns `true` the downstream processors execute; if `false`,
//! processing stops with `Error::Routing` (no mutation performed).
//!
//! # When to Use
//! * Reject early to save downstream work.
//! * Isolate routing criteria from business logic.
//! * Combine with other patterns (correlation initializer, splitter, router).
//!
//! # Behavior Summary
//! * Predicate receives `&Exchange` (immutable view of inbound & optional outbound).
//! * Success: `Ok(())`; Failure: `Err(Error::Routing(..))`.
//! * Filter never mutates headers, body, or outbound message.
//! * Works in sync and async builds (same struct, auto adaptation under `async` feature).
//!
//! # Construction APIs
//! * `Filter::new(|ex| ...)` – custom closure predicate.
//! * `Filter::with_error(|ex| ..., "reason")` – custom failure message.
//! * `Filter::from_apl("expression")` – parse Allora Predicate Language (APL) v1 string.
//!
//! # APL v1 Reference
//! Supported atoms:
//! * `header("Key") == "Value"` – exact header match (case sensitive key/value).
//! * `exists(header("Key"))` – header presence.
//! * `body.contains("Text")` – substring search in inbound body (text payload only).
//! * Fallback literal – any other token is treated as an exact body match.
//!
//! Operators & precedence:
//! * `&&` higher than `||`.
//! * Expression is segmented by `||`; each segment reduces left‑associative `&&` atoms; final result is OR of segment values.
//!
//! Unsupported (yet): parentheses, negation `!`, inequality `!=`, numeric comparisons, regex, JSON path navigation.
//!
//! Error conditions (APL):
//! * Empty expression → `Error::Serialization("empty predicate")`
//! * Leading/trailing or consecutive logical operators → `Error::Serialization("logical operator parse error")`
//! * Unrecognized atom pattern → treated as literal body equality (future versions may tighten and error instead).
//!
//! # Examples
//! Basic closure:
//! ```
//! use allora::{patterns::filter::Filter, route::Route, Exchange, Message};
//! let route = Route::new().add(Filter::new(|ex| ex.in_msg.body_text() == Some("KEEP"))).build();
//! let mut ex = Exchange::new(Message::from_text("KEEP"));
//! #[cfg(feature="async")]
//! tokio::runtime::Runtime::new().unwrap().block_on(async { route.run(&mut ex).await.unwrap(); });
//! #[cfg(not(feature="async"))]
//! route.run(&mut ex).unwrap();
//! ```
//!
//! APL expression:
//! ```
//! use allora::{patterns::filter::Filter, Exchange, Message};
//! let f = Filter::from_apl("header(\"x-bot\") == \"hello\" && body.contains(\"URGENT\")").unwrap();
//! let mut ex = Exchange::new(Message::from_text("URGENT issue"));
//! ex.in_msg.set_header("x-bot", "hello");
//! assert!(f.accepts(&ex));
//! ```
//!
//! Custom error message:
//! ```
//! use allora::{patterns::filter::Filter, route::Route, Exchange, Message, Error};
//! let route = Route::new().add(Filter::with_error(|ex| ex.in_msg.body_text()==Some("OK"), "rejected")).build();
//! let mut ex = Exchange::new(Message::from_text("BAD"));
//! #[cfg(feature="async")]
//! let res = tokio::runtime::Runtime::new().unwrap().block_on(async { route.run(&mut ex).await });
//! #[cfg(not(feature="async"))]
//! let res = route.run(&mut ex);
//! assert!(matches!(res, Err(Error::Routing(msg)) if msg=="rejected"));
//! ```
//!
//! # Testing Tips
//! * Provide accept & reject cases for each atom type.
//! * Exhaustive truth table for multi‑condition expressions.
//! * Assert Exchange immutability (headers/body unchanged).
//! * Test precedence: `A && B || C` vs `A || B && C`.
//!
//! # Roadmap
//! Parentheses, negation, JSON body paths, numeric & regex matching, stricter atom validation.
//! These will be versioned additions keeping backward compatibility for existing APL specs.
//!
//! See also: [`Filter`], [`Filter::from_apl`], [`Exchange`], [`Error::Routing`].

use crate::error::{Error, Result};
use crate::{processor::SyncProcessor, Exchange};
use regex::Regex;
use std::fmt::{Debug, Formatter, Result as FmtResult};

pub type Predicate = Box<dyn Fn(&Exchange) -> bool + Send + Sync + 'static>;

pub struct Filter {
    predicate: Predicate,
    error_message: Option<String>,
}

impl Debug for Filter {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str("Filter{predicate=*closure*}")
    }
}

impl Filter {
    pub fn new<P>(p: P) -> Self
    where
        P: Fn(&Exchange) -> bool + Send + Sync + 'static,
    {
        Self {
            predicate: Box::new(p),
            error_message: None,
        }
    }
    /// Create a filter with a custom rejection error message.
    pub fn with_error<P, S>(p: P, msg: S) -> Self
    where
        P: Fn(&Exchange) -> bool + Send + Sync + 'static,
        S: Into<String>,
    {
        Self {
            predicate: Box::new(p),
            error_message: Some(msg.into()),
        }
    }
    pub fn accepts(&self, exchange: &Exchange) -> bool {
        (self.predicate)(exchange)
    }
    /// Build a filter from an APL (Allora Predicate Language) expression string (v1).
    /// See module docs for supported atoms & operators; returns `Error::Serialization` on structural issues.
    /// Unknown atom formats degrade to literal body equality.
    pub fn from_apl(apl: &str) -> Result<Self> {
        let tokens = tokenize_apl(apl);
        if tokens.is_empty() {
            return Err(Error::serialization("empty predicate"));
        }
        let is_op = |t: &str| t == "&&" || t == "||";
        if is_op(&tokens[0]) {
            return Err(Error::serialization("filter expression cannot start with a logical operator"));
        }
        if is_op(tokens.last().unwrap()) {
            return Err(Error::serialization("filter expression cannot end with a logical operator"));
        }
        for w in tokens.windows(2) {
            if is_op(&w[0]) && is_op(&w[1]) {
                return Err(Error::serialization("consecutive logical operators are not allowed"));
            }
        }
        let mut atoms: Vec<Box<dyn Fn(&Exchange) -> bool + Send + Sync>> = Vec::new();
        let mut ops: Vec<String> = Vec::new();
        for t in &tokens {
            if is_op(t) {
                ops.push(t.clone());
            } else {
                atoms.push(build_atom(t));
            }
        }
        // Build index mapping: atoms interleaved with ops. Implement precedence: evaluate groups separated by ||.
        let predicate = move |ex: &Exchange| {
            // Atoms is guaranteed non-empty due to validation above (lines 133-144).
            debug_assert!(!atoms.is_empty(), "atoms should never be empty after validation");
            // Evaluate left-associative && groups.
            let mut group_values: Vec<bool> = Vec::new();
            let mut current_val = (atoms[0])(ex);
            let mut atom_index = 1; // next atom index
            for op in &ops {
                let next_atom_val = (atoms[atom_index])(ex);
                atom_index += 1;
                match op.as_str() {
                    "&&" => {
                        current_val = current_val && next_atom_val;
                    }
                    "||" => {
                        group_values.push(current_val);
                        current_val = next_atom_val;
                    }
                    _ => {}
                }
            }
            group_values.push(current_val);
            // OR reduction
            group_values.into_iter().any(|v| v)
        };
        Ok(Filter {
            predicate: Box::new(predicate),
            error_message: None,
        })
    }
}

/// Tokenize APL by splitting on logical operators while retaining them.
fn tokenize_apl(apl: &str) -> Vec<&str> {
    let re = Regex::new(r"\s*(?:(&&)|(\|\|))\s*").unwrap();
    let mut parts = Vec::new();
    let mut last = 0;
    for m in re.find_iter(apl) {
        let slice = apl[last..m.start()].trim();
        if !slice.is_empty() {
            parts.push(slice);
        }
        let op = apl[m.start()..m.end()].trim();
        parts.push(op);
        last = m.end();
    }
    let tail = apl[last..].trim();
    if !tail.is_empty() {
        parts.push(tail);
    }
    parts
}

/// Build an atomic predicate closure from a raw token.
fn build_atom(raw: &str) -> Box<dyn Fn(&Exchange) -> bool + Send + Sync> {
    let s = raw.trim();
    if let Some(cap) = Regex::new(r#"^header\("([^"]+)"\)\s*==\s*"([^"]+)"$"#)
        .unwrap()
        .captures(s)
    {
        let key = cap.get(1).unwrap().as_str().to_string();
        let expected = cap.get(2).unwrap().as_str().to_string();
        return Box::new(move |ex: &Exchange| {
            ex.in_msg
                .headers
                .get(&key)
                .map(|v| v == &expected)
                .unwrap_or(false)
        });
    }
    if let Some(cap) = Regex::new(r#"^exists\(header\("([^"]+)"\)\)$"#)
        .unwrap()
        .captures(s)
    {
        let key = cap.get(1).unwrap().as_str().to_string();
        return Box::new(move |ex: &Exchange| ex.in_msg.headers.get(&key).is_some());
    }
    if let Some(cap) = Regex::new(r#"^body\.contains\("([^"]+)"\)$"#)
        .unwrap()
        .captures(s)
    {
        let needle = cap.get(1).unwrap().as_str().to_string();
        return Box::new(move |ex: &Exchange| {
            ex.in_msg
                .body_text()
                .map(|b| b.contains(&needle))
                .unwrap_or(false)
        });
    }
    let literal = s.to_string();
    Box::new(move |ex: &Exchange| ex.in_msg.body_text() == Some(literal.as_str()))
}

impl SyncProcessor for Filter {
    fn process_sync(&self, exchange: &mut Exchange) -> Result<()> {
        if self.accepts(exchange) {
            Ok(())
        } else {
            let em = self.error_message.as_deref().unwrap_or("filtered out");
            Err(crate::error::Error::Routing(em.into()))
        }
    }
}
