//! A terminal diff viewer engine for Ratatui applications.
//!
//! This crate provides the `diffs` command-line viewer and the [`engine::Engine`] it is built
//! on. An `Engine` opens one review (a Git patch, two files, or two Git commits), loads and
//! colors it on background threads, and draws into any [`ratatui::layout::Rect`]. The host keeps
//! ownership of the terminal, input, and layout.
//!
//! ```no_run
//! use std::{sync::mpsc, time::Duration};
//!
//! use diffs_rs::engine::{Engine, Input, Options};
//! use ratatui::crossterm::event::{self, Event, KeyCode};
//!
//! fn main() -> anyhow::Result<()> {
//!     let mut terminal = ratatui::init();
//!     // The engine sends a hint on this channel when background work is ready.
//!     let (wake, woken) = mpsc::sync_channel(1);
//!     let input = Input::Patch("changes.patch".into());
//!     let mut engine = Engine::open(input, Options::default(), wake)?;
//!
//!     let result = (|| -> anyhow::Result<()> {
//!         loop {
//!             let _ = woken.try_recv();
//!             engine.poll()?;
//!             if engine.needs_draw() {
//!                 let mut drawn = Ok(());
//!                 terminal.draw(|frame| {
//!                     let area = frame.area();
//!                     drawn = engine.draw(frame, area);
//!                 })?;
//!                 drawn?;
//!                 engine.after_draw();
//!             }
//!             if event::poll(Duration::from_millis(16))? {
//!                 let event = event::read()?;
//!                 if matches!(&event, Event::Key(key) if key.code == KeyCode::Char('q')) {
//!                     return Ok(());
//!                 }
//!                 engine.handle_event(event, true)?;
//!             }
//!         }
//!     })();
//!
//!     // Joins the background workers and reports cleanup errors.
//!     let closed = engine.close();
//!     ratatui::restore();
//!     result.and(closed)
//! }
//! ```
//!
//! The [embedding guide](https://github.com/plannotator/diffs-rs/blob/main/docs/embedding.md)
//! covers the host loop, focus, source coordinates, reading tools, and shutdown.

pub mod app;
mod cleanup;
pub mod engine;
pub mod geometry;
pub mod highlight;
pub mod ingest;
pub mod input;
pub mod model;
pub mod projection;
pub mod reading;
pub mod render;
pub mod source;
mod source_spans;
pub mod storage;
pub mod text;
pub mod theme;
mod tree;
pub mod view;
mod wake;

#[cfg(feature = "regex-census")]
mod regex_census;

#[cfg(feature = "owner-census")]
pub mod owner_census;

#[cfg(feature = "delivery-trace")]
pub mod delivery_trace;

#[macro_export]
macro_rules! delivery_event {
    ($kind:literal, $a:expr, $b:expr) => {
        #[cfg(feature = "delivery-trace")]
        $crate::delivery_trace::record($kind, $a as u64, $b as u64);
    };
}
