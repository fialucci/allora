//! [`PatternRegistry`]: runtime resolver from YAML-declared names to consumer-supplied
//! impls of Allora's pattern traits (`CompletionCondition`, `AggregationStrategy`,
//! `GroupStore`).
//!
//! # Motivation
//!
//! The chain's quorum logic (validator registry lookups, dynamic threshold over a
//! changing validator set, etc.) cannot be expressed in pure YAML — it needs Rust
//! code. The same is true for any non-trivial completion condition, custom
//! aggregation, or alternative storage backend. The
//! [`crate::spec::AggregatorSpec`] therefore refers to *names* — and those names
//! resolve here at build time. Mirrors how `processor_registry` works downstream
//! (Fialucci chain) for HTTP-route processor lookup.
//!
//! # Default Registrations
//!
//! [`PatternRegistry::with_defaults`] pre-populates Allora's three built-in
//! strategies, keyed by `allora.<name>`:
//!
//! * `allora.concat_text` → [`allora_core::patterns::aggregator::ConcatText`]
//! * `allora.json_array`  → [`allora_core::patterns::aggregator::JsonArray`]
//! * `allora.emit_signal` → [`allora_core::patterns::aggregator::EmitSignal`]
//!
//! No default *completions* or *stores* are registered — those are inherently
//! consumer-specific (the chain registers `chain.validator_quorum`, etc.).
//!
//! # Example
//! ```rust,no_run
//! use std::sync::Arc;
//! use allora_core::Message;
//! use allora_core::patterns::aggregator::CompletionCondition;
//! use allora_runtime::dsl::PatternRegistry;
//!
//! struct MyQuorum;
//! impl CompletionCondition for MyQuorum {
//!     fn is_complete(&self, group: &[Message], _: std::time::Instant) -> bool {
//!         group.len() >= 2
//!     }
//! }
//!
//! let mut registry = PatternRegistry::with_defaults();
//! registry.register_completion("chain.my_quorum", Arc::new(MyQuorum));
//! assert!(registry.completion("chain.my_quorum").is_some());
//! assert!(registry.strategy("allora.emit_signal").is_some());
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use allora_core::patterns::aggregator::{
    AggregationStrategy, CompletionCondition, ConcatText, EmitSignal, GroupStore, JsonArray,
};

/// Registry name for Allora's built-in [`ConcatText`] strategy.
pub const STRATEGY_CONCAT_TEXT: &str = "allora.concat_text";
/// Registry name for Allora's built-in [`JsonArray`] strategy.
pub const STRATEGY_JSON_ARRAY: &str = "allora.json_array";
/// Registry name for Allora's built-in [`EmitSignal`] strategy.
pub const STRATEGY_EMIT_SIGNAL: &str = "allora.emit_signal";

/// Resolver from YAML names → concrete pattern-component impls.
#[derive(Default, Clone)]
pub struct PatternRegistry {
    completions: HashMap<String, Arc<dyn CompletionCondition>>,
    strategies: HashMap<String, Arc<dyn AggregationStrategy>>,
    stores: HashMap<String, Arc<dyn GroupStore>>,
}

impl PatternRegistry {
    /// Empty registry — no defaults. Caller registers every impl explicitly.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registry pre-populated with Allora's built-in strategies:
    /// `allora.concat_text`, `allora.json_array`, `allora.emit_signal`.
    pub fn with_defaults() -> Self {
        let mut r = Self::default();
        r.register_strategy(STRATEGY_CONCAT_TEXT, Arc::new(ConcatText));
        r.register_strategy(STRATEGY_JSON_ARRAY, Arc::new(JsonArray));
        r.register_strategy(STRATEGY_EMIT_SIGNAL, Arc::new(EmitSignal));
        r
    }

    // ── Registration ─────────────────────────────────────────────────────────

    pub fn register_completion<N: Into<String>>(
        &mut self,
        name: N,
        impl_: Arc<dyn CompletionCondition>,
    ) {
        self.completions.insert(name.into(), impl_);
    }

    pub fn register_strategy<N: Into<String>>(
        &mut self,
        name: N,
        impl_: Arc<dyn AggregationStrategy>,
    ) {
        self.strategies.insert(name.into(), impl_);
    }

    pub fn register_store<N: Into<String>>(&mut self, name: N, impl_: Arc<dyn GroupStore>) {
        self.stores.insert(name.into(), impl_);
    }

    // ── Lookup ───────────────────────────────────────────────────────────────

    pub fn completion(&self, name: &str) -> Option<Arc<dyn CompletionCondition>> {
        self.completions.get(name).cloned()
    }

    pub fn strategy(&self, name: &str) -> Option<Arc<dyn AggregationStrategy>> {
        self.strategies.get(name).cloned()
    }

    pub fn store(&self, name: &str) -> Option<Arc<dyn GroupStore>> {
        self.stores.get(name).cloned()
    }

    // ── Diagnostics ──────────────────────────────────────────────────────────

    pub fn completion_names(&self) -> Vec<&str> {
        self.completions.keys().map(|s| s.as_str()).collect()
    }
    pub fn strategy_names(&self) -> Vec<&str> {
        self.strategies.keys().map(|s| s.as_str()).collect()
    }
    pub fn store_names(&self) -> Vec<&str> {
        self.stores.keys().map(|s| s.as_str()).collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use allora_core::Message;
    use std::time::Instant;

    struct CountAtLeast(usize);
    impl CompletionCondition for CountAtLeast {
        fn is_complete(&self, group: &[Message], _: Instant) -> bool {
            group.len() >= self.0
        }
    }

    #[test]
    fn empty_registry_returns_none_for_everything() {
        let r = PatternRegistry::new();
        assert!(r.completion("anything").is_none());
        assert!(r.strategy("anything").is_none());
        assert!(r.store("anything").is_none());
        assert!(r.completion_names().is_empty());
    }

    #[test]
    fn with_defaults_registers_all_three_built_in_strategies() {
        let r = PatternRegistry::with_defaults();
        assert!(r.strategy(STRATEGY_CONCAT_TEXT).is_some());
        assert!(r.strategy(STRATEGY_JSON_ARRAY).is_some());
        assert!(r.strategy(STRATEGY_EMIT_SIGNAL).is_some());
        // No default completions / stores.
        assert!(r.completion_names().is_empty());
        assert!(r.store_names().is_empty());
    }

    #[test]
    fn register_then_lookup_completion() {
        let mut r = PatternRegistry::new();
        r.register_completion("chain.test_quorum", Arc::new(CountAtLeast(3)));
        let c = r.completion("chain.test_quorum").expect("registered");
        let g: Vec<Message> = (0..3).map(|_| Message::default()).collect();
        assert!(c.is_complete(&g, Instant::now()));
        let g2: Vec<Message> = (0..2).map(|_| Message::default()).collect();
        assert!(!c.is_complete(&g2, Instant::now()));
    }

    #[test]
    fn registration_overwrites_existing_entry() {
        let mut r = PatternRegistry::new();
        r.register_completion("k", Arc::new(CountAtLeast(1)));
        r.register_completion("k", Arc::new(CountAtLeast(99)));
        let c = r.completion("k").unwrap();
        let g: Vec<Message> = (0..5).map(|_| Message::default()).collect();
        assert!(!c.is_complete(&g, Instant::now())); // second registration wins, threshold 99
    }
}
