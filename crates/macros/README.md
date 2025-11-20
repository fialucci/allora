# Allora Macros

Procedural macro support for registering services with the runtime inventory.

## Macro: `#[service]`

Annotate an inherent `impl` block that includes a zero-arg `new()` constructor. The macro submits an inventory
descriptor so YAML `ref-name` values can wire your service.

### Example (Async Service)

```rust
use allora::{service, Service, Exchange, Message, Result};
#[derive(Clone, Debug)]
struct Greeter;
impl Greeter { pub fn new() -> Self { Self } }
#[service(name = "greeter")]
impl Greeter {}
#[async_trait::async_trait]
impl Service for Greeter {
    async fn process(&self, ex: &mut Exchange) -> Result<()> {
        if let Some(name) = ex.in_msg.body_text() { ex.in_msg.set_body_text(format!("Hello {name}")); }
        Ok(())
    }
}
fn main() {}
```

## Naming

- Omit `name=` to default to the type name (whitespace removed).
- Use `#[service(name="custom")]` to match `ref-name` in YAML.

## Validation

- Requires inherent impl (not a trait impl).
- Requires zero-arg `new()` in that impl.
- Rejects generic impl blocks.

## License

Apache-2.0
