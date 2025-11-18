# Allora CLI

Command-line utilities for working with Allora integration specs and components.

## Status

Experimental. Interfaces may change.

## Features (Planned)

- Validate YAML specs (`allora.yml`).
- Generate channel/service boilerplate.
- Inspect inventory registered services.
- Run quick local flow simulations.

## Install

Currently unpublished; use path dependency:

```toml
[dependencies]
allora-cli = { path = "crates/cli" }
```

## Usage

```bash
cargo run -p allora-cli -- validate ./allora.yml
```

## License

Apache-2.0

## Contributing

Issues & PRs welcome. Keep changes small and well-tested.

