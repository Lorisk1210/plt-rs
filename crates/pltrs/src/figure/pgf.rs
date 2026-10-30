//! Writing figures as PGF pictures for LaTeX documents (matplotlib's
//! `print_pgf`): images go beside the document as PNG files.

use std::path::Path as FsPath;

use pltrs_render::{PgfDocument, PgfOptions, PgfRenderer};

use super::Figure;
use super::print::SaveOptions;
use super::save::{SaveError, simplify_settings};
use crate::fonts::{FontExt, FontManager, FontProperties};
use crate::rc::{RcParams, keyword_names};

/// The LaTeX program a PGF picture is meant for (`pgf.texsystem`).
///
/// It does not change the picture: matplotlib only runs it to typeset
/// PGF figures into PDFs, which pltrs writes directly instead.
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

    /// The document, its images named after `image_stem`.
    fn pgf_document(
        &self,
        options: &SaveOptions,
        image_stem: Option<String>,
    ) -> Result<PgfDocument, SaveError> {
        let rc = crate::rc::params();
        let fonts = FontManager::global();
        let pgf = PgfOptions {
            simplify: simplify_settings(),
            composite_images: rc.image_composite_image,
            rcfonts: rc.pgf_rcfonts,
            font_names: fonts
                .font_list(FontExt::Ttf)
                .into_iter()
                .map(|entry| entry.name)
                .collect(),
            preamble: pgf_preamble(&rc),
            image_stem,
        };
        self.render_at_save_resolution(options, |width, height, dpi| {
            PgfRenderer::new(width, height, dpi, pgf)
        })?
        .finish()
        .map_err(SaveError::Vector)
    }
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
            if let Ok(file) = fonts.find_font(&props, rc, FontExt::Ttf, true) {
                let path = std::path::Path::new(&file);
                let name = path
                    .file_name()
                    .map_or(file.as_str(), |name| name.to_str().unwrap_or(file.as_str()));
                lines.push(format!(r"  \{command}{{{name}}}"));
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
