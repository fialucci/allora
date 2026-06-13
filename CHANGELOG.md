# Changelog

All notable changes to Allora are documented in this file.

The format is loosely based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project is pre-1.0; breaking changes can land in any release.

## [0.0.9] — Unreleased

### Breaking

- **`http-outbound-adapter` YAML schema**: the `host:` / `port:` / `base-path:`
  triple has been **removed and replaced by a single `url:` field**. The new
  field must include the scheme (`http://` or `https://`). No backwards-compatible
  alias is provided — consumers must update their YAML.

  **Migration:**

  ```diff
   http-outbound-adapters:
     - id: chain_submit
  -    host: 127.0.0.1
  -    port: 8080
  -    base-path: /oracle/submissions
  +    url: http://127.0.0.1:8080/oracle/submissions
       method: POST
       from: signed_submissions
       to: chain_acks
  ```

  HTTPS endpoints just change the scheme:

  ```yaml
  url: https://devnet.fialucci.org/oracle/submissions
  ```

  The same applies to the single-adapter form (`http-outbound-adapter:`).

- **`HttpOutboundAdapterBuilder` (Rust API)**: the `.host(...)`, `.port(...)`,
  `.base_path(...)`, and `.path(...)` builder methods are gone. Use
  `.url("http://host:port/path")` instead. Existing callers must update.

- **`HttpOutboundAdapterSpec` constructors**: signatures of `new` and `with_id`
  changed — they now take a single `url: &str` rather than the previous
  positional `host`, `port`, `base_path`, `path` tuple.

### Added

- **HTTPS support for outbound dispatch.** `HttpOutboundAdapter` now uses
  `reqwest::Client` (with the `rustls-tls` feature) under the hood and can
  dial `https://` endpoints out of the box. Certificate validation uses the
  system trust store; no `accept_invalid_certs` knob is configurable from YAML.
- **Eager URL validation.** Invalid URLs (`":::not a url"`, unsupported
  schemes like `ftp://`, …) surface as `Error::Serialization` at config-load
  time, not on first dispatch.
- **Test-only escape hatch**: `HttpOutboundAdapterBuilder::dangerous_accept_invalid_certs(bool)`
  for using self-signed certificates in integration tests. Not exposed in
  YAML — production code paths cannot enable it.
- **Integration tests for the http crate** under `crates/http/tests/`,
  including `http_outbound_dispatches_over_https` which spins up an in-process
  rustls server with an `rcgen`-minted self-signed cert and proves the new
  code path actually negotiates TLS end-to-end.

### Changed

- All inline YAML fixtures in `crates/runtime/src/runtime.rs` outbound tests
  migrated to the `url:` schema.
- Schemas under `schema/v1/http-outbound-adapter*.schema.yml` updated to the
  new shape (`url` is `required`; old `host`/`port`/`base-path` properties
  removed).
- `examples/basic/http-outbound/allora.yml` updated to the new schema.
- Workspace version bumped: `0.0.8` → `0.0.9` across all crates.

### Unchanged

- HTTP **inbound** adapter, its YAML schema, its struct, its parser, and its
  tests are untouched. Inbound `host` / `port` / `path` fields stay as-is.
