//! Aggregator pattern: collect messages sharing a correlation header into a group, decide when the
//! group is complete via a pluggable [`CompletionCondition`], and fold the completed group into an
//! outbound [`Message`] via a pluggable [`AggregationStrategy`]. The group storage itself is also
//! pluggable via [`GroupStore`] (default: [`InMemoryGroupStore`]).
//!
//! # Composition
//! An aggregator is `correlation key + completion + strategy + store`. Built-in completions:
//! * [`BySize`] — completes when the group reaches `n` messages (the original behavior).
//! * [`ByWeight`] — completes when `Σ weight(msg) ≥ threshold`; the canonical weighted-quorum
//!   condition (e.g. validator voting power, oracle stake).
//! * [`ByPredicate`] — caller-supplied `Fn(&[Message]) -> bool`.
//! * [`ByTimeout`] — completes when the first message in the group is older than a [`Duration`].
//!
//! Built-in strategies:
//! * [`ConcatText`] — UTF-8 concatenation of every payload (legacy behavior); emits nothing if any
//!   payload in the group is non-text.
//! * [`JsonArray`] — emits a single [`Payload::Json`] message containing each payload mapped into
//!   a JSON array element.
//!
//! # Back-compat
//! `Aggregator::new("corr", n)` keeps its original semantics — equivalent to
//! `Aggregator::with_completion("corr", Arc::new(BySize(n)))` (with `ConcatText` + in-memory store).
//!
//! # Timeout semantics
//! [`ByTimeout`] is evaluated **lazily**: completion is only checked when a new message arrives.
//! Timer-driven completion (firing without a triggering message) requires an external tick / poll
//! and is out of scope for this version.
//!
//! # Persistence
//! [`GroupStore`] is the seam for durable storage. The default [`InMemoryGroupStore`] keeps
//! everything in a `Mutex<HashMap>`; downstream crates (e.g. the Fialucci chain) can provide a
//! disk-backed impl by implementing the trait.
//!
//! # Example — size completion (back-compat)
//! ```rust
//! use allora_core::{patterns::aggregator::Aggregator, route::Route, Exchange, Message};
//! let route = Route::new().add(Aggregator::new("corr", 2)).build();
//! let mut ex1 = Exchange::new(Message::from_text("A"));
//! ex1.in_msg.set_header("corr", "grp");
//! let rt = tokio::runtime::Runtime::new().unwrap();
//! rt.block_on(async { route.run(&mut ex1).await.unwrap(); });
//! assert!(ex1.out_msg.is_none());
//! let mut ex2 = Exchange::new(Message::from_text("B"));
//! ex2.in_msg.set_header("corr", "grp");
//! rt.block_on(async { route.run(&mut ex2).await.unwrap(); });
//! assert_eq!(ex2.out_msg.unwrap().body_text(), Some("AB"));
//! ```
//!
//! # Example — weighted-quorum completion (≥ 2/3 voting power)
//! ```rust
//! use allora_core::{patterns::aggregator::{Aggregator, JsonArray}, route::Route, Exchange, Message};
//! use std::sync::Arc;
//! // Three validators with weights 3, 3, 4 (total = 10, ⌈2/3·10⌉ = 7).
//! // First two votes sum to 6 (< 7); the third pushes total to 10 (≥ 7) and fires.
//! let threshold: u64 = 7;
//! let agg = Aggregator::weighted(
//!     "block",
//!     |m: &Message| m.header("voting_power").and_then(|s| s.parse().ok()).unwrap_or(0),
//!     threshold,
//! )
//! .with_strategy(Arc::new(JsonArray));
//! let route = Route::new().add(agg).build();
//! let rt = tokio::runtime::Runtime::new().unwrap();
//! for (vp, complete_expected) in [(3u64, false), (3, false), (4, true)] {
//!     let mut ex = Exchange::new(Message::from_text("vote"));
//!     ex.in_msg.set_header("block", "h=42");
//!     ex.in_msg.set_header("voting_power", vp.to_string());
//!     rt.block_on(async { route.run(&mut ex).await.unwrap(); });
//!     assert_eq!(ex.out_msg.is_some(), complete_expected);
//! }
//! ```

use crate::{error::Result, message::Message, message::Payload, processor::Processor, Exchange};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// CompletionCondition
// ---------------------------------------------------------------------------

