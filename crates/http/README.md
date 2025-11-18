# Allora HTTP

Async HTTP inbound and outbound adapters for Allora integration flows.

## Inbound Message Exchange Patterns

- **InOut**: await downstream processing and return transformed body
- **InOnly202**: immediately return HTTP 202 and process asynchronously

## Outbound

Dispatch `Exchange` content (preferring `out_msg` fallback to `in_msg`) via HTTP request.

## Inbound Example (Async)

```rust
use allora_http::Adapter;
use allora_core::{channel::QueueChannel, ChannelRef};
use std::sync::Arc;
use tokio::runtime::Runtime;
fn main() {
    let ch = Arc::new(QueueChannel::with_id("pipe"));
    let inbound = Adapter::inbound()
        .http()
        .host("127.0.0.1")
        .port(0)
        .channel(ch.clone())
        .in_only_202()
        .build();
    // Runtime::new().unwrap().block_on(async { inbound.serve().await.unwrap(); });
}
```

## Outbound Skeleton

```rust
// let outbound = Adapter::outbound().http().post("http://localhost:8080/endpoint").build();
// outbound.dispatch(exchange).await?;
```

## License

Apache-2.0
