use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

mod sidebar;

use crate::{
    app::{App, Position},
    model::{Kind, RowKind},
    theme::Colors,
    view::LineId,
};

fn code_width(app: &App, file: usize, pane: usize) -> usize {
    let digits = if app.numbers {
        app.line_digits(file)
    } else {
        0
    };
    pane.saturating_sub(if app.split() {
        digits + 3
    } else {
        digits * 2 + usize::from(app.numbers) + 3
    })
    .max(1)
}

/// The widths of a split row's old and new side, the line between them not counted.
fn pane_widths(app: &App) -> (usize, usize) {
    crate::engine::split::sides(app.width, app.split_ratio)
}

fn source_pane(app: &App, id: LineId, old: bool) -> (usize, usize) {
    let (left, right) = pane_widths(app);
    let (x, width) = if !app.split() {
        (0, app.width as usize)
    } else if old {
        (0, left)
    } else {
        (left + 1, right)
    };
    let file = app.line_file(id).id;
    let available = code_width(app, file, width);
    (x + width - available, available)
}

pub fn source_anchor_byte(
    app: &App,
    id: LineId,
    part: usize,
    old: bool,
    clicked_column: Option<u16>,
) -> anyhow::Result<usize> {
    let (code_start, width) = source_pane(app, id, old);
    let line = app.line_geometry(id, app.wrap.then_some(width))?;
    let offset = if app.wrap {
        line.start_column(part).unwrap_or(usize::MAX)
    } else {
        app.horizontal
    };
    let column = offset.saturating_add(clicked_column.map_or(0, |x| {
        (x as usize).saturating_sub(code_start).min(width - 1)
    }));
    Ok(line.byte_at_column(column))
}

pub(crate) fn source_has_part(
    app: &App,
    id: LineId,
    part: usize,
    old: bool,
) -> anyhow::Result<bool> {
    let (_, width) = source_pane(app, id, old);
    Ok(part < app.line_geometry(id, app.wrap.then_some(width))?.height())
}

/// The part and the horizontal scroll that show `byte` of `id`: the scroll is kept when the
/// byte is already in view, and otherwise starts at it.
pub fn anchored_position(
    app: &App,
    id: LineId,
    byte: usize,
    old: bool,
) -> anyhow::Result<(usize, usize)> {
    let (_, width) = source_pane(app, id, old);
    let line = app.line_geometry(id, app.wrap.then_some(width))?;
    let (column, glyph_width) = line.column_at_byte(byte);
    if app.wrap {
        Ok((line.part_for_column(column), app.horizontal))
    } else {
        let offset = if column < app.horizontal
            || column.saturating_add(glyph_width) > app.horizontal.saturating_add(width)
        {
            column
        } else {
            app.horizontal
        };
        Ok((0, offset))
    }
}

/// The lines a code row with `left` and `right` draws, each with the width of its pane. Split,
/// both sides. Unified, the one line it shows: the added line of a pair drawn as unchanged
/// because whitespace is hidden, and otherwise the row's only line. Every measure of a row,
/// its height, how far it scrolls and where the cursor is on it, goes by these lines.
pub(crate) fn drawn_lines(
    app: &App,
    left: Option<LineId>,
    right: Option<LineId>,
) -> [(Option<LineId>, usize); 2] {
    if app.split() {
        let (left_width, right_width) = pane_widths(app);
        [(left, left_width), (right, right_width)]
    } else {
        [(right.or(left), app.width as usize), (None, 0)]
    }
}

pub fn row_height(app: &App, index: usize) -> anyhow::Result<usize> {
    if !app.wrap {
        return Ok(1);
    }
    let Some(row) = app.row(index) else {
        return Ok(1);
    };
    let RowKind::Code { left, right } = row.kind else {
        return Ok(1);
    };
    let mut height = 1;
    for (id, pane) in drawn_lines(app, left, right) {
        if let Some(id) = id {
            height = height.max(
                app.line_geometry(id, Some(code_width(app, row.file, pane)))?
                    .height(),
            );
        }
    }
    Ok(height)
}

