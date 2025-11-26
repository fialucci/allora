# Allora

Async-first integration framework for Rust: compose channels, routes, and HTTP adapters to build clear, message-driven
systems.

This crate is the **user-facing entry point** to the Allora ecosystem. It re-exports the core primitives, HTTP adapters,
runtime, and macros into a single, convenient API surface.

---

## What is Allora?

Allora is a Rust-native take on the classic *Enterprise Integration Patterns* (EIP) toolbox.

Instead of ad-hoc glue code scattered across your services, you describe **message flows**:

- **Messages & exchanges** – strongly-typed payloads plus headers and metadata
- **Channels** – in-memory pipes for moving exchanges between components
- **Processors & services** – async functions that transform or route messages
- **Adapters** – bridge HTTP and other transports into/out of your flows

Think: *“Camel / Spring Integration, but as lean, async Rust.”*

---

## When should I use Allora?

Use Allora when you want to:

- Orchestrate **message-based flows** between async components inside a Rust service
- Replace ad-hoc glue code with **explicit routes and channels**
- Attach **HTTP endpoints** to your flows via inbound/outbound adapters
- Separate **business logic** (`#[service]` functions) from wiring (channels, routes, runtime)

A simpler web framework (Axum, Actix, etc.) may be enough if you only need a few HTTP handlers and no real
routing/correlation logic between them.

---

## Status

- **Version:** `0.0.x` — experimental / early-stage`
- APIs may change between micro releases
- Not yet battle-tested in production
- Async-only, built on `tokio`

More EIP patterns (splitter, aggregator, content-based router, more outbound adapters, etc.) are planned, but consider
the API **unstable** for now.

---

## Installation

Add `allora` to your `Cargo.toml`:

```toml
[dependencies]
allora = "0.0.1"
```

---

## Quick example: in-memory flow

A tiny in-memory flow that sends a message through a channel and prints the result.

```rust
use allora::{Channel, DirectChannel, Exchange, Message, QueueChannel, Result, Runtime};

#[tokio::main]
async fn main() -> Result<()> {
  let rt = Runtime::new().with_config_file("allora.yml").run()?;

  // These channel names must be defined in your Allora configuration (allora.yml)
  let input: DirectChannel = rt.channel("input_channel");
  let output: QueueChannel = rt.channel("output_channel");

  input
          .send(Exchange::new(Message::from_text("World")))
          .await?;

  let ex = output
          .try_receive()
          .await
          .expect("processed message");

  println!("Message: {}", ex.in_msg.body_text().unwrap_or("(empty)"));
  Ok(())
}
```

This example assumes a small YAML spec (for example `allora.yml`) that defines the `input_channel` and `output_channel`
channels and wires them to whatever services or processors you want. See the project documentation and examples for
sample YAML configurations.

---

## Quick example: HTTP inbound adapter

Expose an HTTP endpoint that feeds messages into your flow. This example uses the HTTP inbound adapter and a YAML
config.

```rust,no_run
use allora::{Result, Runtime};

// ensures your services are registered via #[service]
mod http_echo_service;

#[tokio::main]
async fn main() -> Result<()> {
    let adapter = Runtime::new()
        .with_config_file("allora.yml")
        .run()? // load config
        .http_inbound_adapters()
        .get(0)
        .expect("http inbound adapter in config")
        .clone();

    // Start listening; the adapter returns its bound address and base path
    let _server = adapter.clone().spawn_serve();

    // Your flow now receives HTTP requests as messages
    Ok(())
}
```

This requires a small YAML spec (`allora.yml`) describing your channels, services, and HTTP inbound adapter.

---

## Concepts: messages, channels, routes

A few core ideas, all available from the `allora` facade:

- **Message & Exchange**
    - `Message` holds the payload (`Payload`) and headers
    - `Exchange` wraps the inbound message and optional outbound message, plus metadata
- **Channels**
    - Traits: `Channel`, `PollableChannel`, `SubscribableChannel`, `CorrelationSupport`
    - Implementations: `DirectChannel` (in-memory handoff), `QueueChannel` (buffered)
- **Processors & services**
    - `Processor` and `ClosureProcessor` let you build reusable processing steps
    - `#[service]` (from `allora-macros`) turns async functions into discoverable services
- **Runtime & routes**
    - `Runtime` loads your YAML spec and wires channels, endpoints, adapters, and services
    - Routes are defined in the spec as sequences of processors and channels

Together, these pieces give you explicit, type-safe integration flows rather than ad-hoc chains of function calls.

---

## HTTP adapters

The facade re-exports HTTP integration types from `allora-http`:

- **Inbound** – `HttpInboundAdapter`
    - Map HTTP requests into `Exchange` instances
    - Supports multiple message exchange patterns (MEPs), such as:
        - `InOut` – wait for downstream processing and return the transformed response
        - `InOnly202` – respond with `202 Accepted` and continue processing asynchronously
- **Outbound** – `HttpOutboundAdapter`
    - Dispatches exchanges over HTTP, using `out_msg` (or `in_msg` as a fallback)

You typically don’t instantiate these types directly in application code; instead you:

- Describe inbound/outbound HTTP endpoints in a YAML configuration
- Or use the core adapter builder (`allora::adapter::Adapter`) and the HTTP extension traits

---

## Workspace layout

The `allora` crate is a facade over several internal crates in the workspace:

- `allora` (this crate) – user-facing entry point; re-exports the main APIs
- `allora-core` – message model, exchanges, channels, processors, services, adapters
- `allora-http` – HTTP inbound/outbound adapters
- `allora-runtime` – runtime engine and YAML DSL for wiring flows
- `allora-macros` – proc macros like `#[service]` used to define services

---

## License

Licensed under Apache-2.0. See the `LICENSE` file in the repository for details.
