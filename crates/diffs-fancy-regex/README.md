# diffs-fancy-regex

A fork of [fancy-regex](https://github.com/fancy-regex/fancy-regex) maintained only
for [diffs-rs](https://github.com/plannotator/diffs-rs).
**Use upstream `fancy-regex` in your own projects.** This crate follows the needs of
diffs-rs and may change or stop being updated without notice.

- Upstream base: `fancy-regex` 0.16.2, revision
  [`68d300b`](https://github.com/fancy-regex/fancy-regex/commit/68d300b2853cfb390a99020719114c22254878e5).
  The published upstream archive has SHA-256
  `998b056554fbe42e03ae0e152895cd1a7e1002aec800fdc6635d20270260c46f`.
- Fork version: `diffs-fancy-regex` 0.1.0. Fork versions are independent of upstream versions.
- Library name: `fancy_regex`, as upstream. Depend on it under the upstream name:

  ```toml
  [dependencies]
  fancy-regex = { package = "diffs-fancy-regex", version = "0.1.0" }
  ```

## Changes from upstream

All additions are opt-in or additive. Default features, matching behavior, compiled
programs, and errors are unchanged.

1. **Caller-owned search scratch.** `SearchScratch` and
   `Regex::captures_from_pos_with_scratch` let a caller reuse the VM's temporary
   branch, save-history, and delegated-slot buffers across searches.
2. **Scoped engine sharing** (feature `shared-engines`). `EngineCache` and
   `RegexBuilder::build_with_cache` let equal expressions with equal settings share one
   compiled inner engine within a caller-chosen scope.
3. **Required-byte metadata.** `Regex::required_ascii_bytes()` returns a conservative
   mask of ASCII bytes that every match needs somewhere in the input. Callers can skip
   searches that cannot match.
4. **Owner inspection** (feature `owner-census`). A borrowed visitor over existing inner
   engines, for memory diagnostics.

[PATCHES.md](PATCHES.md) describes each change, the changed files, packaging changes,
and the checks that updates need. The upstream README is in
[UPSTREAM_README.md](UPSTREAM_README.md).

## License

MIT, as upstream. See [LICENSE](LICENSE) and [AUTHORS](AUTHORS). The upstream
copyright notice is unchanged, and the fork's changes are distributed under the same license.
