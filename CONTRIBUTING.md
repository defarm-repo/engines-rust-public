# Contributing to DeFarm Engines

Thank you for your interest in contributing to DeFarm Engines! This document provides guidelines and instructions for contributing.

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [How to Contribute](#how-to-contribute)
- [Coding Standards](#coding-standards)
- [Testing Guidelines](#testing-guidelines)
- [Pull Request Process](#pull-request-process)
- [Commit Message Convention](#commit-message-convention)
- [Issue Reporting](#issue-reporting)

---

## 📜 Code of Conduct

This project adheres to a Code of Conduct that all contributors are expected to follow. Please read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before contributing.

---

## 🚀 Getting Started

### Prerequisites

Before you begin, ensure you have the following installed:

- **Rust 1.75+** - [Install Rust](https://rustup.rs/)
- **PostgreSQL 14+** - [Install PostgreSQL](https://www.postgresql.org/download/)
- **Git** - [Install Git](https://git-scm.com/downloads)
- **Node.js 16+** (optional, for SDK/CLI development)

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork locally:

```bash
git clone https://github.com/YOUR_USERNAME/engines.git
cd engines
```

3. Add the upstream repository:

```bash
git remote add upstream https://github.com/defarm/engines.git
```

---

## 💻 Development Setup

### 1. Environment Configuration

```bash
# Copy the example environment file
cp .env.example .env

# Edit .env with your local configuration
# Required: DATABASE_URL
nano .env
```

### 2. Database Setup

```bash
# Create the database
createdb defarm_engines

# Migrations run automatically on first startup
```

### 3. Build and Run

```bash
# Build the project
cargo build

# Run the project
cargo run

# Run with auto-reload (recommended for development)
cargo install cargo-watch
cargo watch -x run
```

### 4. Verify Installation

```bash
# In another terminal, test the API
curl http://localhost:3000/health

# Should return: {"status":"healthy",...}
```

---

## 🤝 How to Contribute

### Types of Contributions

We welcome all types of contributions:

- 🐛 **Bug fixes**
- ✨ **New features**
- 📝 **Documentation improvements**
- 🧪 **Tests**
- 🎨 **Code refactoring**
- 🌍 **Translations**
- 💡 **Ideas and suggestions**

### Before You Start

1. **Check existing issues** - Someone might already be working on it
2. **Create an issue** - For new features or significant changes
3. **Discuss** - Get feedback before starting large changes
4. **Keep it focused** - One feature/fix per pull request

---

## 📐 Coding Standards

### Rust Style Guide

Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/):

- Use `snake_case` for function and variable names
- Use `PascalCase` for types and traits
- Use `SCREAMING_SNAKE_CASE` for constants
- Write documentation comments (`///`) for public APIs
- Keep functions focused and small
- Use meaningful variable names

### Code Quality

```bash
# Format your code
cargo fmt

# Run the linter
cargo clippy

# Both should pass without warnings
cargo fmt --check
cargo clippy -- -D warnings
```

### Project-Specific Guidelines

1. **Concurrency**: Follow the concurrency model defined in `docs/adr/001-concurrency-model.md`
   - Storage backends use `Arc<std::sync::Mutex<T>>`
   - Async engine wrappers use `Arc<tokio::sync::RwLock<T>>`

2. **Error Handling**: Use the project's error types in `src/types/errors.rs`
   - Provide descriptive error messages
   - Include recovery suggestions

3. **Security**: Never commit sensitive data
   - Use environment variables
   - Review `docs/security/SECURITY_CHECKLIST.md`

---

## 🧪 Testing Guidelines

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test module
cargo test --test integration

# Run with output
cargo test -- --nocapture

# Run with coverage
cargo tarpaulin --out Html
```

### Writing Tests

1. **Unit Tests**: Place in the same file as the code being tested

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Arrange
        let input = ...;

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected);
    }
}
```

2. **Integration Tests**: Place in `tests/` directory

```rust
use defarm_engine::*;

#[tokio::test]
async fn test_end_to_end_flow() {
    // Test complete workflows
}
```

### Test Requirements

- ✅ All new features must include tests
- ✅ Bug fixes should include regression tests
- ✅ Aim for >80% code coverage
- ✅ Tests should be deterministic and fast

---

## 🔄 Pull Request Process

### 1. Create a Feature Branch

```bash
# Update your local main branch
git checkout main
git pull upstream main

# Create a feature branch
git checkout -b feature/your-feature-name
# or
git checkout -b fix/bug-description
```

### 2. Make Your Changes

- Write clean, documented code
- Follow coding standards
- Add/update tests
- Update documentation if needed

### 3. Commit Your Changes

```bash
# Stage your changes
git add .

# Commit with a descriptive message
git commit -m "feat: add amazing new feature"
```

See [Commit Message Convention](#commit-message-convention) below.

### 4. Push and Create PR

```bash
# Push to your fork
git push origin feature/your-feature-name

# Create a pull request on GitHub
```

### 5. PR Checklist

Before submitting, ensure:

- [ ] Code follows style guidelines (`cargo fmt`, `cargo clippy`)
- [ ] All tests pass (`cargo test`)
- [ ] New code has tests
- [ ] Documentation is updated
- [ ] Commit messages follow conventions
- [ ] No merge conflicts
- [ ] PR description explains the changes

### 6. Review Process

- A maintainer will review your PR
- Address any requested changes
- Once approved, your PR will be merged

---

## 📝 Commit Message Convention

We follow [Conventional Commits](https://www.conventionalcommits.org/):

### Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `chore`: Build process or auxiliary tool changes
- `ci`: CI/CD changes

### Examples

```bash
feat(items): add local item merge functionality

Implemented merge functionality for local items to enable
deduplication before tokenization.

Closes #123
```

```bash
fix(auth): resolve JWT token expiration edge case

Fixed issue where tokens could expire 1 second early due
to timing precision.
```

```bash
docs(api): update API guide with snapshot endpoints

Added complete documentation for snapshot API including
examples and use cases.
```

### Scope

Optional, but helpful for large projects:
- `items`, `circuits`, `events`, `merkle`, etc. (engine names)
- `api`, `cli`, `sdk` (component names)
- `docs`, `tests`, `ci` (tooling)

---

## 🐛 Issue Reporting

### Before Creating an Issue

1. Search existing issues (open and closed)
2. Check the [FAQ](#) and documentation
3. Try the latest version

### Bug Reports

Include:

- **Description**: Clear description of the bug
- **Steps to Reproduce**: Minimal steps to reproduce
- **Expected Behavior**: What should happen
- **Actual Behavior**: What actually happens
- **Environment**: OS, Rust version, database version
- **Logs**: Relevant error messages or logs

### Feature Requests

Include:

- **Problem**: What problem does this solve?
- **Proposed Solution**: How should it work?
- **Alternatives**: Other solutions you've considered
- **Use Case**: Real-world scenario where this is needed

---

## 🏗️ Development Workflow

### Day-to-Day Development

```bash
# 1. Start your work
git checkout main
git pull upstream main
git checkout -b feature/my-feature

# 2. Make changes
# ... edit files ...

# 3. Test frequently
cargo test

# 4. Commit often
git commit -m "feat: add component X"

# 5. Keep up to date
git fetch upstream
git rebase upstream/main

# 6. Push and create PR
git push origin feature/my-feature
```

### Code Review

When reviewing code:

- Be respectful and constructive
- Focus on the code, not the person
- Explain why, not just what
- Suggest alternatives
- Approve when ready

When receiving reviews:

- Don't take it personally
- Ask for clarification if needed
- Respond to all comments
- Thank reviewers for their time

---

## 📚 Additional Resources

### Documentation

- [Complete Developer Guide](docs/api/COMPLETE_DEVELOPER_GUIDE.md)
- [API Reference](docs/api/API_GUIDE.md)
- [System Architecture](CLAUDE.md)
- [Security Guidelines](docs/security/SECURITY_CHECKLIST.md)

### Learning Rust

- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust By Example](https://doc.rust-lang.org/rust-by-example/)
- [Axum Documentation](https://docs.rs/axum/)

### Getting Help

- **Documentation**: Start with [docs/](docs/)
- **Issues**: GitHub Issues for bugs and features
- **Email**: dev@defarm.net for sensitive questions

---

## 🎯 Areas We Need Help

Looking to contribute but not sure where? These areas need help:

- 📝 **Documentation**: Improving guides and examples
- 🧪 **Testing**: Increasing test coverage
- 🌍 **Internationalization**: Translations
- 🎨 **UI/UX**: CLI and tool improvements
- 🔧 **Tooling**: Development workflow improvements
- 📦 **Examples**: More real-world examples

Check issues labeled `good-first-issue` or `help-wanted`.

---

## ✨ Recognition

Contributors are recognized in:

- Release notes
- Contributors page
- Git history

Significant contributions may result in:

- Maintainer status
- Special recognition
- Swag and goodies!

---

## 📞 Questions?

- **Technical Questions**: GitHub Discussions
- **Bugs**: GitHub Issues
- **Security**: security@defarm.net
- **General**: support@defarm.net

---

Thank you for contributing to DeFarm Engines! 🌱

**Happy Coding!**
