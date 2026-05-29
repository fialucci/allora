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
//! * Async-only: evaluates predicate and returns immediately; no internal blocking.
//!
//! # Construction APIs
//! * `Filter::new(|exchange| ...)` – custom closure predicate.
//! * `Filter::with_error(|exchange| ..., "reason")` – custom failure message.
//! * `Filter::from_apl("expression")` – parse Allora Predicate Language (APL) v1 string.
//!
//! # APL v1 Reference
//! Supported atoms:
//! * `header("Key") == "Value"` – exact header match (case sensitive key/value).
//! * `exists(header("Key"))` – header presence.
//! * `body.contains("Text")` – substring search in inbound body (text payload only).
//! * `body.json("$.foo.bar") == "value"` – parse the body as JSON, navigate
//!   to the field at `$.foo.bar`, compare its stringified primitive
//!   (`true`/`false` for bool, lexical form for number, contents for string,
//!   `"null"` for null) against the quoted literal. Accepts JSON Pointer
//!   (`/foo/bar`) too; both are normalized to RFC 6901 internally. Array
//!   indices via `$.items[0].name`.
//! * Fallback literal – any other token is treated as an exact body match.
//!
//! Operators & precedence:
//! * `&&` higher than `||`.
//! * Expression is segmented by `||`; each segment reduces left‑associative `&&` atoms; final result is OR of segment values.
//!
//! Unsupported (yet): parentheses, negation `!`, inequality `!=`, numeric comparisons, regex, JSONPath query operators (filters, wildcards).
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
//! ```rust
//! use allora_core::{patterns::filter::Filter, route::Route, Exchange, Message};
//! let route = Route::new().add(Filter::new(|exchange| exchange.in_msg.body_text() == Some("KEEP"))).build();
//! let mut exchange = Exchange::new(Message::from_text("KEEP"));
//! let rt = tokio::runtime::Runtime::new().unwrap();
//! rt.block_on(async { route.run(&mut exchange).await.unwrap(); });
//! ```
//!
//! APL expression:
//! ```
//! use allora_core::{patterns::filter::Filter, Exchange, Message};
//! let f = Filter::from_apl("header(\"x-bot\") == \"hello\" && body.contains(\"URGENT\")").unwrap();
//! let mut exchange = Exchange::new(Message::from_text("URGENT issue"));
//! exchange.in_msg.set_header("x-bot", "hello");
//! assert!(f.accepts(&exchange));
//! ```
//!
//! Custom error message:
//! ```rust
//! use allora_core::{patterns::filter::Filter, route::Route, Exchange, Message, Error};
//! let route = Route::new().add(Filter::with_error(|exchange| exchange.in_msg.body_text()==Some("OK"), "rejected")).build();
//! let mut exchange = Exchange::new(Message::from_text("BAD"));
//! let rt = tokio::runtime::Runtime::new().unwrap();
//! let res = rt.block_on(async { route.run(&mut exchange).await });
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
//! Parentheses, negation, numeric & regex matching, stricter atom validation,
//! richer JSONPath query operators. These will be versioned additions keeping
//! backward compatibility for existing APL specs.
//!
//! See also: [`Filter`], [`Filter::from_apl`], [`Exchange`], [`Error::Routing`].

