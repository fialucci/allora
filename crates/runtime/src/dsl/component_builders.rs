//! DSL runtime component builders: instantiate runtime components from validated specs.
//! This module will host builders for multiple component types (Channel, Endpoint, Adapter, etc.).
//! Each builder converts a spec (format-agnostic, already validated) into a concrete runtime type.
//!
//! # Purpose
//! Bridge the gap between a format-specific parsed spec (e.g. YAML -> `ChannelSpec`) and the
//! actual runtime component (e.g. `QueueChannel`). Parsing & validation happen elsewhere
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
//! use allora_runtime::spec::ChannelSpec;
//! use allora_runtime::dsl::component_builders::build_channel_from_spec;
//! use allora_core::Channel; // bring trait into scope for channel.id()
//! let spec = ChannelSpec::queue().id("example-channel");
//! let channel = build_channel_from_spec(spec).unwrap();
//! assert_eq!(channel.id(), "example-channel");
//! ```
//!
//! # Auto-generated IDs (Single vs Multi Build)
//! * Single channel (`build_channel_from_spec` when `spec.channel_id()` is `None`): underlying
//!   `QueueChannel::with_random_id()` assigns a UUID-based id (`queue:<uuid>`).
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

use crate::adapter::Adapter;
use crate::channel::ChannelRef;
// already available via allora-core re-export
use crate::dsl::PatternRegistry;
use crate::spec::{AggregatorSpec, AggregatorsSpec};
use crate::spec::{HttpInboundAdapterSpec, HttpInboundAdaptersSpec};
use crate::spec::{HttpOutboundAdapterSpec, HttpOutboundAdaptersSpec};
use crate::{
    spec::{ChannelKindSpec, ChannelSpec, ChannelsSpec, FilterSpec, FiltersSpec},
    spec::{ServiceActivatorSpec, ServiceActivatorsSpec},
    Channel, ClosureProcessor, Error, Filter, Result,
};
use allora_core::patterns::aggregator::Aggregator;
use allora_http::{HttpInboundAdapter, InboundHttpExt, Mep};
use allora_http::{HttpOutboundAdapter, OutboundHttpExt};
use hyper::Method;
use std::collections::HashMap;

/// Helper: parse HTTP method string into `hyper::Method`.
/// Accepts common verbs (GET, POST, PUT, PATCH, DELETE, OPTIONS, HEAD) in uppercase; falls back
/// to custom method parsing via `Method::from_bytes` (defaulting to POST if invalid).
fn parse_http_method(m: &str) -> Method {
    match m {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "PATCH" => Method::PATCH,
        "DELETE" => Method::DELETE,
        "OPTIONS" => Method::OPTIONS,
        "HEAD" => Method::HEAD,
        other => Method::from_bytes(other.as_bytes()).unwrap_or(Method::POST),
    }
}

// for inbound adapter DSL entry
// Service runtime placeholder: processor representing service logic.
pub type ServiceProcessor =
    ClosureProcessor<Box<dyn Fn(&mut crate::Exchange) -> Result<()> + Send + Sync + 'static>>;

use std::collections::HashSet;

/// Internal helper: build a single channel from spec.
/// When `used_ids` + `auto_ctr` provided, enforces uniqueness and generates deterministic auto IDs.
fn build_channel_spec_internal(
    spec: &ChannelSpec,
    used_ids: Option<&mut HashSet<String>>,
    auto_ctr: Option<&mut u64>,
) -> Result<Box<dyn Channel>> {
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
    let channel: Box<dyn Channel> = match spec.kind() {
        ChannelKindSpec::Queue => match final_id {
            Some(id) => Box::new(crate::channel::QueueChannel::with_id(id)),
            None => Box::new(crate::channel::QueueChannel::with_random_id()),
        },
        ChannelKindSpec::Direct => match final_id {
            Some(id) => Box::new(crate::channel::DirectChannel::with_id(id)),
            None => Box::new(crate::channel::DirectChannel::with_random_id()),
        },
    };
    Ok(channel)
}

/// Build a concrete channel from a validated `ChannelSpec`.
/// Delegates to internal helper without uniqueness / auto-ID tracking (builder handles UUID auto-id).
pub fn build_channel_from_spec(spec: ChannelSpec) -> Result<Box<dyn Channel>> {
    build_channel_spec_internal(&spec, None, None)
}

