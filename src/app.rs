use anyhow::{Context, Result};

mod controls;
pub use controls::{Command, InputEffect};
mod settings;
mod sidebar;
mod source;
pub use sidebar::SidebarPreference;

use crate::{
    highlight::{Cache, Request, Worker},
    model::{Kind, Row, RowKind},
    render,
    storage::Document,
    theme::{self, Appearance, Colors},
    view::{LineId, View},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum Layout {
    #[default]
    Auto,
    Split,
    Unified,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub row: usize,
    pub part: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ReadingAnchor {
    source: LineId,
    display_byte: usize,
    old_side: bool,
    screen_row: Option<usize>,
    /// The cursor is on the "No newline at end of file" mark under `source`, not on its row.
    mark: bool,
}

pub struct App {
    pub(crate) view: View,
    pub(crate) sidebar: sidebar::Sidebar,
    pub doc: Document,
    pub loading: bool,
    pub sources: Option<Vec<crate::input::SourceState>>,
    pub(crate) source_reads: crate::reading::SourceLeases,
    pub label: String,
    pub layout: Layout,
    pub appearance: Appearance,
    pub colors: Colors,
    pub top: Position,
    pub cursor: Position,
    pub old_side: bool,
    pub horizontal: usize,
    pub wrap: bool,
    pub numbers: bool,
    /// Draws the label row, footer and help overlay. Hosts that own those rows turn it off.
    pub chrome: bool,
    pub help: bool,
    pub cache: Cache,
    pub geometry: crate::geometry::Cache,
    pub epoch: u64,
    pub width: u16,
    pub height: u16,
    pub visible: Vec<(Position, u16)>,
    pub cursor_screen_row: usize,
    pub worker: Worker,
    pub(crate) search_match: Option<crate::reading::SearchMatch>,
    pub(crate) selection: Option<crate::reading::LineSelection>,
    pub(crate) markers: crate::engine::markers::Markers,
    /// Draws the base surfaces and plain text in the terminal's own colors.
    pub(crate) terminal_background: bool,
    /// How a split row is shared between the old and the new side.
    pub(crate) split_ratio: crate::engine::SplitRatio,
    /// The theme's text color, which plain syntax tokens carry; drawn as the terminal's own
    /// foreground when `terminal_background` is on.
    pub(crate) plain: ratatui::style::Color,
    /// How changed text inside a replaced line is emphasized.
    pub(crate) inline_changes: crate::engine::InlineChanges,
    /// Whether whitespace-only changes are hidden.
    pub(crate) whitespace: crate::engine::Whitespace,
    /// A reflow scrolls sideways to keep the anchor's text in view, as the standalone viewer
    /// wants. Off, a reflow never moves the horizontal scroll.
    pub(crate) keep_text_in_view: bool,
    /// The top and the cursor as wrapping last went off, with the anchor it left, so turning
    /// it straight back on returns to them. Any reflow forgets them.
    unwrapped: Option<(Position, Position, ReadingAnchor)>,
    /// The files as the patch has them while `doc` hides whitespace, so the setting can be
    /// turned off again and later files checked against the input's order.
    patch: Option<Document>,
    /// Reads removed and added lines of arriving files while whitespace is hidden.
    scan: crate::storage::PatchScan,
    reading_anchor: Option<ReadingAnchor>,
}

impl App {
    pub fn new(
        doc: Document,
        label: String,
        layout: Layout,
        appearance: Appearance,
        wake: std::sync::mpsc::SyncSender<()>,
    ) -> anyhow::Result<Self> {
        let colors = theme::load(appearance)?.0;
        let plain = colors.fg;
        let worker = crate::highlight::start(wake);
        Ok(Self {
            view: View::default(),
            sidebar: sidebar::Sidebar::default(),
            doc,
            loading: true,
            sources: None,
            source_reads: crate::reading::SourceLeases::default(),
            label,
            layout,
            appearance,
            colors,
            worker,
            search_match: None,
            selection: None,
            markers: Default::default(),
            terminal_background: false,
            split_ratio: crate::engine::SplitRatio::EVEN,
            plain,
            inline_changes: crate::engine::InlineChanges::Word,
            whitespace: crate::engine::Whitespace::Show,
            keep_text_in_view: false,
            unwrapped: None,
            patch: None,
            scan: crate::storage::PatchScan::default(),
            top: Position::default(),
            cursor: Position::default(),
            old_side: false,
            horizontal: 0,
            wrap: false,
            numbers: true,
            chrome: true,
            help: false,
            cache: Cache::default(),
            geometry: crate::geometry::Cache::default(),
            epoch: 0,
            width: 160,
            height: 50,
            visible: Vec::new(),
            cursor_screen_row: 0,
            reading_anchor: None,
        })
    }

    pub fn split(&self) -> bool {
        self.layout == Layout::Split || (self.layout == Layout::Auto && self.width >= 120)
    }
    pub fn row(&self, index: usize) -> Option<Row<LineId>> {
        self.view.row(&self.doc, index, self.split())
    }
    pub fn row_len(&self) -> usize {
        self.view.len(&self.doc, self.split())
    }

    fn anchor(&self) -> Option<LineId> {
        let row = self.row(self.cursor.row)?;
        let kind = if matches!(row.kind, RowKind::NoNewline { .. }) {
            self.row(self.cursor.row.checked_sub(1)?)?.kind
        } else {
            row.kind
        };
        let RowKind::Code { left, right } = kind else {
            return None;
        };
        if !self.split() {
            // The line the row draws, as `render::drawn_lines` measures it.
            right.or(left)
        } else if self.old_side {
            left.or(right)
        } else {
            right.or(left)
        }
    }

    fn screen_row(&self) -> usize {
        self.visible
            .iter()
            .find(|(p, _)| *p == self.cursor)
            .map(|(_, y)| usize::from(*y))
            .or_else(|| {
                self.laid_out_row(self.cursor)
                    .ok()
                    .flatten()
                    .map(usize::from)
            })
            .unwrap_or(self.cursor_screen_row + 2)
    }

    /// The one-based screen row the next frame draws `position` on, counted as
    /// `render::draw_diff` lays rows out from the top: under the sticky heading when the top
    /// row is not a heading. `None` when it is off screen or the pane is too small to draw.
    ///
    /// Unlike `visible`, the rows of the last frame, it is right between frames: after the
    /// top moved, and after the list was cleared, however many moves came since the last draw.
    fn laid_out_row(&self, position: Position) -> Result<Option<u16>> {
        self.laid_out_from(self.top, position)
    }

    /// `laid_out_row` with `top` as the top row.
    fn laid_out_from(&self, top: Position, position: Position) -> Result<Option<u16>> {
        if position < top || self.width < 40 || self.height < 10 {
            return Ok(None);
        }
        let sticky = self
            .row(top.row)
            .is_some_and(|row| row.kind != RowKind::Header);
        let first = 1 + u16::from(sticky);
        let rows = self.height.saturating_sub(2 + u16::from(sticky));
        let mut p = top;
        for n in 0..rows {
            if p == position {
                return Ok(Some(first + n));
            }
            if p > position || self.row(p.row).is_none() {
                return Ok(None);
            }
            let next = self.next(p)?;
            if next == p {
                return Ok(None);
            }
            p = next;
        }
        Ok(None)
    }

    /// The lowest top row that leaves no screen row empty below the last row: the top of
    /// the last full page, or the first row when everything fits. A reflow and turning
    /// wrapping off keep the top no lower than this.
    fn last_page_top(&self) -> Result<Position> {
        let Some(row) = self.row_len().checked_sub(1) else {
            return Ok(Position::default());
        };
        let last = Position {
            row,
            part: self.row_height(row)?.saturating_sub(1),
        };
        let mut top = last;
        loop {
            let up = self.previous(top)?;
            if up == top || self.laid_out_from(up, last)?.is_none() {
                return Ok(top);
            }
            top = up;
        }
    }

    /// Whether no screen row is empty under the last row: the top is no lower than the last
    /// full page. A view scrolled past it, by the wheel or a jump to a short last file, is not.
    fn page_full(&self) -> Result<bool> {
        Ok(self.width < 40 || self.height < 10 || self.top <= self.last_page_top()?)
    }

    /// After a reflow or turning wrapping off, when the view was `full` before: raises the top
    /// to the last full page when it is lower, so a pane that grew or rows that shrank leave
    /// no empty rows under the last one. Everything from that top down is on screen, so the
    /// cursor, which was at or below the top, still is. The standalone viewer, which keeps
    /// the text in view, keeps the cursor's screen row as it always has.
    fn keep_page_full(&mut self, full: bool) -> Result<()> {
        if full && !self.keep_text_in_view && self.width >= 40 && self.height >= 10 {
            self.top = self.top.min(self.last_page_top()?);
        }
        Ok(())
    }

    fn remember_position(&mut self, clicked_column: Option<u16>) -> Result<()> {
        if self.reading_anchor.is_none()
            && let Some(source) = self.anchor()
        {
            let old_side = if self.split() {
                self.old_side
            } else {
                match self.line_info(source)?.kind {
                    Kind::Deletion => true,
                    Kind::Addition => false,
                    Kind::Context => self.old_side,
                }
            };
            self.reading_anchor = Some(ReadingAnchor {
                source,
                display_byte: render::source_anchor_byte(
                    self,
                    source,
                    self.cursor.part,
                    old_side,
                    clicked_column,
                )?,
                old_side,
                screen_row: Some(self.screen_row()),
                mark: self
                    .row(self.cursor.row)
                    .is_some_and(|row| matches!(row.kind, RowKind::NoNewline { .. })),
            });
        }
        Ok(())
    }

    pub fn reflow(&mut self, layout: Layout, width: u16, height: u16) -> Result<()> {
        self.reflow_from(layout, width, height, None, None)
    }

    /// `reflow`, keeping the cursor on `screen_row` when it has no anchor that says where it
    /// was, because the rows were just rebuilt, and taking `full` for whether the view had no
    /// empty rows under the last one before they were.
    fn reflow_from(
        &mut self,
        layout: Layout,
        width: u16,
        height: u16,
        screen_row: Option<usize>,
        full: Option<bool>,
    ) -> Result<()> {
        self.remember_position(None)?;
        let full = match full {
            Some(full) => full,
            None => self.page_full()?,
        };
        let anchor = self.reading_anchor;
        let screen_row = anchor
            .and_then(|anchor| anchor.screen_row)
            .or(screen_row)
            .unwrap_or_else(|| self.screen_row());
        let file = self.row(self.cursor.row).map(|r| r.file);
        let old_split = self.split();
        if width < 40 || height < 10 {
            return Ok(());
        }
        self.layout = layout;
        self.width = width;
        self.height = height;
        if let Some(anchor) = anchor {
            self.cursor.row = self
                .shown_row(anchor.source)
                .map(|row| {
                    // The mark stays on the mark, which follows its line in either layout.
                    let mark = row + 1;
                    if anchor.mark
                        && self
                            .row(mark)
                            .is_some_and(|row| matches!(row.kind, RowKind::NoNewline { .. }))
                    {
                        mark
                    } else {
                        row
                    }
                })
                .unwrap_or(self.cursor.row);
            self.old_side = anchor.old_side;
            let horizontal;
            (self.cursor.part, horizontal) = render::anchored_position(
                self,
                anchor.source,
                anchor.display_byte,
                anchor.old_side,
            )?;
            // Unless the host asked to keep the anchor's text in view, the horizontal scroll
            // stays as it is, lowered when drawn if the lines shown no longer reach it.
            if self.keep_text_in_view {
                self.horizontal = horizontal;
            }
        } else if old_split != self.split() {
            self.cursor.row = file
                .map(|f| self.view.file_row(&self.doc, f, self.split()))
                .unwrap_or(0);
        }
        self.cursor.part = self
            .cursor
            .part
            .min(self.row_height(self.cursor.row)?.saturating_sub(1));
        self.top = self.cursor;
        for _ in 0..screen_row
            .min(height.saturating_sub(2) as usize)
            .saturating_sub(1)
        {
            self.top = self.previous(self.top)?;
        }
        if self.top < self.cursor
            && self
                .row(self.top.row)
                .is_some_and(|row| row.kind != RowKind::Header)
        {
            self.top = self.next(self.top)?;
        }
        self.keep_page_full(full)?;
        // The screen row asked for, not the one the end of the diff left: a later reflow
        // with room below puts the cursor back on it.
        if let Some(anchor) = &mut self.reading_anchor {
            anchor.screen_row = Some(screen_row);
        }
        self.unwrapped = None;
        self.visible.clear();
        Ok(())
    }

    /// Turning wrapping on keeps the cursor's source line on its screen row, showing the part
    /// with the text the anchor points at. Turning it off keeps the top row at the top: every
    /// row becomes one screen row and the rows themselves stay, so the cursor stays on screen
    /// on its row, since no row above it got taller, though never lower than the last full
    /// page when the view had no empty rows under the last one before. Turning it straight back on, with
    /// nothing done in between, returns to the wrapped view it left. The horizontal scroll is
    /// kept either way.
    /// With `keep_text_in_view`, turning it off keeps the cursor's line on its screen row and
    /// scrolls sideways to the anchor's text, as any reflow then does.
    pub fn toggle_wrap(&mut self) -> Result<()> {
        self.remember_position(None)?;
        let full = self.page_full()?;
        self.wrap = !self.wrap;
        if self.wrap || self.keep_text_in_view || self.width < 40 || self.height < 10 {
            let unwrapped = self.unwrapped.take();
            self.reflow(self.layout, self.width, self.height)?;
            // Wrapping back on with nothing done since it went off: the wrapped view it left.
            if self.wrap
                && let Some((top, cursor, anchor)) = unwrapped
                && self.reading_anchor == Some(anchor)
                && top.part < self.row_height(top.row)?
                && cursor.part < self.row_height(cursor.row)?
                && cursor.row == self.cursor.row
            {
                (self.top, self.cursor) = (top, cursor);
                let screen_row = self.laid_out_row(self.cursor)?.map(usize::from);
                if let Some(anchor) = &mut self.reading_anchor {
                    anchor.screen_row = screen_row;
                }
            }
            return Ok(());
        }
        let wrapped = (self.top, self.cursor);
        self.cursor.part = 0;
        self.top = Position {
            row: self.top.row.min(self.cursor.row),
            part: 0,
        };
        self.keep_page_full(full)?;
        self.visible.clear();
        let screen_row = self.laid_out_row(self.cursor)?.map(usize::from);
        if let Some(anchor) = &mut self.reading_anchor {
            anchor.screen_row = screen_row;
        }
        self.unwrapped = self
            .reading_anchor
            .map(|anchor| (wrapped.0, wrapped.1, anchor));
        Ok(())
    }

    pub fn toggle_numbers(&mut self) -> Result<()> {
        self.remember_position(None)?;
        self.numbers = !self.numbers;
        self.reflow(self.layout, self.width, self.height)
    }

    /// Shares split rows between the sides by `ratio`. True when it changed. The source line
    /// under the cursor keeps its screen row when the sides drawn change width.
    pub(crate) fn set_split_ratio(&mut self, ratio: crate::engine::SplitRatio) -> Result<bool> {
        if ratio == self.split_ratio {
            return Ok(false);
        }
        let drawn = crate::engine::split::sides(self.width, self.split_ratio);
        if !self.split() || drawn == crate::engine::split::sides(self.width, ratio) {
            self.split_ratio = ratio;
            return Ok(true);
        }
        self.remember_position(None)?;
        self.split_ratio = ratio;
        self.reflow(self.layout, self.width, self.height)?;
        Ok(true)
    }

    pub fn move_horizontal(&mut self, forward: bool) -> Result<()> {
        let horizontal = if forward {
            self.horizontal
                .saturating_add(4)
                .min(render::horizontal_limit(self)?)
        } else {
            self.horizontal.saturating_sub(4)
        };
        if horizontal != self.horizontal {
            self.horizontal = horizontal;
            self.reading_anchor = None;
        }
        Ok(())
    }

    pub fn switch_side(&mut self) {
        self.selection = None;
        self.old_side = !self.old_side;
        self.reading_anchor = None;
    }

    pub fn select(&mut self, position: Position, column: u16) -> Result<()> {
        self.cursor = position;
        if self.split() {
            self.old_side = crate::engine::split::old_at(self.width, self.split_ratio, column);
        }
        self.reading_anchor = None;
        self.remember_position(Some(column))?;
        let old = self
            .source_at_cursor(None)?
            .map_or(self.old_side, |(_, old)| old);
        if self
            .selection
            .as_ref()
            .is_some_and(|selection| (selection.range.side == crate::engine::Side::Old) != old)
        {
            self.selection = None;
        }
        Ok(())
    }

    pub(crate) fn reveal_source(
        &mut self,
        source: LineId,
        display_byte: usize,
        old_side: bool,
    ) -> Result<()> {
        let row = self
            .view
            .source_row(&self.doc, source, self.split())
            .context("Search source is not displayed")?;
        let (part, horizontal) = render::anchored_position(self, source, display_byte, old_side)?;
        let cursor = Position { row, part };
        let mut top = self.top;
        if cursor < top {
            top = cursor;
        } else if self.laid_out_row(cursor)?.is_none() {
            top = cursor;
            for _ in 0..self.height.saturating_sub(4) {
                top = self.previous(top)?;
            }
        }
        if self
            .selection
            .as_ref()
            .is_some_and(|selection| (selection.range.side == crate::engine::Side::Old) != old_side)
        {
            self.selection = None;
        }
        self.cursor = cursor;
        self.top = top;
        self.horizontal = horizontal;
        self.old_side = old_side;
        self.reading_anchor = Some(ReadingAnchor {
            source,
            display_byte,
            old_side,
            screen_row: None,
            mark: false,
        });
        // Whether a later reveal moves the view is worked out from the top, not from this
        // list, so clearing it here never makes the next reveal scroll.
        self.visible.clear();
        Ok(())
    }

    pub fn row_height(&self, row: usize) -> Result<usize> {
        render::row_height(self, row)
    }

    pub fn next(&self, p: Position) -> Result<Position> {
        Ok(if p.part + 1 < self.row_height(p.row)? {
            Position {
                row: p.row,
                part: p.part + 1,
            }
        } else if p.row + 1 < self.row_len() {
            Position {
                row: p.row + 1,
                part: 0,
            }
        } else {
            p
        })
    }

    pub fn previous(&self, p: Position) -> Result<Position> {
        Ok(if p.part > 0 {
            Position {
                row: p.row,
                part: p.part - 1,
            }
        } else if p.row > 0 {
            Position {
                row: p.row - 1,
                part: self.row_height(p.row - 1)? - 1,
            }
        } else {
            p
        })
    }

    pub fn move_cursor(&mut self, amount: isize) -> Result<()> {
        let before = self.cursor;
        for _ in 0..amount.unsigned_abs() {
            self.cursor = if amount > 0 {
                self.next(self.cursor)?
            } else {
                self.previous(self.cursor)?
            };
        }
        if before != self.cursor {
            self.reading_anchor = None;
        }
        self.reveal()
    }

    pub fn scroll(&mut self, amount: isize) -> Result<()> {
        let before = self.cursor;
        for _ in 0..amount.unsigned_abs() {
            self.top = if amount > 0 {
                self.next(self.top)?
            } else {
                self.previous(self.top)?
            };
        }
        let sticky_header = self
            .row(self.top.row)
            .is_some_and(|row| row.kind != RowKind::Header);
        let visible_rows = self.height.saturating_sub(2 + u16::from(sticky_header));
        let mut bottom = self.top;
        for _ in 1..visible_rows {
            bottom = self.next(bottom)?;
        }
        self.cursor = self.cursor.clamp(self.top, bottom);
        if self.cursor != before {
            self.reading_anchor = None;
        } else if let Some(anchor) = &mut self.reading_anchor {
            anchor.screen_row = None;
        }
        Ok(())
    }

    pub fn reveal(&mut self) -> Result<()> {
        if self.cursor < self.top {
            self.top = self.cursor;
        } else if self.laid_out_row(self.cursor)?.is_none() {
            self.top = self.cursor;
            for _ in 0..self.height.saturating_sub(4) {
                self.top = self.previous(self.top)?;
            }
        }
        Ok(())
    }

    pub fn jump(&mut self, end: bool) -> Result<()> {
        let Some(last) = self.doc.source_len().checked_sub(1) else {
            return Ok(());
        };
        let patch = if end { last } else { 0 };
        let file = self.doc.file_for_source(patch).id;
        let mut id = LineId::Patch(patch);
        let mut row = self.shown_row(id).context("Patch endpoint is missing")?;
        let mut index = if end {
            if file + 1 < self.doc.files.len() {
                self.view.file_row(&self.doc, file + 1, self.split()) - 1
            } else {
                self.row_len() - 1
            }
        } else {
            self.view.file_row(&self.doc, file, self.split())
        };
        while if end { index > row } else { index < row } {
            if let Some(Row {
                kind: RowKind::Code { left, right },
                ..
            }) = self.row(index)
                && let Some(context @ LineId::Context { .. }) =
                    if end { right.or(left) } else { left.or(right) }
            {
                id = context;
                row = index;
                break;
            }
            if end {
                index -= 1;
            } else {
                index += 1;
            }
        }
        self.reading_anchor = None;
        self.cursor = Position { row, part: 0 };
        self.old_side = self.line_info(id)?.new.is_none();
        self.reveal()
    }

    pub(crate) fn expand_context(&mut self) -> Result<bool> {
        let Some((file, gap, _)) = self.view.gap(&self.doc, self.cursor.row, self.split()) else {
            return Ok(false);
        };
        let screen_row = self.screen_row();
        let source = self.view.expand(&self.doc, file, gap)?;
        self.reading_anchor = Some(ReadingAnchor {
            source,
            display_byte: 0,
            old_side: self.old_side,
            screen_row: Some(screen_row),
            mark: false,
        });
        self.reflow(self.layout, self.width, self.height)?;
        Ok(true)
    }

    pub fn jump_section(&mut self, forward: bool, file: bool) {
        let target = if file {
            self.row(self.cursor.row)
                .and_then(|row| {
                    if forward {
                        row.file.checked_add(1)
                    } else {
                        row.file.checked_sub(1)
                    }
                })
                .and_then(|index| self.doc.files.get(index))
                .map(|file| self.view.file_row(&self.doc, file.id, self.split()))
        } else {
            self.view
                .boundary(&self.doc, self.cursor.row, self.split(), forward)
        };
        if let Some(row) = target {
            self.reading_anchor = None;
            self.cursor = Position { row, part: 0 };
            self.top = self.cursor;
        }
    }

    pub fn toggle_theme(&mut self) -> anyhow::Result<()> {
        self.appearance = self.appearance.toggle();
        self.load_colors()?;
        self.epoch += 1;
        self.cache = Cache::default();
        Ok(())
    }

    /// Draws the base surfaces and plain text in the terminal's own colors, or in the theme's.
    pub(crate) fn set_terminal_background(&mut self, on: bool) -> Result<()> {
        self.terminal_background = on;
        self.load_colors()
    }

    fn load_colors(&mut self) -> Result<()> {
        let theme = theme::load(self.appearance)?.0;
        self.plain = theme.fg;
        self.colors = if self.terminal_background {
            theme::on_terminal(self.appearance)?
        } else {
            theme
        };
        Ok(())
    }

    pub fn request_highlights(&self) {
        let mut ids = Vec::new();
        let mut files = Vec::new();
        for (p, _) in &self.visible {
            if let Some(Row {
                kind: RowKind::Code { left, right },
                ..
            }) = self.row(p.row)
            {
                for id in left.into_iter().chain(right) {
                    if self.cache.get(id).is_none() {
                        ids.push(id);
                        let file = self.line_file(id);
                        if !files
                            .iter()
                            .any(|f: &crate::highlight::FileSource| f.file().id == file.id)
                        {
                            files.push(self.coloring_source(file));
                        }
                    }
                }
            }
        }
        #[cfg(feature = "delivery-trace")]
        {
            let cached = self
                .visible
                .iter()
                .filter_map(|(p, _)| self.row(p.row))
                .filter_map(|row| match row.kind {
                    RowKind::Code { left, right } => Some(left.into_iter().chain(right)),
                    _ => None,
                })
                .flatten()
                .filter(|id| self.cache.get(*id).is_some())
                .count();
            crate::delivery_event!("viewport_cache_occurrences", cached, ids.len());
        }
        if ids.is_empty() {
            self.worker.cancel();
        } else {
            self.worker.request(Request {
                epoch: self.epoch,
                appearance: self.appearance,
                ids,
                files,
            });
        }
    }

    pub(crate) fn protect_visible_styles(&mut self) {
        let ids = self
            .visible
            .iter()
            .filter_map(|(position, _)| self.row(position.row))
            .flat_map(|row| match row.kind {
                RowKind::Code { left, right } => [left, right],
                _ => [None, None],
            })
            .flatten()
            .collect();
        self.cache.protect_visible(ids);
    }
}
