# Content-grid measurement v1

This protocol measures the preserved original executable. It does not compare projects or measure the changed viewer.
The original executable hash is `52646a55a3635d2df506ab6e3e458b4fca2386f861ead2c0e62c57739308ed78`.

The trace covers all five pinned DiffsHub patches. Linux remains mandatory.
The source decoder derives the expected content. It does not call the viewer or libgit2.
The display reference implements the declared pairing, gutter, clipping, and navigation rules independently.

## Measurement boundary

Each fixture gets one filesystem warmup and five fresh viewer processes, in corpus order.
Trials run serially at 160×50 cells, split layout, dark theme, and truecolor.
The runner starts no GUI terminal and makes no network requests.
Other host activity remains possible. This is a development baseline on the local Mac, not an isolated-machine certification.

Opening starts immediately before `fork`. It includes process creation, execution, input loading, and terminal output.
The original executable publishes content only after the complete index exists.
Its first correct content frame therefore also gives an upper bound on index readiness, not a direct index timestamp.
The opening oracle requires the complete initial viewport and total file count. A loading screen does not pass.
A changed viewer needs an additional prefix-publication oracle before its earlier progressive content can receive an opening measurement.

Navigation starts immediately before the first input byte.
Completion is the receipt of a complete synchronized frame with the expected content grid.
The runner timestamps each PTY read before terminal parsing and grid comparison.
Validation duration remains separate. The runner does not subtract overhead from latency.
These are observed PTY-content latencies. They are not Ghostty pixel-presentation times.

The runner reconstructs incremental output with pyte.
It checks all content rows, file headers, hunk headers, source text, line numbers, active markers, separators, and blank cells.
It checks the exact title, file count, layout, and theme.
Only three exact footer states are allowed: highlighting pending, declared plain fallback, or patch-context status.
The footer alone never proves correct syntax colors.
Combining characters use NFC cell representation to match pyte. The separate supplied-source digest retains exact byte identity.
The oracle does not establish general emoji-cluster compatibility or full terminal-emulator equivalence.

## Fixed inputs and failure rules

The initial public click selects the first source row without moving the viewport.
Each repeat contains 40 wheel-down events and 40 wheel-up events. Each event moves three display rows.
Then 20 `G`/`g` cycles request the last and first source rows.
Forty layout changes and forty resizes use the middle source record in the largest hunk.
Their frozen setup uses public counted movement and a public side-selection click.

The original executable lacks counted navigation. Its layout and resize categories therefore remain unsupported at the specified anchor.
The runner does not substitute a nearby viewport or a different executable.
The original `g` command selects a file header instead of the first source row.
A separate diagnostic grid identifies that known defect. It never supplies a successful latency sample.

Only one measured command can be pending. Input intervals are at least 50 ms.
A late completion delays the next input and remains in the results.
Each command has a 30-second watchdog. A timeout is not a styling fallback.
A failure with an unknown resulting state blocks dependent commands. Their slots remain explicit missing data.
All five trials remain in the report, including parse errors and harness failures.

Each fixture and category has 200 expected samples.
The p95 is sorted rank 190. Median, p95, and maximum appear only when all 200 samples have correct grids.
There is no pooled score and no passing percentile from a shortened or partly incorrect category.
Opening and quit retain individual trial observations. Their summary uses all five repeats.

## Memory and cleanup

A background sampler measures the owned process group at less than 10 Hz.
Each `ps` invocation has a one-second timeout. It cannot block the PTY reader.
The report records sampler wall time, sample count, and the sampled peak.
A sampled peak above 6 GiB stops the trial. This is not an OS-enforced memory limit.

The runner also collects the viewer's OS high-water RSS through `wait4`.
macOS reports this value in bytes. The `ps` RSS values use KiB and are converted to bytes.
These peaks cover this content trace, including parse-error trials. They do not represent the full specified memory tour.
Steady RSS and GUI-terminal RSS remain null, not zero.

Quit starts at the `q` input. Process-exit observation uses the PTY loop, not the memory sampler.
The report separately records terminal-mode restoration and received cleanup sequences.
On failure, cleanup targets only the owned viewer process group, with bounded TERM/KILL escalation.
The runner retains a bounded output tail and final text grid, not a complete replay recording.

## Frozen artifacts and reproduction

The freeze includes fixture hashes, independent references, input bytes, source anchors, complete expected grids, tool hashes, and dependency versions.
It also binds the Python executable and preserved viewer executable.
The runner checks the exact trace bytes that it subsequently loads.
Any change to a frozen input or tool requires a new trace revision and a complete new run.
Pre-freeze correctness rehearsals retain no timing measurements.

Install the measurement dependencies into the private environment:

```sh
cd /Users/ramos/oss/native-diff
python3.12 -m venv .measurement-venv
.measurement-venv/bin/python -m pip install -r tools/measurement-requirements.txt
```

Run the real-process harness checks:

```sh
.measurement-venv/bin/python tools/check_measurement.py
```

Create a new freeze directory, then run the preserved baseline:

```sh
.measurement-venv/bin/python tools/freeze_trace.py fixtures/diffshub/measurement-v1
.measurement-venv/bin/python tools/terminal_baseline.py fixtures/diffshub/measurement-v1 evidence/content-baseline-v1
```

Both commands refuse to overwrite their output directories.
The pinned measurement libraries are [pyte](https://pypi.org/project/pyte/0.8.2/), [wcwidth](https://pypi.org/project/wcwidth/0.2.13/), and [regex](https://pypi.org/project/regex/2024.11.6/).
Their packages remain inside the private environment.

## Remaining acceptance work

This trace does not complete Step A or the large-diff acceptance suite.
The sequential style oracle, style timings, normalized full-metadata reference, three-tour memory measurements, and long-line tours remain pending.
Ghostty visual signoff remains separate. This run produces no GUI screenshots, videos, or display-latency claims.
