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
/// * Missing ids are generated deterministically as `filter:auto.N` starting at 1 (or the next
///   number after the highest explicitly provided `filter:auto.X` id) within a single build invocation.
/// * Users are discouraged from manually supplying IDs with the reserved `filter:auto.` prefix; if
///   they do, generation will skip to the next available integer without scanning the entire set.
/// * Generated ids are not currently stored on the runtime Filter (pure predicate), but the
///   generation + uniqueness contract is enforced for future mapping / diagnostics.
///
/// # Reserved Prefix Behavior
/// If a user explicitly declares `filter:auto.100` and two other filters without ids, this builder
/// will assign `filter:auto.101` and `filter:auto.102`. The gap from 1..99 is intentional and left
/// unused to avoid collision loops. This keeps the algorithm O(n) (single pre-scan) and predictable
/// without repeatedly probing for free lower numbers.
///
/// # Recommendation
/// Do NOT manually use the `filter:auto.` prefix unless you fully control the spec and accept the
/// resulting sequence jump. Prefer descriptive domain IDs (e.g. `orders.priority.filter`). Future
/// versions may emit a warning or error on user-provided reserved prefix usage if confusion proves
/// common.
///
/// # Determinism
/// The resulting auto IDs are stable for a given spec ordering. Adding or removing explicit
/// reserved-pattern IDs only shifts the starting counter upward; it does not affect relative order
/// among generated IDs.
///
/// # Future Option
/// A stricter mode (feature flag) could: (1) reject explicit `filter:auto.*` ids, or (2) ignore
/// high explicit numbers and still start at 1 while skipping collisions via a HashSet probe. Current
/// approach favors performance and simplicity.
pub fn build_filters_from_spec(spec: FiltersSpec) -> Result<Vec<Filter>> {
    let mut result = Vec::with_capacity(spec.filters().len());
    let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
    const AUTO_PREFIX: &str = "filter:auto.";

    // Pre-scan explicit IDs to detect duplicates early and determine starting auto counter.
    let mut max_auto_explicit: u64 = 0;
    for f in spec.filters() {
        if let Some(id) = f.id() {
            let id_str = id.to_string();
            if used.contains(&id_str) {
                return Err(Error::serialization(format!(
                    "duplicate filter.id '{id_str}'"
                )));
            }
            // Track explicit reserved-pattern usages (e.g. filter:auto.7) to avoid collision.
            if let Some(rest) = id_str.strip_prefix(AUTO_PREFIX) {
                if let Ok(n) = rest.parse::<u64>() {
                    max_auto_explicit = max_auto_explicit.max(n);
                }
            }
            used.insert(id_str);
        }
    }

    // Start auto counter at 1 after highest explicit reserved-pattern id.
    let mut auto_ctr: u64 = max_auto_explicit + 1; // if none present, starts at 1

    for f in spec.filters() {
        // Skip if already has an id (was validated and added to `used`).
        if f.id().is_some() {
            // Build from expression directly.
            result.push(Filter::from_apl(f.when())?);
            continue;
        }
        // Generate next deterministic auto id (no collision loop needed).
        let gen_id = format!("{AUTO_PREFIX}{auto_ctr}");
        auto_ctr += 1;
        used.insert(gen_id.clone());
        // No need to clone and set ID, since Filter does not use it.
        result.push(Filter::from_apl(f.when())?);
    }
    Ok(result)
}
