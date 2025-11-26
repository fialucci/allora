# Allora Runtime

Runtime and channels for the Allora integration library.

`allora-runtime` provides the execution engine and channel abstractions used to run Allora integration flows. It
manages channels, scheduling, and glue code so messages (exchanges) can move between services and processors in an
async Rust application.

Most applications interact with the runtime through the top-level `allora` crate, but you can depend on
`allora-runtime` directly when you need finer control over channels or want to embed the runtime into an existing
application.

---

## What is `allora-runtime`?

`allora-runtime` sits between the low-level primitives in `allora-core` and the user-facing `allora` facade:

- Provides **channel management** for direct and queue channels
- Hosts the **runtime** that wires channels, services, and adapters together
- Integrates with **Tokio** for async execution
- Supports configuration-driven flows (for example, via YAML specs when used through `allora`)

It focuses on *running* message flows rather than defining low-level EIP patterns.

---

## Channels

Channels are the primary way messages move between components in Allora. This crate supports several channel types
(re-exported through the `allora` facade):

- **DirectChannel**
    - Point-to-point handoff between producer and consumer
    - Good for simple, low-latency routing inside a single task or small set of tasks
- **QueueChannel**
    - Buffered FIFO queue
    - Decouples producers and consumers and smooths out bursts of traffic

Under the hood, these implement traits from `allora-core` such as `Channel`, `PollableChannel`, and
`SubscribableChannel`.

---

## Using the runtime

A minimal example of using the runtime and channels via the top-level `allora` facade. This shows how to:

- Start the runtime
- Look up channels by ID
- Send an `Exchange` through a channel
- Receive the processed result

```rust
use allora::{Channel, DirectChannel, Exchange, Message, QueueChannel, Result, Runtime};

#[tokio::main]
async fn main() -> Result<()> {
    // Build and start the runtime (configuration loading is handled inside Runtime)
    let rt = Runtime::new().run()?;

    // These channel names must match your Allora configuration
    let input: DirectChannel = rt.channel("input_channel");
    let output: QueueChannel = rt.channel("output_channel");

    // Send a message into the flow
    input
        .send(Exchange::new(Message::from_text("World")))
        .await?;

    // In a real app you might await a signal or use a loop; here we just sleep briefly
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Consume the processed message from the output channel
    if let Some(ex) = output.try_receive().await {
        println!("Received: {}", ex.in_msg.body_text().unwrap_or("(empty)"));
    }

    Ok(())
}
```

This example uses the `Runtime` to obtain channels and move an `Exchange` through a simple flow. In a larger
application, services and HTTP adapters would be wired in through configuration or builder APIs.

---

## When should I use this crate directly?

Most users should start with the `allora` crate, which re-exports `Runtime`, channel types, and higher-level APIs.

Reach for `allora-runtime` directly when you:

- Need **fine-grained control over channels** and runtime lifecycle
- Want to **embed Allora’s runtime** into an existing async application without pulling the full facade
- Are building your own tooling or framework layer on top of Allora

If you just want to define services, configure channels, and run flows, prefer `allora`.

---

## Relationship to other crates

`allora-runtime` is one piece of the Allora ecosystem:

- **`allora-core`** – message model, exchanges, channels traits, processors, and low-level EIP primitives
- **`allora-runtime`** (this crate) – runtime engine and channel management for executing flows
- **`allora-http`** – HTTP inbound/outbound adapters that plug into channels managed by the runtime
- **`allora-macros`** – proc macros like `#[service]` for registering services with the runtime inventory
- **`allora`** – top-level facade that re-exports these pieces into a single user-facing API

---

## Status & stability

- Part of the **0.0.x** Allora ecosystem – APIs are still evolving
- Expect breaking changes between minor versions while the design is refined

Check the `allora` crate for high-level project status and roadmap.

---

## License

Licensed under Apache-2.0. See the `LICENSE` file in the repository for details.
