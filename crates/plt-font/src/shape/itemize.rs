//! libraqm's itemization: bidi level runs in visual order, split where the
//! resolved script, the font or the language changes. Right-to-left runs
//! are split from their end, so that the pieces themselves stay in visual
//! order.

use crate::shape::bidi::visual_runs;
use crate::shape::language::LanguageTag;
use crate::shape::script::{Script, resolve_scripts};
use crate::types::Direction;

/// A run shaped by one call into the shaper.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Run<'a> {
    /// First character index.
    pub(crate) start: usize,
    /// One past the last character index.
    pub(crate) end: usize,
    pub(crate) script: Script,
    pub(crate) direction: Direction,
    /// Index of the face the run is shaped with.
    pub(crate) face: usize,
    /// The run's language (`None`: the default language).
    pub(crate) language: Option<&'a LanguageTag>,
}

/// The script of each character: `forced` for all, or libraqm's
/// resolution.
pub(crate) fn scripts(chars: &[char], forced: Option<Script>) -> Vec<Script> {
    match forced {
        Some(script) => vec![script; chars.len()],
        None => resolve_scripts(chars),
    }
}

/// Splits `chars` into runs in visual order; `scripts`, `faces` and
/// `languages` give each character's script, face and language, `base`
/// the paragraph direction (`None` to detect it).
pub(crate) fn itemize<'a>(
    chars: &[char],
    scripts: &[Script],
    faces: &[usize],
    languages: &[Option<&'a LanguageTag>],
    base: Option<Direction>,
) -> Vec<Run<'a>> {
    let mut runs = Vec::new();
    for level_run in visual_runs(chars, base) {
        let direction = level_run.direction();
        let mut pieces: Vec<Run<'a>> = Vec::new();
        for index in level_run.start..level_run.end {
            match pieces.last_mut() {
                Some(run)
                    if run.script == scripts[index]
                        && run.face == faces[index]
                        && run.language == languages[index] =>
                {
                    run.end = index + 1;
                }
                _ => pieces.push(Run {
                    start: index,
                    end: index + 1,
                    script: scripts[index],
                    direction,
                    face: faces[index],
                    language: languages[index],
                }),
            }
        }
        if direction == Direction::RightToLeft {
            pieces.reverse();
        }
        runs.extend(pieces);
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(
        text: &str,
        faces: &[usize],
        languages: &[Option<&LanguageTag>],
    ) -> Vec<(usize, usize, Direction, usize)> {
        let chars: Vec<char> = text.chars().collect();
        let scripts = scripts(&chars, None);
        itemize(&chars, &scripts, faces, languages, None)
            .iter()
            .map(|run| (run.start, run.end, run.direction, run.face))
            .collect()
    }

    #[test]
    fn runs_follow_levels_scripts_and_faces() {
        use Direction::{LeftToRight as Ltr, RightToLeft as Rtl};
        assert_eq!(
            spans("Ab αβ", &[0; 5], &[None; 5]),
            vec![(0, 3, Ltr, 0), (3, 5, Ltr, 0)]
        );
        // A face change inside a right-to-left run: pieces in visual order.
        assert_eq!(
            spans("\u{645}\u{631}\u{62D}", &[0, 1, 1], &[None; 3]),
            vec![(1, 3, Rtl, 1), (0, 1, Rtl, 0)]
        );
        assert_eq!(
            spans("ab \u{5D0}\u{5D1}", &[0; 5], &[None; 5]),
            vec![(0, 3, Ltr, 0), (3, 5, Rtl, 0)]
        );
        assert_eq!(spans("", &[], &[]), vec![]);
        let forced = scripts(&['a', 'α'], Some(Script::COMMON));
        assert_eq!(forced, vec![Script::COMMON; 2]);
    }

    #[test]
    fn languages_split_runs_when_their_tags_differ() {
        use Direction::LeftToRight as Ltr;
        let (sr, sr_upper, ru) = (
            LanguageTag::new("sr"),
            LanguageTag::new("SR"),
            LanguageTag::new("ru"),
        );
        let languages = [Some(&sr), Some(&sr_upper), Some(&ru), None];
        assert_eq!(
            spans("абвг", &[0; 4], &languages),
            vec![(0, 2, Ltr, 0), (2, 3, Ltr, 0), (3, 4, Ltr, 0)]
        );
    }
}
