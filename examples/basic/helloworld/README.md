# Hello World Example

A minimal send/receive over a single in-memory channel.

## Running (Dev)

```bash
cargo run --manifest-path examples/basic/helloworld/Cargo.toml
```

## Running (Optimized Release)

```bash
cargo run --release --manifest-path examples/basic/helloworld/Cargo.toml
```

## Expected Output

You will see trace logs similar to:

```
TRACE allora::channel: send enqueued channel_id=hello_channel async=true corr_id=None in_body=Some("Hello World!")
TRACE allora::channel: receive dequeued channel_id=hello_channel kind=try_receive_async phase=dequeued async=true queue_size=Some(0) corr_id=None attempts=None elapsed_ms=None timeout_ms=None in_body=Some("Hello World!") out_body=None
```
