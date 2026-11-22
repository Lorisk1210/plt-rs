//! The look of figures and of the windows that show them.
//!
//! Figures use the modern look by default. The modern look is the style
//! sheet `pltrs-modern`. It uses the Inter font, a thin grid behind the
//! data, no top or right spine, left-aligned titles and constrained
//! layout. Its color palette is safe for readers with a color-vision
//! deficiency. Math also uses Inter, and 3-D panes use the grays of the
//! window chrome. Windows use the modern chrome. You can select the
//! matplotlib look for figures, for windows or for both.
//!
//! The looks set some values that matplotlib fixes in code through `pltrs.*`
//! parameters. Their defaults give the matplotlib behavior:
//!
//! * `pltrs.ink`: the color of marks that matplotlib draws black (quiver
//!   arrows, barbs, error bars of bar charts, size bars)
//! * `pltrs.table.*` and `pltrs.offsetbox.*`: the faces and edges of table
//!   cells and annotation boxes
//! * `pltrs.widgets.*`: the colors and frames of widgets
//! * `pltrs.tight_layout.keep_constrained`: whether `tight_layout` keeps
//!   constrained layout
//! * `pltrs.mathtext.dejavu_symbols`: the math symbols with Inter
//!   (display-size sums and integrals, and sans-serif symbols that Inter
//!   does not have)
//! * `pltrs.inset.frame`: framed insets
//! * `pltrs.axes.twin_grid`: whether twin axes have their own grid
//!
//! Code selects the look with [`set_appearance`], [`set_figure_look`] or
//! [`set_chrome_look`]. Until code selects a look, these environment
//! variables select it:
//!
//! | variable            | values                                | selects          |
//! |---------------------|---------------------------------------|------------------|
//! | `PLTRS_LOOK`        | `modern`, `matplotlib`                | figures, windows |
//! | `PLTRS_FIGURE_LOOK` | `modern`, `modern-dark`, `matplotlib` | figures          |
//! | `PLTRS_CHROME_LOOK` | `modern`, `matplotlib`                | windows          |
//!
//! The two specific variables override `PLTRS_LOOK`. For example,
//! `PLTRS_LOOK=matplotlib PLTRS_CHROME_LOOK=modern` gives matplotlib
//! figures in modern windows. pltrs ignores unknown values, and
//! [`Appearance::from_env`] reports them. [`set_default`] replaces the
//! modern look as the default for a program that serves under the name of
//! another library. For example, pltrs (Python) imported as `matplotlib`
//! uses the matplotlib look.
//!
//! Threads start from the parameters of the figure look
//! ([`RcParams::startup`]). The user file `pltrsrc` applies over them, as
//! `matplotlibrc` applies over the matplotlib defaults. pltrs finds
//! `pltrsrc` in the working directory, through `PLTRSRC`, or in the
//! configuration directory (`PLTRSCONFIGDIR` or `~/.config/pltrs`, see
//! [`config`](crate::config)). [`rc::reset`](crate::rc::reset) restores
//! the matplotlib defaults. Style sheets apply over either look:
//!
//! ```
//! use pltrs::look::FigureLook;
//! assert!(FigureLook::Modern.params().axes_grid);
//! assert!(!FigureLook::Matplotlib.params().axes_grid);
//! ```
//!
//! ```
//! use pltrs::look::{self, Appearance, ChromeLook, FigureLook};
//! // The matplotlib look for figures, the modern look for windows.
//! look::set_appearance(Appearance {
//!     figures: FigureLook::Matplotlib,
//!     chrome: ChromeLook::Modern,
//! });
//! assert_eq!(look::appearance().figures, FigureLook::Matplotlib);
//! assert!(!pltrs::rc::params().axes_grid, "threads now start from matplotlib's defaults");
//! ```

use std::fmt;
use std::str::FromStr;
use std::sync::{PoisonError, RwLock};

use pltrs_core::Rgba;
use pltrs_ui::Theme;

use crate::rc::RcParams;
use crate::style::{self, StyleSheet};

/// How figures look by default: the style threads start from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FigureLook {
    /// pltrs' modern look (the `pltrs-modern` style sheet).
    Modern,
    /// The modern look on a charcoal background (`pltrs-modern-dark`).
    ModernDark,
    /// matplotlib's defaults.
    Matplotlib,
}

impl FigureLook {
    /// Every figure look.
    pub const ALL: [Self; 3] = [Self::Modern, Self::ModernDark, Self::Matplotlib];

