# Contributing to AgentVault

Thank you for your interest in contributing to AgentVault! This guide will help you get started.

## Prerequisites

- **Rust 1.75+** — Install via [rustup](https://rustup.rs/)
- **Git** — For version control and submitting pull requests
- **A GitHub account** — To fork the repo and open PRs

## Development Setup

```bash
# Clone your fork
git clone https://github.com/<your-username>/AgentVault.git
cd AgentVault

# Verify your Rust toolchain
rustup show           # Should show stable >= 1.75
rustc --version

# Build the project
cargo build

# Run the test suite
cargo test --workspace
```

## Build & Quality Commands

| Task | Command |
|------|---------|
| Build | `cargo build` |
| Build (release) | `cargo build --release` |
| Test | `cargo test --workspace` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Format | `cargo fmt --all` |
| Format check | `cargo fmt --all -- --check` |

> **Tip:** Run all checks before pushing: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Making Changes

1. **Fork & branch** — Create a feature branch from `main`:
   ```bash
   git checkout -b feat/my-feature
   ```
2. **Write code** — Follow existing patterns and style conventions.
3. **Add tests** — All new functionality must include tests.
4. **Run checks** — Ensure `cargo fmt`, `cargo clippy`, and `cargo test` all pass.
5. **Commit** — Use conventional commit messages (see below).

## Commit Conventions

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <short summary>
```

**Types:**

| Type | When to use |
|------|-------------|
| `feat` | A new feature or capability |
| `fix` | A bug fix |
| `chore` | Maintenance, dependencies, CI config |
| `docs` | Documentation-only changes |
| `refactor` | Code restructuring without behavior change |
| `test` | Adding or updating tests |
| `perf` | Performance improvements |

**Examples:**

```
feat(cli): add --json flag to list command
fix(connector): handle timeout on MCP health check
docs(readme): update installation instructions
chore(deps): bump serde to 1.0.200
```

## Pull Request Process

1. **Open a PR** against `main` with a clear title and description.
2. **Link related issues** using `Closes #123` in the PR body.
3. **Ensure CI passes** — All checks must be green before review.
4. **Request review** — A maintainer will review your PR.
5. **Address feedback** — Push follow-up commits; avoid force-pushing during review.
6. **Squash on merge** — PRs are squash-merged to keep history clean.

### PR Checklist

- [ ] Code compiles without warnings (`cargo clippy`)
- [ ] All tests pass (`cargo test --workspace`)
- [ ] Code is formatted (`cargo fmt --all`)
- [ ] New features include tests
- [ ] Documentation is updated if applicable
- [ ] Commit messages follow conventional format

## Code Review Guidelines

- Be respectful and constructive in all feedback.
- Focus on correctness, readability, and maintainability.
- Suggest improvements rather than demanding changes.
- Approve once all concerns are addressed.

## Reporting Issues

- Use [GitHub Issues](https://github.com/aswin402/AgentVault/issues) to report bugs or request features.
- Include reproduction steps, expected vs. actual behavior, and your environment details.
- Search existing issues before opening a new one.

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you agree to uphold its standards.

## Questions?

Open a [discussion](https://github.com/aswin402/AgentVault/discussions) or reach out to the maintainers. We're happy to help!

---

_Thank you for helping make AgentVault better!_
