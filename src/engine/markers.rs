use std::collections::{BinaryHeap, HashMap};

use anyhow::{Result, ensure};
use ratatui::style::Style;

use super::{Engine, InputEffect, ReviewId, Side};
use crate::reading::SourceRange;

/// A host-owned gutter mark over original source lines `range.first..=range.last` on one side.
///
/// The engine draws `▌` in the first gutter cell of each visible row whose source line on
/// `range.side` falls in the range, patched with `style`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Marker {
    pub range: SourceRange,
    pub style: Style,
}

/// The gutter glyph of a marked row.
pub(crate) const SYMBOL: &str = "▌";

/// Non-overlapping, ordered line spans for one side of one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Span {
    first: u32,
    last: u32,
    style: Style,
}

/// Markers indexed by file and side. Overlaps are resolved when the set is replaced, so a
/// drawn row costs one map lookup and one binary search, whatever the number of markers.
#[derive(Default)]
pub(crate) struct Markers {
    files: HashMap<usize, [Vec<Span>; 2]>,
}

impl Markers {
    /// Later markers in `markers` win where ranges overlap.
    fn new(markers: &[Marker]) -> Self {
        let mut grouped: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
        for (index, marker) in markers.iter().enumerate() {
            grouped
                .entry((marker.range.file, side_index(marker.range.side)))
                .or_default()
                .push(index);
        }
        let mut files: HashMap<usize, [Vec<Span>; 2]> = HashMap::new();
        for ((file, side), indices) in grouped {
            files.entry(file).or_default()[side] = flatten(markers, indices);
        }
        Self { files }
    }

    pub(crate) fn style(&self, file: usize, side: Side, line: Option<u32>) -> Option<Style> {
        let line = line?;
        let spans = &self.files.get(&file)?[side_index(side)];
        let span = spans.get(spans.partition_point(|span| span.last < line))?;
        (span.first <= line).then_some(span.style)
    }
}

fn side_index(side: Side) -> usize {
    match side {
        Side::Old => 0,
        Side::New => 1,
    }
}

/// Sweeps the range endpoints once, keeping the latest marker that covers each interval.
fn flatten(markers: &[Marker], mut indices: Vec<usize>) -> Vec<Span> {
    indices.sort_by_key(|&index| markers[index].range.first);
    let mut bounds: Vec<u64> = indices
        .iter()
        .flat_map(|&index| {
            let range = markers[index].range;
            [u64::from(range.first), u64::from(range.last) + 1]
        })
        .collect();
    bounds.sort_unstable();
    bounds.dedup();
    let mut active = BinaryHeap::new();
    let mut next = 0;
    let mut spans: Vec<Span> = Vec::new();
    for pair in bounds.windows(2) {
        let (start, end) = (pair[0], pair[1]);
        while let Some(&index) = indices.get(next)
            && u64::from(markers[index].range.first) <= start
        {
            active.push(index);
            next += 1;
        }
        while active
            .peek()
            .is_some_and(|&index| u64::from(markers[index].range.last) < start)
        {
            active.pop();
        }
        let Some(&index) = active.peek() else {
            continue;
        };
        // Both bounds come from u32 lines, and `end` is one past a covered line.
        let (first, last) = (start as u32, (end - 1) as u32);
        let style = markers[index].style;
        match spans.last_mut() {
            Some(span) if span.style == style && u64::from(span.last) + 1 == start => {
                span.last = last;
            }
            _ => spans.push(Span { first, last, style }),
        }
    }
    spans
}

pub(super) fn validate(review: ReviewId, markers: &[Marker]) -> Result<()> {
    for marker in markers {
        let range = marker.range;
        ensure!(range.review == review, "Marker belongs to another review");
        ensure!(
            range.first != 0 && range.first <= range.last,
            "Invalid marker line range {}..={}",
            range.first,
            range.last
        );
    }
    Ok(())
}

impl Engine {
    /// Replaces every marker. Markers for files that have not arrived yet, or for lines that
    /// are not in the diff, draw nothing. An empty slice clears them.
    pub fn set_markers(&mut self, markers: &[Marker]) -> Result<()> {
        ensure!(self.input.is_some(), "The review is closed");
        validate(self.id, markers)?;
        let markers = Markers::new(markers);
        match &mut self.app {
            Some(app) => app.markers = markers,
            None => self.markers = markers,
        }
        self.changed(InputEffect::Redraw);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    fn marker(file: usize, side: Side, first: u32, last: u32, color: Color) -> Marker {
        Marker {
            range: SourceRange {
                review: ReviewId(1),
                file,
                side,
                first,
                last,
            },
            style: Style::new().fg(color),
        }
    }

    fn color(markers: &Markers, file: usize, side: Side, line: u32) -> Option<Color> {
        markers.style(file, side, Some(line)).and_then(|s| s.fg)
    }

    #[test]
    fn later_markers_win_overlaps_and_sides_and_files_stay_separate() {
        let markers = Markers::new(&[
            marker(0, Side::New, 3, 10, Color::Yellow),
            marker(0, Side::New, 5, 6, Color::Green),
            marker(0, Side::Old, 1, 2, Color::Red),
            marker(1, Side::New, 4, 4, Color::Blue),
            marker(0, Side::New, 9, u32::MAX, Color::Cyan),
        ]);
        let new: Vec<_> = (1..=12).map(|l| color(&markers, 0, Side::New, l)).collect();
        use Color::*;
        assert_eq!(
            new,
            [
                None,
                None,
                Some(Yellow),
                Some(Yellow),
                Some(Green),
                Some(Green),
                Some(Yellow),
                Some(Yellow),
                Some(Cyan),
                Some(Cyan),
                Some(Cyan),
                Some(Cyan),
            ]
        );
        assert_eq!(color(&markers, 0, Side::New, u32::MAX), Some(Cyan));
        assert_eq!(color(&markers, 0, Side::Old, 2), Some(Red));
        assert_eq!(color(&markers, 0, Side::Old, 3), None);
        assert_eq!(color(&markers, 1, Side::New, 4), Some(Blue));
        assert_eq!(color(&markers, 1, Side::Old, 4), None);
        assert_eq!(color(&markers, 2, Side::New, 4), None);
        assert_eq!(markers.style(0, Side::New, None), None);
        // Equal neighbours merge into one span; the overlap split the yellow range in two.
        assert_eq!(markers.files[&0][1].len(), 4);
    }

    #[test]
    fn a_hidden_marker_reappears_where_the_later_one_ends() {
        let markers = Markers::new(&[
            marker(0, Side::New, 1, 20, Color::Yellow),
            marker(0, Side::New, 2, 4, Color::Green),
            marker(0, Side::New, 3, 3, Color::Yellow),
        ]);
        let spans: Vec<_> = markers.files[&0][1]
            .iter()
            .map(|span| (span.first, span.last, span.style.fg))
            .collect();
        assert_eq!(
            spans,
            [
                (1, 1, Some(Color::Yellow)),
                (2, 2, Some(Color::Green)),
                (3, 3, Some(Color::Yellow)),
                (4, 4, Some(Color::Green)),
                (5, 20, Some(Color::Yellow)),
            ]
        );
    }
}
