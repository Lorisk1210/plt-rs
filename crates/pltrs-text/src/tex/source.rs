//! The LaTeX document a TeX string is typeset in (matplotlib's
//! `TexManager._get_tex_source` and its font selection).

use std::collections::BTreeSet;

use pltrs_core::pyfmt;

use super::number::TexNumber;

/// `_check_cmsuper_installed`: fails the document when cm-super, which
/// the Computer Modern families need, is not installed.
const CHECK_CMSUPER: &str = r"\IfFileExists{type1ec.sty}{}{\PackageError{matplotlib-support}{Missing cm-super package, required by Matplotlib}{}}";

/// The generic families usetex knows (`_font_families`).
const FAMILIES: [&str; 4] = ["serif", "sans-serif", "cursive", "monospace"];

/// `_font_preambles`: the preamble selecting each font usetex knows.
fn font_preamble(font: &str) -> Option<&'static str> {
    Some(match font {
        "new century schoolbook" => r"\renewcommand{\rmdefault}{pnc}",
        "bookman" => r"\renewcommand{\rmdefault}{pbk}",
        "times" => r"\usepackage{mathptmx}",
        "palatino" => r"\usepackage{mathpazo}",
        "zapf chancery" | "cursive" => r"\usepackage{chancery}",
        "charter" => r"\usepackage{charter}",
        "serif" | "sans-serif" => "",
        "helvetica" => r"\usepackage{helvet}",
        "avant garde" => r"\usepackage{avant}",
        "courier" => r"\usepackage{courier}",
        "monospace"
        | "computer modern roman"
        | "computer modern sans serif"
        | "computer modern typewriter" => CHECK_CMSUPER,
        _ => return None,
    })
}

/// `_font_types`: the generic family of each font usetex knows.
fn font_type(font: &str) -> Option<&'static str> {
    Some(match font {
        "new century schoolbook"
        | "bookman"
        | "times"
        | "palatino"
        | "charter"
        | "computer modern roman" => "serif",
        "zapf chancery" => "cursive",
        "helvetica" | "avant garde" | "computer modern sans serif" => "sans-serif",
        "courier" | "computer modern typewriter" => "monospace",
        _ => return None,
    })
}

/// The font settings TeX text is typeset with: `font.family` and the
/// family lists (`font.serif`, `font.sans-serif`, `font.cursive`,
/// `font.monospace`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TexFonts {
    /// `font.family`.
    pub family: Vec<String>,
    /// `font.serif`.
    pub serif: Vec<String>,
    /// `font.sans-serif`.
    pub sans_serif: Vec<String>,
    /// `font.cursive`.
    pub cursive: Vec<String>,
    /// `font.monospace`.
    pub monospace: Vec<String>,
}

impl TexFonts {
    /// The list of `font.<family>`.
    fn list(&self, family: &str) -> &[String] {
        match family {
            "serif" => &self.serif,
            "sans-serif" => &self.sans_serif,
            "cursive" => &self.cursive,
            _ => &self.monospace,
        }
    }

    /// `_get_font_family_and_reduced`: the generic family asked for, and
    /// whether `font.family` names one font rather than a family (serif
    /// when it is neither).
    fn family_and_reduced(&self) -> (&'static str, bool) {
        if let [family] = self.family.as_slice() {
            let family = family.to_lowercase();
            if let Some(&generic) = FAMILIES.iter().find(|&&f| f == family) {
                return (generic, false);
            }
            if font_preamble(&family).is_some() {
                return (font_type(&family).unwrap_or("serif"), true);
            }
        }
        ("serif", false)
    }

    /// `_get_font_preamble_and_command`: the packages selecting the fonts
    /// and the command switching to the family asked for.
    pub fn preamble_and_command(&self) -> (String, &'static str) {
        let (requested, reduced) = self.family_and_reduced();
        let preamble_of = |family: &str| -> &'static str {
            if reduced && family == requested {
                let font = self.family[0].to_lowercase();
                return font_preamble(&font).unwrap_or_default();
            }
            self.list(family)
                .iter()
                .find_map(|font| font_preamble(&font.to_lowercase()))
                .or_else(|| font_preamble(family))
                .unwrap_or_default()
        };
        let mut commands: BTreeSet<&str> = ["serif", "sans-serif", "monospace"]
            .into_iter()
            .map(preamble_of)
            .collect();
        if requested == "cursive" {
            commands.insert(preamble_of("cursive"));
        }
        commands.insert(r"\usepackage{type1cm}");
        let preamble = commands.into_iter().collect::<Vec<_>>().join("\n");
        let command = match requested {
            "sans-serif" => r"\sffamily",
            "monospace" => r"\ttfamily",
            _ => r"\rmfamily",
        };
        (preamble, command)
    }
}

/// `_usepackage_if_not_loaded`: loads `package` (with `option`) unless the
/// custom preamble already did.
///
/// ```
/// use pltrs_text::tex::usepackage_if_not_loaded;
/// assert_eq!(
///     usepackage_if_not_loaded("psfrag", None),
///     r"\makeatletter\@ifpackageloaded{psfrag}{}{\usepackage{psfrag}}\makeatother"
/// );
/// ```
pub fn usepackage_if_not_loaded(package: &str, option: Option<&str>) -> String {
    let option = option.map(|o| format!("[{o}]")).unwrap_or_default();
    format!(
        r"\makeatletter\@ifpackageloaded{{{package}}}{{}}{{\usepackage{option}{{{package}}}}}\makeatother"
    )
}

