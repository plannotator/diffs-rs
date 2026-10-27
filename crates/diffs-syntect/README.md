# diffs-syntect

A fork of [syntect](https://github.com/trishume/syntect) maintained only for
[diffs-rs](https://github.com/plannotator/diffs-rs).
**Use upstream `syntect` in your own projects.** This crate follows the needs of
diffs-rs and may change or stop being updated without notice.

- Upstream base: `syntect` 5.3.0, revision
  [`e467084`](https://github.com/trishume/syntect/commit/e4670846ecf16d8832db6c43d531bec466214e27).
  The published upstream archive has SHA-256
  `656b45c05d95a5704399aeef6bd0ddec7b2b3531b7c9e900abbf7c4d2190c925`.
- Fork version: `diffs-syntect` 0.1.0. Fork versions are independent of upstream versions.
- Library name: `syntect`, as upstream. Depend on it under the upstream name:

  ```toml
  [dependencies]
  syntect = { package = "diffs-syntect", version = "0.1.0", default-features = false, features = ["default-fancy"] }
  ```

- Fancy Regex backend: [`diffs-fancy-regex`](../diffs-fancy-regex), the matching fork.
  Its types are not interchangeable with upstream `fancy-regex` or upstream `syntect`.

## Changes from upstream

Grammar data, rule order, compilation options, and match results are unchanged.

1. **Shared compiled patterns.** Each regex owns its lazily compiled cell through an
   `Arc`. A syntax set interns equal static patterns, so its grammars share compiled
   regexes. Cloning a regex shares its compiled cell instead of recompiling.
2. **Line-local search scratch.** The parser reuses Fancy Regex search buffers within one
   `parse_line` call.
3. **Required-byte rejection.** The parser skips a capture search when the line lacks a
   byte that every match requires.
4. **Scoped engine sharing** (feature `shared-engines`). Static rules in one syntax-set
   family share Fancy Regex inner engines.
5. **Memory accounting.** `ParseState::owned_heap_bytes()` and
   `ScopeStack::owned_heap_bytes()` report owned parser-state capacity.
6. **Diagnostics** (features `regex-census`, `owner-census`, `syntax-timing`). Borrowed
   inspection of compiled regexes and initialization timing. Off by default.

[PATCHES.md](PATCHES.md) describes each change, the changed files, packaging changes,
and the checks that updates need. The upstream README is in
[UPSTREAM_README.md](UPSTREAM_README.md).

## License

MIT, as upstream. See [LICENSE.txt](LICENSE.txt). The upstream copyright notice is
unchanged, and the fork's changes are distributed under the same license.
