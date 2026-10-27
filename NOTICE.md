# Attribution

The theme assets in `assets/pierre/` come from The Pierre Computer Company.
Source: https://github.com/pierrecomputer/pierre
Commit: `0a7a733078f093577cd21ca65d2508806a22238a`.
The assets retain their Apache-2.0 license and upstream notice.

The viewer uses the theme's TextMate token rules through Syntect.
The split-alignment policy is informed by Pierre's bounded offset comparison, with a separately implemented prefix/suffix score.

## Forked dependencies

`crates/` contains forks of three MIT-licensed or dual-licensed crates, built and published as separate packages for diffs-rs.
Each keeps its upstream license file and copyright notice unchanged. Changes in each fork are distributed under that fork's upstream license.

- `crates/diffs-syntect/` is Syntect 5.3.0 with local regex ownership, search, and accounting patches.
  Its MIT license is retained in `crates/diffs-syntect/LICENSE.txt`.
  The [patch note](crates/diffs-syntect/PATCHES.md) records the exact source revision, changes, and restored upstream test inputs.
  Those test inputs (`crates/diffs-syntect/testdata/`) retain their included licenses, are not replacement runtime grammars, and are not part of the published package.
- `crates/diffs-fancy-regex/` is Fancy Regex 0.16.2 with search-scratch, engine-sharing, and required-byte patches.
  Its MIT license is retained in `crates/diffs-fancy-regex/LICENSE`. The [patch note](crates/diffs-fancy-regex/PATCHES.md) records the changes.
- `crates/diffs-two-face/` is two-face 0.5.2+bat-0.26.1, built against `diffs-syntect`. Apart from its manifest, only the acknowledgement URL changes, to name the upstream release.
  Its MIT and Apache-2.0 licenses are retained in `crates/diffs-two-face/LICENSE-MIT` and `crates/diffs-two-face/LICENSE-APACHE`.
  The bundled syntax and theme definitions carry their own licenses, listed by its `acknowledgement` module.
