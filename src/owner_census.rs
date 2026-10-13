use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, SyncSender},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;
use syntect::parsing::SyntaxSet;

use crate::app::App;

pub struct Capture {
    directory: Option<PathBuf>,
    next: usize,
    pending: Option<Receiver<Result<()>>>,
}

pub(crate) struct Request {
    path: PathBuf,
    done: SyncSender<Result<()>>,
}

fn timestamp() -> Result<u128> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos())
}

fn create(path: &Path) -> Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("Cannot create owner census {}", path.display()))
}

fn write(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut output = BufWriter::new(create(path)?);
    serde_json::to_writer(&mut output, value)?;
    output.flush()?;
    Ok(())
}

impl Request {
    pub(crate) fn capture(self, syntaxes: &SyntaxSet, inspect: impl FnOnce() -> serde_json::Value) {
        let result = (|| {
            let started_ns = timestamp()?;
            let census = syntaxes.owner_census();
            let retained = inspect();
            let observed_ns = timestamp()?;
            #[derive(Serialize)]
            struct Observation<T> {
                schema: u8,
                started_ns: u128,
                observed_ns: u128,
                census: T,
                retained: serde_json::Value,
            }
            write(
                &self.path,
                &Observation {
                    schema: 1,
                    started_ns,
                    observed_ns,
                    census,
                    retained,
                },
            )
        })();
        let _ = self.done.send(result);
    }
}

impl Capture {
    pub fn from_env() -> Result<Self> {
        let directory = std::env::var_os("NATIVE_DIFF_OWNER_CENSUS").map(PathBuf::from);
        if let Some(path) = &directory {
            ensure!(
                path.is_dir(),
                "Owner census requires an existing output directory"
            );
        }
        Ok(Self {
            directory,
            next: 0,
            pending: None,
        })
    }

    pub fn request(&mut self, app: &App) -> Result<()> {
        let Some(directory) = &self.directory else {
            return Ok(());
        };
        ensure!(self.pending.is_none(), "Owner census already pending");
        ensure!(self.next < 8, "Owner census snapshot limit reached");
        let (done, received) = mpsc::sync_channel(1);
        app.worker.request_census(Request {
            path: directory.join(format!("syntax-{:02}.json", self.next)),
            done,
        })?;
        self.pending = Some(received);
        Ok(())
    }

    pub fn poll(&mut self, app: &App) -> Result<()> {
        let Some(received) = &self.pending else {
            return Ok(());
        };
        match received.try_recv() {
            Ok(result) => result?,
            Err(mpsc::TryRecvError::Empty) => return Ok(()),
            Err(mpsc::TryRecvError::Disconnected) => bail!("Owner census worker disconnected"),
        }
        let directory = self
            .directory
            .as_ref()
            .context("Missing census directory")?;
        let started_ns = timestamp()?;
        let cache = app.cache.census();
        let context_view = app.view.census();
        let retained = serde_json::json!({
            "schema": 1,
            "source_pages": app.doc.files.first().map(|file| file.spool.cache_census()).transpose()?,
            "geometry": app.geometry.census()?,
        });
        let observed_ns = timestamp()?;
        write(
            &directory.join(format!("app-{:02}.json", self.next)),
            &serde_json::json!({
                "schema": 1, "started_ns": started_ns, "observed_ns": observed_ns,
                "cache": cache, "epoch": app.epoch, "loading": app.loading,
                "context_view": context_view, "retained": retained,
                "top": [app.top.row, app.top.part], "cursor": [app.cursor.row, app.cursor.part],
                "old_side": app.old_side, "split": app.split(), "wrap": app.wrap,
                "horizontal": app.horizontal, "size": [app.width, app.height],
            }),
        )?;
        create(&directory.join(format!("snapshot-{:02}.ready", self.next)))?;
        self.next += 1;
        self.pending = None;
        Ok(())
    }
}