/// Decides when an in-progress correlation group is complete.
///
/// Called after each message is appended to the group. Implementations should be cheap — they may
/// be called on every inbound message.
pub trait CompletionCondition: Send + Sync {
    /// Return `true` if `group` (the current accumulated messages, in arrival order) is complete.
    /// `first_seen` is when the first message of this group arrived; useful for time-based conditions.
    fn is_complete(&self, group: &[Message], first_seen: Instant) -> bool;
}

/// Size-threshold completion: complete once the group contains `n` messages.
#[derive(Debug, Clone, Copy)]
pub struct BySize(pub usize);

impl CompletionCondition for BySize {
    fn is_complete(&self, group: &[Message], _first_seen: Instant) -> bool {
        group.len() >= self.0
    }
}

/// Time-based completion: complete when the elapsed time since the first message ≥ `Duration`.
///
/// Evaluated lazily, on each new arrival — see the module doc.
#[derive(Debug, Clone, Copy)]
pub struct ByTimeout(pub Duration);

impl CompletionCondition for ByTimeout {
    fn is_complete(&self, _group: &[Message], first_seen: Instant) -> bool {
        first_seen.elapsed() >= self.0
    }
}

/// Caller-supplied predicate completion.
pub struct ByPredicate<F: Fn(&[Message]) -> bool + Send + Sync>(pub F);

impl<F> CompletionCondition for ByPredicate<F>
where
    F: Fn(&[Message]) -> bool + Send + Sync,
{
    fn is_complete(&self, group: &[Message], _first_seen: Instant) -> bool {
        (self.0)(group)
    }
}

/// Weighted-quorum completion: sum of `weight(msg)` over the group must reach `threshold`.
///
/// The canonical use case is voting-power quorums where each message carries its weight in a
/// header — e.g. `weight = |m| m.header("voting_power")?.parse().ok()`.
pub struct ByWeight<F: Fn(&Message) -> u64 + Send + Sync> {
    pub weight: F,
    pub threshold: u64,
}

impl<F> CompletionCondition for ByWeight<F>
where
    F: Fn(&Message) -> u64 + Send + Sync,
{
    fn is_complete(&self, group: &[Message], _first_seen: Instant) -> bool {
        group.iter().map(|m| (self.weight)(m)).sum::<u64>() >= self.threshold
    }
}

// ---------------------------------------------------------------------------
// AggregationStrategy
// ---------------------------------------------------------------------------

/// Folds a completed group into an outbound [`Message`]. Returning `None` means "the group is
/// complete but emit nothing" (e.g. `ConcatText` on non-text payloads).
pub trait AggregationStrategy: Send + Sync {
    fn combine(&self, group: Vec<Message>) -> Option<Message>;
}

/// UTF-8 text concatenation. Emits `None` if any payload in the group is not [`Payload::Text`] —
/// preserves the legacy aggregator behavior.
#[derive(Debug, Clone, Copy, Default)]
pub struct ConcatText;

impl AggregationStrategy for ConcatText {
    fn combine(&self, group: Vec<Message>) -> Option<Message> {
        if !group.iter().all(|m| m.body_text().is_some()) {
            return None;
        }
        let concat: String = group.iter().map(|m| m.body_text().unwrap()).collect();
        Some(Message::from_text(concat))
    }
}

/// JSON-array carry: emit a single `Payload::Json` message whose body is a JSON array, with each
/// element mapped from the corresponding payload:
/// * `Text(s)` → `"s"`
/// * `Bytes(b)` → `[u8, u8, …]`
/// * `Json(v)` → `v`
/// * `Empty` → `null`
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonArray;

/// Always emits an empty [`Message`] when the completion condition fires. Use when the aggregator
/// is being treated as a pure "quorum reached" signal — the caller doesn't care about the group's
/// contents (typically it re-reads authoritative state from elsewhere on the signal).
#[derive(Debug, Clone, Copy, Default)]
pub struct EmitSignal;

impl AggregationStrategy for EmitSignal {
    fn combine(&self, _group: Vec<Message>) -> Option<Message> {
        Some(Message::default())
    }
}

