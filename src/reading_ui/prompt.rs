use crossterm::event::{Event, KeyCode, KeyModifiers};
use diffs_rs::{
    reading::SourceRead,
    text::{glyphs, safe_label},
};
use ratatui::{Frame, layout::Rect, widgets::Paragraph};
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Default)]
pub(super) struct Prompt {
    pub(super) mode: Mode,
    input: String,
    cursor: usize,
    error: Option<String>,
}

#[derive(Default)]
pub(super) enum Mode {
    #[default]
    Search,
    Save(Arc<SourceRead>),
}

pub(super) enum Action {
    Edit,
    Submit,
    Cancel,
    Unchanged,
}

impl Prompt {
    pub(super) fn save(read: Arc<SourceRead>, path: String) -> Self {
        Self {
            mode: Mode::Save(read),
            cursor: path.len(),
            input: path,
            error: None,
        }
    }

    pub(super) fn text(&self) -> &str {
        &self.input
    }

    pub(super) fn set_error(&mut self, error: String) {
        self.error = Some(safe_label(error.as_bytes()));
    }

    fn insert(&mut self, text: &str) {
        let label = if matches!(self.mode, Mode::Save(_)) {
            "Save path"
        } else {
            "Search"
        };
        if text.contains(['\r', '\n']) {
            self.error = Some(format!("{label} accepts one line; paste was not inserted"));
        } else if matches!(self.mode, Mode::Save(_)) && text.contains('\0') {
            self.error = Some("Save path cannot contain NUL; paste was not inserted".into());
        } else if self.input.len().saturating_add(text.len()) > 4096 {
            self.error = Some(format!("{label} accepts at most 4096 UTF-8 bytes"));
        } else {
            self.input.insert_str(self.cursor, text);
            self.cursor = self.boundary_at_or_after(self.cursor + text.len());
            self.error = None;
        }
    }

    fn previous(&self) -> usize {
        self.input
            .grapheme_indices(true)
            .take_while(|(at, _)| *at < self.cursor)
            .last()
            .map_or(0, |(at, _)| at)
    }

    fn next(&self) -> usize {
        self.input
            .grapheme_indices(true)
            .find(|(at, _)| *at > self.cursor)
            .map_or(self.input.len(), |(at, _)| at)
    }

    fn boundary_at_or_after(&self, byte: usize) -> usize {
        self.input
            .grapheme_indices(true)
            .find(|(at, _)| *at >= byte)
            .map_or(self.input.len(), |(at, _)| at)
    }