pub(crate) fn horizontal_limit(app: &App) -> anyhow::Result<usize> {
    let sticky_header = app
        .row(app.top.row)
        .is_some_and(|row| row.kind != RowKind::Header);
    let rows = app.height.saturating_sub(2 + u16::from(sticky_header));
    let end = app.top.row.saturating_add(rows as usize).min(app.row_len());
    let mut limit = 0;
    for index in app.top.row..end {
        let Some(row) = app.row(index) else {
            break;
        };
        let RowKind::Code { left, right } = row.kind else {
            continue;
        };
        for (id, width) in drawn_lines(app, left, right) {
            if let Some(id) = id {
                let line = app.line_geometry(id, None)?;
                limit = limit.max(
                    line.columns()
                        .saturating_sub(code_width(app, row.file, width)),
                );
            }
        }
    }
    Ok(limit)
}

pub fn draw(frame: &mut Frame<'_>, app: &mut App, input_error: Option<&str>) -> anyhow::Result<()> {
    draw_in(frame, app, frame.area(), input_error)
}

pub fn draw_in(
    frame: &mut Frame<'_>,
    app: &mut App,
    area: Rect,
    input_error: Option<&str>,
) -> anyhow::Result<()> {
    draw_with_controls(frame, app, area, input_error, None)
}

pub(crate) fn draw_with_controls(
    frame: &mut Frame<'_>,
    app: &mut App,
    area: Rect,
    input_error: Option<&str>,
    controls: Option<&crate::engine::ReadingControls<'_>>,
) -> anyhow::Result<()> {
    let area = area.intersection(frame.area());
    app.prepare_view(area)?;
    let colors = app.colors.clone();
    frame.render_widget(
        Block::new().style(Style::new().bg(colors.bg).fg(colors.fg)),
        area,
    );
    app.visible.clear();
    if area.width < 40 || app.layout_height(area.height) < 10 {
        app.protect_visible_styles();
        let text = if app.chrome {
            "Enlarge terminal to at least 40 columns × 10 rows\nq quit"
        } else {
            "Enlarge pane to at least 40 columns × 8 rows"
        };
        frame.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: true })
                .style(Style::new().fg(colors.fg)),
            area,
        );
        return Ok(());
    }
    if !app.wrap && app.horizontal != 0 {
        app.horizontal = app.horizontal.min(horizontal_limit(app)?);
    }
    if app.chrome {
        title(frame.buffer_mut(), app, area);
    }
    let buf = frame.buffer_mut();
    sidebar::draw(buf, app);
    draw_diff(buf, app, app.sidebar.diff)?;
    app.protect_visible_styles();
    if let Some(index) = app.visible.iter().position(|(p, _)| *p == app.cursor) {
        app.cursor_screen_row = index;
    }
    if app.chrome {
        footer(frame, app, area, input_error, controls);
    }
    Ok(())
}

/// The label row: the review label on the left, file count, layout and theme on the right.
fn title(buf: &mut Buffer, app: &App, area: Rect) {
    let colors = &app.colors;
    let layout = if app.split() { "SPLIT" } else { "UNIFIED" };
    let title = format!(" diffs  /  {}", app.label);
    let right = format!(
        "{} files  ·  {layout}  ·  {} ",
        app.doc.files.len(),
        app.appearance.name()
    );
    let right_width = right.width().min(area.width as usize) as u16;
    // Keep a visible gap so a long label never runs into the file count.
    let title_width = area.width.saturating_sub(right_width + 2);
    fill(buf, Rect::new(area.x, area.y, area.width, 1), colors.chrome);
    label(
        buf,
        area.x,
        area.y,
        title_width,
        &ellipsize(&title, title_width.into()),
        colors.fg,
        colors.chrome,
        false,
    );
    label(
        buf,
        area.right().saturating_sub(right_width),
        area.y,
        right_width,
        &right,
        colors.dim,
        colors.chrome,
        false,
    );
}