impl AggregationStrategy for JsonArray {
    fn combine(&self, group: Vec<Message>) -> Option<Message> {
        let arr: Vec<serde_json::Value> = group
            .into_iter()
            .map(|m| match m.payload {
                Payload::Text(s) => serde_json::Value::String(s),
                Payload::Bytes(b) => {
                    serde_json::Value::Array(b.into_iter().map(serde_json::Value::from).collect())
                }
                Payload::Json(v) => v,
                Payload::Empty => serde_json::Value::Null,
            })
            .collect();
        Some(Message::new(Payload::Json(serde_json::Value::Array(arr))))
    }
}

// ---------------------------------------------------------------------------
// GroupStore
// ---------------------------------------------------------------------------

/// Pluggable storage for in-progress correlation groups. The default [`InMemoryGroupStore`] keeps
/// groups in a `Mutex<HashMap>`; persistent implementations can plug in via [`Aggregator::with_store`].
pub trait GroupStore: Send + Sync {
    /// Append `msg` to the group keyed by `key`, returning a snapshot of the group (post-append)
    /// and the [`Instant`] the group was first seen.
    fn append(&self, key: &str, msg: Message) -> (Vec<Message>, Instant);
    /// Remove and return the group keyed by `key`. Called once completion fires.
    fn take(&self, key: &str) -> Option<Vec<Message>>;
    /// Drop every in-progress group. Primarily a test utility.
    fn clear(&self);
}

struct InMemoryGroup {
    messages: Vec<Message>,
    first_seen: Instant,
}

/// Default in-process group store (one `Mutex<HashMap>`).
#[derive(Default)]
pub struct InMemoryGroupStore {
    inner: Mutex<HashMap<String, InMemoryGroup>>,
}

impl InMemoryGroupStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl GroupStore for InMemoryGroupStore {
    fn append(&self, key: &str, msg: Message) -> (Vec<Message>, Instant) {
        let mut guard = self.inner.lock().unwrap();
        let entry = guard
            .entry(key.to_string())
            .or_insert_with(|| InMemoryGroup {
                messages: Vec::new(),
                first_seen: Instant::now(),
            });
        entry.messages.push(msg);
        (entry.messages.clone(), entry.first_seen)
    }
    fn take(&self, key: &str) -> Option<Vec<Message>> {
        let mut guard = self.inner.lock().unwrap();
        guard.remove(key).map(|g| g.messages)
    }
    fn clear(&self) {
        let mut guard = self.inner.lock().unwrap();
        guard.clear();
    }
}

// ---------------------------------------------------------------------------
// Aggregator
// ---------------------------------------------------------------------------

/// EIP Aggregator: groups messages by a correlation header, completes via a [`CompletionCondition`],
/// and emits the folded result via an [`AggregationStrategy`]. Storage is pluggable via [`GroupStore`].
#[derive(Clone)]
pub struct Aggregator {
    correlation_header: String,
    completion: Arc<dyn CompletionCondition>,
    strategy: Arc<dyn AggregationStrategy>,
    store: Arc<dyn GroupStore>,
}

impl fmt::Debug for Aggregator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Aggregator")
            .field("correlation_header", &self.correlation_header)
            .finish_non_exhaustive()
    }
}

impl Aggregator {
    /// Back-compat constructor: size-threshold completion, text-concat strategy, in-memory store.
    pub fn new<H: Into<String>>(correlation_header: H, completion_size: usize) -> Self {
        Self::with_completion(correlation_header, Arc::new(BySize(completion_size)))
    }

    /// Generic constructor with an arbitrary completion condition. Defaults: [`ConcatText`]
    /// strategy + [`InMemoryGroupStore`]. Override either with [`Aggregator::with_strategy`] /
    /// [`Aggregator::with_store`].
    pub fn with_completion<H: Into<String>>(
        correlation_header: H,
        completion: Arc<dyn CompletionCondition>,
    ) -> Self {
        Self {
            correlation_header: correlation_header.into(),
            completion,
            strategy: Arc::new(ConcatText),
            store: Arc::new(InMemoryGroupStore::new()),
        }
    }

    /// Convenience: weighted-quorum completion (`Σ weight(msg) ≥ threshold`).
    pub fn weighted<H, F>(correlation_header: H, weight: F, threshold: u64) -> Self
    where
        H: Into<String>,
        F: Fn(&Message) -> u64 + Send + Sync + 'static,
    {
        Self::with_completion(correlation_header, Arc::new(ByWeight { weight, threshold }))
    }

    /// Convenience: lazy timeout completion (see [`ByTimeout`]).
    pub fn timed<H: Into<String>>(correlation_header: H, dur: Duration) -> Self {
        Self::with_completion(correlation_header, Arc::new(ByTimeout(dur)))
    }

