use ratatui::{buffer::Buffer, style::Color};
use unicode_width::UnicodeWidthStr;

use super::{Summary, color_summary, digits, ellipsize, fill, label, status_color};
use crate::{app::App, engine::BadgeColor, theme::Colors, tree::Kind};

/// The cells a host badge column takes at the right of each row: a gap, then two cells.
const BADGE_COLUMN: u16 = 3;

pub(super) const HELP: &str = "  ↑/↓, j/k     Move cursor; PgUp/PgDn page; Home/End bounds\n  ←/→          Collapse / expand folders; parent / child\n  Enter        Open file or toggle folder; Space toggles\n  Esc, Tab     Return to diff; f hides Files\n  Wheel        Scroll the pane under the pointer\n  *            Current diff file; > keyboard cursor\n  Esc, ?       Close help\n  q, Ctrl-C    Quit / cancel";

pub(super) fn draw(buf: &mut Buffer, app: &App) {
    let state = &app.sidebar;
    let area = state.area;
    if area.is_empty() {
        return;
    }
    let c = &app.colors;
    fill(buf, area, c.chrome);
    let heading = format!(
        " Files{} · {}{}",
        if state.focused() { " [focus]" } else { "" },
        state.tree.len(),
        if app.loading { " partial" } else { "" }
    );
    label(
        buf, area.x, area.y, area.width, &heading, c.fg, c.chrome, true,
    );
    for y in area.y..area.bottom() {
        label(buf, area.right(), y, 1, "│", c.dim, c.chrome, false);
    }
    if state.tree.len() == 0 {
        label(
            buf,
            area.x + 1,
            area.y + 1,
            area.width - 2,
            "No completed files",
            c.dim,
            c.chrome,
            false,
        );
    }
    for (offset, row) in state
        .tree
        .rows()
        .iter()
        .skip(state.top)
        .take(state.page())
        .enumerate()
    {
        let node = state.tree.node(row.node);
        let focused = state.focused() && state.cursor == Some(row.node);
        let current = node.file().is_some_and(|id| Some(id) == state.current);
        // On the terminal's background the context color is the terminal's own, so the
        // keyboard cursor row takes the hunk header band instead.
        let highlight = if app.terminal_background {
            c.separator
        } else {
            c.context
        };
        let bg = if focused { highlight } else { c.chrome };
        let y = area.y + 1 + offset as u16;
        fill(
            buf,
            ratatui::layout::Rect::new(area.x, y, area.width, 1),
            bg,
        );
        let (name, summary, colors, suffix) = match &node.kind {
            Kind::Folder { expanded, .. } => (
                format!("{} {}/", if *expanded { "▾" } else { "▸" }, node.label),
                format!(
                    "{} +{} −{}{}",
                    node.totals.files,
                    node.totals.additions,
                    node.totals.deletions,
                    if node.totals.binary > 0 { " B" } else { "" }
                ),
                Some(Summary {
                    at: 0,
                    lead: digits(node.totals.files),
                    lead_fg: c.dim,
                    gap: 1,
                    additions: node.totals.additions,
                    deletions: node.totals.deletions,
                }),
                String::new(),
            ),
            Kind::File { id, occurrence } => {
                let file = &app.doc.files[id.0];
                let suffix = if *occurrence == 0 {
                    String::new()
                } else {
                    format!(" [{}]", occurrence)
                };
                let (summary, colors) = if file.status.as_ref() == "Binary change" {
                    ("B binary".into(), None)
                } else {
                    (
                        format!(
                            "{} +{} −{}",
                            file.status.chars().next().unwrap_or('M'),
                            file.additions,
                            file.deletions
                        ),
                        Some(Summary {
                            at: 0,
                            lead: 1,
                            lead_fg: status_color(c, &file.status),
                            gap: 1,
                            additions: file.additions,
                            deletions: file.deletions,
                        }),
                    )
                };
                (node.label.clone(), summary, colors, suffix)
            }
        };
        // With host marks, a fixed badge column sits between the summary and the divider.
        let badge_column = if state.marks.column { BADGE_COLUMN } else { 0 };
        let mark = node.mark;
        let summary_width = summary.width().min(16) as u16;
        let name_width = area.width - summary_width - 3 - badge_column;
        let depth = row.depth.saturating_mul(2);
        let indent = depth.min(6);
        let prefix = format!(
            "{}{}{}{}",
            if focused { ">" } else { " " },
            if current { "*" } else { " " },
            " ".repeat(indent),
            if depth > indent { "…" } else { "" }
        );
        let text = format!(
            "{}{}",
            ellipsize(
                &format!("{prefix}{name}"),
                (name_width as usize).saturating_sub(suffix.width())
            ),
            suffix
        );
        label(
            buf,
            area.x,
            y,
            name_width,
            &text,
            if mark.dimmed { c.dim } else { c.fg },
            bg,
            focused || current,
        );
        let summary_x = area.right() - summary_width - 1 - badge_column;
        label(
            buf,
            summary_x,
            y,
            summary_width,
            &ellipsize(&summary, summary_width as usize),
            c.dim,
            bg,
            false,
        );
        if let Some((text, color)) = mark.badge.and_then(|id| state.marks.badge(id)) {
            let x = area.right() - BADGE_COLUMN;
            label(buf, x, y, 2, text, badge_color(c, color), bg, false);
        }
        if let Some(colors) = colors.filter(|_| !mark.dimmed) {
            // A truncated summary ends in a muted ellipsis.
            let visible = if summary.width() > usize::from(summary_width) {
                summary_width.saturating_sub(1)
            } else {
                summary_width
            };
            color_summary(buf, summary_x, y, visible, colors, c);
        }
    }
}

fn badge_color(c: &Colors, color: BadgeColor) -> Color {
    match color {
        BadgeColor::Text => c.fg,
        BadgeColor::Muted => c.dim,
        BadgeColor::Accent => c.accent,
        BadgeColor::Added => c.added,
        BadgeColor::Deleted => c.deleted,
        BadgeColor::Modified => c.modified,
        BadgeColor::Renamed => c.renamed,
    }
}