/// The footer status and key hints, and the help overlay when it is open.
fn footer(
    frame: &mut Frame<'_>,
    app: &App,
    area: Rect,
    input_error: Option<&str>,
    controls: Option<&crate::engine::ReadingControls<'_>>,
) {
    let colors = &app.colors;
    let buf = frame.buffer_mut();
    let mut pending = false;
    let mut fallback = false;
    for (p, _) in &app.visible {
        if let Some(crate::model::Row {
            kind: RowKind::Code { left, right },
            ..
        }) = app.row(p.row)
        {
            for id in left.into_iter().chain(right) {
                pending |= app.cache.get(id).is_none();
                fallback |= app.cache.get(id).is_some_and(|s| s.fallback);
            }
        }
    }
    let failure = input_error.map(|error| format!("INCOMPLETE · {error}"));
    let source_status = app.source_status();
    let status = if let Some(failure) = &failure {
        failure.as_str()
    } else if app.loading && pending {
        "Loading patch… · Highlighting visible code…"
    } else if app.loading {
        "Loading patch… · completed files available"
    } else if let Some(status) = &source_status {
        &status.text
    } else if pending {
        "Highlighting visible code…"
    } else if fallback {
        "Plain styling fallback"
    } else {
        app.sidebar
            .notice
            .unwrap_or("Patch view · syntax context may be incomplete")
    };
    let status = match controls.and_then(|controls| controls.status) {
        Some(notice) if failure.is_none() && !app.loading => {
            if source_status
                .as_ref()
                .is_some_and(|status| status.attention)
                || pending
                || fallback
            {
                std::borrow::Cow::Owned(format!("{status} · {notice}"))
            } else {
                std::borrow::Cow::Borrowed(notice)
            }
        }
        _ => std::borrow::Cow::Borrowed(status),
    };
    let hint = if failure.is_some() || app.sidebar.notice.is_some() || app.loading {
        " q quit "
    } else if app.sidebar.focused() && area.width < 100 {
        " ↑↓ move  Enter open  f hide  ? help  q quit "
    } else if app.sidebar.focused() {
        " ↑↓ move  ←→ folders  Enter open  Esc diff  f hide  ? help  q quit "
    } else if area.width < 100 {
        " f files  ? help  q quit "
    } else {
        " f files  u layout  w wrap  t theme  ? help  q quit "
    };
    let hint = if controls.is_some() && area.width < 60 {
        " ? help  q quit "
    } else {
        hint
    };
    let hint = controls.map_or_else(
        || hint.to_owned(),
        |controls| {
            if controls.status.is_some() {
                if area.width < 60 {
                    " ?".into()
                } else if app.sidebar.focused() {
                    hint.to_owned()
                } else {
                    " ? help  q quit ".into()
                }
            } else {
                format!("{}  {} ", hint.trim_end(), controls.hint)
            }
        },
    );
    let hint = if failure.is_none() && !app.loading && app.sidebar.notice.is_none() {
        footer_hint(hint, &status, area.width, app.sidebar.focused(), controls)
    } else {
        hint
    };
    let hint_width = hint.width().min(area.width as usize) as u16;
    let status_width = area.width.saturating_sub(hint_width + 1);
    fill(
        buf,
        Rect::new(area.x, area.bottom() - 1, area.width, 1),
        colors.chrome,
    );
    label(
        buf,
        area.x + 1,
        area.bottom() - 1,
        status_width,
        &ellipsize(&status, usize::from(status_width.saturating_sub(1))),
        colors.dim,
        colors.chrome,
        false,
    );
    label(
        buf,
        area.right() - hint_width,
        area.bottom() - 1,
        hint_width,
        &hint,
        colors.dim,
        colors.chrome,
        false,
    );
    if app.help {
        let help = if app.sidebar.focused() {
            sidebar::HELP
        } else {
            "\n  DIFF\n  j/k, ↑/↓       Move one display row\n  PgUp/PgDn      Move one page\n  g/G            First / last source line\n  n/N            Next / previous file (patch order)\n  [ / ]          Previous / next hunk\n  ←/→            Horizontal scroll\n  Tab            Switch active side\n  u / w          Layout / wrap\n  t / l          Theme / line numbers\n  Enter / click  Reveal up to 20 lines at a context gap\n  f              Show and focus Files\n  FILES\n  ↑/↓, j/k       Move; PgUp/PgDn page; Home/End bounds\n  ←/→            Collapse / expand folders\n  Enter          Open file or toggle folder; Space toggle\n  Esc, Tab / f   Return to diff / hide Files\n  *              Current diff file; > keyboard cursor\n  Wheel          Scroll the pane under the pointer\n  Esc, ?         Close help; q, Ctrl-C quit / cancel"
        };
        let mut help = help.to_owned();
        if !app.sidebar.focused() {
            if let Some(controls) = controls {
                help = format!("{}\n{help}", controls.help);
            }
            if help.lines().count() > usize::from(area.height.saturating_sub(2)) {
                help = " j/k ↑↓ move; PgUp/PgDn page\n g/G first/last; n/N next/prev file\n ←/→ scroll; Tab switch side\n u layout; w wrap; t theme\n f files; Enter reveal context\n".into();
                if let Some(controls) = controls {
                    help.push(' ');
                    help.push_str(controls.hint);
                    help.push('\n');
                }
                help.push_str(" Esc, ? close help\n q, Ctrl-C quit / cancel");
            }
        }
        let height = help
            .lines()
            .count()
            .saturating_add(2)
            .min(area.height as usize) as u16;
        let rect = Rect::new(
            area.x + (area.width.saturating_sub(68)) / 2,
            area.y + (area.height.saturating_sub(height)) / 2,
            68.min(area.width),
            height,
        );
        frame.render_widget(Clear, rect);
        frame.render_widget(
            Paragraph::new(help)
                .block(
                    Block::new()
                        .title(" Reading controls ")
                        .borders(Borders::ALL),
                )
                .style(Style::new().bg(colors.chrome).fg(colors.fg)),
            rect,
        );
    }
}