    /// Convenience: predicate completion.
    pub fn when<H, F>(correlation_header: H, predicate: F) -> Self
    where
        H: Into<String>,
        F: Fn(&[Message]) -> bool + Send + Sync + 'static,
    {
        Self::with_completion(correlation_header, Arc::new(ByPredicate(predicate)))
    }

    /// Replace the aggregation strategy. Returns `self` for chaining.
    pub fn with_strategy(mut self, strategy: Arc<dyn AggregationStrategy>) -> Self {
        self.strategy = strategy;
        self
    }

    /// Replace the group store (e.g. swap in a persistent impl). Returns `self` for chaining.
    pub fn with_store(mut self, store: Arc<dyn GroupStore>) -> Self {
        self.store = store;
        self
    }

    /// Clear every in-progress group. Intended for test isolation; not usually needed in production.
    pub fn clear_store(&self) {
        self.store.clear();
    }
}

#[async_trait::async_trait]
impl Processor for Aggregator {
    async fn process(&self, exchange: &mut Exchange) -> Result<()> {
        let key = match exchange.in_msg.header(&self.correlation_header) {
            Some(k) => k.to_string(),
            None => return Ok(()),
        };
        let (group, first_seen) = self.store.append(&key, exchange.in_msg.clone());
        if self.completion.is_complete(&group, first_seen) {
            if let Some(completed) = self.store.take(&key) {
                if let Some(out) = self.strategy.combine(completed) {
                    exchange.out_msg = Some(out);
                }
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Exchange, Message, Payload};
    use crate::route::Route;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Block-on helper matching the existing aggregator tests' style.
    fn run(route: &Route, exchange: &mut Exchange) {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(route.run(exchange))
            .unwrap();
    }

    fn ex_with(header: &str, key: &str, msg: Message) -> Exchange {
        let mut e = Exchange::new(msg);
        e.in_msg.set_header(header, key);
        e
    }

    // ---- back-compat ------------------------------------------------------

    #[test]
    fn back_compat_size_two_concats_ab() {
        let route = Route::new().add(Aggregator::new("corr", 2)).build();
        let mut ex1 = ex_with("corr", "g", Message::from_text("A"));
        run(&route, &mut ex1);
        assert!(ex1.out_msg.is_none());
        let mut ex2 = ex_with("corr", "g", Message::from_text("B"));
        run(&route, &mut ex2);
        assert_eq!(ex2.out_msg.unwrap().body_text(), Some("AB"));
    }

    #[test]
    fn back_compat_three_messages() {
        let route = Route::new().add(Aggregator::new("corr", 3)).build();
        let mut last = None;
        for s in ["A", "B", "C"] {
            let mut ex = ex_with("corr", "123", Message::from_text(s));
            run(&route, &mut ex);
            last = Some(ex);
        }
        assert_eq!(last.unwrap().out_msg.unwrap().body_text(), Some("ABC"));
    }

    #[test]
    fn ignores_messages_without_correlation_header() {
        let route = Route::new().add(Aggregator::new("corr", 2)).build();
        for s in ["A", "B"] {
            let mut ex = Exchange::new(Message::from_text(s));
            run(&route, &mut ex);
            assert!(ex.out_msg.is_none());
        }
    }

    #[test]
    fn aggregates_multiple_batches_for_same_key() {
        let route = Route::new().add(Aggregator::new("corr", 2)).build();
        // First batch
        let mut ex1 = ex_with("corr", "same", Message::from_text("A"));
        run(&route, &mut ex1);
        assert!(ex1.out_msg.is_none());
        let mut ex2 = ex_with("corr", "same", Message::from_text("B"));
        run(&route, &mut ex2);
        assert_eq!(ex2.out_msg.as_ref().unwrap().body_text(), Some("AB"));
        // Second batch — store must have been cleared on completion
        let mut ex3 = ex_with("corr", "same", Message::from_text("C"));
        run(&route, &mut ex3);
        assert!(ex3.out_msg.is_none());
        let mut ex4 = ex_with("corr", "same", Message::from_text("D"));
        run(&route, &mut ex4);
        assert_eq!(ex4.out_msg.as_ref().unwrap().body_text(), Some("CD"));
    }

    #[test]
    fn concat_text_non_text_group_emits_nothing() {
        // Two non-text messages: completion fires, but ConcatText returns None.
        let route = Route::new().add(Aggregator::new("corr", 2)).build();
        let mut ex1 = ex_with("corr", "m", Message::new(Payload::Bytes(vec![0, 1])));
        run(&route, &mut ex1);
        let mut ex2 = ex_with("corr", "m", Message::new(Payload::Bytes(vec![2, 3])));
        run(&route, &mut ex2);
        assert!(ex2.out_msg.is_none());
    }

    #[test]
    fn clear_store_resets_groups() {
        let agg = Aggregator::new("corr", 2);
        let route = Route::new().add(agg.clone()).build();
        let mut ex1 = ex_with("corr", "x", Message::from_text("A"));
        run(&route, &mut ex1);
        agg.clear_store();
        let mut ex2 = ex_with("corr", "x", Message::from_text("B"));
        run(&route, &mut ex2);
        assert!(
            ex2.out_msg.is_none(),
            "clear_store should reset the group; B should be the first of a new batch"
        );
    }

    // ---- new completion conditions ---------------------------------------

    #[test]
    fn by_weight_completes_at_threshold() {
        // Three validators with weights 3, 3, 4 (total = 10, ⌈2/3·10⌉ = 7).
        // Sum trajectory: 3 (no) → 6 (no) → 10 (yes; fires on the 3rd vote).
        let threshold: u64 = 7;
        let route = Route::new()
            .add(Aggregator::weighted(
                "block",
                |m: &Message| {
                    m.header("voting_power")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0)
                },
                threshold,
            ))
            .build();

        for (vp, expect_out) in [(3u64, false), (3, false), (4, true)] {
            let mut ex = Exchange::new(Message::from_text(format!("vote-vp{vp}")));
            ex.in_msg.set_header("block", "h=42");
            ex.in_msg.set_header("voting_power", vp.to_string());
            run(&route, &mut ex);
            assert_eq!(
                ex.out_msg.is_some(),
                expect_out,
                "vp={vp}: expected out_msg={expect_out}"
            );
        }
    }

    #[test]
    fn by_weight_fires_exactly_at_threshold_boundary() {
        // Verify `≥ threshold` semantics: equal sum should fire.
        let route = Route::new()
            .add(Aggregator::weighted(
                "block",
                |m: &Message| {
                    m.header("voting_power")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0)
                },
                6,
            ))
            .build();
        // 3 + 3 = 6 → fires on the second vote.
        let mut ex1 = Exchange::new(Message::from_text("a"));
        ex1.in_msg.set_header("block", "h=1");
        ex1.in_msg.set_header("voting_power", "3");
        run(&route, &mut ex1);
        assert!(ex1.out_msg.is_none());
        let mut ex2 = Exchange::new(Message::from_text("b"));
        ex2.in_msg.set_header("block", "h=1");
        ex2.in_msg.set_header("voting_power", "3");
        run(&route, &mut ex2);
        assert!(ex2.out_msg.is_some(), "sum=6, threshold=6: should fire");
    }

