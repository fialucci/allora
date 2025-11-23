# Allora HTTP Inbound Adapter (Minimal Example)

This example starts a single HTTP inbound adapter that accepts POST requests at:

```
http://127.0.0.1:18080/receiveGateway
```

The adapter forwards each request payload to a direct channel (`receiveChannel`), a service activator (`http_echo`) builds a reply (`Hello <payload>!`) on `replyChannel`, and the adapter returns it (MEP = InOut).

> Note: Using a service activator is not required for an inbound adapter to function; it is included here purely to demonstrate simple message transformation. Remove the `service-activators` section to echo the original payload unchanged.

## Configuration (`allora.yml`)
```yaml
version: 1
channels:
  - id: receiveChannel
    kind: direct
  - id: replyChannel
    kind: queue
http-inbound-adapters:
  - id: http.receiveGateway
    host: 127.0.0.1
    port: 18080
    path: /receiveGateway
    methods: [ POST ]
    request-channel: receiveChannel
    reply-channel: replyChannel
service-activators:
  - id: svc.httpEcho
    ref-name: http_echo
    from: receiveChannel
    to: replyChannel
```

## Run
From repository root or example directory:
```bash
cargo run --manifest-path examples/basic/http-inbound/Cargo.toml
```

Console will show the adapter listening. (Press Ctrl+C to stop.)

## Test with curl
```bash
curl -X POST http://127.0.0.1:18080/receiveGateway -d 'World'
```
Expected response body:
```
Hello World!
```

## Notes
- Request/reply (InOut) is enabled because a `reply-channel` is configured.
- Correlation ID header is ensured automatically.
- Service activator is optional; it only changes the body to a greeting.
- Change the posted text to see a different greeting.

## Shutdown
Press Ctrl+C.