use crate::error::{Error, Result};
use crate::Exchange;
use once_cell::sync::{Lazy, OnceCell};
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
// `body.json("$.foo.bar") == "value"` — parses the message body as JSON,
// navigates the JSONPath-style path to a primitive value, compares its
// string representation to the quoted literal.
//
// Accepts JSON Pointer syntax too (`/foo/bar`); the path normalizer
// converts both to RFC 6901 internally before calling
// `serde_json::Value::pointer`.
//
// Use cases:
//   when: body.json("$.action") == "closed"          (string field)
//   when: body.json("$.pull_request.merged") == "true"  (bool, stringified)
//   when: body.json("$.amount") == "100"             (number, stringified)
static BODY_JSON_EQ_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^body\.json\("([^"]+)"\)\s*==\s*"([^"]*)"$"#).expect("valid body.json eq regex")
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

fn execute_plan(plan: &CompiledPlan, exchange: &Exchange) -> bool {
    // Evaluate left-associative && groups segmented by ||
    let atoms = &plan.atoms;
    let ops = &plan.ops;
    let mut group_values: Vec<bool> = Vec::new();
    let mut current_val = (atoms[0])(exchange);
    let mut atom_index = 1;
    for op in ops {
        let next_atom_val = (atoms[atom_index])(exchange);
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
        return Box::new(move |exchange: &Exchange| {
            exchange
                .in_msg
                .headers
                .get(key.as_ref())
                .map(|v| v.as_str() == expected.as_ref())
                .unwrap_or(false)
        });
    }
    if let Some(cap) = EXISTS_HEADER_RE.captures(s) {
        let key: Arc<str> = Arc::from(cap.get(1).unwrap().as_str());
        return Box::new(move |exchange: &Exchange| {
            exchange.in_msg.headers.get(key.as_ref()).is_some()
        });
    }
    if let Some(cap) = BODY_CONTAINS_RE.captures(s) {
        let needle: Arc<str> = Arc::from(cap.get(1).unwrap().as_str());
        return Box::new(move |exchange: &Exchange| {
            exchange
                .in_msg
                .body_text()
                .map(|b| b.contains(needle.as_ref()))
                .unwrap_or(false)
        });
    }
    if let Some(cap) = BODY_JSON_EQ_RE.captures(s) {
        let raw_path: &str = cap.get(1).unwrap().as_str();
        let pointer: Arc<str> = Arc::from(jsonpath_to_pointer(raw_path));
        let expected: Arc<str> = Arc::from(cap.get(2).unwrap().as_str());
        return Box::new(move |exchange: &Exchange| {
            let body = match exchange.in_msg.body_text() {
                Some(b) if !b.trim().is_empty() => b,
                _ => return false,
            };
            // Parse the body each call. Filters in a chain (e.g. one
            // checking `$.a` then another checking `$.b`) re-parse the
            // same body — a caching layer on the Exchange would amortize
            // it, but the win on the POC's two-filter pipeline is
            // negligible (~tens of microseconds per webhook). Reconsider
            // if a hot path emerges.
            let value: serde_json::Value = match serde_json::from_str(body) {
                Ok(v) => v,
                Err(_) => return false,
            };
            let at_path = match value.pointer(pointer.as_ref()) {
                Some(v) => v,
                None => return false,
            };
            json_primitive_to_string(at_path) == expected.as_ref()
        });
    }
    let literal: Arc<str> = Arc::from(s);
    Box::new(move |exchange: &Exchange| exchange.in_msg.body_text() == Some(literal.as_ref()))
}

/// Convert a JSONPath-ish path (`$.foo.bar`, `$['foo']`, `$.array[0]`) to
/// the subset of RFC 6901 JSON Pointer syntax `serde_json::Value::pointer`
/// expects.
///
/// Supported input shapes:
///   - `$.foo.bar`           → `/foo/bar`
///   - `$.foo[0].bar`        → `/foo/0/bar`
///   - `/foo/bar` (passthrough, raw JSON Pointer)
///   - `foo.bar`             → `/foo/bar` (treated as JSONPath without `$.`)
///
/// Unsupported: filter expressions (`$..*`, `$.foo[?(@.bar)]`), wildcards.
/// Filters are EIP-style boolean predicates; complex query operators belong
/// in a Translator, not a Filter.
///
/// Returns the path verbatim (prefixed with `/`) on shapes we don't try to
/// rewrite — `serde_json::Value::pointer` will reject any path it can't
/// resolve, and the atom drops the message.
fn jsonpath_to_pointer(p: &str) -> String {
    // Pre-formed JSON Pointer: passthrough.
    if p.starts_with('/') {
        return p.to_string();
    }
    let body = p
        .strip_prefix("$.")
        .or_else(|| p.strip_prefix('$'))
        .unwrap_or(p);
    // Convert `foo.bar[0].baz` → `/foo/bar/0/baz`.
    let mut out = String::with_capacity(body.len() + 8);
    for raw_seg in body.split('.') {
        // `array[0]` → `array`, `0`.
        let mut remaining = raw_seg;
        while let Some(open) = remaining.find('[') {
            let (key, rest) = remaining.split_at(open);
            if !key.is_empty() {
                out.push('/');
                out.push_str(key);
            }
            // rest = `[N]...`; find the closing ].
            let rest = &rest[1..]; // strip `[`
            if let Some(close) = rest.find(']') {
                let idx = &rest[..close];
                out.push('/');
                out.push_str(idx);
                remaining = &rest[close + 1..];
            } else {
                // Malformed — emit the rest verbatim; pointer() will reject.
                out.push('/');
                out.push_str(rest);
                remaining = "";
            }
        }
        if !remaining.is_empty() {
            out.push('/');
            out.push_str(remaining);
        }
    }
    if out.is_empty() {
        out.push('/');
    }
    out
}

/// Convert a `serde_json::Value` primitive to a comparable string. The
/// Filter DSL's `==` compares against a quoted literal in the YAML; both
/// sides need to be strings.
///
/// | JSON value | String form |
/// |---|---|
/// | `null`           | `"null"` |
/// | `true` / `false` | `"true"` / `"false"` |
/// | number           | its lexical form (`"100"`, `"1.5"`) |
/// | string           | the inner contents (no surrounding quotes) |
/// | array / object   | the JSON-encoded form |
///
/// Strings strip their enclosing JSON quotes so a YAML predicate like
/// `body.json("$.action") == "closed"` matches a JSON value
/// `"action":"closed"` cleanly.
fn json_primitive_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[async_trait::async_trait]
impl crate::processor::Processor for Filter {
    async fn process(&self, exchange: &mut Exchange) -> crate::error::Result<()> {
        if execute_plan(&self.plan, exchange) {
            Ok(())
        } else {
            Err(Error::Routing(
                self.error_message
                    .clone()
                    .unwrap_or_else(|| "filtered out".into()),
            ))
        }
    }
}

#[cfg(test)]
mod body_json_eq_tests {
    //! Unit coverage for the `body.json("$.path") == "value"` Filter DSL
    //! atom. Verifies the JSONPath → RFC 6901 conversion, string-form
    //! comparison for primitive values, and graceful failure on bodies
    //! that aren't parseable JSON or don't have the requested path.

    use super::*;
    use crate::message::Message;

    fn ex_with_body(body: &str) -> Exchange {
        Exchange::new(Message::from_text(body))
    }

    fn build(when: &str) -> Filter {
        Filter::from_apl(when).expect("valid APL")
    }

    #[test]
    fn jsonpath_to_pointer_strips_dollar_and_dots() {
        assert_eq!(jsonpath_to_pointer("$.action"), "/action");
        assert_eq!(jsonpath_to_pointer("$.foo.bar"), "/foo/bar");
        assert_eq!(
            jsonpath_to_pointer("$.pull_request.merged"),
            "/pull_request/merged"
        );
    }

    #[test]
    fn jsonpath_to_pointer_handles_array_index() {
        assert_eq!(jsonpath_to_pointer("$.items[0]"), "/items/0");
        assert_eq!(jsonpath_to_pointer("$.items[0].name"), "/items/0/name");
        assert_eq!(jsonpath_to_pointer("$.x[1][2]"), "/x/1/2");
    }

    #[test]
    fn jsonpath_to_pointer_passes_through_rfc6901() {
        assert_eq!(jsonpath_to_pointer("/foo/bar"), "/foo/bar");
        assert_eq!(jsonpath_to_pointer("/"), "/");
    }

    #[test]
    fn string_field_match() {
        let f = build(r#"body.json("$.action") == "closed""#);
        assert!(f.accepts(&ex_with_body(r#"{"action":"closed"}"#)));
        assert!(!f.accepts(&ex_with_body(r#"{"action":"opened"}"#)));
    }

    #[test]
    fn nested_string_field_match() {
        let f = build(r#"body.json("$.pull_request.user.login") == "alice""#);
        let body = r#"{"pull_request":{"user":{"login":"alice"}}}"#;
        assert!(f.accepts(&ex_with_body(body)));
    }

    #[test]
    fn bool_field_stringified() {
        let f = build(r#"body.json("$.pull_request.merged") == "true""#);
        assert!(f.accepts(&ex_with_body(r#"{"pull_request":{"merged":true}}"#)));
        assert!(!f.accepts(&ex_with_body(r#"{"pull_request":{"merged":false}}"#)));
    }

    #[test]
    fn number_field_stringified() {
        let f = build(r#"body.json("$.amount") == "100""#);
        assert!(f.accepts(&ex_with_body(r#"{"amount":100}"#)));
        assert!(!f.accepts(&ex_with_body(r#"{"amount":101}"#)));
    }

    #[test]
    fn null_field_matches_null_literal() {
        let f = build(r#"body.json("$.opt") == "null""#);
        assert!(f.accepts(&ex_with_body(r#"{"opt":null}"#)));
    }

    #[test]
    fn missing_path_does_not_match() {
        let f = build(r#"body.json("$.missing") == "anything""#);
        assert!(!f.accepts(&ex_with_body(r#"{"present":1}"#)));
    }

    #[test]
    fn non_json_body_does_not_match() {
        let f = build(r#"body.json("$.action") == "closed""#);
        assert!(!f.accepts(&ex_with_body("not json at all")));
    }

    #[test]
    fn empty_body_does_not_match() {
        let f = build(r#"body.json("$.action") == "closed""#);
        assert!(!f.accepts(&ex_with_body("")));
    }

    #[test]
    fn combinable_with_and() {
        let f = build(
            r#"body.json("$.action") == "closed" && body.json("$.pull_request.merged") == "true""#,
        );
        assert!(f.accepts(&ex_with_body(
            r#"{"action":"closed","pull_request":{"merged":true}}"#
        )));
        assert!(!f.accepts(&ex_with_body(
            r#"{"action":"closed","pull_request":{"merged":false}}"#
        )));
        assert!(!f.accepts(&ex_with_body(
            r#"{"action":"opened","pull_request":{"merged":true}}"#
        )));
    }

    #[test]
    fn combinable_with_or() {
        let f =
            build(r#"body.json("$.action") == "closed" || body.json("$.action") == "synchronize""#);
        assert!(f.accepts(&ex_with_body(r#"{"action":"closed"}"#)));
        assert!(f.accepts(&ex_with_body(r#"{"action":"synchronize"}"#)));
        assert!(!f.accepts(&ex_with_body(r#"{"action":"opened"}"#)));
    }
}
