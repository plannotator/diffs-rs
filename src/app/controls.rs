use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEventKind};

use super::{App, Layout, sidebar::Focus};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEffect {
    Ignored,
    Redraw,
    Highlight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Down,
    Up,
    PageDown,
    PageUp,
    FirstSource,
    LastSource,
    NextFile,
    PreviousFile,
    NextHunk,
    PreviousHunk,
    Left,
    Right,
    SwitchSide,
    ToggleLayout,
    ToggleWrap,
    ToggleNumbers,
    ToggleTheme,
    Files,
    Help,
    Back,
    Activate,
    ToggleFolder,
    TreeStart,
    TreeEnd,
    SelectLines,
    ClearSelection,
}

impl Command {
    pub fn from_key(key: KeyEvent) -> Option<Self> {
        if key.kind == KeyEventKind::Release {
            return None;
        }
        Some(match key.code {
            KeyCode::Down | KeyCode::Char('j') => Self::Down,
            KeyCode::Up | KeyCode::Char('k') => Self::Up,
            KeyCode::PageDown => Self::PageDown,
            KeyCode::PageUp => Self::PageUp,
            KeyCode::Char('g') => Self::FirstSource,
            KeyCode::Char('G') => Self::LastSource,
            KeyCode::Char('n') => Self::NextFile,
            KeyCode::Char('N') => Self::PreviousFile,
            KeyCode::Char(']') => Self::NextHunk,
            KeyCode::Char('[') => Self::PreviousHunk,
            KeyCode::Left => Self::Left,
            KeyCode::Right => Self::Right,
            KeyCode::Tab => Self::SwitchSide,
            KeyCode::Char('u') => Self::ToggleLayout,
            KeyCode::Char('w') => Self::ToggleWrap,
            KeyCode::Char('l') => Self::ToggleNumbers,
            KeyCode::Char('t') => Self::ToggleTheme,
            KeyCode::Char('f') => Self::Files,
            KeyCode::Char('?') => Self::Help,
            KeyCode::Esc => Self::Back,
            KeyCode::Enter => Self::Activate,
            KeyCode::Char(' ') => Self::ToggleFolder,
            KeyCode::Home => Self::TreeStart,
            KeyCode::End => Self::TreeEnd,
            _ => return None,
        })
    }
}

impl App {
    pub fn handle_event(&mut self, event: Event) -> anyhow::Result<InputEffect> {
        let mut highlight = true;
        match event {
            Event::Resize(w, h) => {
                self.prepare_view(ratatui::layout::Rect::new(0, 0, w, h))?;
            }
            Event::Key(key) => {
                return Command::from_key(key)
                    .map_or(Ok(InputEffect::Ignored), |command| self.execute(command));
            }
            Event::Mouse(mouse) if !self.help => {
                let point = ratatui::layout::Position::new(mouse.column, mouse.row);
                if self.sidebar.area.contains(point) {
                    highlight = false;
                    match mouse.kind {
                        MouseEventKind::ScrollDown => self.sidebar.scroll(3),
                        MouseEventKind::ScrollUp => self.sidebar.scroll(-3),
                        MouseEventKind::Down(MouseButton::Left) => {
                            self.sidebar.focus = Focus::Files;
                            if mouse.row > self.sidebar.area.y {
                                let index = self.sidebar.top
                                    + usize::from(mouse.row - self.sidebar.area.y - 1);
                                if let Some(row) = self.sidebar.tree.rows().get(index) {
                                    self.sidebar.cursor = Some(row.node);
                                    highlight = self.activate_tree();
                                }
                            }
                        }
                        _ => return Ok(InputEffect::Ignored),
                    }
                } else if self.sidebar.diff.contains(point) {
                    match mouse.kind {
                        MouseEventKind::ScrollDown => self.scroll(3)?,
                        MouseEventKind::ScrollUp => self.scroll(-3)?,
                        MouseEventKind::Down(MouseButton::Left) => {
                            self.sidebar.focus = Focus::Diff;
                            let y = mouse.row - self.sidebar.diff.y + 1;
                            if let Some((position, _)) =
                                self.visible.iter().find(|(_, row)| *row == y)
                            {
                                self.select(*position, mouse.column - self.sidebar.diff.x)?;
                                self.expand_context()?;
                            }
                        }
                        _ => return Ok(InputEffect::Ignored),
                    }
                } else {
                    return Ok(InputEffect::Ignored);
                }
            }
            _ => return Ok(InputEffect::Ignored),
        }
        Ok(if highlight {
            InputEffect::Highlight
        } else {
            InputEffect::Redraw
        })
    }

    pub fn execute(&mut self, command: Command) -> anyhow::Result<InputEffect> {
        self.sidebar.notice = None;
        let mut highlight = true;
        match command {
            Command::Help => {
                self.help = !self.help;
                highlight = false;
            }
            Command::Back if self.help => {
                self.help = false;
                highlight = false;
            }
            _ if self.help => return Ok(InputEffect::Ignored),
            Command::Files => self.files_command()?,
            command if self.sidebar.focused() => {
                highlight = false;
                match command {
                    Command::Back | Command::SwitchSide => self.sidebar.focus = Focus::Diff,
                    Command::Down => self.sidebar.move_cursor(1),
                    Command::Up => self.sidebar.move_cursor(-1),
                    Command::PageDown => self.sidebar.move_cursor(self.sidebar.page() as isize),
                    Command::PageUp => self.sidebar.move_cursor(-(self.sidebar.page() as isize)),
                    Command::TreeStart => self.sidebar.move_cursor(isize::MIN),
                    Command::TreeEnd => self.sidebar.move_cursor(isize::MAX),
                    Command::Right => self.sidebar.right(),
                    Command::Left => self.sidebar.left(),
                    Command::Activate => highlight = self.activate_tree(),
                    Command::ToggleFolder => {
                        if let Some(id) = self.sidebar.cursor {
                            self.sidebar.toggle(id);
                        }
                    }
                    _ => return Ok(InputEffect::Ignored),
                }
            }
            _ if self.sidebar.diff.is_empty() => return Ok(InputEffect::Ignored),
            Command::Down => self.move_cursor(1)?,
            Command::Up => self.move_cursor(-1)?,
            Command::PageDown => self.move_cursor(self.height.saturating_sub(4).max(1) as isize)?,
            Command::PageUp => {
                self.move_cursor(-(self.height.saturating_sub(4).max(1) as isize))?
            }
            Command::FirstSource => self.jump(false)?,
            Command::LastSource => self.jump(true)?,
            Command::NextHunk => self.jump_section(true, false),
            Command::PreviousHunk => self.jump_section(false, false),
            Command::NextFile => self.jump_section(true, true),
            Command::PreviousFile => self.jump_section(false, true),
            Command::Right if !self.wrap => self.move_horizontal(true)?,
            Command::Left if !self.wrap => self.move_horizontal(false)?,
            Command::SwitchSide => self.switch_side(),
            Command::ToggleLayout => {
                let layout = if self.split() {
                    Layout::Unified
                } else {
                    Layout::Split
                };
                self.reflow(layout, self.width, self.height)?;
            }
            Command::ToggleWrap => self.toggle_wrap()?,
            Command::ToggleNumbers => self.toggle_numbers()?,
            Command::ToggleTheme => self.toggle_theme()?,
            Command::Activate => highlight = self.expand_context()?,
            _ => return Ok(InputEffect::Ignored),
        }
        Ok(if highlight {
            InputEffect::Highlight
        } else {
            InputEffect::Redraw
        })
    }
}