/// Shortens the footer hint until the status fits beside it.
///
/// The status describes the current reading state, so the engine's optional hints give way
/// first: the view toggles, then the Files hint. `? help  q quit` and caller hints remain;
/// callers already choose their hint for the available width. A status that still does not
/// fit is truncated by the caller.
fn footer_hint(
    hint: String,
    status: &str,
    width: u16,
    sidebar_focused: bool,
    controls: Option<&crate::engine::ReadingControls<'_>>,
) -> String {
    if sidebar_focused || controls.is_some_and(|controls| controls.status.is_some()) {
        return hint;
    }
    // One leading cell, the status, and at least one cell before the hint.
    let fits = |hint: &str| status.width() + hint.width() + 2 <= usize::from(width);
    if fits(&hint) {
        return hint;
    }
    let with_controls = |base: &str| match controls {
        Some(controls) => format!("{}  {} ", base.trim_end(), controls.hint),
        None => base.to_owned(),
    };
    let shortest = with_controls(" ? help  q quit ");
    [with_controls(" f files  ? help  q quit ")]
        .into_iter()
        .find(|candidate| candidate.width() < hint.width() && fits(candidate))
        .unwrap_or(if shortest.width() < hint.width() {
            shortest
        } else {
            hint
        })
}

/// Truncates `text` to `width` display cells, marking the cut with an ellipsis.
fn ellipsize(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.into();
    }
    let mut result = String::new();
    let mut used = 0;
    for grapheme in text.graphemes(true) {
        let next = grapheme.width();
        if used + next >= width {
            break;
        }
        result.push_str(grapheme);
        used += next;
    }
    if width > 0 {
        result.push('…');
    }
    result
}

fn draw_diff(buf: &mut Buffer, app: &mut App, area: Rect) -> anyhow::Result<()> {
    let mut y = area.y;
    if let Some(row) = app.row(app.top.row)
        && row.kind != RowKind::Header
    {
        file_header(buf, app, row.file, area.x, y);
        y += 1;
    }
    let mut p = app.top;
    while y < area.bottom() {
        let Some(row) = app.row(p.row) else {
            break;
        };
        draw_row(buf, app, p, (area.x, y), row.file, row.kind)?;
        app.visible.push((p, y - area.y + 1));
        y += 1;
        let next = app.next(p)?;
        if next == p {
            break;
        }
        p = next;
    }
    if app.doc.files.is_empty() {
        label(
            buf,
            area.x + 2,
            area.y + 2,
            area.width - 4,
            if app.loading {
                "Loading patch…"
            } else {
                "No changes"
            },
            app.colors.dim,
            app.colors.bg,
            false,
        );
    }
    Ok(())
}

