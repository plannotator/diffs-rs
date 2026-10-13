use std::{
    fs::File,
    io::{self, BufReader, IsTerminal, Read, Write},
    os::fd::AsFd,
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use crossterm::{
    cursor::{Hide, Show},
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{
        self, BeginSynchronizedUpdate, EndSynchronizedUpdate, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use native_diff::{
    app::{Layout, SidebarPreference},
    engine::{Engine, Input, Options},
    input::{self, LocalInput, SourceState},
    model::Document,
    storage,
    text::safe_text,
    theme::Appearance,
};
use ratatui::{Terminal, backend::CrosstermBackend};

mod cleanup;
mod clipboard;
mod reading_export;
mod reading_ui;
mod reading_worker;
mod terminal_input;
use terminal_input::TerminalInput;

#[derive(Parser)]
#[command(
    version,
    about = "Read-only native terminal diff viewer · visual prototype"
)]
struct Cli {
    #[arg(long, value_enum, default_value = "auto", global = true)]
    layout: Layout,
    #[arg(long, value_enum, default_value = "dark", global = true)]
    theme: Appearance,
    /// Show changed files, including for a single-file patch.
    #[arg(long, global = true, conflicts_with = "no_sidebar")]
    sidebar: bool,
    /// Start with the changed-file sidebar hidden.
    #[arg(long, global = true)]
    no_sidebar: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(hide = true)]
    ClipboardHelper {
        #[arg(long)]
        bytes: u64,
    },
    /// View a patch file. Use - to read the patch from stdin.
    Patch { path: PathBuf },
    /// Compare two captured regular files with checked full-source coloring.
    Files {
        old: PathBuf,
        new: PathBuf,
        /// Check all source bytes and print a JSON report without opening the viewer.
        #[arg(long)]
        check: bool,
    },
    /// Compare two fixed Git commits directly, without changing the repository.
    Git {
        #[arg(long)]
        repo: PathBuf,
        #[arg(long)]
        base: String,
        #[arg(long)]
        head: String,
        /// Check all source bytes and print a JSON report without opening the viewer.
        #[arg(long)]
        check: bool,
    },
    /// Inspect the parsed source model as JSON, without a terminal.
    Inspect {
        path: PathBuf,
        #[arg(long)]
        summary: bool,
        #[arg(long, conflicts_with = "summary")]
        canonical: bool,
    },
}

fn with_input<T>(path: &Path, read: impl FnOnce(&mut dyn Read) -> Result<T>) -> Result<T> {
    if path.as_os_str() == "-" {
        read(&mut io::stdin())
    } else {
        let mut file =
            File::open(path).with_context(|| format!("Cannot open {}", path.display()))?;
        let before = file.metadata()?;
        let result = read(&mut file)?;
        let after = file.metadata()?;
        if before.is_file()
            && (before.len() != after.len() || before.modified()? != after.modified()?)
        {
            bail!("Input changed during reading. Open a stable patch.");
        }
        Ok(result)
    }
}

fn check_sources(input: LocalInput) -> Result<i32> {
    let cancel = input::Cancellation::default();
    let mut signals = signal_hook::iterator::Signals::new([
        signal_hook::consts::SIGINT,
        signal_hook::consts::SIGTERM,
        signal_hook::consts::SIGHUP,
    ])?;
    let handle = signals.handle();
    let stop = cancel.clone();
    let listener = thread::spawn(move || {
        if signals.forever().next().is_some() {
            stop.stop();
        }
    });
    let result = (|| -> Result<i32> {
        let mut review = input::Review::open(input, &cancel, |_| Ok(()))?;
        let report = review.report(&cancel)?;
        let mut output = io::BufWriter::new(io::stdout().lock());
        serde_json::to_writer_pretty(&mut output, &report)?;
        writeln!(output)?;
        output.flush()?;
        Ok(i32::from(
            review
                .sources()
                .iter()
                .any(|state| matches!(state, SourceState::Unavailable(_))),
        ))
    })();
    handle.close();
    let _ = listener.join();
    if cancel.check().is_err() {
        return Ok(130);
    }
    result
}

fn read_indexed(
    path: &Path,
    publish: impl FnMut(Arc<storage::FileIndex>) -> Result<()>,
) -> Result<Arc<storage::ByteStore>> {
    with_input(path, |input| storage::read(input, publish))
}

