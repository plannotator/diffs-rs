# Attribution

The theme assets in `assets/pierre/` come from The Pierre Computer Company.
Source: https://github.com/pierrecomputer/pierre
Commit: `0a7a733078f093577cd21ca65d2508806a22238a`.
The assets retain their Apache-2.0 license and upstream notice.

The viewer uses the theme's TextMate token rules through Syntect.
The split-alignment policy is informed by Pierre's bounded offset comparison, with a separately implemented prefix/suffix score.

`vendor/syntect/` contains Syntect 5.3.0 with a local compiled-pattern ownership patch.
Its MIT license is retained in `vendor/syntect/LICENSE.txt`.
The [patch note](vendor/syntect/NATIVE_DIFF_PATCH.md) records the exact source revision, changes, and restored upstream test inputs.
Those test inputs retain their included licenses and are not replacement runtime grammars.
