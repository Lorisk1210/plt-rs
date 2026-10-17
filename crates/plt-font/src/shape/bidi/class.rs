//! Bidi character properties: classes, paired brackets and mirroring.

use crate::shape::ucd_tables::{BIDI_CLASSES, BRACKETS, MIRRORS};

/// A Unicode bidirectional character type (`Bidi_Class`), in the order of
/// the generator's codes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[allow(
    clippy::upper_case_acronyms,
    reason = "UAX #9's short type names keep the rules recognizable"
)]
pub(crate) enum BidiClass {
    #[default]
    L,
    R,
    AL,
    EN,
    ES,
    ET,
    AN,
    CS,
    NSM,
    BN,
    B,
    S,
    WS,
    ON,
    LRE,
    LRO,
    RLE,
    RLO,
    PDF,
    LRI,
    RLI,
    FSI,
    PDI,
}

impl BidiClass {
    /// Every class, indexed by the codes of `BIDI_CLASSES`.
    const ALL: [Self; 23] = [
        Self::L,
        Self::R,
        Self::AL,
        Self::EN,
        Self::ES,
        Self::ET,
        Self::AN,
        Self::CS,
        Self::NSM,
        Self::BN,
        Self::B,
        Self::S,
        Self::WS,
        Self::ON,
        Self::LRE,
        Self::LRO,
        Self::RLE,
        Self::RLO,
        Self::PDF,
        Self::LRI,
        Self::RLI,
        Self::FSI,
        Self::PDI,
    ];

    /// Bidi class of `c`.
    pub(crate) fn of(c: char) -> Self {
        let code = u32::from(c);
        let index = BIDI_CLASSES.partition_point(|&(start, _)| start <= code);
        BIDI_CLASSES
            .get(index.wrapping_sub(1))
            .and_then(|&(_, class)| Self::ALL.get(usize::from(class)))
            .copied()
            .unwrap_or_default()
    }

    /// LRI, RLI or FSI.
    pub(crate) const fn is_isolate_initiator(self) -> bool {
        matches!(self, Self::LRI | Self::RLI | Self::FSI)
    }

    /// Characters removed by rule X9 (embeddings, overrides, PDF and BN).
    pub(crate) const fn is_removed_by_x9(self) -> bool {
        matches!(
            self,
            Self::RLE | Self::LRE | Self::RLO | Self::LRO | Self::PDF | Self::BN
        )
    }

    /// Neutral and isolate formatting types (UAX #9's `NI`).
    pub(crate) const fn is_neutral_or_isolate(self) -> bool {
        matches!(
            self,
            Self::B | Self::S | Self::WS | Self::ON | Self::LRI | Self::RLI | Self::FSI | Self::PDI
        )
    }

    /// The strong direction a type counts as in rules N0–N2 (numbers count
    /// as R); `None` for other types.
    pub(crate) const fn strong_direction(self) -> Option<Self> {
        match self {
            Self::L => Some(Self::L),
            Self::R | Self::AL | Self::EN | Self::AN => Some(Self::R),
            _ => None,
        }
    }
}

/// Canonical equivalents of the angle brackets (U+2329/U+232A decompose to
/// U+3008/U+3009), so that either form pairs with the other.
fn canonical_bracket(c: u32) -> u32 {
    match c {
        0x2329 => 0x3008,
        0x232A => 0x3009,
        _ => c,
    }
}

/// A paired bracket (`Bidi_Paired_Bracket_Type` open or close).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Bracket {
    /// The bracket's pair, canonicalized.
    pub(crate) pair: u32,
    /// Whether this is the opening bracket.
    pub(crate) opening: bool,
}

impl Bracket {
    /// Bracket properties of `c`.
    pub(crate) fn of(c: char) -> Option<Self> {
        let code = u32::from(c);
        let index = BRACKETS
            .binary_search_by_key(&code, |&(bracket, _, _)| bracket)
            .ok()?;
        let (_, pair, opening) = BRACKETS[index];
        Some(Self {
            pair: canonical_bracket(pair),
            opening,
        })
    }

    /// Whether `closing` closes an opening bracket with these properties.
    pub(crate) fn is_closed_by(self, closing: char) -> bool {
        self.opening && self.pair == canonical_bracket(u32::from(closing))
    }
}

/// The `Bidi_Mirroring_Glyph` of `c`, if any.
pub(crate) fn mirror(c: char) -> Option<char> {
    let code = u32::from(c);
    let index = MIRRORS.binary_search_by_key(&code, |&(c, _)| c).ok()?;
    char::from_u32(MIRRORS[index].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes() {
        assert_eq!(BidiClass::of('a'), BidiClass::L);
        assert_eq!(BidiClass::of('\u{5D0}'), BidiClass::R);
        assert_eq!(BidiClass::of('\u{628}'), BidiClass::AL);
        assert_eq!(BidiClass::of('1'), BidiClass::EN);
        assert_eq!(BidiClass::of('\u{661}'), BidiClass::AN);
        assert_eq!(BidiClass::of('+'), BidiClass::ES);
        assert_eq!(BidiClass::of('$'), BidiClass::ET);
        assert_eq!(BidiClass::of(','), BidiClass::CS);
        assert_eq!(BidiClass::of('\u{301}'), BidiClass::NSM);
        assert_eq!(BidiClass::of('\u{200D}'), BidiClass::BN);
        assert_eq!(BidiClass::of('\n'), BidiClass::B);
        assert_eq!(BidiClass::of('\t'), BidiClass::S);
        assert_eq!(BidiClass::of(' '), BidiClass::WS);
        assert_eq!(BidiClass::of('!'), BidiClass::ON);
        assert_eq!(BidiClass::of('\u{2067}'), BidiClass::RLI);
        assert_eq!(BidiClass::of('\u{202E}'), BidiClass::RLO);
        // Unassigned code points in right-to-left blocks default to R/AL.
        assert_eq!(BidiClass::of('\u{5FF}'), BidiClass::R);
        assert_eq!(BidiClass::of('\u{7BF}'), BidiClass::AL);
        assert!(BidiClass::FSI.is_isolate_initiator() && !BidiClass::PDI.is_isolate_initiator());
        assert!(BidiClass::BN.is_removed_by_x9() && !BidiClass::B.is_removed_by_x9());
        assert!(BidiClass::PDI.is_neutral_or_isolate() && !BidiClass::EN.is_neutral_or_isolate());
        assert_eq!(BidiClass::AN.strong_direction(), Some(BidiClass::R));
        assert_eq!(BidiClass::ON.strong_direction(), None);
    }

    #[test]
    fn brackets_and_mirrors() {
        let open = Bracket::of('(').unwrap();
        assert!(open.opening && open.is_closed_by(')'));
        assert!(!open.is_closed_by(']'));
        assert!(!Bracket::of(')').unwrap().opening);
        assert!(Bracket::of('\u{2329}').unwrap().is_closed_by('\u{3009}'));
        assert!(Bracket::of('\u{3008}').unwrap().is_closed_by('\u{232A}'));
        assert_eq!(Bracket::of('a'), None);
        assert_eq!(mirror('('), Some(')'));
        assert_eq!(mirror('\u{226E}'), Some('\u{226F}'));
        assert_eq!(mirror('a'), None);
    }
}