    #[test]
    fn by_weight_isolated_per_key() {
        let route = Route::new()
            .add(Aggregator::weighted(
                "block",
                |m: &Message| {
                    m.header("voting_power")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0)
                },
                4,
            ))
            .build();

        // Two votes on block "A" (2 + 2 = 4, completes) and two on "B" (1 + 1 = 2, no completion).
        for (block, vp, expect) in [
            ("A", 2, false),
            ("B", 1, false),
            ("A", 2, true),
            ("B", 1, false),
        ] {
            let mut ex = Exchange::new(Message::from_text("v"));
            ex.in_msg.set_header("block", block);
            ex.in_msg.set_header("voting_power", vp.to_string());
            run(&route, &mut ex);
            assert_eq!(ex.out_msg.is_some(), expect, "block={block} vp={vp}");
        }
    }

    #[test]
    fn by_predicate_completes() {
        // Completes when the group contains a message with body "STOP".
        let route = Route::new()
            .add(Aggregator::when("corr", |g: &[Message]| {
                g.iter().any(|m| m.body_text() == Some("STOP"))
            }))
            .build();
        let mut ex1 = ex_with("corr", "x", Message::from_text("go"));
        run(&route, &mut ex1);
        assert!(ex1.out_msg.is_none());
        let mut ex2 = ex_with("corr", "x", Message::from_text("STOP"));
        run(&route, &mut ex2);
        assert_eq!(ex2.out_msg.as_ref().unwrap().body_text(), Some("goSTOP"));
    }

    #[test]
    fn by_timeout_lazy_completes_on_next_arrival() {
        // Lazy timeout: completion only fires when the *next* message arrives after the deadline.
        let route = Route::new()
            .add(Aggregator::timed("corr", Duration::from_millis(40)))
            .build();
        let mut ex1 = ex_with("corr", "t", Message::from_text("A"));
        run(&route, &mut ex1);
        assert!(ex1.out_msg.is_none(), "first message: deadline not reached");
        // Within the window — still no completion.
        let mut ex2 = ex_with("corr", "t", Message::from_text("B"));
        run(&route, &mut ex2);
        assert!(ex2.out_msg.is_none(), "B arrived too soon");
        // Sleep past the deadline, then send C — completion should fire and ConcatText emits "ABC".
        std::thread::sleep(Duration::from_millis(60));
        let mut ex3 = ex_with("corr", "t", Message::from_text("C"));
        run(&route, &mut ex3);
        assert_eq!(ex3.out_msg.as_ref().unwrap().body_text(), Some("ABC"));
    }

    // ---- aggregation strategies ------------------------------------------

    #[test]
    fn json_array_strategy_emits_array_of_mixed_payloads() {
        let route = Route::new()
            .add(Aggregator::new("corr", 4).with_strategy(Arc::new(JsonArray)))
            .build();
        let mut ex1 = ex_with("corr", "j", Message::from_text("hi"));
        run(&route, &mut ex1);
        let mut ex2 = ex_with("corr", "j", Message::new(Payload::Bytes(vec![1, 2])));
        run(&route, &mut ex2);
        let mut ex3 = ex_with(
            "corr",
            "j",
            Message::new(Payload::Json(serde_json::json!({"k": "v"}))),
        );
        run(&route, &mut ex3);
        let mut ex4 = ex_with("corr", "j", Message::new(Payload::Empty));
        run(&route, &mut ex4);

        let out = ex4
            .out_msg
            .expect("JsonArray must always emit on completion");
        let Payload::Json(serde_json::Value::Array(arr)) = out.payload else {
            panic!("JsonArray strategy must emit Payload::Json(Array)");
        };
        assert_eq!(arr.len(), 4);
        assert_eq!(arr[0], serde_json::Value::String("hi".into()));
        assert_eq!(arr[1], serde_json::json!([1, 2]));
        assert_eq!(arr[2], serde_json::json!({"k": "v"}));
        assert_eq!(arr[3], serde_json::Value::Null);
    }

    // ---- pluggable group store -------------------------------------------

    /// Counting wrapper around the in-memory store — proves `with_store` swaps in a custom impl.
    struct CountingStore {
        inner: InMemoryGroupStore,
        appends: AtomicUsize,
        takes: AtomicUsize,
    }
    impl CountingStore {
        fn new() -> Self {
            Self {
                inner: InMemoryGroupStore::new(),
                appends: AtomicUsize::new(0),
                takes: AtomicUsize::new(0),
            }
        }
    }
    impl GroupStore for CountingStore {
        fn append(&self, key: &str, msg: Message) -> (Vec<Message>, Instant) {
            self.appends.fetch_add(1, Ordering::SeqCst);
            self.inner.append(key, msg)
        }
        fn take(&self, key: &str) -> Option<Vec<Message>> {
            self.takes.fetch_add(1, Ordering::SeqCst);
            self.inner.take(key)
        }
        fn clear(&self) {
            self.inner.clear();
        }
    }

    #[test]
    fn custom_group_store_is_used() {
        let store = Arc::new(CountingStore::new());
        let route = Route::new()
            .add(Aggregator::new("corr", 2).with_store(store.clone()))
            .build();
        let mut ex1 = ex_with("corr", "k", Message::from_text("A"));
        run(&route, &mut ex1);
        let mut ex2 = ex_with("corr", "k", Message::from_text("B"));
        run(&route, &mut ex2);
        assert_eq!(ex2.out_msg.as_ref().unwrap().body_text(), Some("AB"));
        assert_eq!(store.appends.load(Ordering::SeqCst), 2);
        assert_eq!(store.takes.load(Ordering::SeqCst), 1);
    }
}
