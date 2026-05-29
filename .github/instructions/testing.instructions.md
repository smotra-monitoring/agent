---
applyTo: "**/*.rs,tests/**"
---

# Testing Requirements

**CRITICAL**: The following rules apply without exception:
- **New features**: unit tests AND integration tests MUST be created alongside the implementation.
- **Bug fixes**: a regression test MUST be added that reproduces the bug before the fix and passes after.
- **Refactors**: existing tests must continue to pass; add new tests for any previously untested code paths uncovered during refactoring.

## Unit Tests

Placed in the **same file** as the implementation, inside a `#[cfg(test)]` module at the bottom.

Requirements:
- **Coverage**: Every public function, method, and struct must have corresponding unit tests.
- **Edge Cases**: Test edge cases, error conditions, and boundary values.
- **Mocking**: Use `mockall` crate for external dependencies.
- **Async Testing**: Use `#[tokio::test]` for async functions.
- **Assertions**: Use clear, descriptive assertions with helpful failure messages.
- **Test Organization**: Group related tests using nested modules.
- **Documentation**: Add doc comments to test functions explaining what is being tested and why.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    mod function_name_tests {
        use super::*;

        #[test]
        fn test_normal_case() { /* ... */ }

        #[test]
        fn test_edge_case() { /* ... */ }

        #[test]
        fn test_error_condition() { /* ... */ }
    }

    #[tokio::test]
    async fn test_async_function() { /* ... */ }
}
```

## Integration Tests

Placed in the `tests/` directory at the project root.

Requirements:
- **Real Scenarios**: Test real-world usage scenarios and complete feature flows end-to-end.
- **File Naming**: Use descriptive names — `heartbeat_integration_tests.rs`, `config_loading_tests.rs`.
- **Setup/Teardown**: Include proper setup and cleanup.
- **External Dependencies**: Mock external services (servers, databases) appropriately.
- **Async Runtime**: Use `#[tokio::test]` for async integration tests.
- **Timeouts**: Include reasonable timeouts for long-running tests.
- **Isolation**: Tests must not depend on each other.

```rust
// tests/feature_integration_tests.rs
use smotra_agent::*;

#[tokio::test]
async fn test_complete_workflow() {
    let config = setup_test_config();
    let result = run_complete_workflow(config).await;
    assert!(result.is_ok());
    cleanup_test_resources();
}
```

## Test Data and Fixtures
- `tests/fixtures/` — test data files (configs, sample data).
- `tests/common/mod.rs` — shared setup helpers.
- Use Builder or factory patterns for creating test objects.

## Testing Tools and Crates
- **tokio** — async testing with `#[tokio::test]`
- **mockall** — mock objects
- **proptest** / **quickcheck** — property-based testing (when appropriate)
- **rstest** — parameterized tests
- **assert_matches** — pattern matching assertions
- **tempfile** — temporary file/directory creation

## Workflow: When Generating Code
1. Write the implementation code.
2. Immediately write unit tests in the same file.
3. Create or update integration tests in `tests/`.
4. Run tests to verify they pass.
5. Document any test assumptions or requirements.

## Workflow: When Fixing Bugs
1. Write a **failing** regression test that reproduces the bug.
2. Fix the bug so the regression test passes.
3. Ensure all existing tests still pass.
4. Document the root cause in the test's doc comment.

## Coverage Goals
| Scope | Minimum |
|---|---|
| All modules | 80% |
| Core functionality (monitoring, reporting, config) | 95% |
| Error paths | 100% |

## Test Scenarios to Always Include
- ✅ Normal/happy path
- ✅ Invalid input handling
- ✅ Null/empty values
- ✅ Boundary conditions (min/max values)
- ✅ Concurrent access (for shared state)
- ✅ Resource exhaustion scenarios
- ✅ Network failures (for I/O operations)
- ✅ Timeout scenarios
- ✅ Configuration errors
- ✅ State transitions
