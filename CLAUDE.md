# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

**Allora** is a Rust-native implementation of **Enterprise Integration Patterns (EIP)** — channels, adapters, routing,
correlation, and message-flow patterns — plus a YAML-spec → runtime DSL. Think "Apache Camel / Spring Integration, but
small, type-safe, async (Tokio), and GC-free." Status: early-alpha (`v0.0.2`, Apache-2.0).

It is consumed by other projects — notably the **Fialucci chain**, whose `allora.yaml` is an Allora spec and whose
HTTP dispatch layer is an Allora `Runtime` built from it.

## Essential commands

Run from this repo root (it's its own cargo workspace):

```bash
cargo build --workspace
cargo test                                            # integration tests live in tests/ (a real tests/ dir)
cargo test -p allora-core <name>                      # one crate / test by substring
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
# examples: examples/basic/{helloworld,http-inbound,http-outbound}
```

CI: `.github/workflows/rust.yml` (build/test/lint) + `release.yml`.

## Architecture

### Crate topology (workspace of 5 + examples)

- **`crates/core`** (`allora-core`) — the primitives; deliberately depends on **no** higher crates (avoids cycles):
  - `message.rs` — `Message` (immutable payload + headers), `Exchange` (mutable processing context: in/out message, headers, correlation), `Payload`.
  - `channel/` — `Channel` trait + `DirectChannel`, `QueueChannel`, `log`; plus `PollableChannel` / `SubscribableChannel` / `CorrelationSupport`.
  - `processor.rs` — `Processor` trait (+ `ClosureProcessor`, `BoxedProcessor`); `route.rs` — `Route` (wires channels → processors).
  - `endpoint.rs`, `adapter.rs` (`BaseAdapter` / `InboundAdapter` / `OutboundAdapter`), `service.rs` (`Service` + `ServiceActivator`), `error.rs`, `logging.rs`.
  - `patterns/` — EIP patterns: `aggregator`, `content_router`, `correlation_initializer`, `filter`, `recipient_list`, `splitter`.
- **`crates/http`** (`allora-http`) — HTTP inbound/outbound adapters over `hyper` 0.14; `adapter_dsl` (`InboundHttpExt` / `OutboundHttpExt`), `Mep` (message-exchange pattern).
- **`crates/macros`** (`allora-macros`) — the `#[service]` proc-macro; registers service activators at compile time via the `inventory` crate.
- **`crates/runtime`** (`allora-runtime`) — the **`Runtime` facade + DSL + YAML spec system** (see below); `service_activator_processor`.
- **`crates/allora`** (`allora`) — the **umbrella crate** most users import; re-exports the public API of the other four (`Message`, `Exchange`, `Channel`, `Route`, `Processor`, `Service`, the `service` macro, `Runtime`, `dsl::build`, HTTP adapters, `ServiceDescriptor` + `all_service_descriptors`).

### The spec → runtime system (key non-obvious design)

`crates/runtime/src/spec/` turns declarative YAML into a live runtime. Each component comes as a **pair**: a struct
spec (`*_spec.rs`) and its serde-YAML form (`*_spec_yaml.rs`) — e.g. `channel_spec.rs` + `channel_spec_yaml.rs`,
`http_inbound_adapter_spec*`, `filter_spec*`, `service_spec*`, and the top-level `allora_spec*`. Facade:
`Runtime::new().with_config_file("allora.yml").run()` (or `dsl::build` / `build_channel_from_str`). Example spec:

```yaml
version: 1
channels:
  - { kind: direct, id: inbound }
  - { kind: direct, id: outbound }
```

This is exactly the mechanism the chain uses: its `allora.yaml` is an Allora spec compiled into the node's HTTP runtime.

### Service registration via `inventory`

`#[service]` (from `allora-macros`) registers a `ServiceDescriptor` through the `inventory` crate, so services
self-register at link time and are discoverable via `all_service_descriptors()`. The macro targets the root `allora`
crate path — it is **not** usable inside `allora-core` itself.

## Conventions

- Async-only (Tokio); HTTP + serialization are always on (no feature flags).
- Tests: real integration files under `tests/` **plus** inline unit tests — run all with `cargo test`.
- Match CI exactly: `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features -- -D warnings`.
