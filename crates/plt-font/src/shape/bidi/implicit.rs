//! Rules applied to each isolating run sequence: weak types (W1–W7),
//! paired brackets (N0), neutrals (N1–N2) and implicit levels (I1–I2).

use super::Level;
use super::class::{BidiClass, Bracket};
use super::explicit::{Sequence, level_direction};

/// Most opening brackets BD16 tracks before giving up on the sequence.
const MAX_OPEN_BRACKETS: usize = 63;

/// Resolves the levels of the characters of `sequence`; `classes` are the
/// classes after directional overrides.
pub(super) fn resolve_sequence(
    sequence: &Sequence,
    chars: &[char],
    classes: &[BidiClass],
    levels: &mut [Level],
) {
    let original: Vec<BidiClass> = sequence.indices.iter().map(|&i| classes[i]).collect();
    let mut types = original.clone();
    resolve_weak(&mut types, sequence.sos);
    let text: Vec<char> = sequence.indices.iter().map(|&i| chars[i]).collect();
    resolve_brackets(&mut types, &original, &text, sequence);
    resolve_neutrals(&mut types, sequence);
    for (&index, &class) in sequence.indices.iter().zip(&types) {
        let level = levels[index];
        levels[index] = match (level.is_multiple_of(2), class) {
            (true, BidiClass::R) | (false, BidiClass::L | BidiClass::EN | BidiClass::AN) => {
                level + 1
            }
            (true, BidiClass::AN | BidiClass::EN) => level + 2,
            _ => level,
        };
    }
}

/// The last strong type (L, R or AL) before position `k`, or `sos`.
fn last_strong(types: &[BidiClass], k: usize, sos: BidiClass) -> BidiClass {
    types[..k]
        .iter()
        .rev()
        .copied()
        .find(|t| matches!(t, BidiClass::L | BidiClass::R | BidiClass::AL))
        .unwrap_or(sos)
}

/// W1–W7.
fn resolve_weak(types: &mut [BidiClass], sos: BidiClass) {
    use BidiClass::{AL, AN, CS, EN, ES, ET, L, NSM, ON, PDI, R};
    // W1: non-spacing marks take the type of the previous character.
    for k in 0..types.len() {
        if types[k] == NSM {
            types[k] = match k.checked_sub(1).map(|p| types[p]) {
                None => sos,
                Some(previous) if previous.is_isolate_initiator() || previous == PDI => ON,
                Some(previous) => previous,
            };
        }
    }
    // W2: European numbers after Arabic letters become Arabic numbers.
    for k in 0..types.len() {
        if types[k] == EN && last_strong(types, k, sos) == AL {
            types[k] = AN;
        }
    }
    // W3.
    for t in types.iter_mut().filter(|t| **t == AL) {
        *t = R;
    }
    // W4: single separators between numbers of the same kind.
    for k in 1..types.len().saturating_sub(1) {
        let (before, after) = (types[k - 1], types[k + 1]);
        types[k] = match (before, types[k], after) {
            (EN, ES | CS, EN) => EN,
            (AN, CS, AN) => AN,
            _ => continue,
        };
    }
    // W5: terminators adjacent to European numbers.
    let mut k = 0;
    while k < types.len() {
        if types[k] != ET {
            k += 1;
            continue;
        }
        let end = (k..types.len())
            .find(|&e| types[e] != ET)
            .unwrap_or(types.len());
        let touches_number = (k > 0 && types[k - 1] == EN) || types.get(end) == Some(&EN);
        if touches_number {
            types[k..end].fill(EN);
        }
        k = end;
    }
    // W6.
    for t in types.iter_mut().filter(|t| matches!(t, ES | ET | CS)) {
        *t = ON;
    }
    // W7: European numbers after left-to-right text become L.
    for k in 0..types.len() {
        if types[k] == EN && last_strong(types, k, sos) == L {
            types[k] = L;
        }
    }
}

/// BD16: the bracket pairs of the sequence as `(opening, closing)`
/// positions sorted by opening position; empty when the stack overflows.
fn bracket_pairs(types: &[BidiClass], text: &[char]) -> Vec<(usize, usize)> {
    let mut stack: Vec<(Bracket, usize)> = Vec::new();
    let mut pairs = Vec::new();
    for (k, &c) in text.iter().enumerate() {
        if types[k] != BidiClass::ON {
            continue;
        }
        let Some(bracket) = Bracket::of(c) else {
            continue;
        };
        if bracket.opening {
            if stack.len() == MAX_OPEN_BRACKETS {
                return Vec::new();
            }
            stack.push((bracket, k));
        } else if let Some(depth) = stack.iter().rposition(|(open, _)| open.is_closed_by(c)) {
            pairs.push((stack[depth].1, k));
            stack.truncate(depth);
        }
    }
    pairs.sort_unstable();
    pairs
}

