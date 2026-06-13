# Allora HTTP Outbound Adapter Example

Minimal demonstration of using the Allora runtime to obtain an HTTP outbound adapter from `allora.yml`, send a single request, print the HTTP 202 response, then exit. The inbound side is a plain Rust (Hyper) one-shot server – no Allora inbound adapter or services.

## What Happens
1. A tiny Hyper server starts on `127.0.0.1:18080` and will return `202 Accepted` with body `accepted`, then shut down after the first request.
2. The Allora runtime loads configuration, builds the outbound adapter.
3. The outbound adapter dispatches a message with body `ping`.
4. Server logs the received payload and responds.
5. Example prints the status and body, then terminates.

## Config (`allora.yml`)
Channels are optional. This example omits them entirely and defines only one outbound adapter:
```yaml
version: 1
http-outbound-adapters:
  - id: http.outboundEcho
    url: http://127.0.0.1:18080/receiveGateway
    method: POST
```

`url` must include the scheme. Use `https://...` for TLS endpoints; certificate
validation uses the system trust store. The old `host` + `port` + `base-path`
fields were removed in 0.0.9 (see CHANGELOG).

## Run
From repository root:
```bash
cargo run --manifest-path examples/basic/http-outbound/Cargo.toml
```
Or inside the example directory:
```bash
cargo run
```

## Sample Output
```
server received payload='ping'
status=202 body=accepted
```

## Key Points
- Outbound adapter is built by the runtime (no manual builder code in `main`).
- Inbound side is intentionally plain Rust to isolate outbound usage.
- One-shot server shuts itself down after the first request to keep the example minimal.
- Channels section is optional at the top level; omitted here.

## Next Steps (Optional)
- Change `ping` to another payload.
- Point the adapter at a real endpoint.
- Add headers / different HTTP method.

Enjoy experimenting with outbound integration in Allora.
