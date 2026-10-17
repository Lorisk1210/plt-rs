//! Unicode general categories (HarfBuzz's `hb_unicode_general_category_t`).

use crate::shape::ucd_tables::GENERAL_CATEGORIES;

/// A Unicode general category, in the order of the generator's codes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum GeneralCategory {
    Control,
    Format,
    #[default]
    Unassigned,
    PrivateUse,
    Surrogate,
    LowercaseLetter,
    ModifierLetter,
    OtherLetter,
    TitlecaseLetter,
    UppercaseLetter,
    SpacingMark,
    EnclosingMark,
    NonspacingMark,
    DecimalNumber,
    LetterNumber,
    OtherNumber,
    ConnectPunctuation,
    DashPunctuation,
    ClosePunctuation,
    FinalPunctuation,
    InitialPunctuation,
    OtherPunctuation,
    OpenPunctuation,
    CurrencySymbol,
    ModifierSymbol,
    MathSymbol,
    OtherSymbol,
    LineSeparator,
    ParagraphSeparator,
    SpaceSeparator,
}

impl GeneralCategory {
    /// Every category, indexed by the codes of `GENERAL_CATEGORIES`.
    const ALL: [Self; 30] = [
        Self::Control,
        Self::Format,
        Self::Unassigned,
        Self::PrivateUse,
        Self::Surrogate,
        Self::LowercaseLetter,
        Self::ModifierLetter,
        Self::OtherLetter,
        Self::TitlecaseLetter,
        Self::UppercaseLetter,
        Self::SpacingMark,
        Self::EnclosingMark,
        Self::NonspacingMark,
        Self::DecimalNumber,
        Self::LetterNumber,
        Self::OtherNumber,
        Self::ConnectPunctuation,
        Self::DashPunctuation,
        Self::ClosePunctuation,
        Self::FinalPunctuation,
        Self::InitialPunctuation,
        Self::OtherPunctuation,
        Self::OpenPunctuation,
        Self::CurrencySymbol,
        Self::ModifierSymbol,
        Self::MathSymbol,
        Self::OtherSymbol,
        Self::LineSeparator,
        Self::ParagraphSeparator,
        Self::SpaceSeparator,
    ];

    /// General category of `c`.
    pub(crate) fn of(c: char) -> Self {
        let code = u32::from(c);
        let index = GENERAL_CATEGORIES.partition_point(|&(start, _)| start <= code);
        GENERAL_CATEGORIES
            .get(index.wrapping_sub(1))
            .and_then(|&(_, category)| Self::ALL.get(usize::from(category)))
            .copied()
            .unwrap_or_default()
    }

    /// Mn, Mc or Me.
    pub(crate) const fn is_mark(self) -> bool {
        matches!(
            self,
            Self::NonspacingMark | Self::SpacingMark | Self::EnclosingMark
        )
    }

    /// Lu, Ll, Lt, Lm or Lo.
    pub(crate) const fn is_letter(self) -> bool {
        matches!(
            self,
            Self::LowercaseLetter
                | Self::ModifierLetter
                | Self::OtherLetter
                | Self::TitlecaseLetter
                | Self::UppercaseLetter
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories() {
        assert_eq!(GeneralCategory::of('A'), GeneralCategory::UppercaseLetter);
        assert_eq!(GeneralCategory::of('a'), GeneralCategory::LowercaseLetter);
        assert_eq!(GeneralCategory::of('7'), GeneralCategory::DecimalNumber);
        assert_eq!(GeneralCategory::of(' '), GeneralCategory::SpaceSeparator);
        assert_eq!(
            GeneralCategory::of('\u{301}'),
            GeneralCategory::NonspacingMark
        );
        assert_eq!(GeneralCategory::of('\u{903}'), GeneralCategory::SpacingMark);
        assert_eq!(
            GeneralCategory::of('\u{20DD}'),
            GeneralCategory::EnclosingMark
        );
        assert_eq!(GeneralCategory::of('\u{200D}'), GeneralCategory::Format);
        assert_eq!(
            GeneralCategory::of('\u{0628}'),
            GeneralCategory::OtherLetter
        );
        assert_eq!(GeneralCategory::of('\u{0378}'), GeneralCategory::Unassigned);
        assert_eq!(GeneralCategory::of('\u{E000}'), GeneralCategory::PrivateUse);
        assert_eq!(
            GeneralCategory::of('\u{10FFFF}'),
            GeneralCategory::Unassigned
        );
        assert!(GeneralCategory::of('\u{64E}').is_mark());
        assert!(GeneralCategory::of('\u{2C6}').is_letter());
        assert!(!GeneralCategory::of('1').is_letter());
    }
}
