//! The Unicode Bidirectional Algorithm (UAX #9) as libraqm applies it
//! through `SheenBidi`: paragraph level, explicit embeddings and isolates,
//! weak and neutral types, paired brackets, implicit levels, line-level
//! resets (L1) and visual reordering of level runs (L2).
//!
//! Characters removed by rule X9 take the level of the preceding character
//! (the paragraph level at its start), as in `SheenBidi`; text after a
//! paragraph separator forms further paragraphs whose runs follow.

mod class;
mod explicit;
mod implicit;

pub(crate) use class::mirror;

use crate::types::Direction;
use class::BidiClass;
use explicit::{first_strong_level, isolating_run_sequences, matching_pdis, resolve};
use implicit::resolve_sequence;

/// An embedding level; odd levels are right to left.
pub(crate) type Level = u8;

/// A maximal run of characters at one level.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct LevelRun {
    /// First character index.
    pub(crate) start: usize,
    /// One past the last character index.
    pub(crate) end: usize,
    pub(crate) level: Level,
}

impl LevelRun {
    /// Direction of the run's text.
    pub(crate) const fn direction(&self) -> Direction {
        if self.level.is_multiple_of(2) {
            Direction::LeftToRight
        } else {
            Direction::RightToLeft
        }
    }
}

/// Resolves `chars` paragraph by paragraph (`base`: forced paragraph
/// direction, `None` for the first strong character's) and returns their
/// level runs, each paragraph's runs in visual order.
pub(crate) fn visual_runs(chars: &[char], base: Option<Direction>) -> Vec<LevelRun> {
    let classes: Vec<BidiClass> = chars.iter().map(|&c| BidiClass::of(c)).collect();
    let mut runs = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let end = paragraph_end(chars, &classes, start);
        let range = start..end;
        let (paragraph, mut levels) =
            paragraph_levels(&chars[range.clone()], &classes[range.clone()], base);
        reset_line_levels(&classes[range], &mut levels, paragraph);
        let mut line = level_runs(&levels, start);
        reorder(&mut line);
        runs.extend(line);
        start = end;
    }
    runs
}

/// End of the paragraph starting at `start`: just after its separator
/// (`\r\n` counts as one).
fn paragraph_end(chars: &[char], classes: &[BidiClass], start: usize) -> usize {
    match (start..chars.len()).find(|&i| classes[i] == BidiClass::B) {
        Some(i) if chars[i] == '\r' && chars.get(i + 1) == Some(&'\n') => i + 2,
        Some(i) => i + 1,
        None => chars.len(),
    }
}

/// The paragraph level and the resolved level of each character.
fn paragraph_levels(
    chars: &[char],
    classes: &[BidiClass],
    base: Option<Direction>,
) -> (Level, Vec<Level>) {
    let matching = matching_pdis(classes);
    let paragraph = match base {
        Some(Direction::LeftToRight) => 0,
        Some(Direction::RightToLeft) => 1,
        None => first_strong_level(classes, &matching, 0, classes.len()).unwrap_or(0),
    };
    let explicit = resolve(classes, &matching, paragraph);
    let mut levels = explicit.levels;
    for sequence in isolating_run_sequences(classes, &matching, &levels, paragraph) {
        resolve_sequence(&sequence, chars, &explicit.classes, &mut levels);
    }
    let mut previous = paragraph;
    for (level, class) in levels.iter_mut().zip(classes) {
        if class.is_removed_by_x9() {
            *level = previous;
        }
        previous = *level;
    }
    (paragraph, levels)
}

/// L1 as `SheenBidi` applies it: separators, trailing whitespace and
/// isolate formatting characters, and the removed characters following
/// them, return to the paragraph level.
fn reset_line_levels(classes: &[BidiClass], levels: &mut [Level], paragraph: Level) {
    let mut pending = 0;
    let mut trailing = true;
    for index in (0..classes.len()).rev() {
        let reset = match classes[index] {
            BidiClass::B | BidiClass::S => {
                trailing = true;
                true
            }
            class if class.is_removed_by_x9() => {
                pending += 1;
                false
            }
            BidiClass::WS | BidiClass::LRI | BidiClass::RLI | BidiClass::FSI | BidiClass::PDI => {
                trailing
            }
            _ => {
                pending = 0;
                trailing = false;
                false
            }
        };
        if reset {
            levels[index..=index + pending].fill(paragraph);
            pending = 0;
        }
    }
}