/// Build multiple concrete channels from a validated `ChannelsSpec`.
/// Enforces uniqueness across provided IDs and generates deterministic auto IDs for missing ones.
pub fn build_channels_from_spec(spec: ChannelsSpec) -> Result<Vec<Box<dyn Channel>>> {
    let mut result: Vec<Box<dyn Channel>> = Vec::with_capacity(spec.channels().len());
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
    let id_opt = spec.id().map(|s| s.to_string());
    Filter::from_apl_with_id(id_opt, spec.when())
}

/// Build multiple Filters from FiltersSpec (collection). Returns Vec<Filter> preserving order.
/// ID Strategy (mirrors channels):
/// * Explicit non-empty `filter.id` values must be unique (error on duplicate).
/// * Missing ids are generated deterministically as `filter:auto.N` starting at 1 (or the next
///   number after the highest explicitly provided `filter:auto.X` id) within a single build invocation.
/// * Users are discouraged from manually supplying IDs with the reserved `filter:auto.` prefix; if
///   they do, generation will skip to the next available integer without scanning the entire set.
/// * Generated ids are stored on the runtime `Filter` for diagnostics and future routing metadata.
/// * Malformed reserved IDs (e.g. `filter:auto.bad`) are ignored for sequence advancement and a
///   warning is emitted via `tracing::warn!`.
pub fn build_filters_from_spec(spec: FiltersSpec) -> Result<Vec<Filter>> {
    let mut result = Vec::with_capacity(spec.filters().len());
    const AUTO_PREFIX: &str = "filter:auto.";
    let mut used = HashSet::new();
    let mut max_auto_explicit = 0u64;
    // First pass: validate explicit ids & find highest reserved pattern
    for f in spec.filters() {
        if let Some(id) = f.id() {
            if used.contains(id) {
                return Err(Error::serialization(format!("duplicate filter.id '{id}'")));
            }
            if let Some(rest) = id.strip_prefix(AUTO_PREFIX) {
                match rest.parse::<u64>() {
                    Ok(n) => max_auto_explicit = max_auto_explicit.max(n),
                    Err(_) => {
                        tracing::warn!(%id, "ignoring malformed reserved auto-id suffix; expected numeric after filter:auto.")
                    }
                }
            }
            used.insert(id.to_string());
        }
    }
    // Second pass: build filters, generate ids for missing ones
    let mut auto_ctr = max_auto_explicit + 1;
    for f in spec.filters() {
        if let Some(id) = f.id() {
            result.push(Filter::from_apl_with_id(Some(id.to_string()), f.when())?);
            continue;
        }
        let gen_id = format!("{AUTO_PREFIX}{auto_ctr}");
        auto_ctr += 1;
        result.push(Filter::from_apl_with_id(Some(gen_id), f.when())?);
    }
    Ok(result)
}

/// Internal: validate required invariant fields on a `ServiceSpec`.
fn validate_service_activator_spec(spec: &ServiceActivatorSpec) -> Result<()> {
    if spec.from().is_empty() {
        return Err(Error::serialization("service.from must not be empty"));
    }
    if spec.to().is_empty() {
        return Err(Error::serialization("service.to must not be empty"));
    }
    if spec.ref_name().is_empty() {
        return Err(Error::serialization("service.ref-name must not be empty"));
    }
    Ok(())
}

/// Internal: build a service processor closure setting headers.
/// Always sets `service-activator.ref-name`; sets `service-activator.id` if provided.
fn service_processor_with_headers(id_opt: Option<&str>, ref_name: &str) -> ServiceProcessor {
    let ref_name_copy = ref_name.to_string();
    let id_copy = id_opt.map(|s| s.to_string());
    let proc_fn: Box<dyn Fn(&mut crate::Exchange) -> Result<()> + Send + Sync + 'static> =
        Box::new(move |exchange: &mut crate::Exchange| {
            if let Some(ref id) = id_copy {
                exchange.in_msg.set_header("service-activator.id", id);
            }
            exchange
                .in_msg
                .set_header("service-activator.ref-name", ref_name_copy.as_str());
            Ok(())
        });
    ClosureProcessor::new(proc_fn)
}