    /// The look's name, as the environment variables spell it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Modern => "modern",
            Self::ModernDark => "modern-dark",
            Self::Matplotlib => "matplotlib",
        }
    }

    /// The style sheet the look applies over matplotlib's defaults, if any:
    /// the bundled one, which sheets of the same name in the user style
    /// library do not change (the user's `pltrsrc` applies over the look).
    pub fn style_sheet(self) -> Option<StyleSheet> {
        let name = match self {
            Self::Modern => "pltrs-modern",
            Self::ModernDark => "pltrs-modern-dark",
            Self::Matplotlib => return None,
        };
        Some(style::bundled_sheet(name).expect("the look sheets are bundled"))
    }

    /// matplotlib's defaults with the look's style sheet applied.
    pub fn params(self) -> RcParams {
        let mut params = RcParams::matplotlib();
        if let Some(sheet) = self.style_sheet() {
            sheet.apply(&mut params).expect("the look sheets are valid");
        }
        params
    }
}

impl fmt::Display for FigureLook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for FigureLook {
    type Err = LookError;

    fn from_str(text: &str) -> Result<Self, LookError> {
        parse_look(text, &Self::ALL, Self::name)
    }
}

/// How the windows around figures look.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChromeLook {
    /// pltrs' modern chrome: light, or dark around figures on dark
    /// backgrounds.
    Modern,
    /// A chrome close to matplotlib's Tk figure windows.
    Matplotlib,
}

impl ChromeLook {
    /// Every chrome look.
    pub const ALL: [Self; 2] = [Self::Modern, Self::Matplotlib];

    /// The look's name, as the environment variables spell it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Modern => "modern",
            Self::Matplotlib => "matplotlib",
        }
    }

    /// The UI theme of windows showing a figure drawn on `background`: in
    /// the modern look the dark theme below mid grey (on an opaque
    /// background), the light one otherwise.
    pub fn theme(self, background: Rgba) -> Theme {
        match self {
            Self::Matplotlib => Theme::classic(),
            Self::Modern if background.a >= 0.5 && luminance(background) < 0.18 => Theme::dark(),
            Self::Modern => Theme::modern(),
        }
    }
}

impl fmt::Display for ChromeLook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for ChromeLook {
    type Err = LookError;

    fn from_str(text: &str) -> Result<Self, LookError> {
        parse_look(text, &Self::ALL, Self::name)
    }
}

/// The relative luminance of an sRGB colour, as WCAG defines it.
fn luminance(color: Rgba) -> f64 {
    let linear = |c: f64| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

/// The look among `looks` called `text` (ignoring case and surrounding
/// space).
fn parse_look<L: Copy>(
    text: &str,
    looks: &[L],
    name: fn(L) -> &'static str,
) -> Result<L, LookError> {
    let wanted = text.trim();
    looks
        .iter()
        .copied()
        .find(|&look| name(look).eq_ignore_ascii_case(wanted))
        .ok_or_else(|| LookError {
            value: text.to_owned(),
            expected: looks.iter().map(|&look| name(look)).collect(),
        })
}

/// The looks of figures and of the windows that show them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Appearance {
    /// How figures look.
    pub figures: FigureLook,
    /// How windows look.
    pub chrome: ChromeLook,
}

impl Appearance {
    /// pltrs' modern figures in modern windows, the default.
    pub const MODERN: Self = Self {
        figures: FigureLook::Modern,
        chrome: ChromeLook::Modern,
    };

    /// matplotlib's figures in windows like matplotlib's.
    pub const MATPLOTLIB: Self = Self {
        figures: FigureLook::Matplotlib,
        chrome: ChromeLook::Matplotlib,
    };

    /// The default appearance with the choices of the `PLTRS_LOOK`,
    /// `PLTRS_FIGURE_LOOK` and `PLTRS_CHROME_LOOK` environment variables.
    ///
    /// # Errors
    ///
    /// When a variable holds an unknown look.
    pub fn from_env() -> Result<Self, LookError> {
        Self::default().with_vars(|name| std::env::var(name).ok())
    }

    /// This appearance with the choices of the variables `var` returns.
    fn with_vars(self, var: impl Fn(&str) -> Option<String>) -> Result<Self, LookError> {
        let mut appearance = self;
        if let Some(both) = var("PLTRS_LOOK") {
            appearance.figures = labelled("PLTRS_LOOK", both.parse())?;
            appearance.chrome = labelled("PLTRS_LOOK", both.parse())?;
        }
        if let Some(figures) = var("PLTRS_FIGURE_LOOK") {
            appearance.figures = labelled("PLTRS_FIGURE_LOOK", figures.parse())?;
        }
        if let Some(chrome) = var("PLTRS_CHROME_LOOK") {
            appearance.chrome = labelled("PLTRS_CHROME_LOOK", chrome.parse())?;
        }
        Ok(appearance)
    }
}

