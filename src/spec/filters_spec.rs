//! FiltersSpec: collection of FilterSpec entries (v1) sharing a version.
//! Provides ordered aggregation prior to runtime building; does not perform uniqueness checks.
//!
//! # Responsibilities
//! * Preserve declaration order (Vec internally).
//! * Carry a single `version` validated by parser (`filters_spec_yaml.rs`).
//! * Allow optional/duplicate/missing filter ids – uniqueness & auto-id handled in builder.
//!
//! # Auto-ID Strategy (Builder Level)
//! * Missing `filter.id` values are assigned deterministic `filter:auto.N` IDs starting at 1.
//! * Duplicate explicit ids trigger a build error (see `build_filters_from_spec`).
//!
//! # Example
//! ```rust
//! use allora::spec::{FilterSpec, FiltersSpec};
//! // Builder pattern (method chaining, returns new value)
//! let spec = FiltersSpec::new(1)
//!     .add(FilterSpec::with_id("filt.a", "inbound.a", "body.contains(\"X\")"))
//!     .add(FilterSpec::new("inbound.b", "header(\"Flag\") == \"on\""));
//! assert_eq!(spec.filters().len(), 2);
//! assert_eq!(spec.filters()[0].id(), Some("filt.a"));
//! assert!(spec.filters()[1].id().is_none());
//!
//! // Mutation pattern (push, mutates in place)
//! let mut spec2 = FiltersSpec::new(1);
//! spec2.push(FilterSpec::with_id("filt.a", "inbound.a", "body.contains(\"X\")"));
//! spec2.push(FilterSpec::new("inbound.b", "header(\"Flag\") == \"on\""));
//! assert_eq!(spec2.filters().len(), 2);
//! assert_eq!(spec2.filters()[0].id(), Some("filt.a"));
//! assert!(spec2.filters()[1].id().is_none());
//! ```
//!

use crate::spec::FilterSpec;

#[derive(Debug, Clone)]
pub struct FiltersSpec {
    version: u32,
    filters: Vec<FilterSpec>,
}

impl FiltersSpec {
    pub fn new(version: u32) -> Self {
        Self {
            version,
            filters: Vec::new(),
        }
    }
    pub fn add(mut self, f: FilterSpec) -> Self {
        self.filters.push(f);
        self
    }
    pub fn push(&mut self, f: FilterSpec) {
        self.filters.push(f);
    }
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn filters(&self) -> &[FilterSpec] {
        &self.filters
    }
    pub fn into_filters(self) -> Vec<FilterSpec> {
        self.filters
    }
}
