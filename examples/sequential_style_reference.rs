use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    path::Path,
};

use anyhow::{Context, Result, ensure};
use native_diff::theme::{self, Appearance};
use serde::{Deserialize, Serialize};
use syntect::{easy::HighlightLines, highlighting::Theme, parsing::SyntaxReference};

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum AppearanceInput {
    Dark,
    Light,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    appearance: AppearanceInput,
    hunks: Vec<Hunk>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Hunk {
    id: String,
    path: String,
    lines: Vec<SourceLine>,
    wanted: Vec<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceLine {
    id: usize,
    text: String,
    old: bool,
    new: bool,
}

#[derive(Serialize)]
struct Token {
    start: usize,
    end: usize,
    rgb: [u8; 3],
}

#[derive(Serialize)]
struct StyledSide {
    tokens: Vec<Token>,
    fallback: bool,
}

#[derive(Default, Serialize)]
struct Counts {
    seen: usize,
    parsed: usize,
    cutoffs: usize,
    parse_errors: usize,
    skipped_after_fallback: usize,
}

#[derive(Serialize)]
struct StyledLine {
    id: usize,
    prefix_records: usize,
    old: Option<StyledSide>,
    new: Option<StyledSide>,
    fallback: bool,
}

#[derive(Serialize)]
struct StyledHunk {
    id: String,
    path: String,
    syntax: String,
    through_source_id: Option<usize>,
    prefix_records: usize,
    old_counts: Counts,
    new_counts: Counts,
    lines: Vec<StyledLine>,
}

#[derive(Serialize)]
struct Response {
    schema_version: u8,
    appearance: AppearanceInput,
    offset_unit: &'static str,
    palette: BTreeMap<&'static str, [u8; 3]>,
    hunks: Vec<StyledHunk>,
}

struct Side<'a> {
    highlighter: HighlightLines<'a>,
    valid: bool,
    counts: Counts,
}

impl<'a> Side<'a> {
    fn new(syntax: &'a SyntaxReference, theme: &'a Theme) -> Self {
        Self {
            highlighter: HighlightLines::new(syntax, theme),
            valid: true,
            counts: Counts::default(),
        }
    }

    fn advance(
        &mut self,
        text: &str,
        syntaxes: &syntect::parsing::SyntaxSet,
        too_long: bool,
        wanted: bool,
    ) -> StyledSide {
        self.counts.seen += 1;
        let mut tokens = Vec::new();
        if too_long {
            self.counts.cutoffs += 1;
            self.valid = false;
        } else if !self.valid {
            self.counts.skipped_after_fallback += 1;
        } else {
            match self.highlighter.highlight_line(text, syntaxes) {
                Ok(fragments) => {
                    self.counts.parsed += 1;
                    if wanted {
                        let mut byte = 0;
                        for (style, fragment) in fragments {
                            let start = byte;
                            byte += fragment.len();
                            tokens.push(Token {
                                start,
                                end: byte,
                                rgb: [style.foreground.r, style.foreground.g, style.foreground.b],
                            });
                        }
                    }
                }
                Err(_) => {
                    self.counts.parse_errors += 1;
                    self.valid = false;
                }
            }
        }
        StyledSide {
            tokens,
            fallback: !self.valid,
        }
    }
}

fn run(request: Request) -> Result<Response> {
    let syntaxes = two_face::syntax::extra_newlines();
    let appearance = match request.appearance {
        AppearanceInput::Dark => Appearance::Dark,
        AppearanceInput::Light => Appearance::Light,
    };
    let (colors, theme) = theme::load(appearance)?;
    let palette = [
        ("bg", colors.bg),
        ("fg", colors.fg),
        ("dim", colors.dim),
        ("chrome", colors.chrome),
        ("separator", colors.separator),
        ("empty", colors.empty),
        ("context", colors.context),
        ("added", colors.added),
        ("deleted", colors.deleted),
        ("accent", colors.accent),
        ("add_bg", colors.add_bg),
        ("del_bg", colors.del_bg),
        ("add_word", colors.add_word),
        ("del_word", colors.del_word),
    ]
    .into_iter()
    .map(|(name, color)| match color {
        ratatui::style::Color::Rgb(r, g, b) => Ok((name, [r, g, b])),
        _ => anyhow::bail!("palette color {name} is not RGB"),
    })
    .collect::<Result<_>>()?;
    let mut hunk_ids = BTreeSet::new();
    let mut output = Vec::with_capacity(request.hunks.len());
    for hunk in request.hunks {
        ensure!(
            hunk_ids.insert(hunk.id.clone()),
            "duplicate hunk ID {}",
            hunk.id
        );
        let mut source_ids = BTreeSet::new();
        let mut previous = None;
        for line in &hunk.lines {
            ensure!(line.old || line.new, "source {} has no side", line.id);
            ensure!(
                !line.text.contains(['\n', '\r']),
                "source {} must contain display-safe text without a line ending",
                line.id
            );
            ensure!(
                previous.is_none_or(|id| id < line.id),
                "source IDs in hunk {} must be strictly increasing",
                hunk.id
            );
            source_ids.insert(line.id);
            previous = Some(line.id);
        }
        let wanted: BTreeSet<_> = hunk.wanted.iter().copied().collect();
        ensure!(
            wanted.len() == hunk.wanted.len(),
            "duplicate wanted source ID"
        );
        ensure!(
            wanted.is_subset(&source_ids),
            "hunk {} requests a source ID absent from its lines",
            hunk.id
        );
        let extension = Path::new(&hunk.path)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let syntax = syntaxes
            .find_syntax_by_extension(extension)
            .unwrap_or_else(|| syntaxes.find_syntax_plain_text());
        let mut old = Side::new(syntax, &theme);
        let mut new = Side::new(syntax, &theme);
        let last = wanted.last().copied();
        let mut styled_lines = Vec::with_capacity(wanted.len());
        let mut prefix_records = 0;
        for line in hunk
            .lines
            .iter()
            .take_while(|line| last.is_some_and(|last| line.id <= last))
        {
            prefix_records += 1;
            let wanted = wanted.contains(&line.id);
            let text = format!("{}\n", line.text);
            let too_long = text.len() > 40_004 || text.chars().count() > 10_001;
            let old_style = line
                .old
                .then(|| old.advance(&text, &syntaxes, too_long, wanted));
            let new_style = line
                .new
                .then(|| new.advance(&text, &syntaxes, too_long, wanted));
            if wanted {
                let fallback = old_style.as_ref().is_some_and(|side| side.fallback)
                    || new_style.as_ref().is_some_and(|side| side.fallback);
                styled_lines.push(StyledLine {
                    id: line.id,
                    prefix_records,
                    old: old_style,
                    new: new_style,
                    fallback,
                });
            }
        }
        output.push(StyledHunk {
            id: hunk.id,
            path: hunk.path,
            syntax: syntax.name.clone(),
            through_source_id: last,
            prefix_records,
            old_counts: old.counts,
            new_counts: new.counts,
            lines: styled_lines,
        });
    }
    Ok(Response {
        schema_version: 1,
        appearance: request.appearance,
        offset_unit: "UTF-8 bytes, including the synthetic trailing LF",
        palette,
        hunks: output,
    })
}

fn main() -> Result<()> {
    let request: Request = serde_json::from_reader(io::stdin().lock())
        .context("read sequential syntax reference request")?;
    let response = run(request)?;
    serde_json::to_writer(io::stdout().lock(), &response)?;
    Ok(())
}
