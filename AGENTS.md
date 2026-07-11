# Agents

## Rust checks

Run these before committing:

```sh
cargo fmt --check
cargo clippy -- -D warnings
cargo build
RUSTUP_TOOLCHAIN=stable cargo test -p auto-water-core --target aarch64-apple-darwin
```
