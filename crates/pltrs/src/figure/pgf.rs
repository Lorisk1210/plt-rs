//! Writing figures as PGF pictures for LaTeX documents (matplotlib's
//! `print_pgf`): images go beside the document as PNG files.

use std::path::Path as FsPath;

use pltrs_render::{PgfDocument, PgfOptions, PgfRenderer};

use super::Figure;
use super::pgf_metrics::measuring;
use super::print::SaveOptions;
use super::save::{SaveError, simplify_settings};
use crate::fonts::{FontExt, FontManager, FontProperties};
use crate::rc::{RcParams, keyword_names};

/// The LaTeX program a PGF picture is meant for (`pgf.texsystem`).
///
/// It does not change the picture: it is the program that typesets PGF
/// pictures into PDF documents ([`Figure::to_pgf_pdf`],
/// [`PgfPages`](super::PgfPages)), as matplotlib runs it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PgfTexSystem {
    /// `xelatex`, matplotlib's default.
    #[default]
    XeLaTeX,
    /// `lualatex`.
    LuaLaTeX,
    /// `pdflatex`.
    PdfLaTeX,
}

pltrs_core::archive_enum!(PgfTexSystem {
    XeLaTeX,
    LuaLaTeX,
    PdfLaTeX,
});

impl PgfTexSystem {
    const ALL: [Self; 3] = [Self::XeLaTeX, Self::LuaLaTeX, Self::PdfLaTeX];

    /// The program's name, as `pgf.texsystem` spells it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::XeLaTeX => "xelatex",
            Self::LuaLaTeX => "lualatex",
            Self::PdfLaTeX => "pdflatex",
        }
    }
}

keyword_names!(PgfTexSystem, "TeX system", Self::ALL);

impl Figure {
    /// The figure as a PGF picture (matplotlib's `savefig(format="pgf")`),
    /// to `\input` into a LaTeX document.
    ///
    /// # Errors
    ///
    /// When some artists cannot be drawn, or the figure has images, which
    /// only [`Figure::savefig`] can write (beside the document).
    pub fn to_pgf(&self) -> Result<String, SaveError> {
        self.to_pgf_with(SaveOptions::default())
    }

    /// The figure as a PGF picture with save options; see
    /// [`Figure::to_pgf`].
    ///
    /// # Errors
    ///
    /// As for [`Figure::to_pgf`].
    // Its own frame, not `to_format`'s: inlined there, every format's
    // renderer would share one frame of tens of kilobytes, which a save in
    // a thread of a small stack (musl's 128 KiB) cannot spare.
    #[inline(never)]
    pub fn to_pgf_with(&self, options: SaveOptions) -> Result<String, SaveError> {
        self.pgf_document(&options, None)
            .map(|document| document.text)
    }

    /// Writes the figure to `path` as a PGF picture, its images beside it
    /// as `<stem>-img<N>.png`.
    pub(super) fn save_pgf(&self, path: &FsPath, options: &SaveOptions) -> Result<(), SaveError> {
        let stem = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned());
        let document = self.pgf_document(options, stem)?;
        let directory = path.parent().unwrap_or_else(|| FsPath::new(""));
        for image in document.images {
            std::fs::write(directory.join(&image.name), image.png).map_err(SaveError::Io)?;
        }
        std::fs::write(path, document.text).map_err(SaveError::Io)
    }

    /// The figure as a PGF picture with the PNG files of its images,
    /// named `<image_stem>-img<N>.png`, for a writer of its own to put
    /// beside the picture's file (matplotlib's `print_pgf` to a file object
    /// with a name).
    ///
    /// # Errors
    ///
    /// As for [`Figure::to_pgf`]; without a stem, a figure with images.
    pub fn to_pgf_document(
        &self,
        options: &SaveOptions,
        image_stem: Option<String>,
    ) -> Result<PgfDocument, SaveError> {
        self.pgf_document(options, image_stem)
    }

    /// The document, its images named after `image_stem`, its text
    /// measured as `options` asks; the families its texts asked for that
    /// no font has are logged, as matplotlib's `draw_text` logs them.
    fn pgf_document(
        &self,
        options: &SaveOptions,
        image_stem: Option<String>,
    ) -> Result<PgfDocument, SaveError> {
        let pgf = pgf_options(image_stem);
        let mut renderer = measuring(options.pgf_text_metrics, || {
            self.render_at_save_resolution(options, |width, height, dpi| {
                PgfRenderer::new(width, height, dpi, pgf)
            })
        })?;
        for family in renderer.take_ignored_fonts() {
            log_unknown_font(&family);
        }
        renderer.finish().map_err(SaveError::Vector)
    }
}

