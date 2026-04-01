# Contributing to Extro

Thanks for your interest in contributing to Extro! Here's how to get started.

## Development Setup

```bash
# Clone the repo
git clone https://github.com/askpext/extro.git
cd extro

# Prerequisites
rustup install stable
rustup target add wasm32-unknown-unknown
cargo install wasm-pack

# Build the CLI
cargo build -p extro-cli

# Run all tests
cargo test --workspace

# Run clippy
cargo clippy --workspace --all-targets -- -D warnings

# Check formatting
cargo fmt --all -- --check
```

## Project Structure

| Directory | Language | Purpose |
|-----------|----------|---------|
| `crates/core/` | Rust | Domain logic, state machine, effects, tool registry |
| `crates/wasm/` | Rust | wasm-bindgen bridge |
| `crates/agent/` | Rust | Agent tracing layer |
| `cli/` | Rust | CLI (scaffold, build, watch, test, package) |
| `extension/` | JS | Browser extension shell (MV3) |
| `npm/runtime/` | JS/TS | Published npm package `@extro/runtime` |

## Making Changes

### Rust Changes

1. Make your changes in the relevant crate
2. Run `cargo test --workspace` to verify
3. Run `cargo clippy --workspace --all-targets -- -D warnings`
4. Run `cargo fmt --all`

### JavaScript Changes

1. Make changes in `extension/` or `npm/runtime/`
2. Run `extro build` to verify the full pipeline works
3. If modifying `npm/runtime/`, update `src/index.d.ts` type definitions

### Adding a New Feature

1. Start with the Rust core (`crates/core/src/lib.rs`)
2. Add types, dispatch logic, and unit tests
3. If it needs a WASM export, update `crates/wasm/src/lib.rs`
4. If it needs browser interaction, update `extension/src/background/index.js`
5. Update `AGENTS.md` so AI agents know about the new feature

## Pull Request Process

1. Fork the repo and create a feature branch
2. Make your changes with tests
3. Ensure CI passes: `cargo test --workspace && cargo clippy --workspace && cargo fmt --check`
4. Open a PR with a clear description of what changed and why
5. Link any related issues

## Code Style

- **Rust**: Follow standard Rust conventions. Use `cargo fmt`.
- **JavaScript**: Use ES modules, minimal dependencies, no bundler in source.
- **Documentation**: All public Rust items must have `///` doc comments.
- **Tests**: All new Rust features must have unit tests.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