impl Default for Appearance {
    /// The [modern](Appearance::MODERN) appearance.
    fn default() -> Self {
        Self::MODERN
    }
}

/// `result` with its error naming the environment variable it came from.
fn labelled<L>(variable: &'static str, result: Result<L, LookError>) -> Result<L, LookError> {
    result.map_err(|err| LookError {
        value: format!("{variable}={}", err.value),
        ..err
    })
}

/// A name that is not a look.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookError {
    value: String,
    expected: Vec<&'static str>,
}

impl fmt::Display for LookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown look {:?} (expected {})",
            self.value,
            self.expected.join(", ")
        )
    }
}

impl std::error::Error for LookError {}

/// The appearance until code or the environment chooses one: the modern
/// one, except in this crate's unit tests, which check parity with
/// matplotlib's defaults.
const UNCHOSEN: Appearance = if cfg!(test) {
    Appearance::MATPLOTLIB
} else {
    Appearance::MODERN
};

/// The appearance in force, once resolved.
static CHOSEN: RwLock<Option<Appearance>> = RwLock::new(None);

/// The appearance in force: the one code set last, else the environment's
/// choice (unknown values ignored), else the modern one.
pub fn appearance() -> Appearance {
    if let Some(chosen) = *CHOSEN.read().unwrap_or_else(PoisonError::into_inner) {
        return chosen;
    }
    let resolved = from_environment(UNCHOSEN, |name| std::env::var(name).ok());
    *CHOSEN
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .get_or_insert(resolved)
}

/// Puts `default` in place of the modern appearance for the whole
/// program, under the environment's choices (`PLTRS_LOOK=modern` still
/// gives the modern look), and makes the result the appearance in force,
/// as [`set_appearance`] does. For a program that serves another
/// library's name before its own code runs: pltrs (Python) imported as
/// `matplotlib` takes [`Appearance::MATPLOTLIB`].
pub fn set_default(default: Appearance) {
    set_appearance(from_environment(default, |name| std::env::var(name).ok()));
}

/// `default` with the choices of the variables `var` returns, unknown
/// values ignored.
fn from_environment(default: Appearance, var: impl Fn(&str) -> Option<String>) -> Appearance {
    default.with_vars(var).unwrap_or(default)
}

/// Chooses the appearance for the whole program. When the figure look
/// changes, the current thread's parameters restart from it
/// ([`rc::restore_startup`](crate::rc::restore_startup)); threads started
/// later start from it, and other running threads keep theirs.
pub fn set_appearance(appearance: Appearance) {
    let previous = self::appearance();
    *CHOSEN.write().unwrap_or_else(PoisonError::into_inner) = Some(appearance);
    if previous.figures != appearance.figures {
        crate::rc::restore_startup();
    }
}

/// Chooses how figures look, keeping the chrome (see [`set_appearance`]).
pub fn set_figure_look(figures: FigureLook) {
    set_appearance(Appearance {
        figures,
        ..appearance()
    });
}

