## Security PR

All four sections below are mandatory. Incomplete security PRs are closed. See [CONTRIBUTING.md](../../CONTRIBUTING.md#security-prs).

### 1. Vulnerability explanation (required)

<!-- Affected component and versions, root cause, impact, severity (e.g. CVSS), and any CVE/CWE reference. -->

### 2. Steps to reproduce (required)

<!-- Exact numbered steps that work on a clean checkout. -->

1.
2.
3.

### 3. Unit test reproducing the vulnerability (required)

- Test name and file path:
- [ ] The test fails without the fix
- [ ] The test passes with the fix

### 4. Proposed fix or mitigation (required)

<!-- Describe the change and why it closes the vulnerability. If no full fix is possible, describe the mitigation. -->

### AI assistance disclosure

<!-- State which tools, if any, were used and which parts of the PR they produced. -->

### Checklist

- [ ] Vulnerability explanation is complete
- [ ] Reproduction steps are complete and verified
- [ ] Unit test reproduces the vulnerability
- [ ] Fix or mitigation is included
- [ ] `cargo test` passes locally
- [ ] I can explain and defend every change in this PR
