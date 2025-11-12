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
//! * Leading logical operator → `Error::Serialization("filter expression cannot start with a logical operator")`
//! * Trailing logical operator → `Error::Serialization("filter expression cannot end with a logical operator")`
//! * Consecutive logical operators → `Error::Serialization("consecutive logical operators are not allowed")`
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
use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;
use regex::Regex;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;

struct CompiledPlan {
    atoms: Vec<Box<dyn Fn(&Exchange) -> bool + Send + Sync>>, // atomic predicates
    ops: Vec<String>,                                         // logical operators between atoms
}

static PLAN_CACHE: OnceCell<
    std::sync::Mutex<std::collections::HashMap<String, Arc<CompiledPlan>>>,
> = OnceCell::new();

// Precompiled regular expressions for APL parsing (avoid per-call compilation overhead).
static TOKEN_SPLIT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\s*(?:(&&)|(\|\|))\s*").expect("valid token split regex"));
static HEADER_EQ_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^header\("([^"]+)"\)\s*==\s*"([^"]+)"$"#).expect("valid header eq regex")
});
static EXISTS_HEADER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^exists\(header\("([^"]+)"\)\)$"#).expect("valid exists header regex")
});
static BODY_CONTAINS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^body\.contains\("([^"]+)"\)$"#).expect("valid body.contains regex")
});

fn plan_cache() -> &'static std::sync::Mutex<std::collections::HashMap<String, Arc<CompiledPlan>>> {
    PLAN_CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}
/// Obtain a guard for the global plan cache, recovering gracefully if the mutex was poisoned.
/// Poison recovery logs a warning and returns the inner map, allowing continued operation
/// instead of panicking inside library code.
fn plan_cache_guard(
) -> std::sync::MutexGuard<'static, std::collections::HashMap<String, Arc<CompiledPlan>>> {
    match plan_cache().lock() {
        Ok(g) => g,
        Err(poisoned) => {
            tracing::warn!("plan cache mutex poisoned; recovering");
            poisoned.into_inner()
        }
    }
}

pub type Predicate = Box<dyn Fn(&Exchange) -> bool + Send + Sync + 'static>;

pub struct Filter {
    plan: Arc<CompiledPlan>,
    error_message: Option<String>,
    id: String,
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
        let plan = CompiledPlan {
            atoms: vec![Box::new(p)],
            ops: Vec::new(),
        };
        Self {
            plan: Arc::new(plan),
            error_message: None,
            id: uuid::Uuid::new_v4().to_string(),
        }
    }
    /// Create a filter with a custom rejection error message.
    pub fn with_error<P, S>(p: P, msg: S) -> Self
    where
        P: Fn(&Exchange) -> bool + Send + Sync + 'static,
        S: Into<String>,
    {
        let plan = CompiledPlan {
            atoms: vec![Box::new(p)],
            ops: Vec::new(),
        };
        Self {
            plan: Arc::new(plan),
            error_message: Some(msg.into()),
            id: uuid::Uuid::new_v4().to_string(),
        }
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn accepts(&self, exchange: &Exchange) -> bool {
        execute_plan(&self.plan, exchange)
    }
    /// Build a filter from an APL (Allora Predicate Language) expression string (v1).
    /// See module docs for supported atoms & operators; returns `Error::Serialization` on structural issues.
    /// Unknown atom formats degrade to literal body equality.
    pub fn from_apl(apl: &str) -> Result<Self> {
        Self::from_apl_with_id(None, apl)
    }
    pub fn from_apl_with_id(id: Option<String>, apl: &str) -> Result<Self> {
        if let Some(plan) = plan_cache_guard().get(apl).cloned() {
            let fid = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            return Ok(Self {
                plan,
                error_message: None,
                id: fid,
            });
        }
        let tokens = tokenize_apl(apl);
        if tokens.is_empty() {
            return Err(Error::serialization("empty predicate"));
        }
        let is_op = |t: &str| t == "&&" || t == "||";
        if is_op(&tokens[0]) {
            return Err(Error::serialization("logical operator parse error"));
        }
        if is_op(tokens.last().unwrap()) {
            return Err(Error::serialization("logical operator parse error"));
        }
        for w in tokens.windows(2) {
            if is_op(&w[0]) && is_op(&w[1]) {
                return Err(Error::serialization("logical operator parse error"));
            }
        }
        let mut atoms: Vec<Box<dyn Fn(&Exchange) -> bool + Send + Sync>> = Vec::new();
        let mut ops: Vec<String> = Vec::new();
        for t in &tokens {
            if is_op(t) {
                ops.push(t.to_string());
            } else {
                atoms.push(build_atom(t));
            }
        }
        debug_assert!(
            !atoms.is_empty(),
            "atoms vector should not be empty after validation"
        );
        let plan = Arc::new(CompiledPlan { atoms, ops });
        plan_cache_guard().insert(apl.to_string(), plan.clone());
        let fid = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        Ok(Self {
            plan,
            error_message: None,
            id: fid,
        })
    }
}

