# Contributing to Cryptofolio

Thank you for your interest in contributing! Cryptofolio is built using agentic development with Claude Code, making it accessible for contributors of all experience levels.

## Ways to Contribute

- Report bugs and suggest features
- Improve documentation
- Write tests
- Add new features
- Fix bugs

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Git
- Basic understanding of Rust (or willingness to learn with AI assistance)

### Setup

```bash
# Fork and clone
git clone https://github.com/YOUR_USERNAME/cryptofolio.git
cd cryptofolio

# Build and test
cargo build
cargo test

# Run the CLI
cargo run -- --help
```

## Development Workflow

### Traditional Approach

```bash
# Create a branch
git checkout -b feature/my-feature

# Make changes
# ... edit code ...

# Run tests
cargo test

# Commit
git commit -m "feat: add new feature"

# Push and create PR
git push origin feature/my-feature
```

### AI-Assisted Development (Recommended)

Cryptofolio welcomes AI-assisted contributions. We encourage using Claude Code for faster, higher-quality development.

#### Using Claude Code

```bash
# Start AI pair programming
claude

you> "I want to add support for JPY currency"
Claude> *Analyzes codebase*
Claude> *Creates currency definition*
Claude> *Adds database migration*
Claude> *Writes tests*
Claude> *Updates documentation*
```

#### Benefits of AI Development

- Faster implementation
- Comprehensive test coverage
- Automatic documentation
- Rust type safety guidance
- Best practices enforcement

## Contribution Guidelines

### Code Style

- Follow Rust conventions
- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Add tests for new functionality

### Commit Messages

Follow conventional commits:

```
feat: add JPY currency support
fix: correct exchange rate calculation
docs: update multi-currency guide
test: add integration tests for swaps
chore: update dependencies
```

### Pull Request Process

1. Update documentation
2. Add tests
3. Ensure all tests pass
4. Update CHANGELOG.md
5. Create PR with clear description

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_currency_add

# Run with output
cargo test -- --nocapture
```

## Project Structure

```
cryptofolio/
├── src/
│   ├── cli/          # CLI commands
│   ├── core/         # Domain models
│   ├── db/           # Database layer
│   ├── exchange/     # Exchange integrations
│   └── main.rs       # Entry point
├── tests/            # Integration tests
├── docs/             # Documentation
└── Cargo.toml        # Dependencies
```

## Good First Issues

Look for issues labeled `good-first-issue`:

- Add new currency support
- Improve error messages
- Add CLI examples
- Write integration tests
- Update documentation

## Questions?

- Check existing issues and discussions
- Ask in GitHub Discussions
- Read the documentation in `docs/`

## Code of Conduct

Please read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before contributing.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

---

Thank you for contributing to Cryptofolio! 🚀
