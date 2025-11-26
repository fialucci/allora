# Allora Core

Core types and primitives for the Allora integration framework.

This crate provides the **message model, exchanges, IDs, metadata, channels, and low-level EIP building blocks** that
power the higher-level `allora` facade crate.

Most users should depend on `allora` directly; reach for `allora-core` when you need fine-grained control or want to
integrate Allora’s patterns into your own runtime or adapters.

---

## What is `allora-core`?

`allora-core` is the foundational library for Allora. It defines:

- **Messages & exchanges** – payloads, headers, and correlation metadata
- **Channels** – in-memory direct and queue channels, channel traits
- **Processors & routes** – traits and types for building processing pipelines
- **Enterprise Integration Patterns (EIP)** – filter, content-based router, splitter, aggregator,
  recipient list, correlation initializer, and more

It deliberately avoids pulling in HTTP, YAML/runtime wiring, or proc macros.

---

## When should I use this crate directly?

Use `allora-core` directly when you:

- Are building your **own runtime or orchestration layer** and just need the primitives
- Want to embed Allora’s **channels, message model, and patterns** into an existing system
- Need to experiment with **low-level EIP constructs** without the top-level `allora` facade and runtime features

If you just want to define services, wire channels, and run flows, prefer the `allora` crate instead.

---

## Key concepts

Some of the most important concepts exposed by `allora-core`:

- **Message**
    - Represents a payload plus headers
    - Helper constructors like `Message::from_text("...")`
- **Exchange**
    - Wraps an inbound message (`in_msg`) and optional outbound message, with correlation metadata
    - Carries headers and IDs across a flow
- **Channels**
    - Traits: `Channel`, `PollableChannel`, `SubscribableChannel`, `CorrelationSupport`
    - Implementations: `DirectChannel` (in-memory handoff), `QueueChannel` (buffered FIFO)
- **Processors & routes**
    - Traits to implement message processors and build routes/compositions over channels
    - Building block for higher-level patterns in the Allora ecosystem

---

## Examples

### Create a message and exchange

```rust
use allora_core::message::{Exchange, Message};

fn main() {
    let msg = Message::from_text("ping");
    let ex = Exchange::new(msg);

    assert_eq!(ex.in_msg.body_text(), Some("ping"));
}
```

### Use a queue channel asynchronously

```rust
use allora_core::{
    channel::{PollableChannel, QueueChannel},
    message::{Exchange, Message},
};

#[tokio::main]
async fn main() {
    let ch = QueueChannel::with_id("demo");

    ch.send(Exchange::new(Message::from_text("ping")))
        .await
        .expect("send should succeed");

    let ex = ch
        .try_receive()
        .await
        .expect("message should be available");

    assert_eq!(ex.in_msg.body_text(), Some("ping"));
}
```

These examples show the raw primitives; higher-level routing, runtime wiring, and HTTP integration live in the other
crates in the Allora workspace.

---

## Relationship to the `allora` crate

The `allora` crate is the **facade** most users should depend on:

- It re-exports the core types from `allora-core`
- It adds runtime wiring (`allora-runtime`), HTTP adapters (`allora-http`), and macros (`allora-macros`)

If you start with `allora` and later need lower-level control (for example, writing a custom adapter, runtime, or
integration with another system), you can drop down to `allora-core` and use these primitives directly.

---

## Status & stability

- Part of the **0.0.x** Allora ecosystem – APIs are still evolving
- Expect breaking changes between minor versions while the design is refined

Check the `allora` crate for the high-level project status and roadmap.

---

## License

Licensed under Apache-2.0. See the `LICENSE` file in the repository for details.