struct TerminalGuard {
    events: TerminalInput,
    restored: bool,
}
impl TerminalGuard {
    fn close(&mut self) -> Result<()> {
        let events = self.events.close();
        if self.restored {
            return events;
        }
        let screen = execute!(
            io::stdout(),
            EndSynchronizedUpdate,
            DisableMouseCapture,
            DisableBracketedPaste,
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
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

fn run(cli: Cli) -> Result<i32> {
    if let Command::ClipboardHelper { bytes } = cli.command {
        clipboard::helper(bytes)?;
        return Ok(0);
    }
    let sidebar_preference = if cli.sidebar {
        SidebarPreference::Show
    } else if cli.no_sidebar {
        SidebarPreference::Hide
    } else {
        SidebarPreference::Auto
    };
    #[cfg(feature = "delivery-trace")]
    let _trace = native_diff::delivery_trace::init()?;
    let (local, check) = match &cli.command {
        Command::Files { old, new, check } => (
            Some(LocalInput::Files {
                old: old.clone(),
                new: new.clone(),
            }),
            *check,
        ),
        Command::Git {
            repo,
            base,
            head,
            check,
        } => (
            Some(LocalInput::Git {
                repo: repo.clone(),
                base: base.clone(),
                head: head.clone(),
            }),
            *check,
        ),
        _ => (None, false),
    };
    if check {
        return check_sources(local.context("Missing local input")?);
    }
    if let Command::Inspect {
        path,
        summary,
        canonical,
    } = &cli.command
    {
        if *canonical {
            if io::stdout().is_terminal() {
                bail!("Canonical source output is binary. Redirect it to a file or pipe.");
            }
            let mut output = io::BufWriter::new(io::stdout().lock());
            read_indexed(path, |file| {
                file.write_canonical(&mut output)?;
                output.flush()?;
                Ok(())
            })?;
            output.flush()?;
            return Ok(0);
        }
        let mut output = io::BufWriter::new(io::stdout().lock());
        if *summary {
            let mut doc = storage::Document::default();
            let spool = read_indexed(path, |file| doc.append(file))?;
            serde_json::to_writer(
                &mut output,
                &serde_json::json!({"files":doc.files.len(),"source_lines":doc.source_len(),"additions":doc.files.iter().map(|f|f.additions).sum::<usize>(),"deletions":doc.files.iter().map(|f|f.deletions).sum::<usize>(),
                    "storage": {"spool_bytes": spool.len(),
                        "source_index_bytes": doc.files.iter().map(|f| f.source_index_bytes()).sum::<usize>(),
                        "source_index_stored_bytes": doc.files.iter().map(|f| f.source_index_stored_bytes()).sum::<u64>(),
                        "projection_bytes": doc.files.iter().map(|f| f.projection_bytes()).sum::<usize>(),
                        "source_run_bytes": doc.files.iter().map(|f| f.source_run_bytes()).sum::<usize>(),
                        "shared_block_bytes": doc.files.iter().map(|f| f.shared_block_bytes()).sum::<usize>(),
                        "side_index_bytes": doc.files.iter().map(|f| f.side_index_bytes()).sum::<usize>(),
                        "marker_index_bytes": doc.files.iter().map(|f| f.marker_index_bytes()).sum::<usize>(),
                        "boundary_index_bytes": doc.files.iter().map(|f| f.boundary_index_bytes()).sum::<usize>(),
                        "metadata_allocation_bytes": doc.files.iter().map(|f| f.metadata_bytes()).sum::<usize>(),
                        "file_descriptor_bytes": doc.files.len() * size_of::<storage::FileIndex>(),
                        "file_list_capacity_bytes": doc.files.capacity() * size_of::<Arc<storage::FileIndex>>(),
                        "cached_source_bytes": spool.cached_bytes()}}),
            )?;
        } else {
            let doc = with_input(path, |input| Document::read(BufReader::new(input)))?;
            serde_json::to_writer(&mut output, &doc)?;
        }
        writeln!(output)?;
        return Ok(0);
    }
    if !io::stdout().is_terminal() {
        bail!("Interactive display requires a terminal. Use `inspect` for source diagnostics.");
    }
    if matches!(&cli.command, Command::Patch { path } if path.as_os_str() == "-")
        && io::stdin().is_terminal()
    {
        bail!(
            "Patch stdin must be piped or redirected; the controlling terminal is used for keyboard input."
        );
    }
    File::open("/dev/tty").context("No controlling terminal is available for keyboard input")?;
    #[cfg(feature = "owner-census")]
    let mut owner_census = native_diff::owner_census::Capture::from_env()?;
    let (wake, ready) = mpsc::sync_channel(1);
    let events = TerminalInput::start(wake.clone())?;
    terminal::enable_raw_mode()?;
    let mut guard = TerminalGuard {
        events,
        restored: false,
    };
    let result = (|| -> Result<i32> {
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            Hide,
            EnableMouseCapture,
            EnableBracketedPaste
        )?;
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
        let input = if let Some(input) = local {
            Input::Local(input)
        } else if let Command::Patch { path } = cli.command {
            if path.as_os_str() == "-" {
                Input::Stream {
                    file: io::stdin().as_fd().try_clone_to_owned()?.into(),
                    label: "-".into(),
                }
            } else {
                Input::Patch(path)
            }
        } else {
            bail!("Missing interactive input");
        };
        let options = Options {
            layout: cli.layout,
            appearance: cli.theme,
            sidebar: sidebar_preference,
            ..Options::default()
        };
        let mut engine = Engine::open(input, options, wake.clone())?;
        let mut reading = reading_ui::Reading::new(wake.clone());
        let mut resized = false;
        let result = (|| -> Result<i32> {
            loop {
                // Clear before draining payloads; notifications arriving during a drain remain pending.
                let _ = ready.try_recv();
                let signal = guard.events.signal();
                if signal != 0 {
                    return Ok(128 + signal);
                }
                engine.poll()?;
                reading.poll(&mut engine)?;
                #[cfg(feature = "owner-census")]
                engine.poll_census(&mut owner_census)?;
                if engine.needs_draw() || reading.needs_draw() {
                    execute!(io::stdout(), BeginSynchronizedUpdate)?;
                    if resized {
                        // Coalesced resize signals can return to the previous dimensions after clipping cells.
                        terminal.resize(terminal.size()?.into())?;
                        resized = false;
                    }
                    let mut display_result = Ok(());
                    let drawn = terminal.draw(|frame| {
                        let controls = reading.controls(frame.area().width);
                        display_result =
                            engine.draw_with_controls(frame, frame.area(), Some(&controls));
                        if display_result.is_ok() {
                            reading.draw_prompt(frame);
                        }
                    });
                    execute!(io::stdout(), EndSynchronizedUpdate)?;
                    drawn?;
                    display_result?;
                    engine.after_draw();
                    reading.after_draw();
                }
                let event = match guard.events.try_event() {
                    Ok(event) => event?,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        bail!("The terminal input worker stopped")
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        native_diff::delivery_event!("wait_ready", 0, 0);
                        let _ = ready.recv_timeout(Duration::from_millis(100));
                        match guard.events.try_event() {
                            Ok(event) => event?,
                            Err(mpsc::TryRecvError::Empty) => continue,
                            Err(mpsc::TryRecvError::Disconnected) => {
                                bail!("The terminal input worker stopped")
                            }
                        }
                    }
                };
                native_diff::delivery_event!("input", 0, 0);
                if let Event::Key(key) = event {
                    if key.kind == KeyEventKind::Release {
                        continue;
                    }
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        return Ok(130);
                    }
                }
                if reading.handle(&event, &mut engine)? {
                    continue;
                }
                if matches!(event, Event::Key(key) if key.code == KeyCode::Char('q')) {
                    let status = engine.status();
                    return Ok(i32::from(status.error.is_some() || status.source_failed));
                }
                if matches!(event, Event::Resize(..)) {
                    resized = true;
                }
                #[cfg(feature = "owner-census")]
                if matches!(event, Event::Key(key) if key.code == KeyCode::F(12)) {
                    engine.request_census(&mut owner_census)?;
                    continue;
                }
                engine.handle_event(event, true)?;
            }
        })();
        let reading_closed = reading.close();
        let viewer_closed = engine.close();
        cleanup::finish(cleanup::finish(result, reading_closed), viewer_closed)
    })();
    let terminal_closed = guard.close();
    cleanup::finish(result, terminal_closed)
}

fn main() {
    let code = match run(Cli::parse()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!(
                "native-diff: {}",
                safe_text(format!("{error:#}").as_bytes())
            );
            1
        }
    };
    std::process::exit(code);
}