/// `_get_tex_source`: the whole document typesetting `tex` at `fontsize`
/// points (written as given, see [`TexNumber`]) with `fonts` and the
/// user's `preamble` (`text.latex.preamble`).
pub fn tex_source(
    tex: &str,
    fontsize: impl Into<TexNumber>,
    fonts: &TexFonts,
    preamble: &str,
) -> String {
    let (font_preamble, fontcmd) = fonts.preamble_and_command();
    let fontsize = fontsize.into();
    let baselineskip = 1.25 * fontsize.value();
    [
        r"\RequirePackage{fix-cm}".to_owned(),
        r"\documentclass{article}".to_owned(),
        r"% Pass-through \mathdefault, which is used in non-usetex mode".to_owned(),
        r"% to use the default text font but was historically suppressed".to_owned(),
        r"% in usetex mode.".to_owned(),
        r"\newcommand{\mathdefault}[1]{#1}".to_owned(),
        font_preamble,
        r"\usepackage[utf8]{inputenc}".to_owned(),
        r"\DeclareUnicodeCharacter{2212}{\ensuremath{-}}".to_owned(),
        r"% geometry is loaded before the custom preamble as ".to_owned(),
        r"% convert_psfrags relies on a custom preamble to change the ".to_owned(),
        r"% geometry.".to_owned(),
        r"\usepackage[papersize=72in, margin=1in]{geometry}".to_owned(),
        preamble.to_owned(),
        r"% Use `underscore` package to take care of underscores in text.".to_owned(),
        r"% The [strings] option allows to use underscores in file names.".to_owned(),
        usepackage_if_not_loaded("underscore", Some("strings")),
        r"% Custom packages (e.g. newtxtext) may already have loaded ".to_owned(),
        r"% textcomp with different options.".to_owned(),
        usepackage_if_not_loaded("textcomp", None),
        r"\pagestyle{empty}".to_owned(),
        r"\begin{document}".to_owned(),
        r"% The empty hbox ensures that a page is printed even for empty".to_owned(),
        r"% inputs, except when using psfrag which gets confused by it.".to_owned(),
        format!(r"\fontsize{{{fontsize}}}{{{}}}%", pyfmt::repr(baselineskip)),
        r"\ifdefined\psfrag\else\hbox{}\fi%".to_owned(),
        format!("{{{fontcmd} {tex}}}%"),
        r"\end{document}".to_owned(),
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fonts(family: &str, list: &str, fonts: &[&str]) -> TexFonts {
        let mut rc = TexFonts {
            family: vec![family.to_owned()],
            ..TexFonts::default()
        };
        let fonts: Vec<String> = fonts.iter().map(|&f| f.to_owned()).collect();
        match list {
            "serif" => rc.serif = fonts,
            "sans-serif" => rc.sans_serif = fonts,
            "cursive" => rc.cursive = fonts,
            "monospace" => rc.monospace = fonts,
            _ => {}
        }
        rc
    }

    /// matplotlib's `test_font_selection`.
    #[test]
    fn fonts_are_selected_as_matplotlib_selects_them() {
        for (rc, preamble, family) in [
            (
                fonts("sans-serif", "sans-serif", &["helvetica"]),
                r"\usepackage{helvet}",
                r"\sffamily",
            ),
            (
                fonts("serif", "serif", &["palatino"]),
                r"\usepackage{mathpazo}",
                r"\rmfamily",
            ),
            (
                fonts("cursive", "cursive", &["zapf chancery"]),
                r"\usepackage{chancery}",
                r"\rmfamily",
            ),
            (
                fonts("monospace", "monospace", &["courier"]),
                r"\usepackage{courier}",
                r"\ttfamily",
            ),
            (
                fonts("helvetica", "", &[]),
                r"\usepackage{helvet}",
                r"\sffamily",
            ),
            (
                fonts("palatino", "", &[]),
                r"\usepackage{mathpazo}",
                r"\rmfamily",
            ),
            (
                fonts("zapf chancery", "", &[]),
                r"\usepackage{chancery}",
                r"\rmfamily",
            ),
            (
                fonts("courier", "", &[]),
                r"\usepackage{courier}",
                r"\ttfamily",
            ),
        ] {
            let source = tex_source("hello, world", 12.0, &rc, "");
            assert!(source.contains(preamble), "{rc:?}");
            let families: Vec<&str> = source
                .match_indices("family")
                .filter_map(|(at, _)| source[..at].rfind('\\').map(|start| &source[start..at + 6]))
                .collect();
            assert_eq!(families, [family], "{rc:?}");
        }
    }

    #[test]
    fn sources_hold_the_size_and_preamble() {
        let rc = fonts("DejaVu Sans", "", &[]);
        let plain = tex_source("x", 10.0, &rc, "");
        assert!(plain.contains("\\fontsize{10.0}{12.5}%\n"));
        assert!(plain.contains("{\\rmfamily x}%\n\\end{document}"));
        assert!(plain.contains(&format!("{CHECK_CMSUPER}\n\\usepackage{{type1cm}}")));
        let custom = tex_source("x", 10.0, &rc, r"\usepackage{txfonts}");
        assert_ne!(plain, custom, "matplotlib's test_fontconfig_preamble");
        assert!(custom.contains("\n\\usepackage{txfonts}\n"));
        assert!(plain.contains(
            r"\makeatletter\@ifpackageloaded{underscore}{}{\usepackage[strings]{underscore}}\makeatother"
        ));
    }
}
