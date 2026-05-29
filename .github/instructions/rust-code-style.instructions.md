---
applyTo: "**/*.rs"
---

# Code Organization

- **Module Structure**: `mod.rs` files should contain only module declarations (`mod`) and re-exports (`pub use`). No functional code (implementations, functions, structs) should be placed in `mod.rs` files — these belong in dedicated files within the module directory.
- **Visibility**: Make methods and functions private by default. Only mark items as `pub` when they are explicitly needed as part of the public API. Avoid proliferating `pub fn` unnecessarily — every public item increases the API surface and maintenance burden. Ask yourself: "Does this need to be public, or is it an implementation detail?"
- `utilities` module — private support functions for the containing module only.
- `support` module — external functions usable by other crates in the cargo workspace.

# Rust Design Patterns

Follow idiomatic Rust design patterns where applicable to improve code quality, maintainability, and API ergonomics.

## Builder Pattern
Use for complex types with many optional fields or configuration options (4+ fields where some are optional):
```rust
struct Config { /* fields */ }

impl Config {
    pub fn builder() -> ConfigBuilder { ConfigBuilder::default() }
}

struct ConfigBuilder { /* same fields, all Option<T> */ }

impl ConfigBuilder {
    pub fn field_name(mut self, value: Type) -> Self {
        self.field_name = Some(value);
        self
    }
    pub fn build(self) -> Result<Config, BuilderError> { /* validation */ }
}
```

## Type State Pattern
Use to enforce correct API usage at compile time for objects with distinct lifecycle states:
```rust
struct Agent<State> {
    // common fields
    state: PhantomData<State>,
}
struct Unclaimed;
struct Claimed;
struct Running;

impl Agent<Unclaimed> {
    pub fn claim(self) -> Result<Agent<Claimed>, Error> { /* ... */ }
}
impl Agent<Claimed> {
    pub fn start(self) -> Result<Agent<Running>, Error> { /* ... */ }
}
```

## Factory Pattern
Use when object creation requires coordination of multiple components or creating different implementations of a trait:
```rust
pub fn create_checker(check_type: CheckType) -> Result<Box<dyn Checker>, Error> {
    match check_type {
        CheckType::Ping => Ok(Box::new(PingChecker::new())),
        CheckType::Http => Ok(Box::new(HttpChecker::new())),
    }
}
```

## Newtype Pattern
Wrap primitive types to prevent mixing incompatible values and add domain-specific methods:
- Examples: `AgentId(Uuid)`, `Timestamp(i64)`, `ResponseTime(Duration)`

## RAII (Resource Acquisition Is Initialization)
Acquire resources in constructors, release in `Drop` implementations. Use guard types to ensure cleanup. Make resource lifetime explicit through type signatures.

## Pattern Usage Guidelines
- **Don't over-engineer**: Use patterns only when they provide clear value.
- **Start simple**: Begin with straightforward implementations, refactor to patterns when complexity grows.
- **Document patterns**: When using a pattern, add a comment explaining why it was chosen.
- **Consistency**: Use the same pattern for similar problems throughout the codebase.

# Logging / Tracing

Use the `tracing` crate for all logging. Support different log levels and structured output formats. Do not use `println!` or `eprintln!` for observability.
