//! Writing figures as SVG (matplotlib's `print_svg`): images embedded, or
//! with `svg.image_inline` off written as PNG files beside the document.

use std::path::Path as FsPath;

use plt_render::{ImageFile, SvgOptions, SvgRenderer};

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
    pub fn to_svg_with(&self, options: SaveOptions) -> Result<String, SaveError> {
        self.svg_document(&options, None)
            .map(|(document, _)| document)
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
        let inline = crate::rc::params().svg_image_inline;
        let name = (!inline).then(|| path.to_string_lossy().into_owned());
        let (document, images) = self.svg_document(options, name)?;
        for image in images {
            std::fs::write(&image.name, image.png).map_err(SaveError::Io)?;
        }
        let bytes = if gzip {
            plt_render::gzip(document.as_bytes())
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
        let rc = crate::rc::params();
        let svg = SvgOptions {
            hashsalt: rc.svg_hashsalt.clone(),
            id: rc.svg_id.clone(),
            metadata: options.metadata.clone().unwrap_or_default(),
            simplify: simplify_settings(),
            composite_images: rc.image_composite_image,
            fonttype: rc.svg_fonttype,
            image_files,
        };
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
        fig.subplot_3d().set_gid("the 3-D axes");
        let svg = fig.to_svg().unwrap();
        for gid in ["the figure", "the axes", "the text", "the 3-D axes"] {
            assert!(svg.contains(&format!("<g id=\"{gid}\">")), "{gid}");
        }
    }
}
