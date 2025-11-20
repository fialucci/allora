# Allora Hello World Example

Demonstrates a minimal send/receive flow using two in-memory channels and a service activator.
The inbound message body `World` is transformed by the `hello_world` service into `Hello World!` and printed.

## Config (`allora.yml`)
```yaml
version: 1
channels:
  - id: input_channel
  - id: output_channel
    kind: queue
service-activators:
  - id: svc.hello
    ref-name: hello_world
    from: input_channel
    to: output_channel
```
Channels:
- `input_channel` (direct) – receives the initial Exchange.
- `output_channel` (queue) – holds the processed Exchange for retrieval.

Service Activator:
- `hello_world` – registered via `#[service(name = "hello_world")]`; updates payload to `Hello <name>!`.

## Run (inside example directory)
From `examples/basic/helloworld`:
```bash
cargo run
```
Auto-discovery locates the local `./allora.yml`.

## Run (from repository root)
Provide the manifest path (auto-discovery usually still finds the config for this example, but be explicit for consistency):
```bash
cargo run --manifest-path examples/basic/helloworld/Cargo.toml
```
Optionally pass a config explicitly (if you alter layout):
```bash
cargo run --manifest-path examples/basic/helloworld/Cargo.toml -- --runtime examples/basic/helloworld/allora.yml
```

## Expected Output
```
Message: Hello World!
```
(Initial message `World` was transformed by the service.)

## Shutdown
Press CTRL+C or allow the program to exit after printing the message.

## Notes
- Direct channel dispatch is synchronous; the service runs immediately upon `send`.
- The queue channel enables polling (`try_receive`) for the processed Exchange.
- Modify `hello_service.rs` or the initial payload in `main.rs` to experiment with different greetings.
- Correlation headers are not demonstrated here; see the HTTP example for request/reply patterns.