/// Maximal runs of equal levels, offset by `start`.
fn level_runs(levels: &[Level], start: usize) -> Vec<LevelRun> {
    let mut runs: Vec<LevelRun> = Vec::new();
    for (index, &level) in levels.iter().enumerate() {
        match runs.last_mut() {
            Some(run) if run.level == level => run.end = start + index + 1,
            _ => runs.push(LevelRun {
                start: start + index,
                end: start + index + 1,
                level,
            }),
        }
    }
    runs
}

/// L2: from the highest level down to 1, reverses every maximal sequence
/// of runs at that level or above.
fn reorder(runs: &mut [LevelRun]) {
    let max = runs.iter().map(|run| run.level).max().unwrap_or(0);
    for level in (1..=max).rev() {
        let mut index = 0;
        while index < runs.len() {
            if runs[index].level < level {
                index += 1;
                continue;
            }
            let end = (index..runs.len())
                .find(|&e| runs[e].level < level)
                .unwrap_or(runs.len());
            runs[index..end].reverse();
            index = end;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runs(text: &str, base: Option<Direction>) -> Vec<(usize, usize, Level)> {
        let chars: Vec<char> = text.chars().collect();
        visual_runs(&chars, base)
            .iter()
            .map(|run| (run.start, run.end, run.level))
            .collect()
    }

    #[test]
    fn left_to_right_and_right_to_left_paragraphs() {
        assert_eq!(runs("abc", None), vec![(0, 3, 0)]);
        assert_eq!(runs("\u{645}\u{631}\u{62D}", None), vec![(0, 3, 1)]);
        assert_eq!(runs("123", None), vec![(0, 3, 0)]);
        assert_eq!(runs("abc", Some(Direction::RightToLeft)), vec![(0, 3, 2)]);
        assert_eq!(runs("", None), vec![]);
    }

    #[test]
    fn mixed_text_is_reordered() {
        // "abc ARABIC 123": the digits follow the Arabic text (W2) and the
        // right-to-left part is reversed as a whole.
        let text = "abc \u{645}\u{631}\u{62D}\u{628}\u{627} 123";
        assert_eq!(runs(text, None), vec![(0, 4, 0), (10, 13, 2), (4, 10, 1)]);
        // Right-to-left paragraph with a bracketed Latin word and a
        // trailing space: the brackets take the paragraph direction.
        let text = "\u{645} (abc) \u{645} ";
        assert_eq!(runs(text, None), vec![(6, 10, 1), (3, 6, 2), (0, 3, 1)]);
    }

    #[test]
    fn line_resets_and_removed_characters() {
        let chars: Vec<char> = "\u{5D0} \u{200D}b\t\u{5D1}  ".chars().collect();
        let classes: Vec<BidiClass> = chars.iter().map(|&c| BidiClass::of(c)).collect();
        let (paragraph, mut levels) = paragraph_levels(&chars, &classes, None);
        assert_eq!(paragraph, 1);
        assert_eq!(levels, vec![1, 1, 1, 2, 1, 1, 1, 1]);
        reset_line_levels(&classes, &mut levels, paragraph);
        assert_eq!(levels, vec![1, 1, 1, 2, 1, 1, 1, 1]);
        let chars: Vec<char> = "a\u{5D0} \u{200D}\tb".chars().collect();
        let classes: Vec<BidiClass> = chars.iter().map(|&c| BidiClass::of(c)).collect();
        let (paragraph, mut levels) = paragraph_levels(&chars, &classes, None);
        assert_eq!(levels, vec![0, 1, 0, 0, 0, 0]);
        reset_line_levels(&classes, &mut levels, paragraph);
        assert_eq!(levels, vec![0, 1, 0, 0, 0, 0]);
    }

    /// Lines of Unicode 17's `BidiCharacterTest.txt` (code points;
    /// paragraph direction; paragraph level; levels; visual order). The
    /// implementation passes the whole file and `BidiTest.txt`.
    const CONFORMANCE: [&str; 8] = [
        "202D 05D0 202B 05D1 202C 2068 05D2 2069 202B 05D3 202C 05D4 202C;2;1;\
         x 2 x 3 x 2 3 2 x 3 x 2 x;1 3 5 6 7 9 11",
        "202D 0661 202B 0662 202C 2068 0663 2069 202B 0664 202C 0665 202C;0;0;\
         x 2 x 4 x 2 6 2 x 4 x 2 x;1 3 5 6 7 9 11",
        "05D0 2067 202A 0041;1;1;1 1 x 4;3 1 0",
        "05D0 05D1 0028 05D2 05D3 005B 0026 0065 0066 005D 002E 0029 0067 0068;0;0;\
         1 1 0 1 1 0 0 0 0 0 0 0 0 0;1 0 2 4 3 5 6 7 8 9 10 11 12 13",
        "05D0 05D1 0028 05D2 05D3 005B 0026 0065 0066 005D 002E 0029 0067 0068;1;1;\
         1 1 1 1 1 1 1 2 2 1 1 1 2 2;12 13 11 10 9 7 8 6 5 4 3 2 1 0",
        "202A 202E 0061 202C 0028 005B 05D0 202C 202D 005D 0029 202C;2;0;\
         x x 3 x 3 3 3 x x 2 2 x;6 5 4 2 9 10",
        "002B 0661 0028 0662 0029;2;0;0 2 1 2 1;0 4 3 2 1",
        "202E 0061 202A 0062 202C 2066 0063 2069 202A 0064 202C 0065 202C;1;1;\
         x 3 x 4 x 3 4 3 x 4 x 3 x;11 9 7 6 5 3 1",
    ];

    #[test]
    fn conformance_sample() {
        for line in CONFORMANCE {
            let fields: Vec<&str> = line.split(';').collect();
            let chars: Vec<char> = fields[0]
                .split_whitespace()
                .filter_map(|h| char::from_u32(u32::from_str_radix(h, 16).ok()?))
                .collect();
            let base = match fields[1] {
                "0" => Some(Direction::LeftToRight),
                "1" => Some(Direction::RightToLeft),
                _ => None,
            };
            let classes: Vec<BidiClass> = chars.iter().map(|&c| BidiClass::of(c)).collect();
            let (paragraph, mut levels) = paragraph_levels(&chars, &classes, base);
            reset_line_levels(&classes, &mut levels, paragraph);
            assert_eq!(paragraph.to_string(), fields[2], "{line}");
            let expected: Vec<Option<Level>> = fields[3]
                .split_whitespace()
                .map(|l| l.parse().ok())
                .collect();
            for (index, level) in expected.iter().enumerate() {
                assert!(
                    level.is_none_or(|l| l == levels[index]),
                    "{line}: {levels:?}"
                );
            }
            let mut runs = level_runs(&levels, 0);
            reorder(&mut runs);
            let order: Vec<String> = runs
                .iter()
                .flat_map(|run| {
                    let forward = run.start..run.end;
                    let indices: Vec<usize> = if run.level % 2 == 1 {
                        forward.rev().collect()
                    } else {
                        forward.collect()
                    };
                    indices
                })
                .filter(|&i| expected[i].is_some())
                .map(|i| i.to_string())
                .collect();
            assert_eq!(order.join(" "), fields[4], "{line}");
        }
    }

    #[test]
    fn paragraphs_are_resolved_separately() {
        let text = "\u{5D0}b\r\nc\u{5D1}";
        assert_eq!(
            runs(text, None),
            vec![(2, 4, 1), (1, 2, 2), (0, 1, 1), (4, 5, 0), (5, 6, 1)]
        );
        assert_eq!(
            LevelRun {
                start: 0,
                end: 1,
                level: 3
            }
            .direction(),
            Direction::RightToLeft
        );
    }
}
