//! PostScript and Encapsulated PostScript output (matplotlib's
//! `backend_ps`): the vector renderer placed on a page per `ps.papersize`
//! and `savefig.orientation`, with fonts embedded or taken from AFM metrics.

use plt_render::{PsOptions, PsRenderer};

use super::Figure;
use super::print::SaveOptions;
use super::save::{SaveError, embedding, simplify_settings};

impl Figure {
    /// A PostScript document with save options: `eps` selects
    /// Encapsulated PostScript; the paper type and orientation of the
    /// options, else `ps.papersize` and `savefig.orientation`, place the
    /// figure on the page.
    fn postscript(&self, options: &SaveOptions, eps: bool) -> Result<String, SaveError> {
        let rc = crate::rc::params();
        let ps = PsOptions {
            eps,
            paper: options.papertype.unwrap_or(rc.ps_papersize),
            orientation: options.orientation.unwrap_or(rc.savefig_orientation),
            metadata: options.metadata.clone().unwrap_or_default(),
            simplify: simplify_settings(),
            composite_images: rc.image_composite_image,
            fonttype: embedding(rc.ps_fonttype),
            afm: rc.ps_useafm,
            usetex: rc.text_usetex,
        };
        // Text the backend draws in AFM fonts is laid out with their metrics.
        crate::text::with_afm_metrics(ps.afm_directory(), || {
            self.render_vector_with(options, |width, height, dpi| {
                PsRenderer::new(width, height, dpi, ps)
            })
        })?
        .finish()
        .map_err(SaveError::Vector)
    }

    /// The figure as a PostScript document centred on a `ps.papersize`
    /// page (matplotlib's `savefig(format="ps")`).
    pub fn to_ps(&self) -> Result<String, SaveError> {
        self.to_ps_with(SaveOptions::default())
    }

    /// The figure as a PostScript document with save options.
    ///
    /// # Errors
    ///
    /// When some artists cannot be drawn or a font cannot be embedded.
    pub fn to_ps_with(&self, options: SaveOptions) -> Result<String, SaveError> {
        self.postscript(&options, false)
    }

    /// The figure as an Encapsulated PostScript document (matplotlib's
    /// `savefig(format="eps")`).
    pub fn to_eps(&self) -> Result<String, SaveError> {
        self.to_eps_with(SaveOptions::default())
    }

    /// The figure as an Encapsulated PostScript document with save
    /// options.
    ///
    /// # Errors
    ///
    /// When some artists cannot be drawn or a font cannot be embedded.
    pub fn to_eps_with(&self, options: SaveOptions) -> Result<String, SaveError> {
        self.postscript(&options, true)
    }
}

#[cfg(test)]
mod tests {
    use plt_core::Bbox;

    use super::*;
    use crate::figure::{BboxInches, PageOrientation, Paper};

    #[test]
    fn postscript_documents() {
        let fig = Figure::with_size(2.0, 1.0, 50.0);
        let ax = fig.subplot();
        ax.plot(vec![0.0, 1.0], vec![0.0, 1.0]).unwrap();
        let ps = fig.to_ps().unwrap();
        assert!(ps.starts_with("%!PS-Adobe-3.0\n%%DocumentPaperSizes: letter"));
        let tight = SaveOptions {
            bbox_inches: Some(BboxInches::Inches(Bbox::from_bounds(0.0, 0.0, 1.0, 0.5))),
            ..SaveOptions::default()
        };
        let eps = fig.to_eps_with(tight).unwrap();
        assert!(eps.contains("%%BoundingBox: 0 0 72 36\n"));
        let dir = std::env::temp_dir().join("plt-rs-postscript");
        std::fs::create_dir_all(&dir).unwrap();
        fig.savefig(dir.join("figure.eps")).unwrap();
        let saved = std::fs::read_to_string(dir.join("figure.eps")).unwrap();
        assert!(saved.contains("%%Title: figure.eps\n"));
    }

    #[test]
    fn save_options_choose_the_paper_and_orientation() {
        let fig = Figure::with_size(8.0, 6.0, 80.0);
        let options = SaveOptions {
            orientation: Some(PageOrientation::Landscape),
            papertype: Some(Paper::Figure),
            ..SaveOptions::default()
        };
        let ps = fig.to_ps_with(options).unwrap();
        assert!(!ps.contains("%%DocumentPaperSizes"));
        assert!(ps.contains("%%Orientation: landscape\n%%BoundingBox: 0 0 432 576\n"));
        assert!(ps.contains("mpldict begin\n432 0 translate\n90 rotate\n"));
        let letter = SaveOptions {
            papertype: Some("letter".parse().unwrap()),
            ..SaveOptions::default()
        };
        let ps = fig.to_ps_with(letter).unwrap();
        assert!(ps.contains("%%DocumentPaperSizes: letter\n"));
        assert!(ps.contains("%%BoundingBox: 18 180 594 612\n"));
    }
}
