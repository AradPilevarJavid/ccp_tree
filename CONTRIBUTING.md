# Contributing to ccp

Thanks for helping improve `ccp`. Please keep changes focused, explain the
user-facing impact, and update documentation when behavior changes.

## Development checks

The project requires a current stable Rust toolchain. From the repository root,
run:

```bash
cargo test --all-targets --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

Run `cargo fmt --all` to format code before committing. If you change a
dependency or build-related file, include the resulting `Cargo.lock` changes.

## Submitting changes

1. Create a focused branch from `master`.
2. Make the smallest change that addresses the issue.
3. Add or update tests and documentation as appropriate.
4. Run the checks above.
5. Open a pull request describing the problem, the solution, and validation
   results.

Please do not include credentials, generated build artifacts, or unrelated
formatting changes.
