//! The rcParams that tick locators and formatters consult.

/// How `view_limits` treats data limits (`axes.autolimit_mode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AutolimitMode {
    /// Use the data limits as they are (`"data"`).
    #[default]
    Data,
    /// Expand the limits outwards to the nearest tick (`"round_numbers"`).
    RoundNumbers,
}

/// Number of minor intervals between major ticks (`xtick.minor.ndivs`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MinorDivisions {
    /// 5 when the major step is 1, 2.5, 5 or 10 (times a power of ten),
    /// otherwise 4 (`"auto"`).
    #[default]
    Auto,
    /// A fixed number of subdivisions.
    Count(u32),
}

/// The subset of rcParams that influences tick placement and labelling.
///
/// Field names follow the rcParam keys; [`TickerSettings::DEFAULT`] holds
/// matplotlib's defaults. Some settings are read when a locator or
/// formatter is constructed (`from_settings` constructors), the others when
/// ticks are computed (through [`crate::ticker::TickContext`]), mirroring
/// when matplotlib reads each rcParam.
#[allow(
    clippy::struct_excessive_bools,
    reason = "mirrors independent boolean rcParams"
)]
#[derive(Clone, Debug, PartialEq)]
pub struct TickerSettings {
    /// `axes.unicode_minus`: typeset minus signs as U+2212.
    pub axes_unicode_minus: bool,
    /// `axes.formatter.limits`: exponents beyond which `ScalarFormatter`
    /// switches to scientific notation.
    pub axes_formatter_limits: (i32, i32),
    /// `axes.formatter.offset_threshold`: digits an offset must save.
    pub axes_formatter_offset_threshold: i32,
    /// `axes.formatter.useoffset`: whether `ScalarFormatter` may use an offset.
    pub axes_formatter_useoffset: bool,
    /// `axes.formatter.use_mathtext`: typeset scientific notation as mathtext.
    pub axes_formatter_use_mathtext: bool,
    /// `axes.formatter.min_exponent`: smallest exponent that
    /// `LogFormatterMathtext` writes as a power.
    pub axes_formatter_min_exponent: i32,
    /// `text.usetex`: labels are rendered by TeX.
    pub text_usetex: bool,
    /// `axes.autolimit_mode`.
    pub axes_autolimit_mode: AutolimitMode,
    /// `_internal.classic_mode`: matplotlib 1.x behaviour.
    pub classic_mode: bool,
    /// `xtick.minor.ndivs`.
    pub xtick_minor_ndivs: MinorDivisions,
    /// `ytick.minor.ndivs`.
    pub ytick_minor_ndivs: MinorDivisions,
    /// `xtick.minor.visible`: linear x axes get an `AutoMinorLocator`.
    pub xtick_minor_visible: bool,
    /// `ytick.minor.visible`: linear y axes get an `AutoMinorLocator`.
    pub ytick_minor_visible: bool,
}

impl TickerSettings {
    /// matplotlib's default rcParams.
    pub const DEFAULT: Self = Self {
        axes_unicode_minus: true,
        axes_formatter_limits: (-5, 6),
        axes_formatter_offset_threshold: 4,
        axes_formatter_useoffset: true,
        axes_formatter_use_mathtext: false,
        axes_formatter_min_exponent: 0,
        text_usetex: false,
        axes_autolimit_mode: AutolimitMode::Data,
        classic_mode: false,
        xtick_minor_ndivs: MinorDivisions::Auto,
        ytick_minor_ndivs: MinorDivisions::Auto,
        xtick_minor_visible: false,
        ytick_minor_visible: false,
    };

    /// Replaces `-` by U+2212 MINUS SIGN when `axes.unicode_minus` is on
    /// (matplotlib's `Formatter.fix_minus`).
    pub fn fix_minus(&self, text: &str) -> String {
        if self.axes_unicode_minus {
            text.replace('-', "\u{2212}")
        } else {
            text.to_owned()
        }
    }
}

impl Default for TickerSettings {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl From<&crate::rc::RcParams> for TickerSettings {
    /// The ticker-relevant subset of the given rcParams.
    fn from(rc: &crate::rc::RcParams) -> Self {
        let limit = |i: usize| {
            rc.axes_formatter_limits
                .get(i)
                .map_or(0, |&v| i32::try_from(v).unwrap_or(0))
        };
        let ndivs = |text: &str| {
            text.parse()
                .map_or(MinorDivisions::Auto, MinorDivisions::Count)
        };
        Self {
            axes_unicode_minus: rc.axes_unicode_minus,
            axes_formatter_limits: (limit(0), limit(1)),
            axes_formatter_offset_threshold: i32::try_from(rc.axes_formatter_offset_threshold)
                .unwrap_or(4),
            axes_formatter_useoffset: rc.axes_formatter_useoffset,
            axes_formatter_use_mathtext: rc.axes_formatter_use_mathtext,
            axes_formatter_min_exponent: i32::try_from(rc.axes_formatter_min_exponent).unwrap_or(0),
            text_usetex: rc.text_usetex,
            axes_autolimit_mode: if rc.axes_autolimit_mode == "round_numbers" {
                AutolimitMode::RoundNumbers
            } else {
                AutolimitMode::Data
            },
            classic_mode: false,
            xtick_minor_ndivs: ndivs(&rc.xtick_minor_ndivs),
            ytick_minor_ndivs: ndivs(&rc.ytick_minor_ndivs),
            xtick_minor_visible: rc.xtick_minor_visible,
            ytick_minor_visible: rc.ytick_minor_visible,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fix_minus_follows_unicode_minus() {
        let mut settings = TickerSettings::default();
        assert_eq!(settings.fix_minus("-1e-5"), "\u{2212}1e\u{2212}5");
        settings.axes_unicode_minus = false;
        assert_eq!(settings.fix_minus("-1e-5"), "-1e-5");
    }
}
