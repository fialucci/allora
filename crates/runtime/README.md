# Allora Runtime

High-level runtime and YAML DSL for building async message-driven integration flows (Enterprise Integration Patterns) in
Rust.

## Features

- YAML spec: channels, filters, service-activators
- Inventory-based service registration (`#[service]` macro)
- HTTP adapters available
- Programmatic builders (channels, filters, services)

## Quick Start (Async)

```rust
use allora::{Runtime, QueueChannel, Exchange, Message, Error};
use tokio::runtime::Runtime as TokioRuntime;
fn main() -> Result<(), Error> {
    let rt = Runtime::new().with_config_file("./allora.yml").run()?; // blocking build
    let queue = rt.channel::<QueueChannel>("my_channel");
    let mut ex = Exchange::new(Message::from_text("hello"));
    TokioRuntime::new().unwrap().block_on(async { queue.send(ex).await.unwrap(); });
    Ok(())
}
```

## Minimal YAML

```yaml
version: 1
channels:
  - id: input
    kind: queue
service-activators:
  - id: svc.greeter
    ref-name: greeter
    from: input
    to: input
```

## Service Macro

```rust
use allora::{service, Service, Exchange, Message, Result};
#[derive(Clone, Debug)]
struct Greeter;
impl Greeter { pub fn new() -> Self { Self } }
#[service(name = "greeter")]
impl Greeter {}
#[async_trait::async_trait]
impl Service for Greeter {
    async fn process(&self, ex: &mut Exchange) -> Result<()> {
        if let Some(name) = ex.in_msg.body_text() { ex.in_msg.set_body_text(format!("Hello {name}")); }
        Ok(())
    }
}
```

## API Notes

All channel and endpoint operations are async-only. Legacy method names like `send_async`, `try_receive_async`,
`send_with_correlation_async` were removed. Use:

- `queue.send(exchange).await?` – enqueue message
- `queue.try_receive().await` – attempt FIFO dequeue (Option)
- `queue.send_with_correlation(exchange).await?` – enqueue and assign correlation id
- `queue.receive_by_correlation(&id).await` – receive correlated exchange

These functions all return `Result` or `Option` variants; there are no sync fallbacks.

## License

Apache-2.0