fn file_header(buf: &mut Buffer, app: &App, file: usize, x: u16, y: u16) {
    let f = &app.doc.files[file];
    let c = &app.colors;
    fill(buf, Rect::new(x, y, app.width, 1), c.chrome);
    let path = if f.old_path != f.path {
        format!(" {} → {}", f.old_path, f.path)
    } else {
        format!(" {}", f.path)
    };
    let summary = format!(" {}   +{} −{} ", f.status, f.additions, f.deletions);
    let summary_width = summary.width().min(app.width as usize) as u16;
    let summary_x = x + app.width.saturating_sub(summary_width);
    label(
        buf,
        x,
        y,
        app.width.saturating_sub(summary_width),
        &path,
        c.fg,
        c.chrome,
        true,
    );
    label(
        buf,
        summary_x,
        y,
        summary_width,
        &summary,
        c.dim,
        c.chrome,
        false,
    );
    color_summary(
        buf,
        summary_x,
        y,
        summary_width,
        Summary {
            at: 1,
            lead: f.status.width() as u16,
            lead_fg: status_color(c, &f.status),
            gap: 3,
            additions: f.additions,
            deletions: f.deletions,
        },
        c,
    );
}

fn draw_row(
    buf: &mut Buffer,
    app: &App,
    p: Position,
    origin: (u16, u16),
    file: usize,
    kind: RowKind<LineId>,
) -> anyhow::Result<()> {
    let c = &app.colors;
    let (x, y) = origin;
    match kind {
        RowKind::Header => file_header(buf, app, file, x, y),
        RowKind::Hunk(h) => {
            fill(buf, Rect::new(x, y, app.width, 1), c.separator);
            let header = &app.doc.files[file].hunks[h].header;
            let hidden;
            label(
                buf,
                x + 1,
                y,
                app.width - 2,
                if app.doc.files[file].hunk_hidden(h) {
                    hidden = format!("{header}  ·  whitespace only");
                    &hidden
                } else {
                    header
                },
                c.dim,
                c.separator,
                false,
            );
        }
        RowKind::Gap => {
            let text = if let Some((_, _, count)) = app.view.gap(&app.doc, p.row, app.split()) {
                format!("{count} unchanged lines · Show {}", count.min(20))
            } else {
                "···  Context not included in patch".into()
            };
            label(
                buf,
                x + 2,
                y,
                app.width - 4,
                &text,
                if p == app.cursor { c.accent } else { c.dim },
                c.bg,
                false,
            );
        }
        RowKind::Metadata => {
            let f = &app.doc.files[file];
            let text = format!(
                " {}  ·  mode {} → {}  ·  no textual hunks",
                f.status, f.old_mode, f.new_mode
            );
            label(buf, x + 1, y, app.width - 2, &text, c.dim, c.bg, false);
        }
        RowKind::Code { left, right } => {
            if app.split() {
                let (lw, rw) = pane_widths(app);
                code(
                    buf,
                    app,
                    CodeCell {
                        x,
                        y,
                        width: lw,
                        file,
                        id: left,
                        other: None,
                        part: p.part,
                        active: p == app.cursor && app.old_side,
                        old: true,
                    },
                )?;
                label(buf, x + lw as u16, y, 1, "│", c.separator, c.bg, false);
                code(
                    buf,
                    app,
                    CodeCell {
                        x: x + lw as u16 + 1,
                        y,
                        width: rw,
                        file,
                        id: right,
                        other: None,
                        part: p.part,
                        active: p == app.cursor && !app.old_side,
                        old: false,
                    },
                )?;
            } else {
                code(
                    buf,
                    app,
                    CodeCell {
                        x,
                        y,
                        width: app.width as usize,
                        file,
                        id: right.or(left),
                        other: left,
                        part: p.part,
                        active: p == app.cursor,
                        old: right.is_none(),
                    },
                )?;
            }
        }
        RowKind::NoNewline { left, right } => {
            let note = " No newline at end of file";
            if app.split() {
                let (lw, rw) = pane_widths(app);
                if left {
                    label(buf, x, y, lw as u16, note, c.dim, c.bg, false);
                }
                if right {
                    label(
                        buf,
                        x + lw as u16 + 1,
                        y,
                        rw as u16,
                        note,
                        c.dim,
                        c.bg,
                        false,
                    );
                }
            } else {
                label(buf, x + 1, y, app.width - 2, note, c.dim, c.bg, false);
            }
        }
    }
    Ok(())
}

struct CodeCell {
    x: u16,
    y: u16,
    width: usize,
    file: usize,
    id: Option<LineId>,
    other: Option<LineId>,
    part: usize,
    active: bool,
    old: bool,
}

