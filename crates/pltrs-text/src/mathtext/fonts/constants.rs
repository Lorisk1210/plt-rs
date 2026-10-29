//! Per-fontset layout constants (matplotlib's `FontConstantsBase` and its
//! subclasses). The values are written as matplotlib computes them so the
//! floating-point results agree bit for bit.

/// Layout constants that cannot be read reliably from the fonts. Most are
/// multiples of the x-height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FontConstants {
    /// Horizontal space after sub/superscripts.
    pub(crate) script_space: f64,
    /// Superscript drop below the top of a large nucleus.
    pub(crate) supdrop: f64,
    /// Subscript drop below the bottom of a large nucleus.
    pub(crate) subdrop: f64,
    /// Superscript raise.
    pub(crate) sup1: f64,
    /// Subscript drop.
    pub(crate) sub1: f64,
    /// Subscript drop when there is also a superscript.
    pub(crate) sub2: f64,
    /// Script offset from an upright nucleus.
    pub(crate) delta: f64,
    /// Extra superscript offset for slanted nuclei.
    pub(crate) delta_slanted: f64,
    /// Script offset for integrals.
    pub(crate) delta_integral: f64,
    /// Numerator shift in display style.
    pub(crate) num1: f64,
    /// Numerator shift in text style with a rule.
    pub(crate) num2: f64,
    /// Numerator shift in text style without a rule.
    pub(crate) num3: f64,
    /// Denominator shift in display style.
    pub(crate) denom1: f64,
    /// Denominator shift in text style.
    pub(crate) denom2: f64,
    /// Math axis height as a multiple of the font size, if fixed.
    pub(crate) axis_height: Option<f64>,
    /// Quad width as a multiple of the font size, if fixed.
    pub(crate) quad: Option<f64>,
    /// x-height as a multiple of the font size, if fixed.
    pub(crate) x_height: Option<f64>,
}

impl FontConstants {
    /// `FontConstantsBase`, used by the custom fontset.
    pub(crate) const BASE: Self = Self {
        script_space: 0.05,
        supdrop: 0.4,
        subdrop: 0.4,
        sup1: 0.7,
        sub1: 0.3,
        sub2: 0.5,
        delta: 0.025,
        delta_slanted: 0.2,
        delta_integral: 0.1,
        num1: 1.4,
        num2: 1.5,
        num3: 1.3,
        denom1: 1.3,
        denom2: 1.1,
        axis_height: None,
        quad: None,
        x_height: None,
    };

    /// `ComputerModernFontConstants` (from the `cmsy10.tfm` metrics).
    pub(crate) fn computer_modern() -> Self {
        let x_height = 451_470.0;
        let design = 1_048_576.0;
        Self {
            script_space: 0.132_861_328_125,
            supdrop: 404_864.0 / x_height,
            subdrop: 52_429.0 / x_height,
            sup1: 432_949.0 / x_height,
            sub1: 157_286.0 / x_height,
            sub2: 259_226.0 / x_height,
            delta: 0.132_861_328_125,
            delta_slanted: 0.3,
            delta_integral: 0.3,
            num1: 709_370.0 / x_height,
            num2: 412_858.0 / x_height,
            num3: 465_286.0 / x_height,
            denom1: 719_272.0 / x_height,
            denom2: 361_592.0 / x_height,
            axis_height: Some(262_144.0 / design),
            quad: Some(1_048_579.0 / design),
            x_height: Some(x_height / design),
        }
    }

    /// `STIXFontConstants` (from the TeX table of `STIXGeneral.ttf`), or
    /// `STIXSansFontConstants`.
    pub(crate) fn stix(sans: bool) -> Self {
        let x_height = 450.0;
        Self {
            script_space: if sans { 0.05 } else { 0.1 },
            supdrop: 386.0 / x_height,
            subdrop: 50.0002 / x_height,
            sup1: 413.0 / x_height,
            sub1: 150.0 / x_height,
            sub2: 309.0 / x_height,
            delta: 0.05,
            delta_slanted: if sans { 0.6 } else { 0.3 },
            delta_integral: 0.3,
            num1: 747.0 / x_height,
            num2: 424.0 / x_height,
            num3: 474.0 / x_height,
            denom1: 756.0 / x_height,
            denom2: 375.0 / x_height,
            axis_height: Some(250.0 / 1000.0),
            quad: Some(1.0),
            x_height: Some(x_height / 1000.0),
        }
    }

    /// `DejaVuSansFontConstants` / `DejaVuSerifFontConstants` (from the
    /// fonts' TeX tables), which differ only in the x-height.
    pub(crate) fn dejavu(serif: bool) -> Self {
        let x_height = if serif { 1063.0 } else { 1120.0 };
        Self {
            supdrop: 790.527 / x_height,
            subdrop: 102.4 / x_height,
            sup1: 845.824 / x_height,
            sub1: 307.199 / x_height,
            sub2: 632.832 / x_height,
            num1: 1529.86 / x_height,
            num2: 868.352 / x_height,
            num3: 970.752 / x_height,
            denom1: 1548.29 / x_height,
            denom2: 768.0 / x_height,
            axis_height: Some(512.0 / 2048.0),
            x_height: Some(x_height / 2048.0),
            ..Self::BASE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_match_matplotlib() {
        let cm = FontConstants::computer_modern();
        assert_eq!(cm.sup1, 0.958_976_233_193_789_2);
        assert_eq!(cm.x_height, Some(0.430_555_343_627_929_7));
        assert_eq!(FontConstants::stix(true).delta_slanted, 0.6);
        assert_eq!(FontConstants::stix(false).script_space, 0.1);
        let serif = FontConstants::dejavu(true);
        assert_eq!(serif.denom2, 768.0 / 1063.0);
        assert_eq!(serif.quad, None);
        assert_eq!(FontConstants::BASE.axis_height, None);
    }
}