/// The family names of the installed fonts, which `pgf.rcfonts` selects
/// texts' fonts by (matplotlib's `fontManager.ttflist`).
fn font_names() -> Vec<String> {
    FontManager::global()
        .font_list(FontExt::Ttf)
        .into_iter()
        .map(|entry| entry.name)
        .collect()
}

/// The options the font commands of texts are written with under `rc`.
fn text_options(rc: &RcParams) -> PgfOptions {
    PgfOptions {
        rcfonts: rc.pgf_rcfonts,
        font_names: font_names(),
        ..PgfOptions::default()
    }
}

/// Logs `family`, which no font has, as matplotlib's PGF backend logs it
/// (`_escape_and_apply_props`): for each text asking for it that a PGF
/// picture draws (see [`PgfRenderer::take_ignored_fonts`]) or measures.
pub fn log_unknown_font(family: &str) {
    crate::warnings::log(
        "backends.backend_pgf",
        &format!("Ignoring unknown font: {family}"),
    );
}

/// The options of a PGF picture whose images go beside a file named after
/// `image_stem`, as the `pgf.*` and `image.composite_image` rcParams
/// configure figures.
pub(super) fn pgf_options(image_stem: Option<String>) -> PgfOptions {
    let rc = crate::rc::params();
    PgfOptions {
        simplify: simplify_settings(),
        composite_images: rc.image_composite_image,
        rcfonts: rc.pgf_rcfonts,
        font_names: font_names(),
        preamble: pgf_preamble(&rc),
        image_stem,
    }
}

/// The TeX source matplotlib's PGF backend typesets `text` with in the
/// font `props` describe under `rc` (its `_escape_and_apply_props`; see
/// [`pltrs_render::pgf_text`]), as `LatexManager` measures it.
///
/// ```
/// use pltrs::fonts::FontProperties;
/// use pltrs::rc::RcParams;
/// let rc = RcParams::matplotlib();
/// let tex = pltrs::figure::pgf_text("x", &FontProperties::from_rc(&rc), &rc);
/// assert!(tex.starts_with(r"{\sffamily\fontsize{10.000000}{12.000000}\selectfont"));
/// ```
pub fn pgf_text(text: &str, props: &FontProperties, rc: &RcParams) -> String {
    pltrs_render::pgf_text(text, &props.request(rc), &text_options(rc))
}

/// [`pgf_text`], logging the family of `props` when no font has it, as
/// matplotlib's `_escape_and_apply_props` logs it.
pub(super) fn pgf_text_logged(text: &str, props: &FontProperties, rc: &RcParams) -> String {
    let request = props.request(rc);
    let options = text_options(rc);
    if let Some(family) = pltrs_render::pgf_unknown_family(&request, &options) {
        log_unknown_font(family);
    }
    pltrs_render::pgf_text(text, &request, &options)
}

