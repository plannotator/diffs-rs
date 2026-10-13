use std::fmt::{self, Write};

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

mod offsets;
pub(crate) use offsets::source_offsets;

pub fn safe_text(bytes: &[u8]) -> String {
    let mut result = String::new();
    append_bytes(&mut result, bytes, true).expect("Writing to a String cannot fail");
    result
}

fn append_bytes(out: &mut impl Write, bytes: &[u8], complete: bool) -> Result<usize, fmt::Error> {
    let mut rest = bytes;
    while !rest.is_empty() {
        match std::str::from_utf8(rest) {
            Ok(text) => {
                append_safe(out, text)?;
                return Ok(bytes.len());
            }
            Err(error) => {
                let n = error.valid_up_to();
                append_safe(
                    out,
                    std::str::from_utf8(&rest[..n]).expect("valid UTF-8 prefix"),
                )?;
                if !complete && error.error_len().is_none() {
                    return Ok(bytes.len() - rest.len() + n);
                }
                let end = n + error.error_len().unwrap_or(rest.len() - n);
                for b in &rest[n..end] {
                    write!(out, "\\x{b:02x}")?;
                }
                rest = &rest[end..];
            }
        }
    }
    Ok(bytes.len())
}

fn append_safe(out: &mut impl Write, text: &str) -> fmt::Result {
    for c in text.chars() {
        if c == '\n' || c == '\t' {
            out.write_char(c)?;
        } else if c.is_control() || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}') {
            write!(out, "\\u{{{:x}}}", c as u32)?;
        } else {
            out.write_char(c)?;
        }
    }
    Ok(())
}

pub fn display_text(bytes: &[u8]) -> String {
    let bytes = if let Some(line) = bytes.strip_suffix(b"\n") {
        line.strip_suffix(b"\r").unwrap_or(line)
    } else {
        bytes
    };
    safe_text(bytes)
}

pub fn safe_label(bytes: &[u8]) -> String {
    safe_text(bytes).replace('\n', "\\n").replace('\t', "\\t")
}

pub struct Glyph<'a> {
    pub text: &'a str,
    pub byte: usize,
    pub column: usize,
    pub width: usize,
}

pub fn glyphs(text: &str, tab_width: usize) -> impl Iterator<Item = Glyph<'_>> {
    glyphs_from(text, tab_width, 0, 0)
}

pub fn glyphs_from(
    text: &str,
    tab_width: usize,
    start_byte: usize,
    mut column: usize,
) -> impl Iterator<Item = Glyph<'_>> {
    text[start_byte..]
        .grapheme_indices(true)
        .map(move |(byte, text)| {
            let width = if text == "\t" {
                tab_width - column % tab_width
            } else {
                text.width().max(1)
            };
            let glyph = Glyph {
                text,
                byte: start_byte + byte,
                column,
                width,
            };
            column += width;
            glyph
        })
}
