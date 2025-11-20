# Allora HTTP Inbound Adapter Example

Demonstrates using an HTTP inbound adapter plus a service activator to implement a simple greeting gateway.
Incoming HTTP POST bodies become Exchanges on an inbound channel; the service produces a reply on the reply channel
which the adapter returns in the HTTP response.

## Config (`allora.yml`)

Defines two channels:

- `receiveChannel` (direct) for inbound HTTP request Exchanges.
- `replyChannel` (queue) for service replies.

Declares one HTTP inbound adapter:

```yaml
http-inbound-adapters:
  - id: http.receiveGateway
    host: 127.0.0.1
    port: 18080
    path: /receiveGateway
    methods: [ POST ]
    request-channel: receiveChannel
    reply-channel: replyChannel
```

Declares one service activator:

```yaml
service-activators:
  - id: svc.httpEcho
    ref-name: http_echo
    from: receiveChannel
    to: replyChannel
```

The `http_echo` service is registered via the `#[service(name="http_echo")]` macro and transforms `World` →
`Hello World!`.

## Run

Inside the example directory (`examples/basic/http`):

```bash
cargo run
```

Auto-discovery will locate the local `./allora.yml`.

From anywhere else (e.g. repository root) pass the manifest path and the runtime config explicitly:

```bash
cargo run --manifest-path examples/basic/http/Cargo.toml -- --runtime examples/basic/http/allora.yml
```

Notes:

- `--` separates Cargo arguments from program arguments.
- `--runtime <path>` tells the example which configuration file to use when auto-discovery would otherwise fail.

## Test Request

Port in config: `18080`.

```bash
curl -X POST http://127.0.0.1:18080/receiveGateway -d 'World'
```

Expected response body:

```
Hello World!
```

## Shutdown

Press CTRL+C to terminate.

## Notes

- Adapter uses request/reply (MEP InOut) behavior because a `reply-channel` is declared.
- A correlation ID header is automatically ensured on inbound Exchanges.
- Path normalization yields `/` internally when the base path matches the full request path; headers reflect the
  normalized path.
- Modify the service or payload to experiment with different greetings.
