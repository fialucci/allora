**allora** (Italian) [ah-LOH-rah]: an adverb used to start, transition, summarize, or gently prompt action. Its
flexibility mirrors this project's goal: move messages forward clearly and deliberately.

> **Allora** is a Rust-native implementation of core Enterprise Integration Patterns (EIP): channels, adapters,
> routing, and correlation for building clear, high-performance integration flows.

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](./LICENSE)
[![Status](https://img.shields.io/badge/status-early--alpha-orange.svg)](#roadmap)
[![Rust](https://img.shields.io/badge/rust-2021%20edition-informational.svg)](https://www.rust-lang.org/)
[![Contributions](https://img.shields.io/badge/contributions-welcome-brightgreen.svg)](#contribute)
[![Issues](https://img.shields.io/github/issues/fialucci/allora)](https://github.com/fialucci/allora/issues)
[![Pull Requests](https://img.shields.io/github/issues-pr/fialucci/allora)](https://github.com/fialucci/allora/pulls)

---

## Why Allora?

Most modern systems are distributed and event-driven. Services communicate over HTTP, queues, streams, and
background jobs. Wiring these pieces together in a reliable and observable way is hard:

- Routing logic gets scattered across services.
- Ad-hoc glue code becomes brittle and hard to test.
- Correlation, error handling, and retries are subtle and often bolted on late.

For years, Enterprise Integration Patterns frameworks like **Apache Camel**, **Spring Integration**, and **Mule** have
addressed this on the JVM. They provide well-understood concepts: channels, filters, routers, splitters, aggregators,
and correlation.

Those frameworks are powerful, but they are also:

- **Heavyweight and reflection-centric** – large dependency graphs, runtime wiring, complex lifecycle.
- **JVM-bound** – less suitable for small containers, edge workloads, and tight latency budgets.
- **Less predictable** – GC pauses and dynamic wiring can make performance analysis harder.

Other ecosystems (Go, Node.js) offer message and HTTP libraries, but typically lack a cohesive, type-safe EIP layer
with first-class routing, correlation, and patterns.

**Rust** is a good fit for a different style of integration framework:

- **Predictable performance**: no GC, low latency, and small static binaries.
- **Compile-time guarantees**: ownership, lifetimes, and types catch many issues before runtime.
- **Async I/O**: via Tokio, enabling high concurrency with low overhead.
- **Memory safety**: without pervasive defensive copying.

**Allora** brings a focused subset of EIP to Rust:

- Minimal, explicit primitives instead of a container or application server.
- Integration flows that look like idiomatic **async** Rust.
- Clear semantics for message IDs, correlation, and routing.

If you like the ideas behind Camel or Spring Integration, but want them as a **lean, predictable Rust library**, Allora
is for you.

---

## Key Features & Principles

Allora is built around a small set of guiding principles. When adding features or designing flows, these are the
reference point.

1. **Clarity over magic**  
   Prefer explicit `Route::new().add(...).build()` over reflection, annotations, or hidden DI containers. Flows are
   code, not opaque configuration.

2. **Lean primitives first**  
   Keep the foundation small and composable:
    - `Message`: payload + headers
    - `Exchange`: inbound/outbound messages + properties
    - `Processor`: async transformation
    - `Route`: ordered pipeline of processors
    - `Channel`: dispatch and delivery abstraction
    - `Adapter`: bridges between external systems and channels

3. **Predictable performance**  
   Favor zero-cost abstractions. Avoid surprise allocations or background threads. Throughput and latency should be
   inspectable and understandable.

4. **Async by design**  
   Allora is built for the async Rust ecosystem. It integrates cleanly with `tokio` and other async runtimes; there is
   no separate synchronous API surface.

5. **Incremental extensibility**  
   New patterns (Aggregator, Splitter, Routers, etc.) and adapters should drop into existing routes without forcing
   rewrites.

6. **Opt-in complexity**  
   HTTP adapters and future AI/DSL features are optional. Your binary only includes what you enable.

7. **Transparent errors**  
   Fail fast with typed `Error` variants; no silent discards. Routing failures, processor issues, and aggregation
   conditions surface clearly.

8. **Deterministic routing**  
   Given the same headers and payloads, routes should behave the same way. Correlation is explicit and avoids racy
   shared global state.

9. **Security & safety before convenience**  
   A YAML-based DSL already exists and is strictly declarative: it validates configuration structure and never
   executes arbitrary user-supplied code.

10. **Observability by design**  
    Well-defined integration points for metrics and tracing so production flows can be measured, debugged, and
    trusted.

---

## Getting Started

Allora is in **early alpha**. APIs may change before 1.0 and the crate is not yet published on crates.io.

All examples assume an async runtime, typically [`tokio`](https://tokio.rs/).

### Installation

Add Allora directly from Git in your `Cargo.toml`:

```toml
[dependencies]
# Latest main branch (may be unstable)
allora = { git = "https://github.com/fialucci/allora" }
tokio = { version = "1", features = ["full"] }
```

Pin to a tagged release (recommended for reproducible builds):

```toml
[dependencies]
allora = { git = "https://github.com/fialucci/allora", tag = "v0.1.0" }
tokio = { version = "1", features = ["full"] }
```

Or pin to a specific commit:

```toml
[dependencies]
allora = { git = "https://github.com/fialucci/allora", rev = "<commit-sha>" }
tokio = { version = "1", features = ["full"] }
```

For local development against a checked-out copy:

```toml
[dependencies]
allora = { path = "../allora" }
tokio = { version = "1", features = ["full"] }
```

### Minimal "Hello World" Route (Async)

The simplest flow: take a message, apply a filter, and run the route asynchronously.

```rust
use allora::{patterns::filter::Filter, route::Route, Exchange, Message};

#[tokio::main]
async fn main() -> allora::Result<()> {
    // Create an exchange with a simple text message
    let mut exchange = Exchange::new(Message::from_text("hello"));

    // Build a route with a single Filter processor
    let route = Route::new()
        .add(Filter::new(|ex| ex.in_msg.body_text() == Some("hello")))
        .build();

    // Run the route asynchronously
    route.run(&mut exchange).await?;

    Ok(())
}
```

### Async Route with Transformation

A slightly richer example that transforms the message:

```rust
use allora::{route::Route, processor::ClosureProcessor, Exchange, Message};

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

---

## Core Concepts

Allora centers on a small set of concepts that map directly to Enterprise Integration Patterns.

### Message

A `Message` represents a unit of data in the system.

- Payload types: text, bytes, JSON, or empty.
- Headers: key/value metadata.
- An auto-generated `message_id` header (UUID v4) for traceability.

```rust
use allora::Message;

let msg = Message::from_text("hello");
let id = msg.message_id();
```

### Exchange

An `Exchange` carries a message through a route.

- Contains an inbound `Message`.
- May contain an optional outbound `Message`.
- Holds internal properties and correlation metadata.

```rust
use allora::{Exchange, Message};

let mut exchange = Exchange::new(Message::from_text("hello"));
let corr_id = exchange.correlation_id(); // generated lazily if missing
```

### Processor

A `Processor` transforms an `Exchange`. Allora currently focuses on async processors and pattern implementations that
work with an async runtime.

Examples include:

- `Filter`
- `ContentBasedRouter`
- `Splitter`
- `Aggregator`
- `RecipientList`
- `CorrelationInitializer`

### Route

A `Route` is an ordered pipeline of processors. It executes asynchronously and stops on the first error, making
behavior explicit and debuggable.

```rust
use allora::{patterns::filter::Filter, route::Route};

fn build_route() -> Route {
    Route::with_correlation(Some("corr"))
        .add(Filter::with_error(
            |ex| ex.in_msg.body_text() == Some("hello"),
            "not_hello",
        ))
        .build()
}
```

### Channel

A `Channel` is a dispatch boundary. It decouples producers and consumers and enables different delivery semantics.

Allora currently provides:

| Channel         | Buffering | Subscribers | Dequeue API | Correlation Helpers | Typical Use                           |
|-----------------|-----------|-------------|-------------|---------------------|---------------------------------------|
| `DirectChannel` | None      | Yes         | No          | No                  | Immediate fan-out / in-memory pub-sub |
| `QueueChannel`  | FIFO      | No          | Yes         | Yes                 | Decoupling, request/reply, handoff    |

```rust
use allora::{channel::PollableChannel, DirectChannel, Exchange, Message, QueueChannel};

#[tokio::main]
async fn main() -> allora::Result<()> {
    let dc = DirectChannel::with_id("notifications");
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

Correlation helpers for `QueueChannel`:

```rust
use allora::{channel::CorrelationSupport, Exchange, Message, QueueChannel};

#[tokio::main]
async fn main() -> allora::Result<()> {
    let q = QueueChannel::with_random_id();

    let corr = q
        .send_with_correlation(Exchange::new(Message::from_text("req")))
        .await?;

    let ex = q.receive_by_correlation(&corr).await.expect("reply");
    assert!(ex.in_msg.body_text().is_some());

    Ok(())
}
```

### Adapter

An **adapter** bridges external systems (HTTP, messaging, etc.) with Allora channels.

Currently, an HTTP inbound adapter is available via the `http` feature:

```rust,no_run
use allora::{adapter::Adapter, channel::QueueChannel, Exchange, Message};

#[tokio::main]
async fn main() -> allora::Result<()> {
    let channel = QueueChannel::with_id("http-pipe");

    let adapter = Adapter::inbound()
        .http()
        .host("127.0.0.1")
        .port(0) // let OS pick a free port
        .channel(std::sync::Arc::new(channel))
        .in_only_202()
        .build();

    let _handle = adapter.serve();

    Ok(())
}
```

The adapter adds headers such as `http.method` and `http.path`, and handles correlation automatically.

---

## Use Cases

Allora is designed for integration-oriented Rust services where message flow is a first-class concern. Typical
scenarios:

- **Microservice orchestration**  
  Apply routing, filtering, and aggregation between REST/gRPC boundaries without scattering glue code across services.

- **Event-driven pipelines**  
  Enrich, route, and correlate domain events before publishing them to brokers, analytics, or downstream systems.

- **Request/reply correlation**  
  Manage asynchronous fan-out and gather responses (for example, using `RecipientList` + `Aggregator` with correlation
  IDs).

- **Legacy modernization**  
  Wrap older HTTP/XML services behind structured routing and transformation logic, keeping the edge clean and
  testable.

- **IoT / telemetry ingestion**  
  Filter noisy sensor data, enrich it, and fan out to multiple downstream processors or data sinks.

- **ETL mini-flows**  
  Split batch payloads into records, process each through a route, and aggregate results.

- **Message enrichment**  
  Attach derived headers, correlation metadata, and computed properties in a reusable way.
  1
- **Protocol bridging**  
  Accept HTTP requests, transform payloads, and dispatch into internal channels for further processing or background
  work.

---

## Roadmap

Allora is early (`0.1.x`). Expect API improvements as we converge on a stable core.

### Near Term

- Outbound adapters (HTTP client, Kafka producer).
- Full fan-out Splitter (multiple `Exchange` outputs).
- Predicate-based Content-Based Router with default route.
- Time- or predicate-based Aggregator completion strategies.
- Adapter middleware (tracing, metrics, auth, rate limiting).
- Initial OpenTelemetry integration design.
- AI feature-flagged crate skeleton (no processing yet).
- YAML DSL schema draft and parser prototype.

### Mid Term

- Additional patterns: Resequencer, Dead Letter Channel, Retry, Circuit Breaker.
- Persistent channel/queue backends (Redis, Kafka, AMQP/RabbitMQ).
- Observability integration (OpenTelemetry spans, metrics facade).
- Structured configuration and builder APIs for larger flows.
- Initial AI processors (classification, enrichment, anomaly detection) behind an `ai` feature.
- Initial YAML DSL loader (read-only, no hot reload).

### Long Term

- Natural language / prompt-based route generation tooling.
- Adaptive routing and dynamic optimization using feedback loops.
- Test generation assistance based on live traffic profiles.
- Built-in model performance dashboards and feedback channels (for AI-enabled routes).
- DSL hot reload with validation gates and rollback.

These plans are directional, not contractual. If you rely on a potential feature, please open an issue or discussion to
help shape the design.

---

## Contribute

Contributions are very welcome. There is significant surface area for patterns, adapters, observability, and
documentation.

Some good first areas:

- **Adapters**: outbound HTTP client, Kafka producer, AMQP consumer.
- **Patterns**: Resequencer, Dead Letter Channel, Circuit Breaker, Retry.
- **Aggregator enhancements**: time-based completion, predicate-based completion, partial emission.
- **Observability**: metrics facade traits, tracing spans, and structured logging.
- **DSL**: YAML schema (JSON Schema), parser with clear validation errors.
- **Testing**: property tests for correlation uniqueness and routing determinism.
- **Documentation**: deeper tutorials, pattern catalog, and troubleshooting guides.

How to get started:

1. Browse open issues: <https://github.com/fialucci/allora/issues>
2. Check open pull requests: <https://github.com/fialucci/allora/pulls>
3. (Optional) Start a discussion: <https://github.com/fialucci/allora/discussions>
4. Outline your idea, including scope and alignment with the [Key Features & Principles](#key-features--principles).
5. Open a focused PR with tests.
6. Iterate based on review.

Future contribution and governance guidelines will be documented in the repository wiki.

---

## License & Governance

This project is governed by the **Fialucci Foundation** and released under
the [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0). See the local [`LICENSE`](./LICENSE) file for the
full license text.

By contributing, you agree that your contributions are provided under Apache-2.0 and that you have the rights to
contribute them.

For trademark or governance questions, please open an issue tagged `governance`.
