# Contributing to Smotra Agent

Thank you for contributing! This document outlines the development workflow and requirements for all contributors.

---

## Development Workflow

### New Features

When implementing a new feature:

1. **Write the implementation** following the code style and patterns described in the project instructions.
2. **Write unit tests** in the same file as the code (inside `#[cfg(test)]` module).
3. **Write integration tests** in the `tests/` directory covering end-to-end workflows.
4. **Create feature documentation** in `docs/features/<FEATURE_NAME>.md` (see [Documentation Requirements](#documentation-requirements)).
5. **Run all tests** to verify they pass: `cargo test`

### Bug Fixes

When fixing a bug:

1. **Write a failing regression test** that reproduces the bug *before* applying the fix.
2. **Fix the bug** so the regression test passes.
3. **Ensure all existing tests still pass**: `cargo test`
4. **Document the root cause** in the test's doc comment.

### Refactors

When refactoring code:

1. Existing tests must continue to pass.
2. Add new tests for any previously untested code paths uncovered during refactoring.
3. Update documentation if the public API or behavior changes.

---

## Testing Requirements

**CRITICAL**: Tests are mandatory. Pull requests without the appropriate tests will not be accepted.

### Unit Tests

- Placed in the **same file** as the code being tested, inside a `#[cfg(test)]` module.
- Every public function, method, and struct must have corresponding unit tests.
- Test edge cases, error conditions, and boundary values.
- Use `#[tokio::test]` for async functions.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_case() { /* ... */ }

    #[test]
    fn test_error_condition() { /* ... */ }

    #[tokio::test]
    async fn test_async_function() { /* ... */ }
}
```

### Integration Tests

- Placed in the `tests/` directory.
- Named descriptively: `<feature>_integration_tests.rs`.
- Test complete feature flows from start to finish.
- Use `tests/common/mod.rs` for shared setup helpers.
- Use `tests/fixtures/` for test data files.

```rust
// tests/my_feature_integration_tests.rs
use smotra_agent::*;

#[tokio::test]
async fn test_complete_workflow() {
    let config = setup_test_config();
    let result = run_complete_workflow(config).await;
    assert!(result.is_ok());
}
```

### Coverage Goals

| Scope | Minimum Coverage |
|---|---|
| All modules | 80% |
| Core functionality (monitoring, reporting, config) | 95% |
| Error paths | 100% |

### Recommended Test Scenarios

- ✅ Normal / happy path
- ✅ Invalid input handling
- ✅ Null / empty values
- ✅ Boundary conditions (min/max values)
- ✅ Concurrent access (shared state)
- ✅ Network failures (I/O operations)
- ✅ Timeout scenarios
- ✅ Configuration errors
- ✅ State transitions

---

## Documentation Requirements

**CRITICAL**: All new features must be documented. Pull requests without documentation will not be accepted.

### Feature Documentation

Create a file in `docs/features/<FEATURE_NAME>.md` containing:

- **Overview**: What the feature does and why it exists.
- **Usage examples**: Complete, runnable code examples.
- **Configuration**: Relevant `config.toml` options.
- **Integration**: How it works with other parts of the system.
- **Edge cases and pitfalls**: Known limitations and common mistakes.
- **API reference**: Key types and functions.

See existing feature docs in `docs/features/` as templates.

### Example Files

Runnable examples go in the `examples/` directory using the naming convention `XXX_name.rs` (e.g., `001_basic_usage.rs`).

### Documentation Location Rules

- All docs live in `docs/` — never in the project root.
- Feature-specific docs belong in `docs/features/`.
- OpenAPI specs belong in `api/openapi/api/spec.yaml`.
- Cross-reference docs using relative paths from `docs/`.

---

## Code Style

- Follow idiomatic Rust patterns (Builder, Type State, Factory, Newtype — see copilot instructions).
- `mod.rs` files contain only `mod` declarations and `pub use` re-exports — no functional code.
- Make everything private by default; only mark `pub` when explicitly needed as part of the public API.
- Use the `tracing` crate for logging.

---

## Running Tests

```bash
# Run all tests
cargo test

# Run a specific test
cargo test test_name

# Run integration tests only
cargo test --test '*'

# Run with output
cargo test -- --nocapture
```

---

## Submitting a Pull Request

1. Fork the repository and create a feature branch.
2. Follow the workflow above (implementation + tests + documentation).
3. Ensure `cargo test` passes with no failures.
4. Submit the PR with a clear description referencing any related issues.
