use ratatui::buffer::Buffer;
use unicode_width::UnicodeWidthStr;

use super::{ellipsize, fill, label};
use crate::{app::App, tree::Kind};

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
        let bg = if focused { c.context } else { c.chrome };
        let y = area.y + 1 + offset as u16;
        fill(
            buf,
            ratatui::layout::Rect::new(area.x, y, area.width, 1),
            bg,
        );
        let (name, summary, suffix) = match &node.kind {
            Kind::Folder { expanded, .. } => (
                format!("{} {}/", if *expanded { "▾" } else { "▸" }, node.label),
                format!(
                    "{} +{} −{}{}",
                    node.totals.files,
                    node.totals.additions,
                    node.totals.deletions,
                    if node.totals.binary > 0 { " B" } else { "" }
                ),
                String::new(),
            ),
            Kind::File { id, occurrence } => {
                let file = &app.doc.files[id.0];
                let suffix = if *occurrence == 0 {
                    String::new()
                } else {
                    format!(" [{}]", occurrence)
                };
                let summary = if file.status.as_ref() == "Binary change" {
                    "B binary".into()
                } else {
                    format!(
                        "{} +{} −{}",
                        file.status.chars().next().unwrap_or('M'),
                        file.additions,
                        file.deletions
                    )
                };
                (node.label.clone(), summary, suffix)
            }
        };
        let summary_width = summary.width().min(16) as u16;
        let name_width = area.width - summary_width - 3;
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
            c.fg,
            bg,
            focused || current,
        );
        label(
            buf,
            area.right() - summary_width - 1,
            y,
            summary_width,
            &ellipsize(&summary, summary_width as usize),
            c.dim,
            bg,
            false,
        );
    }
}
