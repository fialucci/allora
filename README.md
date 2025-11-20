**allora** (Italian) [ah-LOH-rah]: adverb / discourse pivot meaning "so" or "then"; used to start, transition,
summarize, or gently prompt action. Its flexibility mirrors this project's goal: move messages forward clearly and
deliberately.

---
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](./LICENSE) [![Status](https://img.shields.io/badge/status-early--alpha-orange.svg)](#status) [![Rust](https://img.shields.io/badge/rust-2021%20edition-informational.svg)](https://www.rust-lang.org/) [![Contributions](https://img.shields.io/badge/contributions-welcome-brightgreen.svg)](#contribution-opportunities-help-wanted) [![Issues](https://img.shields.io/github/issues/fialucci/allora)](https://github.com/fialucci/allora/issues) [![Pull Requests](https://img.shields.io/github/issues-pr/fialucci/allora)](https://github.com/fialucci/allora/pulls)

## Vision & Core Principles

**Allora** is a Rust-native Enterprise Integration Patterns (EIP) library: an open-source integration framework helping
you connect everything into one continuous, high-performance flow. Its vision is to make integration code feel like
ordinary, idiomatic Rust: explicit, observable, testable, without hidden runtime wiring. The choices throughout the
crate (simple structs, trait-based composition, feature-gated extensions) reflect the following principles:

1. Clarity over magic: Prefer explicit `Route::new().add(...).build()` chains to implicit reflection or
   annotation-driven wiring.
2. Lean primitives first: Keep the foundation small (Message, Exchange, Processor, Route, Channel, Adapter) before
   adding higher-level orchestration or DSL layers.
3. Predictable performance: Favor zero-cost abstractions and avoid surprise allocations or background threads;
   throughput and latency are inspectable.
4. Incremental extensibility: New patterns (Aggregator, Splitter, Router) drop into existing routes without forcing
   rewrites; adapters layer in cleanly via feature flags.
5. Opt-in complexity: Async processing, HTTP ingestion, future AI or DSL features remain optional; your binary is only
   as large as what you enable.
6. Transparent errors: Fail fast with typed `Error` variants; no silent discards; routing failures, processor
   issues, and aggregation conditions surface clearly.
7. Deterministic routing: Same headers and payloads produce the same outcomes; correlation ensures reply matching
   without racy shared global state.
8. Inclusive community and accessible docs: Terms (payload, exchange, correlation) are explained; examples prefer
   clarity over cleverness to lower entry barriers.
9. Security & safety before convenience: No dynamic code execution in configs; upcoming DSL validates structure instead
   of evaluating arbitrary code.
10. Observability by design: Structured points will surface metrics and tracing so production flows can be measured,
    tuned, and trusted.

> When proposing changes or features, reference how they uphold (or intentionally evolve) these principles.

## Background & Motivation

Enterprise Integration Pattern frameworks (Apache Camel, Spring Integration, Mule, etc.) have shown that clear
abstractions for routing, filtering, splitting, aggregating, and correlating messages dramatically reduce glue code.
Those platforms are powerful, but also heavy, reflection-centric, and JVM-focused. Rust offers an opportunity to
re-think a lean subset of these ideas with:

- Compile-time guarantees (types, lifetimes, ownership) instead of runtime reflection.
- Low latency & predictable performance (no GC pauses) for high-throughput flows.
- Memory safety without needing defensive copying.
- Async I/O (Tokio) enabling high concurrency with minimal overhead.
- Small binary footprint suitable for containers, edge, and serverless environments.

Allora aims to be a pragmatic, minimal EIP implementation: 80% of the value (core patterns + correlation) at a fraction
of the conceptual weight. It currently has no built-in XML DSL or runtime dependency injection container; configuration
is code-first today. A declarative DSL (starting with YAML) is planned, and community contributions for alternative
formats (JSON, TOML, XML) are welcome.

## Why Rust (vs JVM / Go / Python / Node.js)

| Dimension                | [Rust](https://www.rust-lang.org/) / Allora | Traditional JVM ([Java](https://www.java.com/) / [Apache Camel](https://camel.apache.org/) / [Spring Integration](https://spring.io/projects/spring-integration)) | [Go](https://go.dev/) (e.g. [Watermill](https://watermill.io/)) | [Python](https://www.python.org/) (e.g. [Celery](https://docs.celeryq.dev/)) | [Node.js](https://nodejs.org/) (e.g. [NestJS](https://nestjs.com/)) |
|--------------------------|---------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------|------------------------------------------------------------------------------|---------------------------------------------------------------------|
| Latency predictability   | High (no GC)                                | Good but GC pauses possible                                                                                                                                       | Good (GC, but simpler)                                          | Moderate (GIL, interpreter)                                                  | Moderate (single-threaded event loop)                               |
| Type safety              | Strong static types                         | Strong (JVM generics + reflection in frameworks)                                                                                                                  | Moderate (interfaces, evolving generics)                        | Dynamic (runtime errors more likely)                                         | Dynamic/TS (optional)                                               |
| Memory footprint         | Low                                         | Higher (JVM + framework overhead)                                                                                                                                 | Low                                                             | Moderate                                                                     | Moderate                                                            |
| Concurrency model        | Async/await (Tokio) + fearless concurrency  | Threads / executors                                                                                                                                               | Goroutines + channels                                           | Event loop / threads (Celery workers)                                        | Event loop (libuv)                                                  |
| Zero-cost abstractions   | Yes                                         | Often reflection/proxy overhead                                                                                                                                   | Mostly                                                          | N/A                                                                          | N/A                                                                 |
| Ecosystem integration    | Growing (crates, async libs)                | Mature (huge ecosystem)                                                                                                                                           | Mature for networking                                           | Mature for scripting/data                                                    | Mature for web tooling                                              |
| Footprint in edge/device | Very suitable                               | Heavy                                                                                                                                                             | Suitable                                                        | Less suitable                                                                | Suitable                                                            |

Rust is chosen because its compiler enforces correctness early and facilitates building dependable integration flows
where faults (missing headers, ownership race conditions) surface at compile-time rather than mid-run.

## Differentiation & Scope

Allora intentionally *does not* attempt to:

- Replicate full Camel route DSLs or large expression languages (a lightweight declarative DSL will be added; initial
  focus is YAML, other formats like XML may be contributed).
- Provide built-in persistence, transactions, or heavyweight orchestration.
- Hide async complexity with excessive macros.

Instead, it focuses on:

- Clear, explicit composition (`Route::new().add(...).add(...).build()`)
- Small pattern implementations with predictable behavior.
- Extensible adapters (start with HTTP inbound, later outbound HTTP/Kafka/etc.)
- Opt-in features (async, http, serde) to keep default footprint minimal.

## Comparison to Existing Projects

| Project                                                                       | Language | Focus                               | Differences in Allora                                       |
|-------------------------------------------------------------------------------|----------|-------------------------------------|-------------------------------------------------------------|
| [Apache Camel](https://camel.apache.org/)                                     | Java     | Broad EIP coverage + huge ecosystem | Allora is narrower, no heavy DSL, no container dependencies |
| [Spring Integration](https://spring.io/projects/spring-integration)           | Java     | Deep Spring stack integration       | Allora removes framework coupling, favors pure composition  |
| [Mule ESB](https://www.mulesoft.com/) / [WSO2](https://wso2.com/integration/) | Java     | Enterprise service bus              | Allora is code-first, minimal, no GUI flows                 |
| [Watermill](https://watermill.io/)                                            | Go       | Messaging (Pub/Sub) patterns        | Allora emphasizes message transformation + correlation      |
| [Celery](https://docs.celeryq.dev/)                                           | Python   | Task distribution, queues           | Allora is pattern-centric, not a task queue                 |
| [NestJS](https://nestjs.com/)                                                 | TS/JS    | Web & microservice scaffolding      | Allora supplies EIP primitives rather than app scaffolding  |

## Status

Early scaffold (`0.1.x`). APIs will evolve; expect breaking changes before 1.0. Not production-ready yet.

## Goals

- Composable building blocks: `Message`, `Exchange`, `Processor`, `Route`, `Channel`.
- Core EIP patterns implemented first: Filter, Content-Based Router, Splitter, Aggregator, Recipient List, Correlation
  Initializer.
- Clear correlation & message ID semantics without magic/reflection.
- First-class asynchronous processing (Tokio) always enabled.
- Minimal dependencies; explicitness over implicit side effects.

## Installation

Add to your `Cargo.toml` using the GitHub repo (until published on crates.io):

```toml
# Latest main branch (may be unstable)
allora = { git = "https://github.com/fialucci/allora" }
```

Pin to a tagged release (recommended for reproducible builds):

```toml
allora = { git = "https://github.com/fialucci/allora", tag = "v0.1.0" }
```

Or pin to a specific commit:

```toml
allora = { git = "https://github.com/fialucci/allora", rev = "<commit-sha>" }
```

Local path development:

```toml
allora = { path = "../allora" }
```

## Feature Flags

| Feature | Default | Purpose                      |
|---------|---------|------------------------------|
| `http`  | yes     | HTTP inbound adapter support |

The project is permanently async; there is no `async` feature flag anymore. Tokio and related async capabilities are
always compiled in.

> Note: `serde` is a required dependency (always compiled in) for message/exchange (de)serialization and is not
> controlled by a feature flag. There is currently no way to disable it via Cargo features.

## Core Concepts

- **Payload**: Text / Bytes / JSON / Empty.
- **Message**: Payload + headers. Auto-generated `message_id` header on construction.
- **Exchange**: Inbound `Message`, optional outbound `Message`, and internal properties.
- **Processor**: Transforms an `Exchange`. Synchronous (`SyncProcessor`) or async (`Processor` under `async`).
- **Route**: Ordered pipeline of processors; stops on first error.
- **Channel**: Dispatch abstraction (currently in-memory) with optional correlation queue support.
- **Adapter**: Bridges external systems inbound/outbound. Currently: HTTP inbound.

## Correlation & Message IDs

- `message_id`: always created for every `Message` (UUID v4).
- `correlation_id`: lazily generated. Use `Exchange::correlation_id()`, `Message::ensure_correlation_id()`,
  `Route::with_correlation(...)`, or the `CorrelationInitializer` processor early.
- Optional mirror header (e.g. `corr`): `Route::with_correlation(Some("corr"))`.

## Quick Start (Sync)

```rust
use allora::{patterns::filter::Filter, route::Route, Exchange, Message};

fn example() -> allora::Result<()> {
    let mut exchange = Exchange::new(Message::from_text("hello"));
    let route = Route::new()
        .add(Filter::new(|exchange| exchange.in_msg.body_text() == Some("hello")))
        .build();
    route.run(&mut exchange)?;
    Ok(())
}
```

## Quick Start (Async)

```rust
use allora::{Message, Exchange, route::Route, processor::ClosureProcessor};
#[tokio::main]
async fn main() -> allora::Result<()> {
    let route = Route::new()
        .add(ClosureProcessor::new(|exchange| {
            exchange.out_msg = Some(Message::from_text("done"));
            Ok(())
        }))
        .build();
    let mut exchange = Exchange::new(Message::from_text("ping"));
    route.run(&mut exchange).await?;
    assert_eq!(exchange.out_msg.unwrap().body_text(), Some("done"));
    Ok(())
}
```

## Channels

Two channel implementations provide distinct semantics:

| Channel         | Buffering | Subscribers | Dequeue API | Correlation Helpers | Typical Use                              |
|-----------------|-----------|-------------|-------------|---------------------|------------------------------------------|
| `DirectChannel` | None      | Yes         | No          | No                  | Immediate fan-out / in-memory pub-sub    |
| `QueueChannel`  | FIFO      | No          | Yes         | Yes                 | Decoupling, request/reply, async handoff |

Construction is explicit:

```rust
use allora::{DirectChannel, QueueChannel};
let dc = DirectChannel::with_random_id();
let qc = QueueChannel::with_id("events");
```

Send/Receive (async-only API):

```rust
use allora::channel::PollableChannel;
use allora::{DirectChannel, Exchange, Message, QueueChannel};
#[tokio::main]
async fn main() -> allora::Result<()> {
    let dc = DirectChannel::with_id("notifications");
    let qc = QueueChannel::with_random_id();
    // fan-out to subscribers
    dc.subscribe(|ex| {
        assert_eq!(ex.in_msg.body_text(), Some("ping"));
        Ok(())
    });
    dc.send(Exchange::new(Message::from_text("ping"))).await?;
    qc.send(Exchange::new(Message::from_text("work"))).await?;
    let ex = qc.try_receive().await.expect("queued message");
    assert_eq!(ex.in_msg.body_text(), Some("work"));
    Ok(())
}
```

Example (additional):

```rust
use allora::{Exchange, Message, DirectChannel, QueueChannel, Channel};
use allora::channel::PollableChannel;
#[tokio::main]
async fn main() -> allora::Result<()> {
    let dc = DirectChannel::with_random_id();
    dc.subscribe(|ex| {
        assert_eq!(ex.in_msg.body_text(), Some("ping"));
        Ok(())
    });
    dc.send(Exchange::new(Message::from_text("ping"))).await?;
    let qc = QueueChannel::with_id("jobs");
    qc.send(Exchange::new(Message::from_text("job"))).await?;
    let ex = qc.try_receive().await.expect("job present");
    assert_eq!(ex.in_msg.body_text(), Some("job"));
    Ok(())
}
```

Correlation (QueueChannel only):

```rust
use allora::{Exchange, Message, QueueChannel};
use allora::channel::CorrelationSupport;
#[tokio::main]
async fn main() -> allora::Result<()> {
    let q = QueueChannel::with_random_id();
    let corr = q.send_with_correlation(Exchange::new(Message::from_text("req"))).await?;
    let ex = q.receive_by_correlation(&corr).await.expect("reply");
    assert!(ex.in_msg.body_text().is_some());
    Ok(())
}
```

### Current Limitations

- No persistent / disk-backed queues yet
- No backpressure or capacity controls
- DirectChannel errors short-circuit subscriber invocation (first error stops fan-out)

## HTTP Inbound Adapter (`http` feature)

> **Note:** Example runs a server; production code should add graceful shutdown.

```rust,no_run
use allora::{Exchange, Message};
use allora::channel::QueueChannel;
use allora::adapter::Adapter; // inbound adapter facade
#[tokio::main]
async fn main() -> allora::Result<()> {
    let channel = QueueChannel::with_id("http-pipe");
    let adapter = Adapter::inbound()
        .http()
        .host("127.0.0.1")
        .port(0)
        .channel(std::sync::Arc::new(channel))
        .in_only_202()
        .build();
    let _handle = adapter.serve();
    Ok(())
}
```

Adds headers: `http.method`, `http.path`; correlation ensured automatically.

## Implemented Patterns

| Pattern                 | Purpose                      | Notes                                             |
|-------------------------|------------------------------|---------------------------------------------------|
| Filter                  | Conditional pass/fail        | Custom error message via `with_error`.            |
| Content-Based Router    | Header-driven routing        | Equality match; one processor selected.           |
| Splitter                | Break composite message      | Currently sets only first piece as `out_msg`.     |
| Aggregator              | Size-based group aggregation | Text-only aggregation; `Clone` + clearable store. |
| Recipient List          | Sequential fan-out           | Short-circuits on error.                          |
| Correlation Initializer | Ensure correlation id        | Optional mirror header support.                   |

## Error Handling

Single enum: `Error` (`Processor`, `Routing`, `Aggregation`, `Serialization`, `Other`). Use helpers like
`Error::routing("...")`. Errors short-circuit route execution.

## Testing & Examples

- Integration tests under `tests/` (patterns, adapters, correlation helper, channels, route).
- Doc tests in each module (runnable or compile-only). Avoid ignored examples unless external resources required.
- Smoke test in `basic.rs` validates an empty route.

## Use Cases

Allora can help in a wide range of integration and messaging scenarios:

- Microservice orchestration: apply routing, filtering, aggregation between REST / gRPC boundaries.
- Event-driven pipelines: enrich, route, and correlate domain events before publishing to brokers.
- Request/Reply correlation: manage asynchronous fan-out and collect results (Aggregator + correlation id).
- Legacy modernization: wrap older HTTP/XML services with structured routing & transformation.
- IoT / telemetry ingestion: filter noisy sensor data and fan-out to enrichment processors.
- ETL mini-flows: split raw batch payloads, aggregate transformed records, forward to storage.
- Message enrichment: add headers, derive correlation, attach computed properties.
- Protocol bridging: accept HTTP, transform payloads, dispatch into internal channels for further processing.
- Pre-processing for ML/analytics: normalize / filter / route messages before downstream model inference.

## Future AI Integration (Design Ideas)

AI functionality is not implemented yet; these are forward-looking concepts that could be added behind an optional `ai`
feature to maintain lightweight defaults.

Potential directions:

- Intelligent routing: use an embedding or LLM classifier processor to choose a target recipient dynamically when static
  header routing is insufficient.
- Semantic content-based router: route on meaning rather than an explicit header value (e.g. classify ticket priority,
  document type).
- AI enrichment: call out to an inference endpoint to summarize or extract structured metadata from unstructured
  payloads and add headers/properties.
- Anomaly detection: flag unusual message patterns using a lightweight statistical or ML model before normal processing.
- Schema inference & mapping: assist in generating transformation processors by inferring field correspondences from
  sample payloads.
- Natural language route configuration: prototype CLI or prompt-based generation of Route definitions that are then
  persisted as Rust code.
- Auto test generation: suggest integration tests based on observed message samples and edge cases.
- Adaptive throttling: predict load and adjust rate limits on adapters dynamically.

Design considerations:

- Determinism: AI-assisted processors should degrade gracefully to pure rule-based logic if a model/endpoint is
  unavailable.
- Privacy & compliance: avoid sending sensitive payloads to external model providers unless explicitly configured.
- Observability: expose metrics (classification confidence, enrichment latency, model error rate) to trace AI impact.
- Caching: short-lived in-memory caching for repeated classification requests.

Suggested components (future optional crates or modules):

- `AiClassifierProcessor` – wraps a classification endpoint; adds `classification` header.
- `AiEnricherProcessor` – extracts entities or summaries into headers/properties.
- `AdaptiveRouter` – chooses downstream processor using a scoring function.
- `AnomalyFilter` – rejects or flags outliers prior to normal routing.

> NOTE: These names are conceptual; they are **not** present in the code yet and are intentionally omitted from current
> public API to keep the crate lean.

## Planned YAML DSL (Not Implemented Yet)

A declarative YAML DSL is planned so routes can be defined outside of Rust code and loaded at startup. This enables:

- Configuration driven deployment (change routing without recompiling)
- Easier sharing of integration recipes
- Generation and validation tooling (linting, diagram export)
- Potential authoring by non-Rust specialists (ops, integration teams)

Design principles (draft):

- Explicit and minimal (no hidden defaults that change semantics)
- One-to-one mapping to core primitives (Route, Processors, Patterns, Adapters)
- Validatable schema (YAML + JSON Schema or custom validator)
- Deterministic ordering (list order == execution order)
- Leverages standard formats: [YAML](https://yaml.org/), with potential
  for [JSON](https://www.json.org/), [TOML](https://toml.io/en/), [XML](https://www.w3.org/XML/) contributions.

### Example YAML (illustrative only)

```yaml
version: 0.1
features:
  async: true
  http: true
route:
  prelude:
    ensure_correlation:
      mirror_header: corr
  steps:
    - filter:
        id: only_hello
        expr: header:in_msg.body == "hello"   # simple literal equality
        on_reject:
          routing: not_hello
    - content_router:
        header: kind
        routes:
          hi:
            set_out:
              text: HI
          bye:
            set_out:
              text: BYE
    - aggregator:
        correlation_header: corr
        completion_size: 3
    - splitter:
        strategy: first_token_whitespace
adapters:
  inbound_http:
    bind: 0.0.0.0:8080
    route: route   # reference to defined route name
```

### Equivalent (conceptual) Rust

```rust
use allora::{Message, route::Route, patterns::filter::Filter,
             patterns::content_router::ContentBasedRouter,
             patterns::aggregator::Aggregator, patterns::splitter::Splitter,
             processor::ClosureProcessor};

fn build_route() -> Route {
    Route::with_correlation(Some("corr"))
        .add(Filter::with_error(|exchange| exchange.in_msg.body_text() == Some("hello"), "not_hello"))
        .add(ContentBasedRouter::new("kind")
            .when("hi", Box::new(ClosureProcessor::new(|exchange| {
                exchange.out_msg = Some(Message::from_text("HI"));
                Ok(())
            })))
            .when("bye", Box::new(ClosureProcessor::new(|exchange| {
                exchange.out_msg = Some(Message::from_text("BYE"));
                Ok(())
            }))))
        .add(Aggregator::new("corr", 3))
        .add(Splitter::new(|exchange| {
            exchange.in_msg.body_text()
                .map(|t| t.split_whitespace().map(Message::from_text).collect())
                .unwrap_or_else(Vec::new)
        }))
        .build()
}
```

### Planned Loader API (draft)

```rust
// Not implemented yet - conceptual example
fn load_route_from_yaml(path: &str) -> allora::Result<Route> {
    let spec = std::fs::read_to_string(path)?;
    let route = allora_dsl::parse_route(&spec)?; // returns Route
    Ok(route)
}
```

### Validation Ideas

- Required keys: version, route.steps
- Unknown keys produce warnings or errors (strict mode)
- Static checks: completion_size > 0, unique processor ids, valid header names
- Optional dry-run mode to simulate route on sample payloads

### Security & Safety Considerations

- No dynamic code execution (pure data specification)
- Disallow unbounded recursion or self-referential route graphs
- Size limits on inline literals to prevent accidental large config blobs

### Open Questions

- Expression language: minimal (only equality) vs embedding a small parser (e.g. header.kind == "hi")
- Extensibility: plugin registry for custom processors referenced by symbolic name
- Hot reload semantics: swap route atomically or drain in-flight exchanges

> NOTE: The YAML DSL is **not** available yet. Examples above are for illustration only and may change prior to
> implementation. Other serialization formats (JSON, TOML, XML) can be proposed via PR once the core YAML grammar
> stabilizes.

## Roadmap

**Near term**

- Outbound adapters (HTTP client, [Kafka](https://kafka.apache.org/) producer)
- Full fan-out Splitter (multiple Exchanges)
- Predicate-based Content Router + default route
- Time/predicate-based Aggregator completion
- Adapter middleware (tracing, metrics, auth, rate-limit) via [OpenTelemetry](https://opentelemetry.io/) integration
  planning
- (Planning) AI feature flag crate skeleton (no processing yet)
- (Planning) YAML DSL schema draft and parser prototype

**Mid term**

- Additional patterns: Resequencer, Dead Letter Channel, Retry, Circuit Breaker
- Persistent channel/queue backends ([Redis](https://redis.io/), Kafka topic, AMQP
  via [RabbitMQ](https://www.rabbitmq.com/))
- Observability integration (OpenTelemetry spans, metrics facade)
- Structured configuration & builder APIs
- Initial AI processors (classification, enrichment, anomaly detection) behind `ai` feature
- Initial YAML DSL loader (read-only, no hot reload)

**Long term**

- Natural language / prompt-based route generation tooling
- Adaptive routing and dynamic optimization using feedback loops
- Test generation assistance based on live traffic profiles
- Built-in model performance dashboards & feedback channel
- DSL hot reload with validation gates and rollback

## Contribution Opportunities (Help Wanted)

Near term tasks where community impact is high:

- Additional Adapters: outbound HTTP client, Kafka producer, AMQP consumer.
- New Patterns: Resequencer, Dead Letter Channel, Circuit Breaker, Retry.
- Aggregator Enhancements: time-based completion, predicate completion, partial emission.
- Observability: metrics facade trait, tracing spans integration.
- DSL: YAML schema draft (JSON Schema), parser with validation error mapping.
- Test Harness: property tests for correlation uniqueness & routing determinism.
- Docs: deeper tutorial (building a multi-step route), pattern decision matrix.

How to get started contributing:

1. Review the issues: https://github.com/fialucci/allora/issues
2. Check open pull requests: https://github.com/fialucci/allora/pulls
3. (Optional) Start a discussion: https://github.com/fialucci/allora/discussions
4. Outline scope, reference principles, link minimal pseudo-code.
5. Draft PR with focused changes + tests.
6. Request review; incorporate feedback quickly.

CI / workflows and future governance guidelines will be published under: https://github.com/fialucci/allora/wiki (
placeholder).

## License & Governance

This project is governed by the **Fialucci Foundation** and released under
the [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0). See the local [`LICENSE`](./LICENSE) file for
full terms.

By contributing, you agree that your code is provided under Apache-2.0 and that you have the rights to do so.

For trademark or governance questions, open an issue tagged `governance`.

---

## Architectural Overview

Allora centers on a small, composable set of types:

- **Message**: Data + headers. Automatically gets a `message_id` header; correlation can be added or ensured.
- **Exchange**: Carries an inbound message plus optional outbound message and internal properties across processors.
- **Processor**: Pure transformation unit. Provided patterns implement the `Processor` trait (or sync variant) without
  hidden side effects.
- **Route**: Ordered list of processors. Execution stops on the first error; correlation can be enforced early.
- **Channel**: A dispatch boundary. Currently an in-memory channel enabling synchronous or asynchronous route execution.
- **Adapter**: Bridges external I/O (e.g. HTTP inbound) and pushes constructed Exchanges into a Channel.

Data flow sketch (conceptual):
`Adapter -> Channel -> Route -> [Processor...Pattern...Processor] -> Exchange(out_msg)`

Design choices: explicit builder APIs, header-based selection, error-first failure, lazy correlation initialization,
optional async. No reflection, no runtime dependency graph generation.

## Expanded Use Cases & Pattern Mapping

Below are practical scenarios with the pattern(s) you would typically combine.

1. Microservice request enrichment
    - Patterns: Filter + Content-Based Router + Recipient List
    - Goal: Validate inbound headers, choose service variant, fan-out to supplemental processors.
    - Benefit: Reduces conditional logic sprawl inside service handlers.

2. Correlated fan-out / gather (parallel responses)
    - Patterns: Recipient List + Aggregator
    - Goal: Send a single request to multiple downstream endpoints, combine results when all arrive.
    - Benefit: Simplifies waiting logic; correlation identifies response sets.

3. Conditional transformation pipeline
    - Patterns: Filter + Splitter + Aggregator (batch post-processing)
    - Goal: Accept bulk payload, split into atomic records, aggregate a summary.
    - Benefit: Normalizes record handling while retaining a final batch summary.

4. Content-directed routing for heterogeneous formats
    - Patterns: Content-Based Router + Splitter (format-specific split)
    - Goal: Route messages based on a `format` header to appropriate parsing logic.
    - Benefit: Avoids nested match statements around formats.

5. Incremental data normalization for ML pre-processing
    - Patterns: Filter (schema checks) + Processor closures + Aggregator
    - Goal: Validate structure, add derived headers, build consolidated feature vector string.
    - Benefit: Reduces pre-processing code scattered across modules.

6. Reply correlation in a synchronous API facade
    - Patterns: Correlation Initializer + Aggregator (timeout planned) + Channel
    - Goal: Provide synchronous view over asynchronous internal fan-out.
    - Benefit: Transparent correlation ID bridging.

7. Progressive enrichment & early exit
    - Patterns: Filter + Content-Based Router
    - Goal: If a header says `skip_enrich`, route directly to responder; else progress through enrichment chain.
    - Benefit: Minimizes unnecessary compute and latency.

8. Dead letter / quarantine (planned)
    - Patterns (future): Filter + DeadLetterChannel
    - Goal: Divert malformed or repeatedly failing messages for inspection.
    - Benefit: Protects main flow reliability.

### Mini Example: Correlated Aggregation

```rust
use allora::{patterns::aggregator::Aggregator, processor::ClosureProcessor, route::Route, Exchange, Message};

fn example() -> allora::Result<()> {
    let route = Route::with_correlation(Some("corr"))
        .add(Aggregator::new("corr", 3))
        .add(ClosureProcessor::new(|exchange| {
            if exchange.out_msg.is_none() {
                exchange.out_msg = Some(Message::from_text("complete"));
            }
            Ok(())
        }))
        .build();
    Ok(())
}
```