/// Build a single Service from a validated `ServiceSpec`.
/// Currently materializes as a `ClosureProcessor` placeholder executing no-op logic.
/// Future: compile & load user-provided implementation from `ref_name`.
pub fn build_service_from_spec(spec: ServiceActivatorSpec) -> Result<ServiceProcessor> {
    validate_service_activator_spec(&spec)?;
    // For a single build we preserve existing behavior: only impl header if id absent.
    Ok(service_processor_with_headers(spec.id(), spec.ref_name()))
}

/// Build multiple services from `ServicesSpec` preserving order.
/// Auto-ID Strategy:
/// * Explicit non-empty ids must be unique; duplicates -> error.
/// * Missing ids generated deterministically as `service:auto.N` starting at 1 (or next after any explicit reserved pattern).
pub fn build_service_activators_from_spec(
    spec: ServiceActivatorsSpec,
) -> Result<Vec<ServiceProcessor>> {
    let mut result = Vec::with_capacity(spec.services_activators().len());
    const AUTO_PREFIX: &str = "service:auto.";
    let mut used = HashSet::new();
    let mut max_auto_explicit = 0u64;
    // First pass: explicit IDs & validation
    for s in spec.services_activators() {
        validate_service_activator_spec(s)?;
        if let Some(id) = s.id() {
            if used.contains(id) {
                return Err(Error::serialization(format!("duplicate service.id '{id}'")));
            }
            if let Some(rest) = id.strip_prefix(AUTO_PREFIX) {
                if let Ok(n) = rest.parse::<u64>() {
                    max_auto_explicit = max_auto_explicit.max(n);
                }
            }
            used.insert(id.to_string());
        }
    }
    // Second pass: build processors with generated IDs where missing
    let mut auto_ctr = max_auto_explicit + 1;
    for s in spec.services_activators() {
        let id_final = match s.id() {
            Some(id) => id.to_string(),
            None => {
                let gen = format!("{AUTO_PREFIX}{auto_ctr}");
                auto_ctr += 1;
                gen
            }
        };
        let proc = service_processor_with_headers(Some(&id_final), s.ref_name());
        result.push(proc);
    }
    Ok(result)
}

/// Internal helper: common base builder for HTTP inbound adapter to reduce duplication.
fn http_inbound_builder_base(
    host: &str,
    port: u16,
    path: &str,
    channel: ChannelRef,
) -> allora_http::HttpInboundBuilder {
    Adapter::inbound()
        .http()
        .host(host)
        .port(port)
        .base_path(path)
        .channel(channel)
}

/// Build a single HTTP inbound adapter from a validated `HttpInboundAdapterSpec`.
///
/// ID Handling:
/// * If `spec.id()` is `Some("")` an error is returned (`http-inbound-adapter.id must not be empty`).
/// * If `spec.id()` is `None` the underlying inbound builder will auto-generate an id (e.g. `http-inbound:<addr>`).
/// * Provided non-empty ids are accepted as-is (no uniqueness enforcement here; collection build enforces uniqueness).
///
/// Channel Resolution:
/// * `request-channel` must be a non-empty string and must resolve via `channel_lookup` or an error is returned.
/// * If `reply-channel` is provided it must resolve; presence implies InOut MEP automatically.
/// * If no `reply-channel` is provided, MEP is forced to `InOnly202` (fire-and-forget with 202 acknowledgement).
///
/// Errors Returned:
/// * Empty `request-channel`.
/// * Unknown `request-channel` id.
/// * Unknown `reply-channel` id.
/// * Empty adapter id when explicitly supplied.
///
/// Auto-ID Strategy (single build): if `spec.id()` absent, use `HttpInboundBuilder` default; if present, enforce non-empty.
pub fn build_http_inbound_adapter_from_spec(
    spec: HttpInboundAdapterSpec,
    channel_lookup: &dyn Fn(&str) -> Option<ChannelRef>,
) -> Result<HttpInboundAdapter> {
    let req_id = spec.request_channel();
    if req_id.is_empty() {
        return Err(Error::serialization(
            "http-inbound-adapter.request-channel must not be empty",
        ));
    }
    let ch = channel_lookup(req_id).ok_or_else(|| {
        Error::serialization(format!(
            "unknown channel id '{}' for http-inbound-adapter.request-channel",
            req_id
        ))
    })?;
    let mut builder = http_inbound_builder_base(spec.host(), spec.port(), spec.path(), ch.clone());
    if let Some(id) = spec.id() {
        builder = builder.id(id);
    }
    if let Some(reply_id) = spec.reply_channel() {
        let rc = channel_lookup(reply_id).ok_or_else(|| {
            Error::serialization(format!(
                "unknown channel id '{}' for http-inbound-adapter.reply-channel",
                reply_id
            ))
        })?;
        // Setting reply_channel automatically forces InOut MEP in builder.build(); no need to set .mep explicitly.
        builder = builder.reply_channel(rc);
    }
    // Only set explicit MEP for fire-and-forget when no reply channel.
    if spec.reply_channel().is_none() {
        builder = builder.mep(Mep::InOnly202);
    }
    Ok(builder.build())
}