/// The LaTeX preamble a PGF picture assumes (matplotlib's
/// `_get_preamble()`): `\mathdefault` removed, display-style math, the
/// normal size at `font.size`, `pgf.preamble`, with `pgf.rcfonts` the
/// rcParams' fonts for `XeLaTeX` and `LuaLaTeX`, and the `underscore`
/// package. PGF pictures quote it, and documents typesetting them (the
/// pages of a PGF `PdfPages`) use it.
pub fn pgf_preamble(rc: &RcParams) -> String {
    let fonts = FontManager::global();
    let size = rc.font_size;
    let mut lines = vec![
        r"\def\mathdefault#1{#1}".to_owned(),
        r"\everymath=\expandafter{\the\everymath\displaystyle}".to_owned(),
        r"\IfFileExists{scrextend.sty}{".to_owned(),
        format!(r"  \usepackage[fontsize={size:.6}pt]{{scrextend}}"),
        "}{".to_owned(),
        format!(
            r"  \renewcommand{{\normalsize}}{{\fontsize{{{size:.6}}}{{{:.6}}}\selectfont}}",
            1.2 * size
        ),
        r"  \normalsize".to_owned(),
        "}".to_owned(),
        rc.pgf_preamble.clone(),
    ];
    if rc.pgf_rcfonts {
        lines.push(r"\ifdefined\pdftexversion\else  % non-pdftex case.".to_owned());
        lines.push(r"  \usepackage{fontspec}".to_owned());
        for (command, family) in [
            ("setmainfont", "serif"),
            ("setsansfont", "sans-serif"),
            ("setmonofont", "monospace"),
        ] {
            let props = FontProperties {
                families: vec![family.to_owned()],
                ..FontProperties::from_rc(rc)
            };
            if let Ok((path, index)) = fonts.find_font_path(&props, rc) {
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                // fontspec wants forward slashes, as `as_posix` gives.
                let directory = path
                    .parent()
                    .map(|parent| parent.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default();
                // A face of a collection is picked by its index (matplotlib
                // compares the suffix exactly).
                let face = if path.extension().is_some_and(|ext| ext == "ttc") {
                    format!(",FontIndex={index}")
                } else {
                    String::new()
                };
                lines.push(format!(
                    r"  \{command}{{{name}}}[Path=\detokenize{{{directory}/}}{face}]"
                ));
            }
        }
        lines.push(r"\fi".to_owned());
    }
    lines.push(
        r"\makeatletter\@ifpackageloaded{underscore}{}{\usepackage[strings]{underscore}}\makeatother"
            .to_owned(),
    );
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::SaveFormat;

    #[test]
    fn pictures_draw_the_figure_in_inches() {
        let fig = Figure::with_size(2.0, 1.0, 100.0);
        let ax = fig.subplot();
        ax.plot(vec![0.0, 1.0], vec![0.0, 1.0]).unwrap();
        ax.set_title("A $x^2$ title");
        let pgf = fig.to_pgf().unwrap();
        assert!(pgf.contains(r"\pgfqpoint{2.000000in}{1.000000in}"));
        assert!(pgf.contains(r"\pgfpathlineto"));
        assert!(pgf.contains(r"A $x^2$ title}"));
        assert!(pgf.contains(r"\sffamily\fontsize{12.000000}{14.400000}"));
        assert!(
            pgf.contains("%% pltrs used the following preamble\n%%   \\def\\mathdefault#1{#1}\n")
        );
        assert_eq!("pgf".parse::<SaveFormat>().unwrap(), SaveFormat::Pgf);
    }

    #[test]
    fn the_preamble_follows_the_rc_params() {
        let mut rc = RcParams::matplotlib();
        rc.pgf_preamble = r"\usepackage{amsmath}".to_owned();
        let preamble = pgf_preamble(&rc);
        assert!(preamble.starts_with(r"\def\mathdefault#1{#1}"));
        assert!(preamble.contains("\n\\usepackage{amsmath}\n"));
        assert!(preamble.contains(r"\usepackage[fontsize=10.000000pt]{scrextend}"));
        // fontspec finds the fonts where they are, as matplotlib tells it.
        let serif = (preamble.lines())
            .find(|line| line.contains(r"\setmainfont"))
            .expect("rcfonts names the fonts");
        assert!(serif.starts_with(r"  \setmainfont{DejaVuSerif.ttf}[Path=\detokenize{"));
        assert!(serif.ends_with("/fonts/ttf/}]"));
        assert!(preamble.ends_with(r"{\usepackage[strings]{underscore}}\makeatother"));
    }

    #[test]
    fn tex_systems_round_trip_their_names() {
        for system in PgfTexSystem::ALL {
            assert_eq!(system.to_string().parse(), Ok(system));
        }
        assert_eq!(PgfTexSystem::default(), PgfTexSystem::XeLaTeX);
        assert_eq!(
            "tex".parse::<PgfTexSystem>().map_err(|err| err.to_string()),
            Err("\"tex\" is not a valid TeX system".to_owned())
        );
    }

    #[test]
    fn images_go_beside_the_file() {
        let fig = Figure::with_size(1.0, 1.0, 50.0);
        let ax = fig.subplot();
        ax.imshow(
            crate::numeric::Grid::from_fn(2, 2, |i, j| (i + j) as f64),
            crate::axes::ImshowOptions::default(),
        )
        .unwrap();
        assert!(matches!(fig.to_pgf(), Err(SaveError::Vector(_))), "no file");
        let dir = std::env::temp_dir().join(format!("pltrs-pgf-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("figure.pgf");
        fig.savefig(&path).unwrap();
        let pgf = std::fs::read_to_string(&path).unwrap();
        assert!(pgf.contains("{figure-img0.png}"));
        assert!(dir.join("figure-img0.png").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
