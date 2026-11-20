//! A host owns terminal modes, input and focus; Engine owns each review and its workers.

use std::{ffi::OsString, io, path::PathBuf, sync::mpsc, time::Duration};

use anyhow::{Context, Result, bail};
use clap::{Arg, ArgAction, builder::OsStringValueParser, value_parser};
use crossterm::{
    cursor::{Hide, Show},
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseButton, MouseEventKind,
    },
    execute,
    terminal::{
        self, BeginSynchronizedUpdate, EndSynchronizedUpdate, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use diffs_rs::engine::{
    Badge, BadgeColor, Engine, FileMark, Input, Layout, Marker, Options, Side, SidebarPreference,
};
use diffs_rs::input::LocalInput;
use diffs_rs::reading::SourceRange;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

#[path = "../src/cleanup.rs"]
mod cleanup;
#[path = "../src/terminal_input.rs"]
mod terminal_input;
use terminal_input::TerminalInput;

enum ReviewInput {
    Patch(PathBuf),
    Local(LocalInput),
}

impl ReviewInput {
    fn open(&self) -> Input {
        match self {
            Self::Patch(path) => Input::Patch(path.clone()),
            Self::Local(input) => Input::Local(input.clone()),
        }
    }
}

struct Arguments {
    inputs: Vec<ReviewInput>,
    chrome: bool,
}

fn review_inputs() -> Result<Arguments> {
    let matches = clap::Command::new("embedded_viewer")
        .about("Open local reviews in one terminal host, in argument order")
        .arg_required_else_help(true)
        .arg(
            Arg::new("patch")
                .value_name("PATCH")
                .num_args(0..)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("files")
                .long("files")
                .value_names(["OLD", "NEW"])
                .num_args(2)
                .action(ArgAction::Append)
                .value_parser(value_parser!(PathBuf))
                .help("Compare captured files; repeat to add another review"),
        )
        .arg(
            Arg::new("git")
                .long("git")
                .value_names(["REPO", "BASE", "HEAD"])
                .num_args(3)
                .action(ArgAction::Append)
                .value_parser(OsStringValueParser::new())
                .help("Compare Git commits; use full commit IDs to pin repeated opens"),
        )
        .arg(
            Arg::new("no-chrome")
                .long("no-chrome")
                .action(ArgAction::SetTrue)
                .help("Hide the viewer's label row, footer and help; the host draws a status row"),
        )
        .get_matches();
    let mut inputs = Vec::new();
    if let Some(paths) = matches.get_many::<PathBuf>("patch") {
        for (index, path) in matches.indices_of("patch").unwrap().zip(paths) {
            inputs.push((index, ReviewInput::Patch(path.clone())));
        }
    }
    if let Some(pairs) = matches.get_occurrences::<PathBuf>("files") {
        for (index, mut pair) in matches.indices_of("files").unwrap().step_by(2).zip(pairs) {
            inputs.push((
                index,
                ReviewInput::Local(LocalInput::Files {
                    old: pair.next().unwrap().clone(),
                    new: pair.next().unwrap().clone(),
                }),
            ));
        }
    }
    if let Some(reviews) = matches.get_occurrences::<OsString>("git") {
        for (index, mut review) in matches.indices_of("git").unwrap().step_by(3).zip(reviews) {
            inputs.push((
                index,
                ReviewInput::Local(LocalInput::Git {
                    repo: review.next().unwrap().into(),
                    base: review
                        .next()
                        .unwrap()
                        .to_str()
                        .context("BASE must be UTF-8")?
                        .into(),
                    head: review
                        .next()
                        .unwrap()
                        .to_str()
                        .context("HEAD must be UTF-8")?
                        .into(),
                }),
            ));
        }
    }
    inputs.sort_unstable_by_key(|(index, _)| *index);
    Ok(Arguments {
        inputs: inputs.into_iter().map(|(_, input)| input).collect(),
        chrome: !matches.get_flag("no-chrome"),
    })
}

struct HostTerminal {
    events: TerminalInput,
    restored: bool,
}

impl HostTerminal {
    fn close(&mut self) -> Result<()> {
        let events = self.events.close();
        if self.restored {
            return events;
        }
        let screen = execute!(
            io::stdout(),
            EndSynchronizedUpdate,
            DisableMouseCapture,
            Show,
            LeaveAlternateScreen
        )
        .map_err(Into::into);
        let mode = terminal::disable_raw_mode().map_err(Into::into);
        let restore = cleanup::finish(screen, mode);
        self.restored = restore.is_ok();
        cleanup::finish(events, restore)
    }
}

impl Drop for HostTerminal {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

fn host_text(engine: &Engine, review: usize, focus: bool) -> Result<String> {
    let status = engine.status();
    let state = if status.closed {
        "closed"
    } else if status.error.is_some() {
        "input error"
    } else if status.loading {
        "loading"
    } else if status.coloring || status.preparing_source {
        "coloring"
    } else if status.source_failed {
        "source unavailable"
    } else {
        "ready"
    };
    let location = match engine.selected_source()? {
        Some(source) => format!(
            "File: {}\nSide: {:?}\nOld: {:?}\nNew: {:?}",
            source.file, source.side, source.old_line, source.new_line
        ),
        None => "No source selected".into(),
    };
    let focus = if focus { "viewer" } else { "host" };
    Ok(format!(
        "Host pane\nFocus: {focus}\nReview: {}\nState: {state}\n\n{location}\n\n\
         Tab  switch focus\nr    next review\no    reopen review\nc    close review\n\
         m    mark selection\n'    select last mark\nv    mark file viewed\nq    exit host\n\n\
         Viewer keys stay local.",
        review + 1
    ))
}

/// The host's own status row, used when the viewer draws no footer.
fn status_text(engine: &Engine, marks: usize) -> String {
    let status = engine.status();
    let state = if status.closed {
        "Review closed"
    } else if let Some(error) = status.error {
        error
    } else if status.loading {
        "Loading patch…"
    } else if status.coloring || status.preparing_source {
        "Coloring visible code…"
    } else if status.source_failed {
        "Source unavailable · patch view retained"
    } else {
        "Ready"
    };
    format!(" {state} · {} files · {marks} marks", status.files)
}

/// Marks the selection, or the cursor line when nothing is selected.
fn mark(engine: &mut Engine, marks: &mut Vec<Marker>) -> Result<()> {
    let range = match engine.selection() {
        Some(range) => range,
        None => {
            let Some(source) = engine.selected_source()? else {
                return Ok(());
            };
            let line = match source.side {
                Side::Old => source.old_line,
                Side::New => source.new_line,
            };
            let Some(line) = line else {
                return Ok(());
            };
            SourceRange {
                review: source.review,
                file: source.file,
                side: source.side,
                first: line,
                last: line,
            }
        }
    };
    marks.push(Marker {
        range,
        style: Style::new().fg(Color::Yellow),
    });
    engine.set_markers(marks)
}

/// Marks the cursor's file viewed, or unmarks it: a tick in the Files sidebar and a dimmed row.
fn toggle_viewed(engine: &mut Engine, viewed: &mut Vec<String>) -> Result<()> {
    let Some(source) = engine.selected_source()? else {
        return Ok(());
    };
    let Some(file) = engine.file(source.file) else {
        return Ok(());
    };
    let path = file.path.to_string();
    match viewed.iter().position(|p| *p == path) {
        Some(index) => {
            viewed.remove(index);
        }
        None => viewed.push(path),
    }
    let marks: Vec<FileMark<'_>> = viewed
        .iter()
        .map(|path| FileMark {
            path,
            badge: Some(Badge {
                text: "✓",
                color: BadgeColor::Added,
            }),
            dimmed: true,
        })
        .collect();
    engine.set_file_marks(&marks)
}

fn run() -> Result<()> {
    let Arguments { inputs, chrome } = review_inputs()?;
    let (wake, ready) = mpsc::sync_channel(1);
    let events = TerminalInput::start(wake.clone())?;
    terminal::enable_raw_mode()?;
    let mut host = HostTerminal {
        events,
        restored: false,
    };
    let result = (|| -> Result<()> {
        execute!(io::stdout(), EnterAlternateScreen, Hide, EnableMouseCapture)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
        let mut review = 0;
        let mut engine = Engine::open(
            inputs[review].open(),
            Options {
                layout: Layout::Split,
                sidebar: SidebarPreference::Hide,
                chrome,
                ..Options::default()
            },
            wake,
        )?;
        let mut marks = Vec::new();
        let mut viewed = Vec::new();
        let mut focus = true;
        let mut dirty = true;
        let mut resized = false;
        let mut viewer = Rect::default();
        let result = (|| -> Result<()> {
            loop {
                let _ = ready.try_recv();
                if host.events.signal() != 0 {
                    return Ok(());
                }
                engine.poll()?;
                if dirty || engine.needs_draw() {
                    execute!(io::stdout(), BeginSynchronizedUpdate)?;
                    if resized {
                        terminal.resize(terminal.size()?.into())?;
                        resized = false;
                    }
                    let mut rendered = Ok(());
                    let drawn = terminal.draw(|frame| {
                        let area = frame.area();
                        let width = area.width.min(28);
                        let beside = Rect::new(area.x, area.y, width, area.height);
                        viewer = Rect::new(area.x + width, area.y, area.width - width, area.height);
                        if !chrome {
                            // Without viewer chrome, the host owns a status row below the diff.
                            viewer.height = viewer.height.saturating_sub(1);
                            frame.render_widget(
                                Paragraph::new(status_text(&engine, marks.len())),
                                Rect::new(viewer.x, viewer.bottom(), viewer.width, 1)
                                    .intersection(area),
                            );
                        }
                        rendered = engine.draw(frame, viewer).and_then(|()| {
                            let text = host_text(&engine, review, focus)?;
                            frame.render_widget(
                                Paragraph::new(text).block(Block::new().borders(Borders::RIGHT)),
                                beside,
                            );
                            Ok(())
                        });
                    });
                    execute!(io::stdout(), EndSynchronizedUpdate)?;
                    drawn?;
                    rendered?;
                    engine.after_draw();
                    dirty = false;
                }
                let event = match host.events.try_event() {
                    Ok(event) => event?,
                    Err(mpsc::TryRecvError::Disconnected) => bail!("Host input stopped"),
                    Err(mpsc::TryRecvError::Empty) => {
                        let _ = ready.recv_timeout(Duration::from_millis(100));
                        continue;
                    }
                };
                if let Event::Key(key) = event {
                    if key.kind == KeyEventKind::Release {
                        continue;
                    }
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            return Ok(());
                        }
                        KeyCode::Tab => {
                            focus = !focus;
                            dirty = true;
                            continue;
                        }
                        KeyCode::Char('c') => {
                            engine.close()?;
                            marks.clear();
                            viewed.clear();
                            dirty = true;
                            continue;
                        }
                        KeyCode::Char('r' | 'o') => {
                            if key.code == KeyCode::Char('r') {
                                review = (review + 1) % inputs.len();
                            }
                            engine.replace(inputs[review].open())?;
                            marks.clear();
                            viewed.clear();
                            dirty = true;
                            continue;
                        }
                        KeyCode::Char('m') if !engine.status().closed => {
                            mark(&mut engine, &mut marks)?;
                            dirty = true;
                            continue;
                        }
                        KeyCode::Char('v') if !engine.status().closed => {
                            toggle_viewed(&mut engine, &mut viewed)?;
                            dirty = true;
                            continue;
                        }
                        KeyCode::Char('\'') => {
                            // A mark on revealed context leaves the diff if its source is
                            // withdrawn; the error is recoverable, so drop that mark.
                            if let Some(last) = marks.last()
                                && engine.select_range(last.range).is_err()
                            {
                                marks.pop();
                                engine.set_markers(&marks)?;
                            }
                            dirty = true;
                            continue;
                        }
                        _ => {}
                    }
                }
                if let Event::Mouse(mouse) = event
                    && mouse.kind == MouseEventKind::Down(MouseButton::Left)
                {
                    focus = viewer.contains((mouse.column, mouse.row).into());
                    dirty = true;
                }
                if matches!(event, Event::Resize(..)) {
                    resized = true;
                    dirty = true;
                }
                engine.handle_event(event, focus)?;
            }
        })();
        let review_closed = engine.close();
        cleanup::finish(result, review_closed.context("Closing review"))
    })();
    let terminal_closed = host.close();
    cleanup::finish(result, terminal_closed.context("Closing host terminal"))
}

fn main() {
    if let Err(error) = run() {
        eprintln!(
            "embedded_viewer: {}",
            diffs_rs::text::safe_text(format!("{error:#}").as_bytes())
        );
        std::process::exit(1);
    }
}
