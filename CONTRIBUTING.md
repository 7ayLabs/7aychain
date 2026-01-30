# Contributing to 7aychain

Thank you for your interest in contributing to 7aychain. This document provides guidelines for contributing to the project.

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment.

## Development Workflow

### Branch Strategy

```
main (production)
  │
  └── develop (integration)
        │
        ├── feature/*    # New features
        ├── fix/*        # Bug fixes
        └── hotfix/*     # Critical fixes
```

### Branch Rules

| Branch | Source | Target | Purpose |
|--------|--------|--------|---------|
| `main` | - | - | Production releases |
| `develop` | `main` | `main` | Integration |
| `feature/*` | `develop` | `develop` | New features |
| `fix/*` | `develop` | `develop` | Bug fixes |
| `hotfix/*` | `main` | `main` + `develop` | Critical fixes |

## Commit Conventions

We use [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Types

| Type | Description |
|------|-------------|
| `feat` | New feature |
| `fix` | Bug fix |
| `refactor` | Code restructuring |
| `perf` | Performance improvement |
| `test` | Adding/updating tests |
| `docs` | Documentation |
| `build` | Build system changes |
| `ci` | CI/CD changes |
| `chore` | Maintenance |
| `security` | Security fixes |

### Scopes

| Scope | Description |
|-------|-------------|
| `presence` | Presence pallet |
| `epoch` | Epoch pallet |
| `validator` | Validator pallet |
| `dispute` | Dispute pallet |
| `primitives` | Core types |
| `runtime` | Runtime configuration |
| `node` | Node implementation |
| `rpc` | RPC extensions |

### Examples

```bash
feat(presence): implement state machine transitions

fix(validator): enforce INV46 minimum validator count

security(crypto): harden key derivation function

refactor(epoch): optimize lifecycle state tracking

test(presence): add invariant violation tests
```

## Code Standards

### Rust Guidelines

1. **No unsafe code** without explicit security review and documentation
2. **Saturating arithmetic** for all numeric operations
3. **Checked operations** with explicit error handling
4. **No unwrap/expect** in production code (use proper error handling)
5. **Document all public APIs** with rustdoc comments

### Security Requirements

- All invariants (INV1-78) must have corresponding tests
- Cryptographic operations must use constant-time comparisons
- No hardcoded secrets or credentials
- Input validation at all entry points

### Formatting & Linting

```bash
# Format code
cargo fmt --all

# Run clippy with strict lints
cargo clippy --all -- -D warnings

# Run tests
cargo test --all
```

## Pull Request Process

1. **Create feature branch** from `develop`
2. **Implement changes** following code standards
3. **Write tests** for new functionality
4. **Update documentation** as needed
5. **Run all checks** locally before pushing
6. **Create PR** with clear description
7. **Address review feedback** promptly

### PR Checklist

- [ ] Code follows project style guidelines
- [ ] Tests pass locally (`cargo test --all`)
- [ ] Clippy passes (`cargo clippy --all -- -D warnings`)
- [ ] Documentation updated if needed
- [ ] Commit messages follow conventions
- [ ] No sensitive data in commits

## Testing Requirements

### Test Categories

| Category | Coverage | Location |
|----------|----------|----------|
| Unit Tests | Required | `src/tests.rs` |
| Integration Tests | Required | `tests/` |
| Invariant Tests | Required | Per invariant |
| Benchmarks | Recommended | `benches/` |

### Running Tests

```bash
# All tests
cargo test --all

# Specific pallet
cargo test -p pallet-presence

# With coverage (requires cargo-tarpaulin)
cargo tarpaulin --all
```

## Documentation

- Use rustdoc for API documentation
- Include examples in doc comments
- Update README for significant changes
- Add inline comments for complex logic

## Questions?

- Open an issue for questions
- Join discussions in existing issues
- Review existing PRs for context

---

Thank you for contributing to 7aychain!
