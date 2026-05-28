//! `AggregatorSpec`: specification for a single EIP Aggregator instance.
//!
//! An aggregator is declared in `allora.yaml` by id; it references consumer-supplied
//! implementations of [`allora_core::patterns::aggregator::CompletionCondition`],
//! [`allora_core::patterns::aggregator::AggregationStrategy`], and
//! [`allora_core::patterns::aggregator::GroupStore`] **by name** — those names resolve
//! through the runtime [`crate::dsl::PatternRegistry`] at build time.
//!
//! # Field Semantics
//!
//! * `id` (optional) — explicit identifier for referencing the built aggregator from
//!   consumer code. If absent, [`crate::dsl::build_aggregators_from_spec`] generates
//!   a deterministic `aggregator:auto.N` id.
//! * `correlation_header` (required, non-empty) — header key used to group messages
//!   (e.g. `"block_hash"` for the chain's finality quorum).
//! * `completion` (required, non-empty) — registry name of the
//!   [`allora_core::patterns::aggregator::CompletionCondition`] to use. *No defaults* —
//!   the chain's quorum logic is registry-supplied, and that's the canonical use case.
//! * `strategy` (optional, non-empty if present) — registry name of the
//!   [`allora_core::patterns::aggregator::AggregationStrategy`]. Defaults to
//!   `allora.concat_text` (matching `Aggregator::new`'s default behavior) at build time.
//! * `store` (optional, non-empty if present) — registry name of the
//!   [`allora_core::patterns::aggregator::GroupStore`]. Defaults to an
//!   `InMemoryGroupStore` instance at build time.
//!
//! # Uniqueness & Validation
//! YAML parser ([`crate::spec::AggregatorSpecYamlParser`]) enforces presence + type +
//! non-empty strings. Uniqueness of `id` is deferred to the collection builder
//! ([`crate::dsl::build_aggregators_from_spec`]).
//!
//! # Construction Helpers
//! * `new(correlation_header, completion)` — minimal spec.
//! * `with_id(id, correlation_header, completion)` — explicit id.
//! * `set_strategy(name)` / `set_store(name)` — chainable setters (return `self`).
//! * `set_id(id)` — internal use (builder assigning auto-generated id).
//! * Accessors: `id()`, `correlation_header()`, `completion()`, `strategy()`, `store()`.

#[derive(Debug, Clone)]
pub struct AggregatorSpec {
    id: Option<String>,
    correlation_header: String,
    completion: String,
    strategy: Option<String>,
    store: Option<String>,
}

impl AggregatorSpec {
    pub fn new<H: Into<String>, C: Into<String>>(correlation_header: H, completion: C) -> Self {
        Self {
            id: None,
            correlation_header: correlation_header.into(),
            completion: completion.into(),
            strategy: None,
            store: None,
        }
    }

    pub fn with_id<I, H, C>(id: I, correlation_header: H, completion: C) -> Self
    where
        I: Into<String>,
        H: Into<String>,
        C: Into<String>,
    {
        Self {
            id: Some(id.into()),
            correlation_header: correlation_header.into(),
            completion: completion.into(),
            strategy: None,
            store: None,
        }
    }

    pub fn set_strategy<S: Into<String>>(mut self, strategy: S) -> Self {
        self.strategy = Some(strategy.into());
        self
    }

    pub fn set_store<S: Into<String>>(mut self, store: S) -> Self {
        self.store = Some(store.into());
        self
    }

    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }
    pub fn correlation_header(&self) -> &str {
        &self.correlation_header
    }
    pub fn completion(&self) -> &str {
        &self.completion
    }
    pub fn strategy(&self) -> Option<&str> {
        self.strategy.as_deref()
    }
    pub fn store(&self) -> Option<&str> {
        self.store.as_deref()
    }

    /// Internal: builder assigns an auto-generated id.
    pub fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}
