//! Rasterizing figures and writing PNG files: matplotlib's `savefig` with
//! the Agg canvas.

use std::fmt;
use std::path::Path as FsPath;

use plt_core::path::convert::SimplifySettings;
use plt_render::{RasterBackendError, RasterRenderer};

use super::{DrawErrors, Figure};

/// Why a figure could not be rendered or saved.
#[derive(Debug)]
pub enum SaveError {
    /// The canvas could not be created or encoded.
    Backend(RasterBackendError),
    /// Some artists could not be drawn.
    Draw(DrawErrors),
    /// Writing the file failed.
    Io(std::io::Error),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(err) => write!(f, "cannot render the figure: {err}"),
            Self::Draw(err) => err.fmt(f),
            Self::Io(err) => write!(f, "cannot write the figure: {err}"),
        }
    }
}

impl std::error::Error for SaveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Backend(err) => Some(err),
            Self::Draw(err) => Some(err),
            Self::Io(err) => Some(err),
        }
    }
}

impl Figure {
    /// A raster canvas for this figure — `size_inches × dpi` pixels
    /// (truncated, as `FigureCanvasAgg` does) with the current
    /// `path.simplify` settings — ready to be drawn into.
    pub fn raster_canvas(&self) -> Result<RasterRenderer, SaveError> {
        let (width, height) = self.size_inches();
        Self::raster_canvas_sized(width, height, self.dpi())
    }

    /// A raster canvas of `width × height` inches at `dpi` with the
    /// current `path.simplify` settings.
    pub(crate) fn raster_canvas_sized(
        width: f64,
        height: f64,
        dpi: f64,
    ) -> Result<RasterRenderer, SaveError> {
        let rc = crate::rc::params();
        let simplify = SimplifySettings {
            enabled: rc.path_simplify,
            threshold: rc.path_simplify_threshold,
        };
        RasterRenderer::new(width * dpi, height * dpi, dpi)
            .map(|canvas| canvas.with_simplify(simplify))
            .map_err(SaveError::Backend)
    }

    /// Renders the figure with the Agg-compatible raster backend.
    pub fn render_raster(&self) -> Result<RasterRenderer, SaveError> {
        let mut canvas = self.raster_canvas()?;
        self.draw(&mut canvas).map_err(SaveError::Draw)?;
        Ok(canvas)
    }

    /// The figure as PNG bytes (matplotlib's `savefig(format="png")`).
    pub fn to_png(&self) -> Result<Vec<u8>, SaveError> {
        self.render_raster()?.to_png().map_err(SaveError::Backend)
    }

    /// Writes the figure to `path` as PNG.
    pub fn savefig(&self, path: impl AsRef<FsPath>) -> Result<(), SaveError> {
        std::fs::write(path, self.to_png()?).map_err(SaveError::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_the_figure_background() {
        let fig = Figure::with_size(1.0, 0.5, 20.0);
        let canvas = fig.render_raster().unwrap();
        let pixmap = canvas.pixmap();
        assert_eq!((pixmap.width(), pixmap.height()), (20, 10));
        assert_eq!(&pixmap.data()[(3 * 20 + 3) * 4..][..4], &[255; 4]);
        let png = fig.to_png().unwrap();
        assert_eq!(&png[1..4], b"PNG");
    }

    #[test]
    fn savefig_reports_io_errors() {
        let fig = Figure::with_size(0.5, 0.5, 10.0);
        let missing = std::env::temp_dir().join("plt-rs-missing-dir/x/figure.png");
        assert!(matches!(fig.savefig(missing), Err(SaveError::Io(_))));
    }
}
