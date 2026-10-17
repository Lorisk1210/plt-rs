//! Paragraph level (P2–P3), explicit levels and directions (X1–X9) and
//! isolating run sequences (X10).

use super::Level;
use super::class::BidiClass;

/// Deepest embedding level (`max_depth`).
const MAX_DEPTH: Level = 125;

/// Resolved explicit embedding state of a paragraph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Explicit {
    /// Embedding level of each character.
    pub(super) levels: Vec<Level>,
    /// Classes after directional overrides.
    pub(super) classes: Vec<BidiClass>,
}

/// One isolating run sequence: character indices plus boundary types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Sequence {
    pub(super) indices: Vec<usize>,
    pub(super) level: Level,
    pub(super) sos: BidiClass,
    pub(super) eos: BidiClass,
}

/// The direction (L or R) a level stands for.
pub(super) const fn level_direction(level: Level) -> BidiClass {
    if level.is_multiple_of(2) {
        BidiClass::L
    } else {
        BidiClass::R
    }
}

/// BD9: the matching PDI of each isolate initiator.
pub(super) fn matching_pdis(classes: &[BidiClass]) -> Vec<Option<usize>> {
    let mut matches = vec![None; classes.len()];
    let mut open: Vec<usize> = Vec::new();
    for (index, &class) in classes.iter().enumerate() {
        if class.is_isolate_initiator() {
            open.push(index);
        } else if class == BidiClass::PDI
            && let Some(initiator) = open.pop()
        {
            matches[initiator] = Some(index);
        }
    }
    matches
}

/// P2–P3: the level of the first strong character in `start..end`,
/// skipping isolates; `None` when there is none.
pub(super) fn first_strong_level(
    classes: &[BidiClass],
    matching: &[Option<usize>],
    start: usize,
    end: usize,
) -> Option<Level> {
    let mut index = start;
    while index < end {
        match classes[index] {
            BidiClass::L => return Some(0),
            BidiClass::R | BidiClass::AL => return Some(1),
            class if class.is_isolate_initiator() => index = matching[index]?,
            _ => {}
        }
        index += 1;
    }
    None
}

/// One entry of the directional status stack.
#[derive(Clone, Copy, Debug)]
struct Status {
    level: Level,
    override_class: Option<BidiClass>,
    isolate: bool,
}

/// The directional status stack with its overflow counters (X1).
struct StatusStack {
    entries: Vec<Status>,
    overflow_isolates: usize,
    overflow_embeddings: usize,
    valid_isolates: usize,
}

impl StatusStack {
    fn new(paragraph: Level) -> Self {
        Self {
            entries: vec![Status {
                level: paragraph,
                override_class: None,
                isolate: false,
            }],
            overflow_isolates: 0,
            overflow_embeddings: 0,
            valid_isolates: 0,
        }
    }

    fn top(&self) -> Status {
        *self
            .entries
            .last()
            .expect("the stack keeps its paragraph entry")
    }

    /// The least odd (`odd`) or even level above the current one, if it is
    /// valid and nothing overflowed.
    fn next_level(&self, odd: bool) -> Option<Level> {
        let level = self.top().level;
        let next = if odd {
            (level + 1) | 1
        } else {
            (level + 2) & !1
        };
        (next <= MAX_DEPTH && self.overflow_isolates == 0 && self.overflow_embeddings == 0)
            .then_some(next)
    }

    /// X2–X5: an embedding or override initiator.
    fn embed(&mut self, odd: bool, override_class: Option<BidiClass>) {
        match self.next_level(odd) {
            Some(level) => self.entries.push(Status {
                level,
                override_class,
                isolate: false,
            }),
            None if self.overflow_isolates == 0 => self.overflow_embeddings += 1,
            None => {}
        }
    }

    /// X5a–X5c: an isolate initiator (after it took the current level).
    fn isolate(&mut self, odd: bool) {
        match self.next_level(odd) {
            Some(level) => {
                self.valid_isolates += 1;
                self.entries.push(Status {
                    level,
                    override_class: None,
                    isolate: true,
                });
            }
            None => self.overflow_isolates += 1,
        }
    }

    /// X6a: a PDI closes the innermost valid isolate.
    fn close_isolate(&mut self) {
        if self.overflow_isolates > 0 {
            self.overflow_isolates -= 1;
        } else if self.valid_isolates > 0 {
            self.overflow_embeddings = 0;
            while self.entries.last().is_some_and(|status| !status.isolate) {
                self.entries.pop();
            }
            self.entries.pop();
            self.valid_isolates -= 1;
        }
    }

    /// X7: a PDF closes the innermost embedding.
    fn close_embedding(&mut self) {
        if self.overflow_isolates > 0 {
            return;
        }
        if self.overflow_embeddings > 0 {
            self.overflow_embeddings -= 1;
        } else if !self.top().isolate && self.entries.len() >= 2 {
            self.entries.pop();
        }
    }
}

