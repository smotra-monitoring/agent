## Feature PR

Feature PRs are accepted only via a `SKILL.md` file. See [CONTRIBUTING.md](../../CONTRIBUTING.md#feature-prs).

### Skill reference (required)

- Path to `SKILL.md`: `.github/skills/<feature-name>/SKILL.md`
- Approved feature issue: #

### Summary

<!-- What does the feature do and why is it needed? Write this in your own words. -->

### AI assistance disclosure

<!-- State which tools, if any, were used and which parts of the PR they produced. -->

### Checklist

- [ ] I opened an issue and a maintainer approved the feature
- [ ] This PR adds/references a `SKILL.md` that defines purpose, inputs/outputs, constraints, and acceptance criteria
- [ ] The implementation follows the referenced `SKILL.md`
- [ ] Unit tests added in `#[cfg(test)]` modules
- [ ] Integration tests added in `tests/`
- [ ] Feature documentation added in `docs/features/<FEATURE_NAME>.md`
- [ ] `cargo test` passes locally
- [ ] I can explain and defend every change in this PR