/// N0: paired brackets take the direction of their content, or of the
/// context when the content is opposite to the embedding direction.
fn resolve_brackets(
    types: &mut [BidiClass],
    original: &[BidiClass],
    text: &[char],
    sequence: &Sequence,
) {
    let embedding = level_direction(sequence.level);
    for (open, close) in bracket_pairs(types, text) {
        let mut inner = types[open + 1..close]
            .iter()
            .filter_map(|t| t.strong_direction());
        let resolved = if inner.clone().any(|d| d == embedding) {
            embedding
        } else if inner.next().is_some() {
            let before = types[..open]
                .iter()
                .rev()
                .find_map(|t| t.strong_direction())
                .unwrap_or(sequence.sos);
            if before == embedding {
                embedding
            } else {
                before
            }
        } else {
            continue;
        };
        for bracket in [open, close] {
            types[bracket] = resolved;
            // Marks following a bracket share its new type.
            for k in bracket + 1..types.len() {
                if original[k] != BidiClass::NSM {
                    break;
                }
                types[k] = resolved;
            }
        }
    }
}

/// N1–N2: runs of neutrals take the direction of the surrounding strong
/// text when both sides agree, the embedding direction otherwise.
fn resolve_neutrals(types: &mut [BidiClass], sequence: &Sequence) {
    let embedding = level_direction(sequence.level);
    let mut k = 0;
    while k < types.len() {
        if !types[k].is_neutral_or_isolate() {
            k += 1;
            continue;
        }
        let end = (k..types.len())
            .find(|&e| !types[e].is_neutral_or_isolate())
            .unwrap_or(types.len());
        let before = k
            .checked_sub(1)
            .and_then(|p| types[p].strong_direction())
            .unwrap_or(sequence.sos);
        let after = types
            .get(end)
            .and_then(|t| t.strong_direction())
            .unwrap_or(sequence.eos);
        let resolved = if before == after { before } else { embedding };
        types[k..end].fill(resolved);
        k = end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use BidiClass::{AL, AN, CS, EN, ET, L, NSM, ON, R, WS};

    fn weak(classes: &[BidiClass], sos: BidiClass) -> Vec<BidiClass> {
        let mut types = classes.to_vec();
        resolve_weak(&mut types, sos);
        types
    }

    fn sequence(len: usize, level: Level) -> Sequence {
        Sequence {
            indices: (0..len).collect(),
            level,
            sos: level_direction(level),
            eos: level_direction(level),
        }
    }

    #[test]
    fn weak_types() {
        assert_eq!(weak(&[AL, EN, CS, EN], L), vec![R, AN, AN, AN]);
        assert_eq!(weak(&[L, EN, CS, EN, ET], R), vec![L, L, L, L, L]);
        assert_eq!(weak(&[ET, ET, EN], R), vec![EN, EN, EN]);
        assert_eq!(weak(&[R, NSM, ON, NSM], L), vec![R, R, ON, ON]);
        assert_eq!(weak(&[NSM, CS, EN], R), vec![R, ON, EN]);
        assert_eq!(weak(&[EN, CS, CS, EN], L), vec![L, ON, ON, L]);
    }

    #[test]
    fn neutrals_and_levels() {
        let seq = sequence(5, 0);
        let mut types = vec![R, WS, R, WS, L];
        resolve_neutrals(&mut types, &seq);
        assert_eq!(types, vec![R, R, R, L, L]);
        let chars: Vec<char> = "\u{5D0} 1\u{5D1}".chars().collect();
        let classes: Vec<BidiClass> = chars.iter().map(|&c| BidiClass::of(c)).collect();
        let mut levels = vec![0; 4];
        resolve_sequence(&sequence(4, 0), &chars, &classes, &mut levels);
        assert_eq!(levels, vec![1, 1, 2, 1]);
    }

    #[test]
    fn brackets() {
        // RTL embedding: "(a)" after R takes the embedding direction R ...
        let chars: Vec<char> = "\u{5D0}(a)".chars().collect();
        let classes: Vec<BidiClass> = chars.iter().map(|&c| BidiClass::of(c)).collect();
        let mut levels = vec![1; 4];
        resolve_sequence(&sequence(4, 1), &chars, &classes, &mut levels);
        assert_eq!(levels, vec![1, 1, 2, 1]);
        // ... but L context makes them L.
        let chars: Vec<char> = "b(a)\u{301}".chars().collect();
        let classes: Vec<BidiClass> = chars.iter().map(|&c| BidiClass::of(c)).collect();
        let mut levels = vec![1; 5];
        resolve_sequence(&sequence(5, 1), &chars, &classes, &mut levels);
        assert_eq!(levels, vec![2, 2, 2, 2, 2]);
        let text: Vec<char> = "([)]".chars().collect();
        assert_eq!(bracket_pairs(&[ON; 4], &text), vec![(0, 2)]);
        let deep: Vec<char> = std::iter::repeat_n('(', 64).collect();
        assert_eq!(bracket_pairs(&[ON; 64], &deep), vec![]);
    }
}