/// X1–X8: explicit levels and overrides of a paragraph at `paragraph`
/// level.
pub(super) fn resolve(
    classes: &[BidiClass],
    matching: &[Option<usize>],
    paragraph: Level,
) -> Explicit {
    let mut levels = vec![paragraph; classes.len()];
    let mut resolved = classes.to_vec();
    let mut stack = StatusStack::new(paragraph);
    for (index, &class) in classes.iter().enumerate() {
        let top = stack.top();
        levels[index] = top.level;
        let overridable = !class.is_removed_by_x9() && class != BidiClass::B;
        if let Some(class) = top.override_class.filter(|_| overridable) {
            resolved[index] = class;
        }
        match class {
            BidiClass::RLE | BidiClass::LRE => stack.embed(class == BidiClass::RLE, None),
            BidiClass::RLO => stack.embed(true, Some(BidiClass::R)),
            BidiClass::LRO => stack.embed(false, Some(BidiClass::L)),
            BidiClass::RLI => stack.isolate(true),
            BidiClass::LRI => stack.isolate(false),
            BidiClass::FSI => {
                let end = matching[index].unwrap_or(classes.len());
                stack.isolate(first_strong_level(classes, matching, index + 1, end) == Some(1));
            }
            BidiClass::PDI => {
                stack.close_isolate();
                let top = stack.top();
                levels[index] = top.level;
                resolved[index] = top.override_class.unwrap_or(class);
            }
            BidiClass::PDF => stack.close_embedding(),
            BidiClass::B => levels[index] = paragraph,
            _ => {}
        }
    }
    Explicit {
        levels,
        classes: resolved,
    }
}

/// X10: groups the characters kept by X9 into level runs and links runs
/// across isolates into isolating run sequences.
pub(super) fn isolating_run_sequences(
    classes: &[BidiClass],
    matching: &[Option<usize>],
    levels: &[Level],
    paragraph: Level,
) -> Vec<Sequence> {
    let kept: Vec<usize> = (0..classes.len())
        .filter(|&i| !classes[i].is_removed_by_x9())
        .collect();
    let mut runs: Vec<Vec<usize>> = Vec::new();
    for &index in &kept {
        match runs.last_mut() {
            Some(run) if levels[run[0]] == levels[index] => run.push(index),
            _ => runs.push(vec![index]),
        }
    }
    let run_starting_at = |index: usize| runs.iter().position(|run| run[0] == index);
    let is_continuation =
        |run: &[usize]| classes[run[0]] == BidiClass::PDI && matching.contains(&Some(run[0]));
    let neighbour_level = |index: usize, forward: bool| {
        let position = kept.binary_search(&index).unwrap_or(0);
        let neighbour = if forward {
            kept.get(position + 1)
        } else {
            position.checked_sub(1).and_then(|p| kept.get(p))
        };
        neighbour.map_or(paragraph, |&n| levels[n])
    };
    let mut sequences = Vec::new();
    for run in runs.iter().filter(|run| !is_continuation(run)) {
        let mut indices = run.clone();
        while let Some(next) = indices
            .last()
            .and_then(|&last| matching[last])
            .and_then(run_starting_at)
        {
            indices.extend_from_slice(&runs[next]);
        }
        let level = levels[indices[0]];
        let first = indices[0];
        let last = *indices.last().unwrap_or(&first);
        let sos = level_direction(level.max(neighbour_level(first, false)));
        let eos_level = if classes[last].is_isolate_initiator() && matching[last].is_none() {
            paragraph
        } else {
            neighbour_level(last, true)
        };
        sequences.push(Sequence {
            indices,
            level,
            sos,
            eos: level_direction(level.max(eos_level)),
        });
    }
    sequences
}

#[cfg(test)]
mod tests {
    use super::*;
    use BidiClass::{AL, B, EN, FSI, L, LRE, ON, PDF, PDI, R, RLI, RLO, WS};

    #[test]
    fn matching_and_first_strong() {
        let classes = [RLI, L, PDI, R, FSI, AL];
        let matching = matching_pdis(&classes);
        assert_eq!(matching, vec![Some(2), None, None, None, None, None]);
        assert_eq!(first_strong_level(&classes, &matching, 0, 6), Some(1));
        assert_eq!(first_strong_level(&classes, &matching, 4, 6), None);
        assert_eq!(first_strong_level(&[ON, EN], &[None, None], 0, 2), None);
    }

    #[test]
    fn embeddings_overrides_and_isolates() {
        let classes = [L, RLO, L, PDF, LRE, R, PDF, RLI, L, PDI, B];
        let matching = matching_pdis(&classes);
        let explicit = resolve(&classes, &matching, 0);
        assert_eq!(explicit.levels, vec![0, 0, 1, 1, 0, 2, 2, 0, 1, 0, 0]);
        assert_eq!(explicit.classes[2], R);
        assert_eq!(explicit.classes[5], R);
        let sequences = isolating_run_sequences(&classes, &matching, &explicit.levels, 0);
        let indices: Vec<Vec<usize>> = sequences.iter().map(|s| s.indices.clone()).collect();
        assert_eq!(
            indices,
            vec![vec![0], vec![2], vec![5], vec![7, 9, 10], vec![8]]
        );
        assert_eq!((sequences[1].sos, sequences[1].eos), (R, L));
        assert_eq!((sequences[3].sos, sequences[3].eos), (L, L));
        assert_eq!((sequences[4].sos, sequences[4].eos), (R, R));
    }

    #[test]
    fn overflowing_embeddings_are_ignored() {
        let classes: Vec<BidiClass> = std::iter::repeat_n(LRE, 70).chain([WS]).collect();
        let matching = matching_pdis(&classes);
        let explicit = resolve(&classes, &matching, 0);
        assert_eq!(explicit.levels[70], 124);
        let unmatched = [RLI, L];
        let explicit = resolve(&unmatched, &matching_pdis(&unmatched), 0);
        assert_eq!(explicit.levels, vec![0, 1]);
        let sequences = isolating_run_sequences(&unmatched, &[None, None], &explicit.levels, 0);
        assert_eq!(sequences[0].eos, L);
    }
}
