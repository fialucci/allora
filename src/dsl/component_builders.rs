//! DSL runtime component builders: instantiate runtime components from validated specs.
//! This module will host builders for multiple component types (Channel, Endpoint, Adapter, etc.).
//! Each builder converts a spec (format-agnostic, already validated) into a concrete runtime type.
//!
//! # Purpose
//! Bridge the gap between a format-specific parsed spec (e.g. YAML -> `ChannelSpec`) and the
//! actual runtime component (e.g. `InMemoryChannel`). Parsing & validation happen elsewhere
//! (under `spec/` parsers). Builders assume the spec is structurally valid and focus solely on
//! instantiation and enforcing runtime constraints (like non-empty IDs).
//!
//! # Design Principles
//! * One builder per component type (not per serialization format).
//! * Builders accept only strongly typed specs (no raw YAML/JSON here).
//! * Fail fast on remaining invariants (e.g. empty string ID) that are easier to check post-parse.
//! * Keep side-effects minimal: no I/O, no global state modifications.
//!
//! # Usage Example
//! ```rust
//! use allora::spec::ChannelSpec;
//! use allora::dsl::component_builders::build_channel_from_spec;
//! use allora::Channel; // bring trait into scope for channel.id()
//! let spec = ChannelSpec::in_memory().id("example-channel");
//! let channel = build_channel_from_spec(spec).unwrap();
//! assert_eq!(channel.id(), "example-channel");
//! ```
//!
//! # Auto-generated IDs (Single vs Multi Build)
//! * Single channel (`build_channel_from_spec` when `spec.channel_id()` is `None`): underlying
//!   `ChannelBuilder` assigns a UUID-based id (`channel:<uuid>`).
//! * Multi-channel (`build_channels_from_spec`) with missing ids: this module generates
//!   deterministic sequential ids of the form `channel:auto.<N>` starting at 1 and incrementing
//!   for each missing id within that build invocation. The sequence resets each time you call
//!   `build_channels_from_spec` (no global counter).
//!
//! Rationale: deterministic ids in multi-build scenarios improve testability and reproducibility
//! without leaking global mutable state.
//!
//! # Uniqueness Enforcement
//! * Duplicate provided ids (two specs supplying the same non-empty id) -> `Error::Serialization("duplicate channel.id '<id>'")`.
//! * Empty id string -> `Error::Serialization("channel.id must not be empty")`.
//! * Generated ids are checked against previously used ids in the same build to avoid collisions.
//!
//! # Extending Channel Kinds
//! When additional kinds (e.g. `Kafka`, `Amqp`) are introduced, extend `ChannelKindSpec` and add
//! match arms inside `build_channel_spec_internal`. Keep generation & uniqueness logic centralized
//! so tests remain stable.
//!
//! # Internal Helper
//! `build_channel_spec_internal` encapsulates ID resolution (provided vs generated), uniqueness
//! checks, and final builder dispatch. It is intentionally private so external callers use only
//! the stable public functions.
//!
//! # Error Semantics
//! * `Error::Serialization` – structural or invariant violation (empty id, duplicate id).
//! * `Error::Other` – reserved for future runtime construction failures.
//!
//! # Future Improvements
//! * Shared trait for all component specs (e.g. `ComponentSpec` with `fn kind(&self)` + `fn id(&self)`)
//!   enabling generic multi-component builders.
//! * Pluggable id generation strategy (configure prefix / starting counter).
//! * Metrics hooks (time to build, count of auto-generated ids) gated behind a feature flag.
//!
//! This documentation focuses on current behavior while outlining evolution points to minimize
//! refactors as new component types are added.

use crate::{
    channel::{ChannelBuilder, InMemoryChannel},
    error::{Error, Result},
    patterns::filter::Filter,
    spec::{ChannelKindSpec, ChannelSpec, ChannelsSpec, FilterSpec, FiltersSpec},
};
use std::collections::HashSet;

