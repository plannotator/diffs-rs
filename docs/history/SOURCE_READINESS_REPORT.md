# Source-readiness position correction

Date: 2026-09-22.

One end-of-diff command now retains the selected source on screen while checked full-file data arrives.
The implementation follows `evidence/source-readiness-v1/PLAN.md`.

## Defect and correction

Checked sources insert context-gap rows into the view.
The previous code restored the cursor and top source identities independently.
It restored their screen relationship only when the line-number gutter changed width.
New gap rows could therefore move the selected endpoint below the viewport without changing either source identity.
The footer could also remain at `coloring visible code…`: it included the offscreen cursor, but the worker received only visible rows.

`App::set_source` now uses the existing reading-anchor reflow whenever that anchor survives the source update.
This retains source identity, active side, text byte, and screen position through the inserted rows.
The existing fallback remains in place when the selected context is removed by a source failure.
No second navigation command, eager source preparation, suppressed status, or special endpoint case was added.

The file-mode CLI help now describes checked full-source coloring instead of the obsolete patch-context behavior.
These are the only production-source changes from the preceding checkpoint.

## Correctness evidence

Both new regressions first failed before the correction:

- The real-source integration check lost the selected visible row after a neighboring file became ready.
- The real-PTY check lost selected line 763 after one `G`. The last visible line was 762, and coloring remained pending.

The corrected integration check covers both layouts, wrapping on/off, two source-arrival orders, and per-reply or batched redraws.
It checks the exact old-side source, text byte, wrapped part, and visible row, including Unicode and tabs.
The PTY check uses the actual Git command and lazy source worker in split and unified layouts.
It checks the selected new-side line before and after coloring settles, then terminal restoration and private-file cleanup.

Final results:

| Check | Result |
| --- | --- |
| Ordinary suite | 111 passed, including 61 real-process checks; four opt-in checks excluded |
| Owner-census suite, separate target directory | 125 passed; four opt-in checks excluded |
| Final ordinary executable endpoint contract | Passed in both layouts |
| Preserved ordinary-build comparison | 96 complete terminal grids and two canonical source streams matched |
| Local-source rehearsal | 146 source-and-color views passed over three tours per case |
| Corrected timing reference | 22 views passed; 16 explicit selected-source destinations checked |
| Previous invalid reference controls | All three rejected |
| Formatting and ordinary Clippy | Passed |

The local-source cases contain one 12,000-line file pair and a 12-file Git comparison with 800 lines per file.
They retain the preceding fixture contents and independent sequential-color answers.
The tours cover repeated context expansion, file visits, endpoints, both layouts, wrapping, width changes, and themes.
The full suite retains source reconstruction, mismatch handling, source failure, cancellation, and cleanup coverage.
These checks do not prove all possible inputs or grammar behavior.

## Qualification-checker correction

The old short timing script used `g`, Up, Enter for leading expansion.
That sequence selected the section header, not the gap. It did not expand source.
The corrected sequence uses `g`, Up, Up, Enter and requires selected source line 77 after revealing 20 lines.
The last supplied sources must be selected at line 11963 or 763, depending on the fixture.
The checker verifies those destinations independently before saving reference screens.

The new checker rejects both old non-expansion views and the old offscreen multi-file endpoint.
The old evidence remains unchanged. None of its timing values is accepted.
The new reference is preparation for measurement, not a measurement result.

## Code review

The correction reuses the existing typed source anchor and bounded viewport reflow.
It adds no production type, dependency, source copy, cache allowance, worker, or unsafe code.
Source ownership and correspondence checks remain unchanged.
Review covered reply batches, unrelated-file updates, removed context, line-number width changes, and anchor lifetime.
The two regression checks share only their real Git-fixture construction.
The existing vendored Syntect lifetime warning remains unchanged.

## Artifacts and limits

The ordinary executable is preserved at `evidence/source-readiness-v1/native-diff`.
Its SHA-256 is `c3b337c3549efab30d7fee3f5089d4f9af733a143e28c091cbbd45a21b462074`.
The comparison reference is `baseline-native-diff`, SHA-256 `8fee6ce72866c3f23dfbe7ad3d926d35712f12fde8ae43df00eeebaa5aa4625e`.
The directory also contains the source archive, failure logs, passing logs, terminal-cell captures, qualification inputs, and hash manifest.
`qualification/verified.json` records the independent checks. The qualification README records repeatable commands.

The machine remained on battery. No controlled timing capture ran.
The rehearsal retained diagnostic RSS samples, but they do not establish sustained-memory behavior or a performance improvement.
The five-patch campaign, physical terminal presentation, and broader production qualification remain open.
No GUI automation, network input, global installation, or public release was performed.
