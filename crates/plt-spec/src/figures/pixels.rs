//! Pixel comparison of rendered figure cases with the golden PNGs, using
//! matplotlib's `compare_images` metric (RMS over channel values).

use std::error::Error;
use std::fmt;

use plt::figure::SaveOptions;
use plt_render::RasterRenderer;

use super::FigureCase;

/// How a rendered case compares with its golden image.
#[derive(Clone, Debug, PartialEq)]
pub struct PixelReport {
    /// matplotlib's `calculate_rms`: root mean square of the channel
    /// differences (RGB when both images are opaque, RGBA otherwise).
    pub rms: f64,
    /// Largest difference of any channel.
    pub max_diff: u8,
    /// Pixels with any differing channel.
    pub differing_pixels: usize,
    /// Pixels compared.
    pub total_pixels: usize,
    /// Artists that could not be drawn (their pixels are missing).
    pub draw_errors: usize,
}

impl fmt::Display for PixelReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rms {:7.3}  max {:3}  differing {:5.2}%",
            self.rms,
            self.max_diff,
            100.0 * self.differing_pixels as f64 / self.total_pixels.max(1) as f64
        )?;
        if self.draw_errors > 0 {
            write!(f, "  ({} artists not drawn)", self.draw_errors)?;
        }
        Ok(())
    }
}

/// Renders a case with the raster backend under its rc configuration.
/// Artists that fail to draw are skipped; their count is returned.
pub fn render_pixels(case: &FigureCase) -> Result<(RasterRenderer, usize), Box<dyn Error>> {
    case.with_figure(|fig| {
        let (canvas, problems) = fig.render_raster_partial(&SaveOptions::default())?;
        Ok((canvas, problems.map_or(0, |errors| errors.0.len())))
    })
}

/// A case rendered next to its golden image.
struct Rendered {
    width: u32,
    height: u32,
    actual: Vec<u8>,
    golden: Vec<u8>,
    draw_errors: usize,
}

impl Rendered {
    /// Renders `case` and loads its golden PNG, both as RGBA.
    fn new(case: &FigureCase) -> Result<Self, String> {
        let golden =
            std::fs::read(case.png_path()).map_err(|e| format!("cannot read golden: {e}"))?;
        let golden =
            plt_codec::png::decode(&golden).map_err(|e| format!("cannot decode golden: {e}"))?;
        let (canvas, draw_errors) =
            render_pixels(case).map_err(|e| format!("render failed: {e}"))?;
        let actual = canvas.pixmap();
        let size = (actual.width(), actual.height());
        if size != (golden.width(), golden.height()) {
            return Err(format!(
                "image sizes do not match: rendered {}x{}, golden {}x{}",
                size.0,
                size.1,
                golden.width(),
                golden.height()
            ));
        }
        Ok(Self {
            width: size.0,
            height: size.1,
            actual: actual.data().to_vec(),
            golden: golden.to_rgba8(),
            draw_errors,
        })
    }

    fn report(&self) -> PixelReport {
        PixelReport {
            draw_errors: self.draw_errors,
            ..compare_rgba(&self.actual, &self.golden)
        }
    }

    fn png(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        plt_codec::png::ImageRef::rgba8(self.width, self.height, data)
            .and_then(plt_codec::png::encode)
            .map_err(|e| format!("cannot encode PNG: {e}"))
    }
}

/// Renders a case and compares it with its golden PNG.
pub fn pixel_report(case: &FigureCase) -> Result<PixelReport, String> {
    Rendered::new(case).map(|rendered| rendered.report())
}

/// A rendered case for inspection: its PNG, a PNG highlighting where it
/// differs from the golden image, and the comparison.
#[derive(Clone, Debug)]
pub struct PixelArtifacts {
    /// The rendered image.
    pub rendered: Vec<u8>,
    /// The golden image faded, with differing pixels in red (brighter for
    /// larger differences).
    pub diff: Vec<u8>,
    /// The comparison.
    pub report: PixelReport,
}

/// Renders a case and builds [`PixelArtifacts`].
pub fn pixel_artifacts(case: &FigureCase) -> Result<PixelArtifacts, String> {
    let rendered = Rendered::new(case)?;
    let diff = diff_rgba(&rendered.actual, &rendered.golden);
    Ok(PixelArtifacts {
        rendered: rendered.png(&rendered.actual)?,
        diff: rendered.png(&diff)?,
        report: rendered.report(),
    })
}

/// The golden image faded to a quarter of its contrast, with pixels that
/// differ painted red in proportion to their largest channel difference.
fn diff_rgba(actual: &[u8], expected: &[u8]) -> Vec<u8> {
    actual
        .as_chunks::<4>()
        .0
        .iter()
        .zip(expected.as_chunks::<4>().0)
        .flat_map(|(a, e)| {
            let d = a
                .iter()
                .zip(e)
                .map(|(x, y)| x.abs_diff(*y))
                .max()
                .unwrap_or(0);
            if d == 0 {
                e.map(|c| 191 + c / 4)
            } else {
                let fade = 255 - d.saturating_mul(16).max(64);
                [255, fade, fade, 255]
            }
        })
        .collect()
}

/// Compares two RGBA buffers of equal size like `compare_images`: an image
/// whose alpha is 255 everywhere is compared as RGB.
fn compare_rgba(actual: &[u8], expected: &[u8]) -> PixelReport {
    let opaque = |data: &[u8]| data.as_chunks::<4>().0.iter().all(|px| px[3] == 255);
    let channels = if opaque(actual) && opaque(expected) {
        3
    } else {
        4
    };
    let (mut sum, mut max_diff, mut differing) = (0.0, 0u8, 0usize);
    for (a, e) in actual
        .as_chunks::<4>()
        .0
        .iter()
        .zip(expected.as_chunks::<4>().0)
    {
        let mut pixel_differs = false;
        for (x, y) in a[..channels].iter().zip(&e[..channels]) {
            let d = x.abs_diff(*y);
            sum += f64::from(d) * f64::from(d);
            max_diff = max_diff.max(d);
            pixel_differs |= d != 0;
        }
        differing += usize::from(pixel_differs);
    }
    let total_pixels = actual.len() / 4;
    PixelReport {
        rms: (sum / (total_pixels * channels).max(1) as f64).sqrt(),
        max_diff,
        differing_pixels: differing,
        total_pixels,
        draw_errors: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_matches_matplotlib_definition() {
        // Opaque images compare RGB only: one channel off by 6 in 2 pixels.
        let a = [10, 20, 30, 255, 0, 0, 0, 255];
        let e = [16, 20, 30, 255, 0, 0, 0, 255];
        let report = compare_rgba(&a, &e);
        assert!((report.rms - (36.0_f64 / 6.0).sqrt()).abs() < 1e-12);
        assert_eq!(
            (
                report.max_diff,
                report.differing_pixels,
                report.total_pixels
            ),
            (6, 1, 2)
        );
        let diff = diff_rgba(&a, &e);
        assert_eq!(&diff[..4], [255, 159, 159, 255]);
        assert_eq!(&diff[4..], [191, 191, 191, 254]);
        // Translucent images include alpha.
        let report = compare_rgba(&[0, 0, 0, 0], &[0, 0, 0, 4]);
        assert!((report.rms - 2.0).abs() < 1e-12);
        assert!(report.to_string().starts_with("rms   2.000"));
    }
}