fn code(buf: &mut Buffer, app: &App, cell: CodeCell) -> anyhow::Result<()> {
    let CodeCell {
        x,
        y,
        width,
        file,
        id,
        other,
        part,
        active,
        old,
    } = cell;
    let c = &app.colors;
    let Some(id) = id else {
        fill(buf, Rect::new(x, y, width as u16, 1), c.empty);
        return Ok(());
    };
    let source = app.line_info(id)?;
    let other_info = other.map(|id| app.line_info(id)).transpose()?;
    // A unified row shows the old number of the line on its left, which is a removed line
    // paired with this added one when whitespace is hidden.
    let old_number = if app.split() {
        source.old
    } else {
        other_info.and_then(|i| i.old).or(source.old)
    };
    let selected = app.selection.as_ref().is_some_and(|selection| {
        let range = selection.range;
        let range_old = range.side == crate::engine::Side::Old;
        let number = if range_old { old_number } else { source.new };
        range.file == file
            && (!app.split() || range_old == old)
            && number.is_some_and(|line| range.first <= line && line <= range.last)
    });
    // A unified row shows both of its numbers, so either side's marker applies to it.
    let marker = if app.split() {
        let side = if old {
            crate::engine::Side::Old
        } else {
            crate::engine::Side::New
        };
        app.markers
            .style(file, side, if old { source.old } else { source.new })
    } else {
        app.markers
            .style(file, crate::engine::Side::New, source.new)
            .or_else(|| {
                app.markers
                    .style(file, crate::engine::Side::Old, old_number)
            })
    };
    // The cursor and selection indicators take precedence over a marker in the first cell.
    let indicator = if active {
        "▎"
    } else if selected {
        "┃"
    } else if marker.is_some() {
        crate::engine::markers::SYMBOL
    } else {
        " "
    };
    // A line that differs from its pair only in whitespace is drawn as unchanged.
    let kind = match id {
        LineId::Patch(id)
            if app.whitespace != crate::engine::Whitespace::Show
                && app.doc.file_for_source(id).whitespace_only(id) =>
        {
            Kind::Context
        }
        _ => source.kind,
    };
    let (bg, accent, word, sign) = match kind {
        Kind::Context => (c.context, c.dim, c.context, " "),
        Kind::Addition => (c.add_bg, c.added, c.add_word, "+"),
        Kind::Deletion => (c.del_bg, c.deleted, c.del_word, "−"),
    };
    fill(buf, Rect::new(x, y, width as u16, 1), bg);
    let digits = if app.numbers {
        app.line_digits(file)
    } else {
        0
    };
    let number = |n: Option<u32>| {
        if digits == 0 {
            String::new()
        } else {
            n.map(|n| format!("{n:>digits$}"))
                .unwrap_or_else(|| " ".repeat(digits))
        }
    };
    let gutter = if app.split() {
        format!(
            "{indicator}{}{} ",
            number(if part == 0 {
                if old { source.old } else { source.new }
            } else {
                None
            }),
            if part == 0 { sign } else { "↪" }
        )
    } else {
        format!(
            "{indicator}{}{}{}{} ",
            number(if part == 0 { old_number } else { None }),
            if digits > 0 { " " } else { "" },
            number(if part == 0 { source.new } else { None }),
            if part == 0 { sign } else { "↪" }
        )
    };
    let gutter_width = width.saturating_sub(code_width(app, file, width));
    label(
        buf,
        x,
        y,
        gutter_width as u16,
        &gutter,
        if active { c.accent } else { accent },
        bg,
        false,
    );
    if selected {
        buf[(x, y)]
            .set_fg(c.accent)
            .set_style(Style::new().add_modifier(Modifier::BOLD));
    } else if let Some(style) = marker.filter(|_| !active) {
        buf[(x, y)].set_style(style);
    }
    let available = code_width(app, file, width);
    let line = app.line_geometry(id, app.wrap.then_some(available))?;
    let offset = if app.wrap {
        let Some(offset) = line.start_column(part) else {
            fill(buf, Rect::new(x, y, width as u16, 1), c.empty);
            return Ok(());
        };
        offset
    } else {
        app.horizontal
    };
    let style = app.cache.get(id);
    let current_match = app.search_match.as_ref().filter(|found| {
        found.hit.source == id
            && found.check(found.location().review).is_ok()
            && (!app.split() || (found.location().side == crate::engine::Side::Old) == old)
    });
    for glyph in line.glyphs_from_column(offset) {
        if glyph.column < offset {
            continue;
        }
        let pos = glyph.column - offset;
        if pos + glyph.width > available {
            break;
        }
        let fg = style
            .and_then(|s| {
                let tokens = s.tokens_for(old);
                tokens
                    .get(tokens.partition_point(|t| t.range.end <= glyph.byte))
                    .filter(|t| t.range.contains(&glyph.byte))
            })
            .map(|t| t.color)
            // Plain text carries the theme's text color; on the terminal's colors it is `c.fg`.
            .filter(|color| !(app.terminal_background && *color == app.plain))
            .unwrap_or(c.fg);
        let emphasized = style.is_some_and(|s| {
            s.changed
                .get(s.changed.partition_point(|range| range.end <= glyph.byte))
                .is_some_and(|range| range.start < glyph.byte + glyph.text.len())
        });
        let matched = current_match.is_some_and(|found| {
            found.display.start < glyph.byte + glyph.text.len() && glyph.byte < found.display.end
        });
        let mut style = Style::new().fg(fg).bg(if emphasized { word } else { bg });
        if selected {
            style = style.add_modifier(Modifier::UNDERLINED);
        }
        if matched {
            style = style.add_modifier(Modifier::UNDERLINED | Modifier::BOLD);
        }
        let col = x + gutter_width as u16 + pos as u16;
        if glyph.text == "\t" {
            for n in 0..glyph.width {
                buf[(col + n as u16, y)].set_symbol(" ").set_style(style);
            }
        } else {
            buf.set_stringn(col, y, glyph.text, glyph.width, style);
        }
    }
    Ok(())
}

