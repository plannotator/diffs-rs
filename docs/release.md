# Releasing

The workspace publishes four crates to crates.io:

| Package | Source | Notes |
| --- | --- | --- |
| `diffs-fancy-regex` | `crates/diffs-fancy-regex/` | Fork of fancy-regex 0.16.2 |
| `diffs-syntect` | `crates/diffs-syntect/` | Fork of syntect 5.3.0, depends on `diffs-fancy-regex` |
| `diffs-two-face` | `crates/diffs-two-face/` | Fork of two-face 0.5.2, depends on `diffs-syntect` |
| `diffs-rs` | repository root | Depends on all three forks |

Fork versions are independent of upstream versions. crates.io ignores build metadata when it
compares versions, so a fork cannot carry the upstream version plus a suffix. Each fork records
its upstream base in its description, README, and `[package.metadata.diffs-rs.upstream]`.

`diffs-rs` depends on each fork with a `package =` rename and both `path` and `version`.
Local builds use the workspace sources; the published package uses the registry versions.
A `diffs-rs` release that does not change a fork does not republish it.

## Steps

1. Update the version in `Cargo.toml` (and the forks' manifests if they changed), then run
   `cargo update -p diffs-rs` so `Cargo.lock` matches.
2. Add an entry to `CHANGELOG.md`.
3. Run the checks listed in [CONTRIBUTING.md](../CONTRIBUTING.md) and
   `cargo package --workspace --locked`.
4. Merge to `main`, then publish in dependency order, skipping crates whose version is already
   on crates.io:

   ```sh
   cargo publish --locked -p diffs-fancy-regex
   cargo publish --locked -p diffs-syntect
   cargo publish --locked -p diffs-two-face
   cargo publish --locked -p diffs-rs
   ```

5. Tag the published commit with an annotated tag, for example
   `git tag -a v0.1.1 -m "diffs-rs 0.1.1"`, and push it.

The record of the 0.1.0 release preparation and earlier local qualification is in
[history/RELEASE_READINESS.md](history/RELEASE_READINESS.md).