/// Chooses how windows look, keeping the figure look.
pub fn set_chrome_look(chrome: ChromeLook) {
    set_appearance(Appearance {
        chrome,
        ..appearance()
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|&(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
        }
    }

    #[test]
    fn the_environment_chooses_each_part() {
        let modern = Appearance::MODERN;
        assert_eq!(modern.with_vars(vars(&[])), Ok(modern));
        assert_eq!(
            modern.with_vars(vars(&[("PLTRS_LOOK", "Matplotlib")])),
            Ok(Appearance::MATPLOTLIB)
        );
        let mixed = modern.with_vars(vars(&[
            ("PLTRS_LOOK", "matplotlib"),
            ("PLTRS_CHROME_LOOK", " modern "),
        ]));
        assert_eq!(
            mixed,
            Ok(Appearance {
                figures: FigureLook::Matplotlib,
                chrome: ChromeLook::Modern,
            })
        );
        let dark = modern.with_vars(vars(&[("PLTRS_FIGURE_LOOK", "modern-dark")]));
        assert_eq!(dark.unwrap().figures, FigureLook::ModernDark);
    }

    #[test]
    fn the_environment_chooses_over_another_default() {
        let matplotlib = Appearance::MATPLOTLIB;
        assert_eq!(from_environment(matplotlib, vars(&[])), matplotlib);
        assert_eq!(
            from_environment(matplotlib, vars(&[("PLTRS_LOOK", "modern")])),
            Appearance::MODERN
        );
        assert_eq!(
            from_environment(matplotlib, vars(&[("PLTRS_FIGURE_LOOK", "modern-dark")])),
            Appearance {
                figures: FigureLook::ModernDark,
                chrome: ChromeLook::Matplotlib,
            }
        );
        // An unknown value leaves the default, as when pltrs starts.
        assert_eq!(
            from_environment(matplotlib, vars(&[("PLTRS_LOOK", "classic")])),
            matplotlib
        );
    }

    #[test]
    fn unknown_looks_are_reported_with_their_variable() {
        let err = Appearance::MODERN
            .with_vars(vars(&[("PLTRS_CHROME_LOOK", "modern-dark")]))
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "unknown look \"PLTRS_CHROME_LOOK=modern-dark\" (expected modern, matplotlib)"
        );
        // `modern-dark` is a figure look only.
        assert!(
            Appearance::MODERN
                .with_vars(vars(&[("PLTRS_LOOK", "modern-dark")]))
                .is_err()
        );
        assert!("classic".parse::<FigureLook>().is_err());
    }

    #[test]
    fn figure_looks_are_style_sheets_over_matplotlib() {
        assert_eq!(FigureLook::Matplotlib.params(), RcParams::matplotlib());
        let modern = FigureLook::Modern.params();
        assert!(modern.axes_grid && modern.figure_constrained_layout_use);
        assert!(!modern.axes_spines_top && !modern.axes_spines_right);
        assert_eq!(modern.font_sans_serif[0], "Inter");
        let dark = FigureLook::ModernDark.params();
        assert!(luminance(dark.figure_facecolor.resolve().unwrap()) < 0.18);
        for look in FigureLook::ALL {
            assert_eq!(look.name().parse::<FigureLook>(), Ok(look));
            let params = look.params();
            let modern = look != FigureLook::Matplotlib;
            assert_eq!(params.pltrs_mathtext_dejavu_symbols, modern, "{look}");
            assert_eq!(params.pltrs_inset_frame, modern, "{look}");
            assert_eq!(params.pltrs_axes_twin_grid, !modern, "{look}");
        }
    }

    #[test]
    fn the_dark_look_suits_its_background() {
        let dark = FigureLook::ModernDark.params();
        let lum = |color: &crate::colors::Color| luminance(color.resolve().expect("a colour"));
        let background = lum(&dark.figure_facecolor);
        assert!(lum(&dark.pltrs_ink) > 0.5, "marks matplotlib draws black");
        for surface in [
            &dark.axes3d_xaxis_panecolor,
            &dark.axes3d_yaxis_panecolor,
            &dark.axes3d_zaxis_panecolor,
            &dark.pltrs_table_facecolor,
            &dark.pltrs_offsetbox_facecolor,
            &dark.pltrs_widgets_button_color,
            &dark.pltrs_widgets_textbox_color,
        ] {
            assert!((lum(surface) - background).abs() < 0.05, "{surface:?}");
        }
    }

    #[test]
    fn what_matplotlib_fixes_in_code_follows_the_look() {
        use crate::colors::Color;
        use crate::quiver::QuiverOptions;
        use crate::widgets::{ButtonOptions, SliderOptions};

        crate::rc::context(|| {
            crate::rc::replace(FigureLook::ModernDark.params());
            let rc = crate::rc::params();
            assert_eq!(QuiverOptions::default().color, rc.pltrs_ink);
            assert_eq!(
                ButtonOptions::default().color,
                rc.pltrs_widgets_button_color
            );
            let slider = SliderOptions::default();
            assert_eq!(slider.track_color, rc.pltrs_widgets_slider_track_color);
        });
        // matplotlib's look keeps matplotlib's colours.
        assert_eq!(QuiverOptions::default().color, Color::from("black"));
        assert_eq!(ButtonOptions::default().color, Color::from("0.85"));
        assert_eq!(
            SliderOptions::default().track_color,
            Color::from("lightgrey")
        );
    }

    #[test]
    fn modern_chrome_follows_the_figure_background() {
        let white = Rgba::new(1.0, 1.0, 1.0, 1.0);
        let charcoal = Rgba::new(0.12, 0.13, 0.14, 1.0);
        let clear = Rgba::new(0.0, 0.0, 0.0, 0.0);
        assert_eq!(ChromeLook::Modern.theme(white), Theme::modern());
        assert_eq!(ChromeLook::Modern.theme(charcoal), Theme::dark());
        let mid_grey = Rgba::new(0.45, 0.45, 0.45, 1.0);
        assert_eq!(
            ChromeLook::Modern.theme(mid_grey),
            Theme::dark(),
            "below mid grey"
        );
        assert_eq!(
            ChromeLook::Modern.theme(clear),
            Theme::modern(),
            "transparent"
        );
        assert_eq!(ChromeLook::Matplotlib.theme(charcoal), Theme::classic());
    }

    #[test]
    fn unit_tests_start_from_matplotlib() {
        assert_eq!(UNCHOSEN, Appearance::MATPLOTLIB);
    }
}
