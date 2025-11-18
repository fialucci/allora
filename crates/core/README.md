# Allora Core

Core primitives for the Allora integration framework: Message, Exchange, Channels (direct & queue), Route, Processors,
and Enterprise Integration Patterns (filter, content-based router, splitter, aggregator, recipient list, correlation
initializer).

## Highlights

- Lightweight in-memory channels (direct handoff + FIFO queue)
- Async-first API (`send_async`, `try_receive_async`, correlation helpers)
- Extensible processor & route abstraction
- Common EIP patterns with thorough inline docs & doctests

## Install

```toml
[dependencies]
allora-core = "0.1"
```

## Quick Example (Async)

```rust
use allora_core::{channel::QueueChannel, PollableChannel, Exchange, Message};
use tokio::runtime::Runtime;
fn main() {
    let ch = QueueChannel::with_id("demo");
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        ch.send_async(Exchange::new(Message::from_text("ping"))).await.unwrap();
        let ex = ch.try_receive_async().await.unwrap();
        assert_eq!(ex.in_msg.body_text(), Some("ping"));
    });
}
```

## Patterns

- Filter (message predicate / early reject)
- Content-Based Router (header-driven routing)
- Splitter / Aggregator (fan-out, fan-in by correlation)
- Recipient List (sequential fan-out)
- Correlation Initializer (ensures `correlation_id` header)

## License

Apache-2.0