    pub(super) fn handle(&mut self, event: &Event) -> Action {
        match event {
            Event::Paste(text) => self.insert(text),
            Event::Key(key) => match key.code {
                KeyCode::Esc => return Action::Cancel,
                KeyCode::Enter => return Action::Submit,
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.input.clear();
                    self.cursor = 0;
                    self.error = None;
                }
                KeyCode::Char(c)
                    if !key.modifiers.intersects(
                        KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
                    ) =>
                {
                    self.insert(&c.to_string())
                }
                KeyCode::Tab => self.insert("\t"),
                KeyCode::Left => self.cursor = self.previous(),
                KeyCode::Right => self.cursor = self.next(),
                KeyCode::Home => self.cursor = 0,
                KeyCode::End => self.cursor = self.input.len(),
                KeyCode::Backspace => {
                    let at = self.previous();
                    self.input.drain(at..self.cursor);
                    self.cursor = self.boundary_at_or_after(at);
                    self.error = None;
                }
                KeyCode::Delete => {
                    let end = self.next();
                    self.input.drain(self.cursor..end);
                    self.cursor = self.boundary_at_or_after(self.cursor);
                    self.error = None;
                }
                _ => return Action::Unchanged,
            },
            _ => return Action::Unchanged,
        }
        Action::Edit
    }

    pub(super) fn draw(&self, frame: &mut Frame<'_>) {
        let area = frame.area();
        if area.width < 12 || area.height == 0 {
            return;
        }
        let y = area.bottom() - 1;
        let style = frame.buffer_mut()[(area.x, y)].style();
        let saving = matches!(self.mode, Mode::Save(_));
        let prefix = if saving { "/save: " } else { "/diff: " };
        let hint = if area.width >= 70 {
            if saving {
                " Enter save new file · Esc cancel "
            } else {
                " Enter search · Esc cancel "
            }
        } else {
            ""
        };
        let available = area.width as usize - prefix.len() - hint.chars().count();
        let display = safe_label(self.input.as_bytes());
        let prefix_text = safe_label(&self.input.as_bytes()[..self.cursor]);
        let column = glyphs(&prefix_text, 4)
            .last()
            .map_or(0, |g| g.column + g.width);
        let start = column.saturating_sub(available.saturating_sub(1));
        let first = glyphs(&display, 4).find(|g| g.column >= start);
        let (byte, offset) = first.map_or((display.len(), column), |g| (g.byte, g.column));
        let input_area = Rect::new(area.x + prefix.len() as u16, y, available as u16, 1);
        frame.render_widget(
            Paragraph::new(" ".repeat(area.width as usize)).style(style),
            Rect::new(area.x, y, area.width, 1),
        );
        frame.render_widget(
            Paragraph::new(prefix).style(style),
            Rect::new(area.x, y, prefix.len() as u16, 1),
        );
        frame.render_widget(Paragraph::new(&display[byte..]).style(style), input_area);
        frame.render_widget(
            Paragraph::new(hint).style(style),
            Rect::new(input_area.right(), y, area.right() - input_area.right(), 1),
        );
        frame.set_cursor_position((
            input_area.x + column.saturating_sub(offset).min(available - 1) as u16,
            y,
        ));
        if let Some(error) = &self.error
            && area.height > 1
        {
            frame.render_widget(
                Paragraph::new(" ".repeat(area.width as usize)).style(style),
                Rect::new(area.x, y - 1, area.width, 1),
            );
            frame.render_widget(
                Paragraph::new(format!(" {error}")).style(style),
                Rect::new(area.x, y - 1, area.width, 1),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;

    fn key(prompt: &mut Prompt, code: KeyCode) {
        prompt.handle(&Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
    }

    #[test]
    fn joining_edits_keep_the_cursor_on_whole_input_grapheme_boundaries() {
        let mut prompt = Prompt::default();
        prompt.insert("👩💻!");
        key(&mut prompt, KeyCode::Home);
        key(&mut prompt, KeyCode::Right);
        prompt.insert("\u{200d}");
        assert_eq!(prompt.text(), "👩‍💻!");
        assert_eq!(prompt.cursor, "👩‍💻".len());
        key(&mut prompt, KeyCode::Left);
        assert_eq!(prompt.cursor, 0);
        key(&mut prompt, KeyCode::Right);
        assert_eq!(prompt.cursor, "👩‍💻".len());
        key(&mut prompt, KeyCode::Backspace);
        assert_eq!(prompt.text(), "!");
        assert_eq!(prompt.cursor, 0);

        for deletion in [KeyCode::Backspace, KeyCode::Delete] {
            let mut prompt = Prompt::default();
            prompt.insert("e\t\u{301}!");
            key(&mut prompt, KeyCode::Home);
            key(&mut prompt, KeyCode::Right);
            if deletion == KeyCode::Backspace {
                key(&mut prompt, KeyCode::Right);
            }
            key(&mut prompt, deletion);
            assert_eq!(prompt.text(), "e\u{301}!");
            assert_eq!(prompt.cursor, "e\u{301}".len());
            key(&mut prompt, KeyCode::Left);
            key(&mut prompt, KeyCode::Delete);
            assert_eq!(prompt.text(), "!");
            assert_eq!(prompt.cursor, 0);
        }
    }
}