/// Internal helper: build a single channel from spec.
/// When `used_ids` + `auto_ctr` provided, enforces uniqueness and generates deterministic auto IDs.
fn build_channel_spec_internal(
    spec: &ChannelSpec,
    used_ids: Option<&mut HashSet<String>>,
    auto_ctr: Option<&mut u64>,
) -> Result<InMemoryChannel> {
    // Resolve (possibly generated) id
    let final_id: Option<String> = match (spec.channel_id(), used_ids) {
        (Some(""), _) => return Err(Error::serialization("channel.id must not be empty")),
        (Some(id), Some(used)) => {
            if used.contains(id) {
                return Err(Error::serialization(format!("duplicate channel.id '{id}'")));
            }
            used.insert(id.to_string());
            Some(id.to_string())
        }
        (Some(id), None) => Some(id.to_string()), // no uniqueness enforcement
        (None, Some(used)) => {
            // generate deterministic channel:auto.N pattern
            let ctr = auto_ctr.expect("auto counter must be provided when used_ids is set");
            let mut gen = format!("channel:auto.{}", *ctr);
            while used.contains(&gen) {
                *ctr += 1;
                gen = format!("channel:auto.{}", *ctr);
            }
            *ctr += 1;
            used.insert(gen.clone());
            Some(gen)
        }
        (None, None) => None, // let underlying builder generate UUID-based id
    };

    // Match kind -> builder (future kinds centralize here)
    let channel = match spec.kind() {
        ChannelKindSpec::InMemory => {
            let builder = ChannelBuilder::point_to_point().in_memory();
            match final_id {
                Some(id) => builder.id(id).build(),
                None => builder.build(),
            }
        }
    };
    Ok(channel)
}

/// Build a concrete channel from a validated `ChannelSpec`.
/// Delegates to internal helper without uniqueness / auto-ID tracking (builder handles UUID auto-id).
pub fn build_channel_from_spec(spec: ChannelSpec) -> Result<InMemoryChannel> {
    build_channel_spec_internal(&spec, None, None)
}

/// Build multiple concrete channels from a validated `ChannelsSpec`.
/// Enforces uniqueness across provided IDs and generates deterministic auto IDs for missing ones.
pub fn build_channels_from_spec(spec: ChannelsSpec) -> Result<Vec<InMemoryChannel>> {
    let mut result = Vec::with_capacity(spec.channels().len());
    let mut used: HashSet<String> = HashSet::new();
    let mut auto_ctr: u64 = 1;
    for ch in spec.channels() {
        let built = build_channel_spec_internal(ch, Some(&mut used), Some(&mut auto_ctr))?;
        result.push(built);
    }
    Ok(result)
}

/// Build a Filter from a validated `FilterSpec`.
pub fn build_filter_from_spec(spec: FilterSpec) -> Result<Filter> {
    Filter::from_apl(spec.when())
}

/// Build multiple Filters from FiltersSpec (collection). Returns Vec<Filter> preserving order.
/// ID Strategy (mirrors channels):
/// * Explicit non-empty `filter.id` values must be unique (error on duplicate).
/// * Missing ids are generated deterministically as `filter:auto.N` starting at 1 within a single build invocation.
/// * Generated ids are not currently stored on the runtime Filter (pure predicate), but the
///   generation + uniqueness contract is enforced for future mapping / diagnostics.
pub fn build_filters_from_spec(spec: FiltersSpec) -> Result<Vec<Filter>> {
    let mut result = Vec::with_capacity(spec.filters().len());
    let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut auto_ctr: u64 = 1;
    for f in spec.filters() {
        let id_opt = f.id().map(|s| s.to_string());
        if let Some(ref idv) = id_opt {
            if used.contains(idv) {
                return Err(Error::serialization(format!("duplicate filter.id '{idv}'")));
            }
        }
        let final_id = match id_opt {
            Some(id) => {
                used.insert(id.clone());
                id
            }
            None => {
                let mut gen = format!("filter:auto.{}", auto_ctr);
                while used.contains(&gen) {
                    auto_ctr += 1;
                    gen = format!("filter:auto.{}", auto_ctr);
                }
                used.insert(gen.clone());
                auto_ctr += 1;
                gen
            }
        };
        let mut built_spec = f.clone();
        if built_spec.id().is_none() {
            built_spec.set_id(final_id);
        }
        result.push(Filter::from_apl(built_spec.when())?);
    }
    Ok(result)
}
