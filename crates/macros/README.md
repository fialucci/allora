# Allora Macros

Procedural macros for the Allora integration library.

`allora-macros` provides proc-macros that reduce boilerplate when defining services and registering them with the
Allora runtime inventory.

Most users access these macros via the top-level `allora` crate (for example, `use allora::service;`). This crate is
primarily useful if you need to depend on the macros without pulling in the full facade, or if you are extending
Allora itself.

---

## What is `allora-macros`?

This crate contains the procedural macros used by Allora to:

- Annotate **services** so they can be discovered by the runtime
- Generate the necessary inventory entries to connect YAML configuration to Rust types

It is tightly coupled to Allora’s types and traits. It is **not** intended as a general-purpose macro toolbox.

---

## When should I use this crate directly?

You typically don’t need to depend on `allora-macros` directly. Prefer the `allora` facade crate, which re-exports the
macros you need.

Reach for `allora-macros` directly only when:

- You are building **library code** that wants to use the macros but should not depend on the full `allora` facade
- You are **extending Allora** itself and need a direct dependency on the macro crate

Application code should usually use:

```rust
use allora::service; // re-export from the facade crate
```

---

## Provided macros

Currently this crate exposes one primary attribute macro:

### `#[service]`

Attribute macro for registering a service implementation with the Allora runtime inventory.

- **Placement:** apply to an **inherent `impl` block** for your service type (not a trait impl)
- **Purpose:** registers a constructor in the service inventory so YAML `ref-name` values can refer to your type
- **Arguments:**
    - `#[service(name = "custom_name")]` – explicit identifier; must match the YAML `ref-name`
    - `#[service]` – omit `name` to default to the type name with whitespace removed

Constraints / validation:

- Must be on an **inherent impl** (`impl Type { ... }`), not on a trait impl
- The impl block must define a **zero-arg `new()` constructor**
- Generic impl blocks are rejected (no generics support yet)

---

## Examples

### Async service registered with `#[service]`

The most common usage is via the `allora` facade:

```rust
use allora::{service, Exchange, Result, Service};

#[derive(Clone, Debug)]
struct Greeter;

impl Greeter {
    pub fn new() -> Self {
        Self
    }
}

// Register this type with the Allora service inventory using an explicit name
#[service(name = "greeter")]
impl Greeter {}

#[async_trait::async_trait]
impl Service for Greeter {
    async fn process(&self, ex: &mut Exchange) -> Result<()> {
        if let Some(name) = ex.in_msg.body_text() {
            ex.in_msg.set_body_text(format!("Hello {name}"));
        }
        Ok(())
    }
}

fn main() {}
```

In your YAML configuration, you can now refer to this service by `ref-name: greeter`.

### Using the default name

If you omit the `name = ...` argument, the macro will derive the service name from the type:

```rust
use allora::{service, Service};

#[derive(Clone, Debug)]
struct MyProcessor;

impl MyProcessor {
    pub fn new() -> Self {
        Self
    }
}

// Name will default to "MyProcessor"
#[service]
impl MyProcessor {}

fn main() {}
```

The derived name is currently the Rust type name with whitespace removed.

---

## Notes & caveats

- `allora-macros` follows the same **versioning** as the rest of the Allora crates
- The macro APIs may change while Allora is in the **0.x** phase
- The macros assume Allora’s traits and inventory types are available; they are not intended to be used outside the
  Allora ecosystem

For a higher-level introduction to Allora and its integration patterns, see the `allora` crate documentation.

---

## License

Licensed under Apache-2.0. See the `LICENSE` file in the repository for details.
