//! Arabic joining: joining types (`ArabicShaping.txt`) and HarfBuzz's
//! joining state machine, which picks the positional form of every
//! character from its neighbours.

use crate::shape::ucd_tables::JOINING_TYPES;
use Action::{Fin2, Fin3, Fina, Init, Isol, Med2, Medi};

/// A joining type, with the Syriac joining groups HarfBuzz treats
/// specially.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum JoiningType {
    /// Non-joining.
    U,
    /// Left-joining.
    L,
    /// Right-joining.
    R,
    /// Dual-joining (also join-causing characters such as ZWJ).
    D,
    /// Syriac ALAPH.
    Alaph,
    /// Syriac DALATH and RISH.
    DalathRish,
    /// Transparent (marks and format characters).
    T,
}

impl JoiningType {
    /// Every type, indexed by the codes of `JOINING_TYPES`.
    const ALL: [Self; 7] = [
        Self::U,
        Self::L,
        Self::R,
        Self::D,
        Self::Alaph,
        Self::DalathRish,
        Self::T,
    ];

    /// Joining type of `c` (HarfBuzz's `get_joining_type`: unlisted marks
    /// and format characters are transparent).
    pub(crate) fn of(c: char) -> Self {
        let code = u32::from(c);
        let index = JOINING_TYPES.partition_point(|&(start, _, _)| start <= code);
        match JOINING_TYPES.get(index.wrapping_sub(1)) {
            Some(&(_, end, kind)) if code <= end => {
                Self::ALL.get(usize::from(kind)).copied().unwrap_or(Self::U)
            }
            _ => Self::U,
        }
    }

    /// Column of the state machine (`None` for transparent characters).
    const fn column(self) -> Option<usize> {
        match self {
            Self::U => Some(0),
            Self::L => Some(1),
            Self::R => Some(2),
            Self::D => Some(3),
            Self::Alaph => Some(4),
            Self::DalathRish => Some(5),
            Self::T => None,
        }
    }
}

/// The positional form a character takes, in the order of HarfBuzz's
/// `arabic_features`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Action {
    Isol,
    Fina,
    Fin2,
    Fin3,
    Medi,
    Med2,
    Init,
}

impl Action {
    /// Position of the action's feature in HarfBuzz's feature order.
    pub(crate) const fn index(self) -> usize {
        self as usize
    }
}

/// One transition: action for the previous character, action for the
/// current one, next state.
type Entry = (Option<Action>, Option<Action>, usize);

const fn entry(previous: Option<Action>, current: Option<Action>, next: usize) -> Entry {
    (previous, current, next)
}

/// HarfBuzz's `arabic_state_table`, columns U, L, R, D, ALAPH,
/// DALATH/RISH.
const STATES: [[Entry; 6]; 7] = [
    // 0: previous was U, not willing to join.
    [
        entry(None, None, 0),
        entry(None, Some(Isol), 2),
        entry(None, Some(Isol), 1),
        entry(None, Some(Isol), 2),
        entry(None, Some(Isol), 1),
        entry(None, Some(Isol), 6),
    ],
    // 1: previous was R or ISOL/ALAPH, not willing to join.
    [
        entry(None, None, 0),
        entry(None, Some(Isol), 2),
        entry(None, Some(Isol), 1),
        entry(None, Some(Isol), 2),
        entry(None, Some(Fin2), 5),
        entry(None, Some(Isol), 6),
    ],
    // 2: previous was D/L in ISOL form, willing to join.
    [
        entry(None, None, 0),
        entry(None, Some(Isol), 2),
        entry(Some(Init), Some(Fina), 1),
        entry(Some(Init), Some(Fina), 3),
        entry(Some(Init), Some(Fina), 4),
        entry(Some(Init), Some(Fina), 6),
    ],
    // 3: previous was D in FINA form, willing to join.
    [
        entry(None, None, 0),
        entry(None, Some(Isol), 2),
        entry(Some(Medi), Some(Fina), 1),
        entry(Some(Medi), Some(Fina), 3),
        entry(Some(Medi), Some(Fina), 4),
        entry(Some(Medi), Some(Fina), 6),
    ],
    // 4: previous was FINA ALAPH, not willing to join.
    [
        entry(None, None, 0),
        entry(None, Some(Isol), 2),
        entry(Some(Med2), Some(Isol), 1),
        entry(Some(Med2), Some(Isol), 2),
        entry(Some(Med2), Some(Fin2), 5),
        entry(Some(Med2), Some(Isol), 6),
    ],
    // 5: previous was FIN2/FIN3 ALAPH, not willing to join.
    [
        entry(None, None, 0),
        entry(None, Some(Isol), 2),
        entry(Some(Isol), Some(Isol), 1),
        entry(Some(Isol), Some(Isol), 2),
        entry(Some(Isol), Some(Fin2), 5),
        entry(Some(Isol), Some(Isol), 6),
    ],
    // 6: previous was DALATH/RISH, not willing to join.
    [
        entry(None, None, 0),
        entry(None, Some(Isol), 2),
        entry(None, Some(Isol), 1),
        entry(None, Some(Isol), 2),
        entry(None, Some(Fin3), 5),
        entry(None, Some(Isol), 6),
    ],
];

