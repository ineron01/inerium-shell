## Description

Briefly describe the purpose of this Pull Request and the problem it resolves.

## Type of Change

- [ ] `fix`: Bug fix (non-breaking change which fixes an issue)
- [ ] `feat`: New feature (non-breaking change which adds functionality)
- [ ] `refactor`: Code reorganization or cleanup without behavior changes
- [ ] `chore`: Maintenance, dependencies, or CI updates
- [ ] `docs`: Documentation updates

## Checklist

- [ ] My code adheres to the project's coding and formatting standards (`.editorconfig`).
- [ ] `cargo check --workspace --all-targets` passes without errors.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` reports zero warnings.
- [ ] `cargo test --workspace` passes all tests.
- [ ] Shell scripts pass `bash -n` and Lua files pass `luac -p`.
- [ ] No hardcoded usernames, host paths, credentials, or private keys leak into git.
