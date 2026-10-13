# Measurement policy for future development runs

Effective 2026-09-12. Policy ID: `uniform-development-v1`.

Fixture identity must not select a performance target. The same observations must receive the same result after a fixture or repository is renamed.

## Shared development targets

Future runs use the stricter values already present in the previous policy, uniformly:

| Observation | Target |
| --- | ---: |
| First correct text | 500 ms |
| First defined styling state | 1,000 ms |
| Complete input index | 2,000 ms |
| Navigation, layout, resize | p95 <= 50 ms and maximum <= 200 ms |
| Quit/cancellation | 250 ms |
| Settled / peak process RSS | 128 / 192 MiB |

These are provisional engineering goals, not supported-input limits or a guarantee for unlimited input sizes. Failure must not cause the viewer to refuse input, omit content, disable colors, or change its algorithm for a known fixture. Record absolute memory and latency alongside input bytes, source records, file/hunk counts, and line-size distribution. Evaluate speed, CPU cost, and memory together.

This removes the previous name-based exceptions, not by introducing a corpus-sized crossover or a new fitted formula. If a scaling policy is needed, derive it from general workload dimensions and state its rationale before evaluating the next candidate. Never adjust it to turn an observed failure into a pass.

The existing cold-styling and retained-revisit goals remain 2,000 ms and 100 ms. The current five-category trace does not establish those goals. They require explicit cache-state trials. Watchdogs and the process-group safety stop remain separate from all performance targets.

## Styling and correctness

A recorded styling state can include a declared plain-styling fallback. Such a state is not evidence that syntax highlighting was performed. Report the state and its coverage. It must not be pooled into an unqualified claim about syntax throughput or complete visual fidelity.

Wrong source, side, numbers, pairing relative to the adopted contract, or settled cells is a correctness failure. Retain the failure. A timeout is not a successful fallback. Compare first text, styling, and index-ready milestones separately. Keep maxima and incomplete trials visible.

## Evidence and compatibility

New freezes include this document's hash and the complete `targets` object from `tools/corpus_measure.py`. Each new summary records the same object. The current runner rejects older freezes rather than interpreting them under new rules.

Historical measurements and their policies remain unchanged. Their complete source/runner is preserved in `evidence/memory-v1/final/source.tar.gz`; earlier packets remain available too. The 2026-09-11 results still mean what they meant under that archived policy. This policy does not retroactively certify or recalculate them.

The aggregate report summarizes supplied evidence. It does not assert source-digest, test, formatting, or Clippy passes without reading their results. Those checks are explicitly unverified by that generator. Completion language must agree with the supplied completion status, including failures before the first usable viewport.

No measurements are authorized or launched merely by writing this policy.
