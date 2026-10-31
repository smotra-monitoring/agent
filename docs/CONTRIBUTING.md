# Contributing to Smotra Agent

Thank you for contributing! This document outlines the development workflow and requirements for all contributors.

---

## Contribution Policy

We receive a high volume of automated, machine-generated pull requests. To keep contributions genuine and reviewable, every PR must fall into exactly one of the categories below and use the matching [pull request template](../.github/PULL_REQUEST_TEMPLATE/). PRs that do not follow these rules are closed without review.

| Category | Accepted only when | PR template |
|---|---|---|
| Feature | Submitted via a `SKILL.md` file (see [Feature PRs](#feature-prs)) | [feature.md](../.github/PULL_REQUEST_TEMPLATE/feature.md) |
| Security | Includes explanation, reproduction steps, unit test and proposed fix (see [Security PRs](#security-prs)) | [security.md](../.github/PULL_REQUEST_TEMPLATE/security.md) |
| Bug fix | Linked to a GitHub issue (see [Bug Fixes](#bug-fixes)) | [bugfix.md](../.github/PULL_REQUEST_TEMPLATE/bugfix.md) |

Additional rules:

- You must be able to explain and defend every line of your PR in review. Unexplained or unverifiable generated code is rejected.
- Do not open PRs for issues that have not been confirmed by a maintainer.
- One PR per issue. Do not bundle unrelated changes.
- Disclose any AI assistance used in the PR template.

---

## Development Workflow

### Feature PRs

Feature PRs are accepted **only via a `SKILL.md` file**. Features are not accepted as unsolicited code changes.

1. Open an issue proposing the feature and wait for maintainer approval.
2. Describe the feature as a skill in `.github/skills/<feature-name>/SKILL.md` (see existing skills in `.github/skills/`). The skill must define the feature's purpose, when it applies, inputs/outputs, constraints, and acceptance criteria.
3. Submit a PR using the [feature template](../.github/PULL_REQUEST_TEMPLATE/feature.md) that references the `SKILL.md` file.
4. After the skill is approved, the implementation must follow it, including:
   - **Unit tests** in the same file as the code (inside `#[cfg(test)]` module).
   - **Integration tests** in the `tests/` directory covering end-to-end workflows.
   - **Feature documentation** in `docs/features/<FEATURE_NAME>.md` (see [Documentation Requirements](#documentation-requirements)).
   - A passing `cargo test`.

### Security PRs

Security PRs must contain **all** of the following. Incomplete security PRs are closed.

1. **Detailed explanation of the vulnerability**: affected component and versions, root cause, impact, and severity.
2. **Steps to reproduce**: exact, numbered steps that a maintainer can follow on a clean checkout.
3. **Unit test that reproduces the vulnerability**: a test that fails before the fix and passes after it.
4. **Proposed fix or mitigation**: the code change, or a documented mitigation if a full fix is not possible.

Submit using the [security template](../.github/PULL_REQUEST_TEMPLATE/security.md).

### Bug Fixes

Bugs must be **submitted as GitHub issues** first, using the [bug report template](../.github/ISSUE_TEMPLATE/bug_report.md). A bug-fix PR is only considered when it links to a confirmed issue (for example `Fixes #123`).

When fixing a bug:

1. **Open (or find) the GitHub issue** and wait for a maintainer to confirm it.
2. **Write a failing regression test** that reproduces the bug *before* applying the fix.
3. **Fix the bug** so the regression test passes.
4. **Ensure all existing tests still pass**: `cargo test`
5. **Document the root cause** in the test's doc comment.
6. Submit using the [bug fix template](../.github/PULL_REQUEST_TEMPLATE/bugfix.md), linking the issue.

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

- All docs live in `docs/` — including this `CONTRIBUTING.md`.
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

1. Confirm your change fits one of the categories in the [Contribution Policy](#contribution-policy).
2. Fork the repository and create a branch.
3. Follow the workflow above (implementation + tests + documentation).
4. Ensure `cargo test` passes with no failures.
5. Open the PR with the matching template and complete every checklist item.
