# Contributing

Bug reports and pull requests are welcome at
[plannotator/diffs-rs](https://github.com/plannotator/diffs-rs).

## Build and test

The tests need Rust 1.88 or newer, Git 2.50 or newer, Python 3.11 or newer, `shasum`, and a
Unix PTY. Many of them run the real `diffs` binary against files, pipes, keys, and signals.

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --release --locked -- --test-threads=1
```

CI runs the same commands on Linux and macOS, plus a `cargo check` with Rust 1.88.

The embedded-host checks need the example binary:

```sh
cargo build --release --locked --example embedded_viewer
NATIVE_DIFF_HOST_BINARY="$PWD/target/release/examples/embedded_viewer" \
  cargo test --release --locked --test process engine_host_contract -- --ignored --test-threads=1
```

Build instrumented features (`--all-features`) in a separate target directory so they do not
replace the normal binary.

## Repository layout

- `src/`: the library (`diffs_rs`) and the `diffs` binary.
- `crates/`: forks of syntect, fancy-regex, and two-face. Record any change to a fork in its
  `PATCHES.md`.
- `tests/`, `fixtures/`: unit, process, and rendering tests and their inputs.
- `tools/`: Python scripts for the large-patch measurements in
  [docs/measurements](docs/measurements/README.md). They are not needed for normal development.
- `docs/`: user guides, design notes, and the measurement and development records.

## Commits

Use lowercase conventional commit subjects, for example `fix: keep the cursor on resize`.
