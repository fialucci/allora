# Allora HTTP

HTTP adapters for the Allora integration library.

`allora-http` connects HTTP requests and responses to Allora **channels** and **exchanges**. It lets you expose
webhook-style HTTP endpoints as inbound gateways into your flows, and send HTTP requests from within those flows as
outbound adapters.

Most applications reach HTTP through the top-level `allora` crate (which re-exports these types), but you can also
depend on `allora-http` directly if you need lower-level control.

---

## What is `allora-http`?

`allora-http` provides:

- **HTTP inbound adapters** – accept HTTP requests, turn them into `Exchange` instances, and forward them to channels
- **HTTP outbound adapters** – send HTTP requests based on the content of an `Exchange`
- **Runtime glue** for running async HTTP servers/clients on top of a `tokio` runtime

At a high level:

- An inbound adapter listens on a host/port and publishes each incoming request to a configured channel
- An outbound adapter consumes exchanges and performs HTTP requests, typically as part of a processor chain

---

## When should I use this crate directly?

Most users should access HTTP adapters through the `allora` facade crate, which re-exports the main adapter types and
builder traits.

Use `allora-http` directly when you:

- Want to build custom HTTP integration behavior at a lower level
- Are implementing your own facade or runtime on top of Allora
- Need to depend on the HTTP layer without pulling in the full `allora` crate

Application code can typically just enable HTTP via the top-level crate and import from there:

```rust
use allora::{HttpInboundAdapter, HttpOutboundAdapter};
```

---

## Common patterns

Typical integration flows built with `allora-http` include:

- **HTTP inbound gateway → channel → service**
    - An `HttpInboundAdapter` accepts an HTTP request
    - The request is mapped to an `Exchange` and sent to a channel (for example, a `QueueChannel`)
    - A service or processor reads from the channel, performs business logic, and may produce a response
- **HTTP outbound from processors**
    - A processor prepares an `Exchange` and hands it to an `HttpOutboundAdapter`
    - The adapter sends an HTTP request (e.g. POST to a webhook URL) and optionally maps the response back into the
      exchange

Inbound adapters support different **Message Exchange Patterns (MEPs)**:

- `InOut` – wait for downstream processing and return the transformed body to the caller
- `InOnly202` – immediately return HTTP 202 and continue processing asynchronously

---

## Example: HTTP inbound adapter to a channel

This example shows a minimal HTTP inbound adapter that:

- Binds to `127.0.0.1` on an ephemeral port
- Forwards each request body into a `QueueChannel` as an `Exchange`
- Lets a consumer task read and print the messages

```rust,no_run
use allora_core::{
    channel::{PollableChannel, QueueChannel},
    message::{Exchange, Message},
};
use allora_http::Adapter;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Channel that will receive requests as Exchanges
    let channel = Arc::new(QueueChannel::with_id("http-inbound"));

    // Build an inbound HTTP adapter bound to localhost on an ephemeral port (0)
    let inbound = Adapter::inbound()
        .http()
        .host("127.0.0.1")
        .port(0)
        .channel(channel.clone())
        .in_only_202() // respond 202 immediately, process asynchronously
        .build();

    // Start serving in the background
    let _server = inbound.clone().spawn_serve();

    // In a real app, another task would receive from the channel in a loop
    if let Some(ex) = channel.try_receive().await {
        if let Some(body) = ex.in_msg.body_text() {
            println!("Received HTTP payload: {body}");
        }
    }
}
```

In this pattern the HTTP adapter is a gateway: HTTP requests are turned into `Exchange` instances and pushed into an
Allora channel, where normal processors and services can act on them.

---

## Example: outbound skeleton

An outbound adapter can be used from within a processor or service to send HTTP requests based on an `Exchange`:

```rust,no_run
use allora_core::message::{Exchange, Message};
use allora_http::Adapter;

#[tokio::main]
async fn main() {
    // Build an outbound HTTP adapter (for example, POST to a webhook)
    let outbound = Adapter::outbound()
        .http()
        .post("http://localhost:8080/endpoint")
        .build();

    // Prepare an Exchange to send
    let ex = Exchange::new(Message::from_text("payload"));

    // Dispatch the exchange as an HTTP request
    outbound
        .dispatch(ex)
        .await
        .expect("outbound HTTP dispatch should succeed");
}
```

Outbound adapters typically prefer the `out_msg` of an exchange, falling back to `in_msg` when deciding what to send.

---

## Configuration & runtime notes

- `allora-http` is **async-first** and expects a `tokio` runtime
- When used via the `allora` facade, HTTP adapters integrate with the Allora runtime and configuration model (for
  example, YAML-based specs)
- Direct use of this crate gives you full control over how adapters are built and started, but you are responsible for
  wiring channels and services yourself

---

## Relationship to `allora`

The top-level `allora` crate re-exports the main HTTP adapter types, so most applications only need to:

- Depend on `allora`
- Import `HttpInboundAdapter` / `HttpOutboundAdapter` (or use the runtime/DSL that wires them automatically)

Use `allora-http` directly only if you need fine-grained access to the HTTP layer itself.

---

## Status & limitations

- Part of the **0.0.x** Allora ecosystem – APIs may change as the framework evolves
- Focused on async HTTP (server and client) for integration scenarios

Check the `allora` crate for overall project status and roadmap.

---

## License

Licensed under Apache-2.0. See the `LICENSE` file in the repository for details.