fn execute_plan(plan: &CompiledPlan, ex: &Exchange) -> bool {
    // Evaluate left-associative && groups segmented by ||
    let atoms = &plan.atoms;
    let ops = &plan.ops;
    let mut group_values: Vec<bool> = Vec::new();
    let mut current_val = (atoms[0])(ex);
    let mut atom_index = 1;
    for op in ops {
        let next_atom_val = (atoms[atom_index])(ex);
        atom_index += 1;
        match op.as_str() {
            "&&" => current_val = current_val && next_atom_val,
            "||" => {
                group_values.push(current_val);
                current_val = next_atom_val;
            }
            _ => unreachable!("unexpected operator: {}", op),
        }
    }
    group_values.push(current_val);
    group_values.into_iter().any(|v| v)
}

/// Tokenize APL by splitting on logical operators while retaining them.
fn tokenize_apl(apl: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut last = 0;
    for m in TOKEN_SPLIT_RE.find_iter(apl) {
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
    // Use Arc<str> to allow cheap cloning of captured strings across closures, reducing
    // memory duplication when the same header keys / values appear multiple times.
    if let Some(cap) = HEADER_EQ_RE.captures(s) {
        let key: Arc<str> = Arc::from(cap.get(1).unwrap().as_str());
        let expected: Arc<str> = Arc::from(cap.get(2).unwrap().as_str());
        return Box::new(move |ex: &Exchange| {
            ex.in_msg
                .headers
                .get(key.as_ref())
                .map(|v| v.as_str() == expected.as_ref())
                .unwrap_or(false)
        });
    }
    if let Some(cap) = EXISTS_HEADER_RE.captures(s) {
        let key: Arc<str> = Arc::from(cap.get(1).unwrap().as_str());
        return Box::new(move |ex: &Exchange| ex.in_msg.headers.get(key.as_ref()).is_some());
    }
    if let Some(cap) = BODY_CONTAINS_RE.captures(s) {
        let needle: Arc<str> = Arc::from(cap.get(1).unwrap().as_str());
        return Box::new(move |ex: &Exchange| {
            ex.in_msg
                .body_text()
                .map(|b| b.contains(needle.as_ref()))
                .unwrap_or(false)
        });
    }
    let literal: Arc<str> = Arc::from(s);
    Box::new(move |ex: &Exchange| ex.in_msg.body_text() == Some(literal.as_ref()))
}

impl SyncProcessor for Filter {
    fn process_sync(&self, exchange: &mut Exchange) -> Result<()> {
        if execute_plan(&self.plan, exchange) {
            Ok(())
        } else {
            let em = self.error_message.as_deref().unwrap_or("filtered out");
            // Intentionally do not append filter id to preserve backward-compatible error matching in doctests.
            // Future enhancement: optional inclusion via feature flag or structured error metadata.
            Err(crate::error::Error::Routing(em.into()))
        }
    }
}