fn fill(buf: &mut Buffer, rect: Rect, bg: Color) {
    for y in rect.top()..rect.bottom() {
        for x in rect.left()..rect.right() {
            buf[(x, y)].reset();
            buf[(x, y)].set_bg(bg);
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Small drawing primitive with explicit cell geometry and colors"
)]
fn label(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    width: u16,
    text: &str,
    fg: Color,
    bg: Color,
    bold: bool,
) {
    let style = Style::new().fg(fg).bg(bg).add_modifier(if bold {
        Modifier::BOLD
    } else {
        Modifier::empty()
    });
    buf.set_stringn(x, y, text, width as usize, style);
}

/// The color of a file status in the Files tree and file headers.
fn status_color(c: &Colors, status: &str) -> Color {
    match status {
        "Added" => c.added,
        "Deleted" => c.deleted,
        "Renamed" | "Copied" => c.renamed,
        "Binary change" => c.dim,
        _ => c.modified,
    }
}

/// The parts of a `STATUS +A −D` change summary that `color_summary` colors.
struct Summary {
    /// The first cell of the status or file count.
    at: u16,
    /// The width of the status or file count.
    lead: u16,
    lead_fg: Color,
    /// The cells between the status and `+A`.
    gap: u16,
    additions: usize,
    deletions: usize,
}

/// Colors a change summary already drawn as muted text from `x`: the status in its color,
/// `+A` in the addition color and `−D` in the deletion color. A zero count stays muted, as do
/// cells from `visible` on.
fn color_summary(buf: &mut Buffer, x: u16, y: u16, visible: u16, s: Summary, c: &Colors) {
    let count = |n: usize, color: Color| if n == 0 { c.dim } else { color };
    let plus = s.at + s.lead + s.gap;
    let minus = plus + 1 + digits(s.additions) + 1;
    let spans = [
        (s.at, s.at + s.lead, s.lead_fg),
        (plus, minus - 1, count(s.additions, c.added)),
        (
            minus,
            minus + 1 + digits(s.deletions),
            count(s.deletions, c.deleted),
        ),
    ];
    for (start, end, fg) in spans {
        for cell in start..end.min(visible) {
            buf[(x + cell, y)].set_fg(fg);
        }
    }
}

/// The number of decimal digits in `n`.
fn digits(n: usize) -> u16 {
    n.checked_ilog10().map_or(1, |log| log as u16 + 1)
}

#[cfg(test)]
mod tests;
