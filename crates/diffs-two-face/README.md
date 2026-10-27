# diffs-two-face

A fork of [two-face](https://codeberg.org/CosmicHarper/two-face) maintained only for
[diffs-rs](https://github.com/plannotator/diffs-rs).
**Use upstream `two-face` in your own projects.** This crate follows the needs of
diffs-rs and may change or stop being updated without notice.

- Upstream base: `two-face` 0.5.2+bat-0.26.1 (syntaxes and themes from
  [bat](https://github.com/sharkdp/bat) 0.26.1), revision
  [`67b64e6`](https://codeberg.org/CosmicHarper/two-face/commit/67b64e632ceefe70e5f20f3eb797d9e33560a11c).
  The published upstream archive has SHA-256
  `915be7adc2ff6f4338acbf71f3eda0a146f46003b992a1669c674308610efdac`.
- Fork version: `diffs-two-face` 0.1.0. Fork versions are independent of upstream versions.
- Library name: `two_face`, as upstream. Depend on it under the upstream name:

  ```toml
  [dependencies]
  two-face = { package = "diffs-two-face", version = "0.1.0", default-features = false, features = ["syntect-fancy"] }
  ```

## Changes from upstream

The embedded syntax, theme, and acknowledgement data are unchanged, and so is the
source code apart from one line. The fork exists so that its `SyntaxSet` and `ThemeSet`
types are the ones from [`diffs-syntect`](../diffs-syntect), the syntect fork diffs-rs uses.

- `acknowledgement::url()` names the upstream release tag (`v0.5.2+bat-0.26.1`)
  directly. Upstream builds it from the package version, which in the fork would point
  to a tag that does not exist.
- The `syntect` dependency is `diffs-syntect`, renamed to `syntect`, so `syntect::`
  paths and the `syntect/...` features resolve to the fork.
- The manifest drops upstream's workspace, and keeps only the `strum` dev-dependency that
  the unit tests use. Upstream's snapshot, fuzz and xtask tooling is not part of the fork.
- The package is renamed and versioned independently, with this README. The upstream
  README is in [UPSTREAM_README.md](UPSTREAM_README.md).
- Upstream lints are allowed in the manifest, because diffs-rs runs Clippy with
  `-D warnings` across its workspace.

## License

MIT OR Apache-2.0, as upstream. See [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE). The embedded syntax and theme definitions carry their
own licenses, listed by the `acknowledgement` module.
