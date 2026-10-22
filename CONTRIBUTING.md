# Contributing to claude-code-sync

Thank you for your interest in contributing to claude-code-sync! This document provides guidelines and instructions for contributing.

## Code of Conduct

- Be respectful and inclusive
- Welcome newcomers and help them get started
- Focus on constructive feedback
- Prioritize the project's goals and user needs

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Git
- A GitHub account

### Setting Up Your Development Environment

1. Fork the repository on GitHub
2. Clone your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/claude-code-sync.git
   cd claude-code-sync
   ```

3. Add the upstream repository:
   ```bash
   git remote add upstream https://github.com/ORIGINAL_OWNER/claude-code-sync.git
   ```

4. Install dependencies and build:
   ```bash
   cargo build
   ```

5. Run tests to verify setup:
   ```bash
   cargo test
   ```

## Development Workflow

**Work goes straight to `main`.**

### 1. Make Your Changes

- Write clear, concise code
- Follow Rust best practices and idioms
- Add tests for new functionality
- Update documentation as needed
- Keep commits focused and atomic

### 2. Run Quality Checks

Before committing, ensure your code passes all checks:

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Run tests
cargo test

# Check compilation
cargo check
```

### 3. Commit Your Changes

Write clear commit messages following this format:

```
<type>: <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Formatting, missing semicolons, etc.
- `refactor`: Code restructuring
- `test`: Adding tests
- `chore`: Maintenance tasks

**Example:**
```
feat: add support for encrypted repositories

- Implement GPG encryption for sensitive conversations
- Add --encrypt flag to init command
- Update documentation with encryption examples

Closes #123
```

### 4. Push

```bash
git push origin main
```

## Coding Standards

### Rust Style Guide

- Follow the [Rust Style Guide](https://doc.rust-lang.org/1.0.0/style/)
- Use `cargo fmt` for formatting
- Address all `cargo clippy` warnings
- Prefer explicit over implicit
- Write self-documenting code with clear names

### Code Organization

```rust
// 1. External crate imports
use anyhow::Result;
use serde::Deserialize;

// 2. Standard library imports
use std::fs;
use std::path::Path;

// 3. Internal module imports
use crate::parser::ConversationSession;

// 4. Type definitions
struct MyStruct { ... }

// 5. Implementation
impl MyStruct { ... }

// 6. Tests
#[cfg(test)]
mod tests { ... }
```

### Documentation

- Add doc comments for public items
- Include examples in doc comments
- Update README.md for user-facing changes
- Add entries to EXAMPLES.md for new features

**Example:**
```rust
/// Parse a JSONL conversation file
///
/// # Arguments
/// * `path` - Path to the JSONL file
///
/// # Returns
/// A `ConversationSession` containing all parsed entries
///
/// # Example
/// ```
/// let session = ConversationSession::from_file("session.jsonl")?;
/// println!("Session has {} messages", session.message_count());
/// ```
pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
    // implementation
}
```

### Testing

> **Run the test suite single-threaded:**
> ```bash
> cargo test -- --test-threads=1
> ```
> Tests share a process-global configuration directory (resolved by
> `ConfigManager`, overridable via the `CLAUDE_CODE_SYNC_CONFIG_DIR` environment
> variable). Running them in parallel would race on that shared location. The CI
> workflows already pass `--test-threads=1`. On macOS in particular, a parallel
> run could otherwise fall back to and clobber your real
> `~/Library/Application Support/claude-code-sync` directory.

All new code should include tests:

1. **Unit Tests**: Test individual functions
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_parser_handles_empty_file() {
           // test implementation
       }
   }
   ```

2. **Integration Tests**: Test module interactions
   ```rust
   // tests/integration_tests.rs
   #[test]
   fn test_end_to_end_sync() {
       // test implementation
   }
   ```

3. **Edge Cases**: Test error conditions, empty inputs, large datasets

### Error Handling

- Use `anyhow::Result` for fallible functions
- Provide context with `.context()`:
  ```rust
  fs::read_to_string(&path)
      .with_context(|| format!("Failed to read file: {}", path.display()))?
  ```
- Return descriptive errors to users
- Log errors for debugging

## Releasing

**A pushed `v*` tag is a release.** CI checks the tag matches `Cargo.toml`, creates the GitHub release with the commits since the last tag as notes, and attaches every platform's binary.

```bash
# bump `version` in Cargo.toml, then:
cargo check
git commit -am "chore: release 0.4.15"
git tag v0.4.15
git push origin main v0.4.15
```

Follow [Semantic Versioning](https://semver.org/).

## Areas for Contribution

### High Priority

- [ ] Full end-to-end integration tests
- [ ] Performance optimizations for large datasets
- [ ] Windows-specific testing and fixes
- [ ] Export conversations to Markdown/HTML
- [ ] Compression support for large files

### Feature Ideas

- [ ] Selective sync by date/project
- [ ] Web UI for browsing history
- [ ] Encryption support
- [ ] Smart merge for non-conflicting changes
- [ ] Plugin system for Claude Code
- [ ] Incremental sync (only changed files)
- [ ] Search functionality across all sessions

### Documentation Improvements

- [ ] Video tutorials
- [ ] More usage examples
- [ ] Troubleshooting guide expansion
- [ ] Platform-specific installation guides
- [ ] Architecture diagrams

## Issue Reporting

### Bug Reports

Include:
- Clear, descriptive title
- Steps to reproduce
- Expected behavior
- Actual behavior
- System information (OS, Rust version)
- Relevant logs or error messages

**Template:**
```markdown
**Description**
A clear description of the bug

**To Reproduce**
1. Run `claude-code-sync init ...`
2. Execute `claude-code-sync push`
3. See error

**Expected behavior**
What should happen

**Actual behavior**
What actually happens

**Environment**
- OS: Ubuntu 22.04
- Rust: 1.75.0
- claude-code-sync: 0.1.0

**Logs**
```
Error message here
```
```

### Feature Requests

Include:
- Clear use case
- Proposed solution
- Alternative solutions considered
- Implementation ideas (optional)

## Questions?

- Open a discussion on GitHub
- Check existing issues
- Read the documentation (README.md, EXAMPLES.md)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

## Recognition

Contributors will be:
- Listed in the README.md (optional)
- Credited in release notes
- Appreciated in the community!

---

Thank you for contributing to claude-code-sync! 🎉
