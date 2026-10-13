# Local-source lifecycle hardening

Date: 2026-09-22.

This pass reviews the local full-source and context-expansion implementation.
It does not start controlled measurements or add an input mode.
The preserved source-readiness measurement executable remains unchanged.

## Review scope

1. Trace ownership from input preparation through checked snapshots, view ranges, geometry, styles, and worker replies.
2. Check queue bounds, shared cache accounting, cancellation, rollback, source failure, and final-owner cleanup.
3. Compare each suspected failure with the existing real-file, real-worker, and terminal-process checks.
4. Add a focused regression only for a concrete defect or important uncovered lifecycle boundary.
5. Update vendored patch notes from current code and the existing provenance records.

Source bytes, source identities, side-specific colors, and line destinations remain the correctness requirements.
The audit distinguishes intentional retained review state from cached payload and abandoned work.
Static risks remain separate from measured performance findings.

## Completion

The report records the ownership map, findings, evidence, and remaining limits.
Any production correction requires a failing regression, a passing correction, and proportionate nearby checks.
If no runtime correction is justified, the report states that result explicitly.
Tests use real files, Git processes, workers, or terminal input/output. No mock framework is added.
The final checks include preservation of the prepared executable and its prior evidence.

Remote input, physical terminal capture, controlled timing, and the five-patch memory campaign remain separate work.
