//! `AggregatorsSpec`: collection of [`AggregatorSpec`] entries (v1) sharing a version.
//!
//! Preserves declaration order (Vec internally). Like [`crate::spec::FiltersSpec`],
//! this collection does *not* perform uniqueness checks; uniqueness + deterministic
//! auto-ids (`aggregator:auto.N`) are enforced in
//! [`crate::dsl::build_aggregators_from_spec`].

use crate::spec::AggregatorSpec;

#[derive(Debug, Clone)]
pub struct AggregatorsSpec {
    version: u32,
    aggregators: Vec<AggregatorSpec>,
}

impl AggregatorsSpec {
    pub fn new(version: u32) -> Self {
        Self {
            version,
            aggregators: Vec::new(),
        }
    }
    pub fn add(mut self, a: AggregatorSpec) -> Self {
        self.aggregators.push(a);
        self
    }
    pub fn push(&mut self, a: AggregatorSpec) {
        self.aggregators.push(a);
    }
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn aggregators(&self) -> &[AggregatorSpec] {
        &self.aggregators
    }
    pub fn into_aggregators(self) -> Vec<AggregatorSpec> {
        self.aggregators
    }
}