/// Build multiple HTTP inbound adapters from a validated `HttpInboundAdaptersSpec`.
///
/// ID Handling & Uniqueness:
/// * Explicit non-empty ids must be unique; duplicates -> `Error::Serialization("duplicate http-inbound-adapter.id '<id>'")`.
/// * Empty id string (explicit) -> error (`http-inbound-adapter.id must not be empty`).
/// * Missing ids are generated deterministically as `http-inbound-adapter:auto.N` starting at 1 (or the next number after any explicitly supplied reserved `http-inbound-adapter:auto.<X>` ids) within this build invocation.
/// * Generated ids skip values already used (explicit or previously generated) to avoid collisions.
///
/// Channel Resolution:
/// * Each adapter must declare a non-empty `request-channel` that resolves via `channel_lookup`; otherwise an error is returned.
/// * Optional `reply-channel` must resolve if present; its presence implies InOut MEP (request/reply) implicitly.
/// * Absence of `reply-channel` forces MEP to `InOnly202` (fire-and-forget, 202 Accepted).
///
/// MEP Behavior:
/// * InOut when `reply-channel` provided (adapter waits for correlated reply).
/// * InOnly202 when `reply-channel` absent (adapter acknowledges immediately with 202).
///
/// Errors Returned:
/// * Empty `request-channel`.
/// * Unknown `request-channel` id.
/// * Unknown `reply-channel` id.
/// * Empty adapter id when explicitly supplied.
/// * Duplicate explicit adapter id.
///
/// Order Preservation: Output vector preserves the input spec order.
pub fn build_http_inbound_adapters_from_spec(
    spec: HttpInboundAdaptersSpec,
    channel_lookup: &dyn Fn(&str) -> Option<ChannelRef>,
) -> Result<Vec<HttpInboundAdapter>> {
    let mut result = Vec::with_capacity(spec.adapters().len());
    const AUTO_PREFIX: &str = "http-inbound-adapter:auto.";
    let mut used = HashSet::new();
    let mut max_auto_explicit = 0u64;
    // First pass: explicit id validation & reserved pattern tracking
    for a in spec.adapters() {
        let req_id = a.request_channel();
        if req_id.is_empty() {
            return Err(Error::serialization(
                "http-inbound-adapter.request-channel must not be empty",
            ));
        }
        if let Some(id) = a.id() {
            if id.is_empty() {
                return Err(Error::serialization(
                    "http-inbound-adapter.id must not be empty",
                ));
            }
            if used.contains(id) {
                return Err(Error::serialization(format!(
                    "duplicate http-inbound-adapter.id '{}'",
                    id
                )));
            }
            if let Some(rest) = id.strip_prefix(AUTO_PREFIX) {
                if let Ok(n) = rest.parse::<u64>() {
                    max_auto_explicit = max_auto_explicit.max(n);
                }
            }
            used.insert(id.to_string());
        }
    }
    // Second pass: build adapters with generated IDs for missing ones
    let mut auto_ctr = max_auto_explicit + 1;
    for a in spec.adapters() {
        let id_final = match a.id() {
            Some(id) => id.to_string(),
            None => {
                let gen = format!("{AUTO_PREFIX}{auto_ctr}");
                auto_ctr += 1;
                let mut candidate = gen;
                while used.contains(&candidate) {
                    candidate = format!("{AUTO_PREFIX}{auto_ctr}");
                    auto_ctr += 1;
                }
                used.insert(candidate.clone());
                candidate
            }
        };
        let ch = channel_lookup(a.request_channel()).ok_or_else(|| {
            Error::serialization(format!(
                "unknown channel id '{}' for http-inbound-adapter.request-channel",
                a.request_channel()
            ))
        })?;
        let mut builder = http_inbound_builder_base(a.host(), a.port(), a.path(), ch.clone())
            .id(id_final.clone());
        if let Some(reply_id) = a.reply_channel() {
            let rc = channel_lookup(reply_id).ok_or_else(|| {
                Error::serialization(format!(
                    "unknown channel id '{}' for http-inbound-adapter.reply-channel",
                    reply_id
                ))
            })?;
            // reply_channel implies InOut; builder will override MEP internally.
            builder = builder.reply_channel(rc);
        } else {
            builder = builder.mep(Mep::InOnly202);
        }
        result.push(builder.build());
    }
    Ok(result)
}

