# Allora (Facade Crate)

This crate is the **user-facing entry point** to the Allora ecosystem. It re-exports the core primitives, HTTP
adapters, runtime, and macros into a single, convenient API surface.

Use this crate when you want to **build integration flows** (channels, routes, adapters, YAML DSL) without managing the
underlying sub-crates directly.

## What You Get

- Re-exports from:
    - `allora-core` (messages, exchanges, channels, processors, patterns)
    - `allora-http` (HTTP inbound adapters)
    - `allora-runtime` (YAML DSL + runtime engine)
    - `allora-macros` (proc macros such as `#[service]`)
- A cohesive, async-only integration API
- Types and traits intended for application authors, not just framework internals

## Install

```toml
[dependencies]
allora = "0.0.1"
tokio = { version = "1", features = ["full"] }
```

## Minimal Example (Async Route)

```rust
use allora::{patterns::filter::Filter, route::Route, Exchange, Message};

#[tokio::main]
async fn main() -> allora::Result<()> {
    let mut exchange = Exchange::new(Message::from_text("hello"));

    let route = Route::new()
        .add(Filter::new(|ex| ex.in_msg.body_text() == Some("hello")))
        .build();

    route.run(&mut exchange).await?;

    Ok(())
}
```

## YAML DSL & Runtime

The `allora` crate re-exports `allora-runtime`, so you only need `allora` as a dependency to use the YAML DSL. Point the
runtime at a YAML spec:

```rust,no_run
use allora::{Runtime, Exchange, Message};

#[tokio::main]
async fn main() -> allora::Result<()> {
    let runtime = Runtime::from_file("./allora.yml")?;

    let mut ex = Exchange::new(Message::from_text("hello"));
    runtime.send("input", ex).await?;

    Ok(())
}
```

See the main project README for the high-level overview and patterns catalog.

## License

Apache-2.0