/// HarfBuzz's `arabic_joining`: the form of each of `chars`, given the
/// characters before (`pre`, closest first) and after (`post`) the run.
pub(crate) fn actions(chars: &[char], pre: &[char], post: &[char]) -> Vec<Option<Action>> {
    let mut actions = vec![None; chars.len()];
    let mut state = pre
        .iter()
        .find_map(|&c| JoiningType::of(c).column())
        .map_or(0, |column| STATES[0][column].2);
    let mut previous: Option<usize> = None;
    for (index, &c) in chars.iter().enumerate() {
        let Some(column) = JoiningType::of(c).column() else {
            continue;
        };
        let (previous_action, current_action, next) = STATES[state][column];
        if let (Some(action), Some(p)) = (previous_action, previous) {
            actions[p] = Some(action);
        }
        actions[index] = current_action;
        previous = Some(index);
        state = next;
    }
    if let (Some(column), Some(p)) = (
        post.iter().find_map(|&c| JoiningType::of(c).column()),
        previous,
    ) && let (Some(action), _, _) = STATES[state][column]
    {
        actions[p] = Some(action);
    }
    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joining_types() {
        assert_eq!(JoiningType::of('\u{628}'), JoiningType::D); // BEH
        assert_eq!(JoiningType::of('\u{627}'), JoiningType::R); // ALEF
        assert_eq!(JoiningType::of('\u{621}'), JoiningType::U); // HAMZA
        assert_eq!(JoiningType::of('\u{64E}'), JoiningType::T); // FATHA
        assert_eq!(JoiningType::of('\u{200D}'), JoiningType::D); // ZWJ joins
        assert_eq!(JoiningType::of('\u{200C}'), JoiningType::U); // ZWNJ
        assert_eq!(JoiningType::of('\u{710}'), JoiningType::Alaph);
        assert_eq!(JoiningType::of('\u{715}'), JoiningType::DalathRish);
        assert_eq!(JoiningType::of('\u{301}'), JoiningType::T);
        assert_eq!(JoiningType::of('a'), JoiningType::U);
    }

    #[test]
    fn forms_of_a_word() {
        // MEEM REH HAH BEH ALEF: init fina init medi fina.
        let word: Vec<char> = "\u{645}\u{631}\u{62D}\u{628}\u{627}".chars().collect();
        assert_eq!(
            actions(&word, &[], &[]),
            vec![Some(Init), Some(Fina), Some(Init), Some(Medi), Some(Fina)]
        );
        // Marks are transparent; a lone letter is isolated.
        let marked: Vec<char> = "\u{628}\u{64E}\u{628}".chars().collect();
        assert_eq!(
            actions(&marked, &[], &[]),
            vec![Some(Init), None, Some(Fina)]
        );
        assert_eq!(actions(&['\u{628}'], &[], &[]), vec![Some(Isol)]);
        assert_eq!(actions(&['a', ' '], &[], &[]), vec![None, None]);
    }

    #[test]
    fn context_continues_joining() {
        let beh = ['\u{628}'];
        assert_eq!(actions(&beh, &['\u{628}'], &[]), vec![Some(Fina)]);
        assert_eq!(
            actions(&beh, &[], &['\u{64E}', '\u{628}']),
            vec![Some(Init)]
        );
        assert_eq!(actions(&beh, &['\u{628}'], &['\u{628}']), vec![Some(Medi)]);
        assert_eq!(actions(&beh, &['\u{627}'], &[]), vec![Some(Isol)]);
    }

    #[test]
    fn syriac_alaph() {
        // BETH ALAPH: the final alaph takes `fin2` after a non-joining
        // letter, `fina` after a joining one.
        let text: Vec<char> = "\u{712}\u{710}".chars().collect();
        assert_eq!(actions(&text, &[], &[]), vec![Some(Init), Some(Fina)]);
        let text: Vec<char> = "\u{715}\u{710}".chars().collect();
        assert_eq!(actions(&text, &[], &[]), vec![Some(Isol), Some(Fin3)]);
        assert_eq!((Isol.index(), Init.index()), (0, 6));
    }
}