/// Build a single HTTP outbound adapter from a validated `HttpOutboundAdapterSpec`.
///
/// ID Handling:
/// * If `spec.id()` is `Some("")` an error is returned (`http-outbound-adapter.id must not be empty`).
/// * If `spec.id()` is `None` the underlying outbound builder will auto-generate a UUID-based id.
/// * Provided non-empty ids are accepted as-is (no uniqueness enforcement here; collection build enforces).
///
/// Method Handling:
/// * Recognized verbs (GET, POST, PUT, PATCH, DELETE, OPTIONS, HEAD) are mapped directly.
/// * Unrecognized verb strings are passed to `Method::from_bytes`; failure falls back to POST.
///
/// Errors:
/// * Empty id string.
/// * (Indirect) build errors from the adapter builder (e.g. missing host/port handled upstream by spec parser).
pub fn build_http_outbound_adapter_from_spec(
    spec: HttpOutboundAdapterSpec,
) -> Result<HttpOutboundAdapter> {
    if let Some(id) = spec.id() {
        if id.is_empty() {
            return Err(Error::serialization(
                "http-outbound-adapter.id must not be empty",
            ));
        }
    }
    let mut builder = Adapter::outbound()
        .http()
        .host(spec.host())
        .port(spec.port())
        .base_path(spec.base_path());
    if let Some(p) = spec.path() {
        builder = builder.path(p);
    }
    if let Some(m) = spec.method() {
        builder = builder.method(parse_http_method(m));
    }
    if !spec.use_out_msg() {
        builder = builder.use_out_msg(false);
    }
    if let Some(id) = spec.id() {
        builder = builder.id(id);
    }
    builder.build()
}

/// Build multiple HTTP outbound adapters from a validated `HttpOutboundAdaptersSpec`.
///
/// Uniqueness & Auto-ID Strategy (mirrors inbound):
/// * Explicit non-empty ids must be unique; duplicates -> `Error::Serialization("duplicate http-outbound-adapter.id '<id>'")`.
/// * Empty id string -> error.
/// * Missing ids are generated deterministically as `http-outbound-adapter:auto.N` starting at 1
///   (or after any explicitly provided reserved `http-outbound-adapter:auto.<X>` ids) within a single build invocation.
/// * Generated ids skip already used values (including explicitly supplied ones and earlier generated ones).
///
/// Method Handling uses `parse_http_method` (see single builder).
///
/// Returned adapters preserve the input order.
pub fn build_http_outbound_adapters_from_spec(
    spec: HttpOutboundAdaptersSpec,
) -> Result<Vec<HttpOutboundAdapter>> {
    let mut result = Vec::with_capacity(spec.adapters().len());
    const AUTO_PREFIX: &str = "http-outbound-adapter:auto.";
    let mut used = HashSet::new();
    let mut max_auto_explicit = 0u64;
    // First pass: validate explicit ids & track highest explicit auto id suffix.
    for a in spec.adapters() {
        if let Some(id) = a.id() {
            if id.is_empty() {
                return Err(Error::serialization(
                    "http-outbound-adapter.id must not be empty",
                ));
            }
            if used.contains(id) {
                return Err(Error::serialization(format!(
                    "duplicate http-outbound-adapter.id '{}'",
                    id
                )));
            }
            if let Some(rest) = id.strip_prefix(AUTO_PREFIX) {
                if let Ok(n) = rest.parse::<u64>() {
                    max_auto_explicit = max_auto_explicit.max(n);
                }
            }
            used.insert(id.to_string());
        }
    }
    // Second pass: build adapters assigning deterministic auto ids where missing.
    let mut auto_ctr = max_auto_explicit + 1;
    for a in spec.adapters() {
        let id_final = match a.id() {
            Some(id) => id.to_string(),
            None => {
                let mut candidate = format!("{AUTO_PREFIX}{auto_ctr}");
                auto_ctr += 1;
                while used.contains(&candidate) {
                    candidate = format!("{AUTO_PREFIX}{auto_ctr}");
                    auto_ctr += 1;
                }
                used.insert(candidate.clone());
                candidate
            }
        };
        let mut builder = Adapter::outbound()
            .http()
            .host(a.host())
            .port(a.port())
            .base_path(a.base_path())
            .id(id_final);
        if let Some(p) = a.path() {
            builder = builder.path(p);
        }
        if let Some(m) = a.method() {
            builder = builder.method(parse_http_method(m));
        }
        if !a.use_out_msg() {
            builder = builder.use_out_msg(false);
        }
        let built = builder.build()?;
        result.push(built);
    }
    Ok(result)
}

