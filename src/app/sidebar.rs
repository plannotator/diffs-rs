use ratatui::layout::Rect;

use super::{App, Position};
use crate::tree::{FileId, Kind, NodeId, Tree};

const TREE_WIDTH: u16 = 32;
const MIN_WIDTH: u16 = TREE_WIDTH + 1 + 64;

#[derive(Default, Clone, Copy)]
pub enum SidebarPreference {
    #[default]
    Auto,
    Show,
    Hide,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Focus {
    #[default]
    Diff,
    Files,
}

#[derive(Default)]
pub(crate) struct Sidebar {
    pub tree: Tree,
    pub preference: SidebarPreference,
    pub focus: Focus,
    pub cursor: Option<NodeId>,
    pub top: usize,
    pub current: Option<FileId>,
    pub screen: Rect,
    pub area: Rect,
    pub diff: Rect,
    pub notice: Option<&'static str>,
}

impl Sidebar {
    pub fn focused(&self) -> bool {
        self.focus == Focus::Files
    }
    pub fn page(&self) -> usize {
        usize::from(self.area.height.saturating_sub(1)).max(1)
    }

    pub fn keep_visible(&mut self) {
        let page = self.page();
        if let Some(position) = self.cursor.and_then(|id| self.tree.position(id)) {
            self.top = self
                .top
                .min(position)
                .max(position.saturating_sub(page - 1));
        }
        self.top = self.top.min(self.tree.rows().len().saturating_sub(page));
    }

    pub fn move_cursor(&mut self, amount: isize) {
        let position = self
            .cursor
            .and_then(|id| self.tree.position(id))
            .unwrap_or(0);
        let target = position
            .saturating_add_signed(amount)
            .min(self.tree.rows().len().saturating_sub(1));
        self.cursor = self.tree.rows().get(target).map(|r| r.node);
        self.keep_visible();
    }

    pub fn scroll(&mut self, amount: isize) {
        let page = self.page();
        self.top = self
            .top
            .saturating_add_signed(amount)
            .min(self.tree.rows().len().saturating_sub(page));
        if let Some(position) = self.cursor.and_then(|id| self.tree.position(id)) {
            let target = position.clamp(self.top, self.top + page - 1);
            self.cursor = self.tree.rows().get(target).map(|r| r.node);
        }
    }

    pub fn toggle(&mut self, id: NodeId) {
        if let Kind::Folder { expanded, .. } = self.tree.node(id).kind {
            self.tree.set_expanded(id, !expanded);
            self.tree.refresh();
            self.keep_visible();
        }
    }

    pub fn right(&mut self) {
        let Some(id) = self.cursor else {
            return;
        };
        if let Kind::Folder { expanded, .. } = self.tree.node(id).kind {
            if expanded {
                self.move_cursor(1);
            } else {
                self.toggle(id);
            }
        }
    }

    pub fn left(&mut self) {
        let Some(id) = self.cursor else {
            return;
        };
        if matches!(self.tree.node(id).kind, Kind::Folder { expanded: true, .. }) {
            self.toggle(id);
        } else if let Some(parent) = self.tree.parent(id) {
            self.cursor = Some(parent);
            self.keep_visible();
        }
    }
}

impl App {
    pub fn set_sidebar_preference(&mut self, preference: SidebarPreference) {
        self.sidebar.preference = preference;
    }

    pub(crate) fn prepare_view(&mut self, screen: Rect) -> anyhow::Result<()> {
        for file in &self.doc.files[self.sidebar.tree.len()..] {
            self.sidebar.tree.append(file)?;
        }
        let current = self.row(self.cursor.row).map(|row| FileId(row.file));
        if current != self.sidebar.current {
            self.sidebar.current = current;
            if self.sidebar.focus == Focus::Diff
                && let Some(node) = current.and_then(|id| self.sidebar.tree.file_node(id))
            {
                self.sidebar.tree.reveal(node);
                self.sidebar.cursor = Some(node);
            }
        }
        self.sidebar.tree.refresh();
        if self.sidebar.cursor.is_none() {
            self.sidebar.cursor = self.sidebar.tree.rows().first().map(|r| r.node);
        }
        let wants_tree = match self.sidebar.preference {
            SidebarPreference::Auto => self.doc.files.len() >= 2,
            SidebarPreference::Show => true,
            SidebarPreference::Hide => false,
        };
        let shown = wants_tree && screen.width >= MIN_WIDTH && screen.height >= 10;
        if shown {
            self.sidebar.notice = None;
        }
        self.sidebar.screen = screen;
        self.sidebar.area = if shown {
            Rect::new(screen.x, screen.y + 1, TREE_WIDTH, screen.height - 2)
        } else {
            Rect::default()
        };
        let inset = if shown { TREE_WIDTH + 1 } else { 0 };
        self.sidebar.diff = if screen.width >= 40 && screen.height >= 10 {
            Rect::new(
                screen.x + inset,
                screen.y + 1,
                screen.width - inset,
                screen.height - 2,
            )
        } else {
            Rect::default()
        };
        if !shown {
            self.sidebar.focus = Focus::Diff;
        }
        self.sidebar.keep_visible();
        if self.sidebar.diff.is_empty() {
            self.visible.clear();
            return Ok(());
        }
        let changed = self.width != self.sidebar.diff.width || self.height != screen.height;
        if changed {
            self.reflow(self.layout, self.sidebar.diff.width, screen.height)?;
        }
        Ok(())
    }

    pub(crate) fn files_command(&mut self) -> anyhow::Result<()> {
        if self.sidebar.focused() {
            self.sidebar.preference = SidebarPreference::Hide;
            self.sidebar.focus = Focus::Diff;
        } else {
            self.sidebar.preference = SidebarPreference::Show;
            if self.sidebar.screen.width < MIN_WIDTH || self.sidebar.screen.height < 10 {
                self.sidebar.notice = Some("Files need at least 97 columns × 10 rows");
            } else {
                self.sidebar.focus = Focus::Files;
                if let Some(id) = self
                    .row(self.cursor.row)
                    .and_then(|r| self.sidebar.tree.file_node(FileId(r.file)))
                {
                    self.sidebar.tree.reveal(id);
                    self.sidebar.cursor = Some(id);
                }
            }
        }
        self.prepare_view(self.sidebar.screen)?;
        Ok(())
    }

    pub(crate) fn activate_tree(&mut self) -> bool {
        let Some(id) = self.sidebar.cursor else {
            return false;
        };
        if let Some(file) = self.sidebar.tree.node(id).file() {
            self.cursor = Position {
                row: self.view.file_row(&self.doc, file.0, self.split()),
                part: 0,
            };
            self.top = self.cursor;
            self.reading_anchor = None;
            self.sidebar.focus = Focus::Diff;
            true
        } else {
            self.sidebar.toggle(id);
            false
        }
    }
}
