//! Writing figures as SVG (matplotlib's `print_svg`): images embedded, or
//! with `svg.image_inline` off written as PNG files beside the document.

use std::path::Path as FsPath;

use pltrs_render::{ImageFile, Metadata, SvgOptions, SvgRenderer};

use super::Figure;
use super::print::SaveOptions;
use super::save::{SaveError, simplify_settings};

impl Figure {
    /// The figure as an SVG document (matplotlib's `savefig(format="svg")`).
    pub fn to_svg(&self) -> Result<String, SaveError> {
        self.to_svg_with(SaveOptions::default())
    }

    /// The figure as an SVG document with save options; the `svg.*`
    /// rcParams configure the document. Images are embedded: without a
    /// file to put them beside, `svg.image_inline` does not apply.
    ///
    /// # Errors
    ///
    /// When some artists cannot be drawn or the document cannot be written.
    // Its own frame, not `to_format`'s: inlined there, every format's
    // renderer would share one frame of tens of kilobytes, which a save in
    // a thread of a small stack (musl's 128 KiB) cannot spare.
    #[inline(never)]
    pub fn to_svg_with(&self, options: SaveOptions) -> Result<String, SaveError> {
        self.svg_document(&options, None)
            .map(|(document, _)| document)
    }

    /// The figure as an SVG document with save options, and the images
    /// to write beside it: with `svg.image_inline` off, its images are
    /// files named after `image_stem` (`<stem>.image<N>.png`, matplotlib's
    /// name of the file written to, `""` for an unnamed stream), else
    /// embedded.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::figure::SaveOptions;
    /// let fig = Figure::with_size(2.0, 2.0, 50.0);
    /// fig.subplot().imshow(Grid::from_rows([vec![0.0, 1.0]])?, Default::default())?;
    /// let (embedded, none) = fig.to_svg_document(SaveOptions::default(), "fig.svg")?;
    /// assert!(embedded.contains("data:image/png;base64") && none.is_empty());
    /// let (document, images) = pltrs::rc::context(|| {
    ///     pltrs::rc::set("svg.image_inline", "False").expect("a valid flag");
    ///     fig.to_svg_document(SaveOptions::default(), "fig.svg")
    /// })?;
    /// assert_eq!(images[0].name, "fig.svg.image0.png");
    /// assert!(document.contains("xlink:href=\"fig.svg.image0.png\""));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When some artists cannot be drawn or the document cannot be written.
    pub fn to_svg_document(
        &self,
        options: SaveOptions,
        image_stem: &str,
    ) -> Result<(String, Vec<ImageFile>), SaveError> {
        let inline = crate::rc::params().svg_image_inline;
        self.svg_document(&options, (!inline).then(|| image_stem.to_owned()))
    }

    /// Writes the figure to `path` as SVG (gzipped for SVGZ); with
    /// `svg.image_inline` off, its images go beside it as
    /// `<path>.image<N>.png`, named after the path as given.
    pub(super) fn save_svg(
        &self,
        path: &FsPath,
        options: &SaveOptions,
        gzip: bool,
    ) -> Result<(), SaveError> {
        let (document, images) = self.to_svg_document(options.clone(), &path.to_string_lossy())?;
        for image in images {
            std::fs::write(&image.name, image.png).map_err(SaveError::Io)?;
        }
        let bytes = if gzip {
            pltrs_render::gzip(document.as_bytes()).map_err(|err| SaveError::Vector(err.into()))?
        } else {
            document.into_bytes()
        };
        std::fs::write(path, bytes).map_err(SaveError::Io)
    }

    /// The SVG document and the images to write beside it when
    /// `image_files` names the document.
    fn svg_document(
        &self,
        options: &SaveOptions,
        image_files: Option<String>,
    ) -> Result<(String, Vec<ImageFile>), SaveError> {
        let svg = svg_options(options.metadata.clone().unwrap_or_default(), image_files);
        // matplotlib's SVG backend measures text with `TextToPath`.
        crate::text::with_text_path_metrics(|| {
            self.render_vector_with(options, |width, height, dpi| {
                SvgRenderer::new(width * 72.0, height * 72.0, dpi, svg)
            })
        })?
        .finish_with_images()
        .map_err(SaveError::Vector)
    }
}

/// The options of an SVG document with `metadata`, its images written to
/// files named after `image_files` (else embedded), as the `svg.*` and
/// `image.composite_image` rcParams configure figures.
pub(super) fn svg_options(metadata: Metadata, image_files: Option<String>) -> SvgOptions {
    let rc = crate::rc::params();
    SvgOptions {
        hashsalt: rc.svg_hashsalt.clone(),
        id: rc.svg_id.clone(),
        metadata,
        simplify: simplify_settings(),
        composite_images: rc.image_composite_image,
        fonttype: rc.svg_fonttype,
        image_files,
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;

    #[test]
    fn figures_axes_and_texts_write_their_group_ids() {
        let fig = Figure::new();
        fig.set_gid("the figure");
        let ax = fig.subplot();
        ax.set_gid("the axes");
        ax.text(0.5, 0.5, "label").set_gid("the text");
        // A 3-D axes draws its background and axes before its group,
        // which holds nothing here (and closes at once).
        fig.subplot_3d().set_gid("the 3-D axes");
        let svg = fig.to_svg().unwrap();
        for gid in ["the figure", "the axes", "the text", "the 3-D axes"] {
            assert!(svg.contains(&format!("<g id=\"{gid}\"")), "{gid}");
        }
    }
}