// ─────────────────────────────────────────────────────────────────────────────
// Aggregator (EIP pattern in YAML DSL — see P9)
// ─────────────────────────────────────────────────────────────────────────────

/// Build a single [`Aggregator`] from a validated [`AggregatorSpec`].
///
/// Resolves `completion`, `strategy`, and `store` registry names through
/// `registry`. Defaults applied when fields are absent:
/// * `strategy` → none set on the aggregator (back-compat `ConcatText` from
///   `Aggregator::new` semantics is still the active default at runtime).
/// * `store` → none set; the aggregator uses its built-in `InMemoryGroupStore`.
///
/// Errors when:
/// * `aggregator.correlation_header` is empty (defensive — parser should have
///   already rejected this).
/// * `aggregator.id` (if explicitly provided) is empty.
/// * Any referenced name fails to resolve in `registry`.
pub fn build_aggregator_from_spec(
    spec: AggregatorSpec,
    registry: &PatternRegistry,
) -> Result<Aggregator> {
    if let Some(id) = spec.id() {
        if id.is_empty() {
            return Err(Error::serialization("aggregator.id must not be empty"));
        }
    }
    if spec.correlation_header().is_empty() {
        return Err(Error::serialization(
            "aggregator.correlation_header must not be empty",
        ));
    }

    let completion = registry.completion(spec.completion()).ok_or_else(|| {
        Error::serialization(format!(
            "unknown completion '{}' for aggregator (registered completions: [{}])",
            spec.completion(),
            registry.completion_names().join(", ")
        ))
    })?;

    let mut agg = Aggregator::with_completion(spec.correlation_header(), completion);

    if let Some(strategy_name) = spec.strategy() {
        let strategy = registry.strategy(strategy_name).ok_or_else(|| {
            Error::serialization(format!(
                "unknown strategy '{}' for aggregator (registered strategies: [{}])",
                strategy_name,
                registry.strategy_names().join(", ")
            ))
        })?;
        agg = agg.with_strategy(strategy);
    }

    if let Some(store_name) = spec.store() {
        let store = registry.store(store_name).ok_or_else(|| {
            Error::serialization(format!(
                "unknown store '{}' for aggregator (registered stores: [{}])",
                store_name,
                registry.store_names().join(", ")
            ))
        })?;
        agg = agg.with_store(store);
    }

    Ok(agg)
}

