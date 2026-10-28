# diffs-rs

[![crates.io](https://img.shields.io/crates/v/diffs-rs.svg)](https://crates.io/crates/diffs-rs)
[![docs.rs](https://img.shields.io/docsrs/diffs-rs)](https://docs.rs/diffs-rs)
[![CI](https://github.com/plannotator/diffs-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/plannotator/diffs-rs/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/diffs-rs.svg)](LICENSE)

A fast terminal diff viewer, and the Rust diff engine behind it that you can embed in a Ratatui
application.

![diffs showing a split diff with the changed-file sidebar](https://raw.githubusercontent.com/plannotator/diffs-rs/main/assets/screenshot.webp)

## Install

```sh
cargo install diffs-rs --locked
```

This installs the `diffs` binary. Building needs Rust 1.88 or newer and a C compiler for the
bundled libgit2.

## Usage

```sh
diffs patch changes.patch                      # a patch file
git diff --no-color | diffs patch -            # a patch on stdin
git log -p -3 --no-color | diffs patch -       # git show, log -p, and format-patch output work
diffs files old.rs new.rs                      # two files
diffs git --repo . --base main --head HEAD     # two commits, compared tree to tree
diffs --layout unified --theme light patch changes.patch
```

`--layout` is `auto`, `split`, or `unified`; `--theme` is `dark` or `light`. `--sidebar` and
`--no-sidebar` override the changed-file sidebar. `diffs files` and `diffs git` accept `--check`
to verify every source byte and print a JSON report instead of opening the viewer. Press `?` in
the viewer for all key bindings.

`diffs` never changes the repository, index, or working tree, and makes no network requests.

## Features

- Split and unified layouts, wrapping, line numbers, and dark and light themes.
- Syntax colors for the bundled grammars, with changed-word emphasis.
- For `files` and `git`, colors are computed from the complete old and new sources, and unchanged
  gaps can be expanded 20 lines at a time.
- A changed-file sidebar with the patch's directory tree and per-file and per-folder counts.
- Large patches open progressively; the first screen appears while the rest is still indexing.
- Literal search across the whole diff (`/`, F3).
- Source-line selection (`V`), saving the original bytes (Ctrl-S), and plain-text copy (`y`, macOS).
- Mouse support: wheel scrolling, clicking files, folders, and context gaps.

## Performance

On the Linux `v6.0...v7.0` patch (711 MB, 76,872 files, 421,819 hunks):

| Measure | Result |
| --- | ---: |
| First text on screen | 11.8 ms |
| First syntax colors | 35.5 ms |
| Navigation, p95 of the slowest action type | 8.6 ms |
| Peak memory | 594 MiB |

The first prototype used 2,590 MiB on the same patch and still failed to open it. Times are
medians of five fresh processes on an Apple M5 Max with a warm file cache, measured when the
correct output reached a 160 x 50 PTY, on pre-release builds. The
[measurements](docs/measurements/README.md) page has all five test patches, the method, and
the open memory goals.

## Embedding

```toml
[dependencies]
diffs-rs = { version = "0.1", default-features = false }
anyhow = "1"
ratatui = { version = "0.30.2", default-features = false, features = ["crossterm"] }
```

The host owns the terminal and the event loop. The engine loads and colors the review on
background threads and draws into any `Rect`:

```rust
use std::{sync::mpsc, time::Duration};

use diffs_rs::engine::{Engine, Input, Options};
use ratatui::crossterm::event::{self, Event, KeyCode};

fn main() -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    // The engine sends a hint on this channel when background work is ready.
    let (wake, woken) = mpsc::sync_channel(1);
    let input = Input::Patch("changes.patch".into());
    let mut engine = Engine::open(input, Options::default(), wake)?;

    let result = (|| -> anyhow::Result<()> {
        loop {
            let _ = woken.try_recv();
            engine.poll()?;
            if engine.needs_draw() {
                let mut drawn = Ok(());
                terminal.draw(|frame| {
                    let area = frame.area();
                    drawn = engine.draw(frame, area);
                })?;
                drawn?;
                engine.after_draw();
            }
            if event::poll(Duration::from_millis(16))? {
                let event = event::read()?;
                if matches!(&event, Event::Key(key) if key.code == KeyCode::Char('q')) {
                    return Ok(());
                }
                engine.handle_event(event, true)?;
            }
        }
    })();

    // Joins the background workers and reports cleanup errors.
    let closed = engine.close();
    ratatui::restore();
    result.and(closed)
}
```

`Input` also accepts an open stream, two files, or two Git commits. The
[embedding guide](docs/embedding.md) covers the host loop, focus, source coordinates, the reading
tools, and shutdown. [`examples/embedded_viewer.rs`](examples/embedded_viewer.rs) is a complete
host with its own pane beside the viewer.

## Platform support

Unix only: the viewer needs a PTY, `/dev/tty`, and Unix signals. macOS on Apple silicon is the
most tested platform. Linux runs the full test suite in CI. Windows is not supported.
`diffs files` and `diffs git` need Git 2.50 or newer.

## Forked dependencies

Syntax highlighting uses forks of three crates, published as
[`diffs-syntect`](crates/diffs-syntect/PATCHES.md) (syntect 5.3.0),
[`diffs-fancy-regex`](crates/diffs-fancy-regex/PATCHES.md) (fancy-regex 0.16.2), and
[`diffs-two-face`](crates/diffs-two-face/README.md) (two-face 0.5.2, rebuilt against
`diffs-syntect`). The patches share compiled regex engines between identical grammar patterns,
reuse search buffers within a line, and skip searches that cannot match because a required byte
is missing from the line. Matching results are unchanged. On the test traces this lowered live
heap from 275 MiB to 213 MiB and halved the time to color a deep, first-visited file, from
1,494 ms to 752 ms. The forks exist for diffs-rs; other projects should use the upstream crates.

## License

Apache-2.0. See [LICENSE](LICENSE). The bundled theme and the forked crates keep their own
licenses, listed in [NOTICE.md](NOTICE.md).