/// Build multiple aggregators from an [`AggregatorsSpec`], keyed by id.
///
/// ID Strategy (mirrors filters / channels):
/// * Explicit non-empty `aggregator.id` values must be unique; duplicates →
///   `Error::Serialization("duplicate aggregator.id '<id>'")`.
/// * Missing ids are generated deterministically as `aggregator:auto.N` starting
///   at 1 (or the next number after any explicitly supplied reserved
///   `aggregator:auto.<X>` ids) within this build invocation.
/// * Generated ids skip values already used.
///
/// Returns a `HashMap<id, Aggregator>` so consumers can look up by id.
/// Order-preserving callers can iterate `spec.aggregators()` separately.
pub fn build_aggregators_from_spec(
    spec: AggregatorsSpec,
    registry: &PatternRegistry,
) -> Result<HashMap<String, Aggregator>> {
    const AUTO_PREFIX: &str = "aggregator:auto.";
    let mut used: HashSet<String> = HashSet::new();
    let mut max_auto_explicit = 0u64;

    // First pass: validate explicit ids + track highest reserved auto-suffix.
    for a in spec.aggregators() {
        if let Some(id) = a.id() {
            if id.is_empty() {
                return Err(Error::serialization("aggregator.id must not be empty"));
            }
            if used.contains(id) {
                return Err(Error::serialization(format!(
                    "duplicate aggregator.id '{id}'"
                )));
            }
            if let Some(rest) = id.strip_prefix(AUTO_PREFIX) {
                if let Ok(n) = rest.parse::<u64>() {
                    max_auto_explicit = max_auto_explicit.max(n);
                }
            }
            used.insert(id.to_string());
        }
    }

    // Second pass: build with deterministic auto-ids where missing.
    let mut auto_ctr = max_auto_explicit + 1;
    let mut result: HashMap<String, Aggregator> = HashMap::with_capacity(spec.aggregators().len());
    for a in spec.aggregators() {
        let id_final = match a.id() {
            Some(id) => id.to_string(),
            None => {
                let mut candidate = format!("{AUTO_PREFIX}{auto_ctr}");
                auto_ctr += 1;
                while used.contains(&candidate) {
                    candidate = format!("{AUTO_PREFIX}{auto_ctr}");
                    auto_ctr += 1;
                }
                used.insert(candidate.clone());
                candidate
            }
        };
        let built = build_aggregator_from_spec(a.clone(), registry)?;
        result.insert(id_final, built);
    }
    Ok(result)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests (aggregator builder + registry)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod aggregator_tests {
    use super::*;
    use allora_core::patterns::aggregator::CompletionCondition;
    use allora_core::Message;
    use std::sync::Arc;
    use std::time::Instant;

    struct CountAtLeast(usize);
    impl CompletionCondition for CountAtLeast {
        fn is_complete(&self, group: &[Message], _: Instant) -> bool {
            group.len() >= self.0
        }
    }

    fn registry_with_test_completion() -> PatternRegistry {
        let mut r = PatternRegistry::with_defaults();
        r.register_completion("test.count_at_least_3", Arc::new(CountAtLeast(3)));
        r
    }

    #[test]
    fn build_single_with_completion_only() {
        let spec = AggregatorSpec::new("corr", "test.count_at_least_3");
        let _agg = build_aggregator_from_spec(spec, &registry_with_test_completion())
            .expect("build succeeds");
        // Aggregator constructed; further behavior covered by allora_core tests.
    }

    #[test]
    fn build_single_with_strategy_and_store_refs_resolved() {
        // Resolve both the built-in strategy `allora.emit_signal` and a
        // consumer-registered (in-memory) store.
        let mut registry = registry_with_test_completion();
        registry.register_store(
            "test.in_mem",
            Arc::new(allora_core::patterns::aggregator::InMemoryGroupStore::new()),
        );
        let spec = AggregatorSpec::new("corr", "test.count_at_least_3")
            .set_strategy(crate::dsl::STRATEGY_EMIT_SIGNAL)
            .set_store("test.in_mem");
        let _agg = build_aggregator_from_spec(spec, &registry).expect("build succeeds");
    }

    #[test]
    fn build_fails_on_unknown_completion_with_helpful_message() {
        let spec = AggregatorSpec::new("corr", "nope.does.not.exist");
        let err = build_aggregator_from_spec(spec, &PatternRegistry::with_defaults())
            .expect_err("must error");
        let msg = err.to_string();
        assert!(
            msg.contains("unknown completion 'nope.does.not.exist'"),
            "error message should name the missing key, got: {msg}"
        );
    }

    #[test]
    fn build_fails_on_unknown_strategy() {
        let spec = AggregatorSpec::new("corr", "test.count_at_least_3").set_strategy("nope");
        let err = build_aggregator_from_spec(spec, &registry_with_test_completion())
            .expect_err("must error");
        assert!(err.to_string().contains("unknown strategy 'nope'"));
    }

    #[test]
    fn build_fails_on_unknown_store() {
        let spec = AggregatorSpec::new("corr", "test.count_at_least_3").set_store("nope");
        let err = build_aggregator_from_spec(spec, &registry_with_test_completion())
            .expect_err("must error");
        assert!(err.to_string().contains("unknown store 'nope'"));
    }

    #[test]
    fn collection_auto_ids_and_uniqueness() {
        let r = registry_with_test_completion();
        let spec = AggregatorsSpec::new(1)
            .add(AggregatorSpec::with_id(
                "explicit.a",
                "corr",
                "test.count_at_least_3",
            ))
            .add(AggregatorSpec::new("corr", "test.count_at_least_3"))
            .add(AggregatorSpec::new("corr2", "test.count_at_least_3"));
        let built = build_aggregators_from_spec(spec, &r).expect("build succeeds");
        assert_eq!(built.len(), 3);
        assert!(built.contains_key("explicit.a"));
        assert!(built.contains_key("aggregator:auto.1"));
        assert!(built.contains_key("aggregator:auto.2"));
    }

    #[test]
    fn collection_rejects_duplicate_explicit_ids() {
        let r = registry_with_test_completion();
        let spec = AggregatorsSpec::new(1)
            .add(AggregatorSpec::with_id("dup", "corr", "test.count_at_least_3"))
            .add(AggregatorSpec::with_id("dup", "corr2", "test.count_at_least_3"));
        let err = build_aggregators_from_spec(spec, &r).expect_err("must error");
        assert!(err.to_string().contains("duplicate aggregator.id 'dup'"));
    }

    #[test]
    fn auto_ids_skip_past_explicit_reserved_pattern() {
        let r = registry_with_test_completion();
        // Explicit `aggregator:auto.5` should push generated ids past 5.
        let spec = AggregatorsSpec::new(1)
            .add(AggregatorSpec::with_id(
                "aggregator:auto.5",
                "corr",
                "test.count_at_least_3",
            ))
            .add(AggregatorSpec::new("corr", "test.count_at_least_3"));
        let built = build_aggregators_from_spec(spec, &r).expect("build succeeds");
        assert!(built.contains_key("aggregator:auto.5"));
        assert!(built.contains_key("aggregator:auto.6"));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Full pipeline: parse `allora.yaml` → AlloraSpec → build aggregators via
    // PatternRegistry. The shape of the YAML below is exactly what the
    // Fialucci chain's `allora.yaml` will use for the Q1c declarative
    // finality cut-over.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn end_to_end_allora_yaml_with_aggregators_section() {
        use crate::spec::AlloraSpecYamlParser;
        let raw = r#"
version: 1
channels:
  - kind: direct
    id: attestations
aggregators:
  - id: finality_quorum
    correlation_header: block_hash
    completion: test.count_at_least_3
    strategy: allora.emit_signal
  - correlation_header: oracle_submission_id
    completion: test.count_at_least_3
"#;
        let parsed = AlloraSpecYamlParser::parse_str(raw).expect("yaml parse ok");
        assert_eq!(parsed.version(), 1);
        assert_eq!(parsed.channels_spec().channels().len(), 1);
        let aggs_spec = parsed
            .aggregators_spec()
            .expect("aggregators section present");
        assert_eq!(aggs_spec.aggregators().len(), 2);
        // First one has explicit id; second one will get auto-id.
        assert_eq!(aggs_spec.aggregators()[0].id(), Some("finality_quorum"));
        assert_eq!(aggs_spec.aggregators()[1].id(), None);

        // Build them via a registry. The chain follows exactly this pattern
        // — register completion / strategy / store impls, then build.
        let mut registry = PatternRegistry::with_defaults();
        registry.register_completion("test.count_at_least_3", Arc::new(CountAtLeast(3)));
        let built = build_aggregators_from_spec(aggs_spec.clone(), &registry)
            .expect("aggregators build ok");
        assert_eq!(built.len(), 2);
        assert!(built.contains_key("finality_quorum"));
        assert!(built.contains_key("aggregator:auto.1"));
    }

    #[test]
    fn end_to_end_yaml_unknown_strategy_surfaces_clear_error() {
        use crate::spec::AlloraSpecYamlParser;
        let raw = r#"
version: 1
aggregators:
  - id: bad
    correlation_header: x
    completion: test.count_at_least_3
    strategy: nope.unknown
"#;
        let parsed = AlloraSpecYamlParser::parse_str(raw).expect("yaml parse ok");
        let mut registry = PatternRegistry::with_defaults();
        registry.register_completion("test.count_at_least_3", Arc::new(CountAtLeast(3)));
        let err = build_aggregators_from_spec(
            parsed.into_aggregators_spec().expect("aggregators present"),
            &registry,
        )
        .expect_err("must error on unknown strategy");
        let msg = err.to_string();
        assert!(
            msg.contains("unknown strategy 'nope.unknown'"),
            "error must name the unknown ref, got: {msg}"
        );
    }
}
