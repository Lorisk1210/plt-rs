//! Pixel comparison of rendered figure cases with the golden PNGs, using
//! matplotlib's `compare_images` metric (RMS over channel values, from
//! [`plt::testing`]).

use std::error::Error;
use std::fmt;

use plt::figure::SaveOptions;
use plt::look::FigureLook;
use plt::numeric::Grid;
use plt::testing::calculate_rms;
use plt_render::RgbaImage;

use super::{Baseline, Capture, FigureCase};

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

/// Renders a case with the raster backend under its rc configuration, at
/// the case's resolution as the spec's `savefig(..., dpi=case.dpi)` does
/// (so `savefig.dpi` does not apply), or takes the figure's canvas for
/// [`Capture::Canvas`] cases. Artists that fail to draw are skipped; their
/// count is returned.
pub fn render_pixels(case: &FigureCase) -> Result<(RgbaImage, usize), Box<dyn Error>> {
    render_pixels_from(case, Baseline::Reference)
}

/// Renders a case as [`render_pixels`] does, under `baseline`.
fn render_pixels_from(
    case: &FigureCase,
    baseline: Baseline,
) -> Result<(RgbaImage, usize), Box<dyn Error>> {
    case.with_figure_from(baseline, |fig| match case.capture {
        Capture::Savefig => {
            let options = SaveOptions {
                dpi: Some(case.dpi),
                ..SaveOptions::default()
            };
            let (canvas, problems) = fig.render_raster_partial(options)?;
            Ok((
                canvas.to_image(),
                problems.map_or(0, |errors| errors.0.len()),
            ))
        }
        Capture::Canvas => Ok((fig.canvas().buffer_rgba()?, 0)),
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
        let (actual, draw_errors) =
            render_pixels(case).map_err(|e| format!("render failed: {e}"))?;
        let side = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        let size = (side(actual.width()), side(actual.height()));
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
            actual: actual.into_data(),
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
        encode_png(self.width, self.height, data)
    }
}

/// RGBA pixels of a `width` × `height` image as PNG.
fn encode_png(width: u32, height: u32, data: &[u8]) -> Result<Vec<u8>, String> {
    plt_codec::png::ImageRef::rgba8(width, height, data)
        .and_then(plt_codec::png::encode)
        .map_err(|e| format!("cannot encode PNG: {e}"))
}

/// Renders a case in a plt-rs figure look, to see how it looks there (no
/// golden exists to compare with), as PNG.
pub fn look_png(case: &FigureCase, look: FigureLook) -> Result<Vec<u8>, String> {
    let (image, _) = render_pixels_from(case, Baseline::Look(look))
        .map_err(|e| format!("render failed: {e}"))?;
    let side = |n: usize| u32::try_from(n).map_err(|_| format!("{n} pixels is too large"));
    encode_png(side(image.width())?, side(image.height())?, image.data())
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

/// Compares two RGBA buffers of equal size like `compare_images`: both
/// are compared as RGB when their alpha is 255 everywhere, else as RGBA,
/// by matplotlib's RMS ([`calculate_rms`]).
fn compare_rgba(actual: &[u8], expected: &[u8]) -> PixelReport {
    let pixels = |data: &[u8]| data.as_chunks::<4>().0.to_vec();
    let opaque = |data: &[u8]| data.as_chunks::<4>().0.iter().all(|px| px[3] == 255);
    let channels = if opaque(actual) && opaque(expected) {
        3
    } else {
        4
    };
    let rms = if channels == 3 {
        let rgb = |data: &[u8]| one_row(pixels(data).into_iter().map(|[r, g, b, _]| [r, g, b]));
        calculate_rms(&rgb(expected), &rgb(actual))
    } else {
        calculate_rms(&one_row(pixels(expected)), &one_row(pixels(actual)))
    }
    .expect("the images have the same size");
    let (mut max_diff, mut differing) = (0u8, 0usize);
    for (a, e) in actual
        .as_chunks::<4>()
        .0
        .iter()
        .zip(expected.as_chunks::<4>().0)
    {
        let pixel_diff = a[..channels]
            .iter()
            .zip(&e[..channels])
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap_or(0);
        max_diff = max_diff.max(pixel_diff);
        differing += usize::from(pixel_diff != 0);
    }
    PixelReport {
        rms,
        max_diff,
        differing_pixels: differing,
        total_pixels: actual.len() / 4,
        draw_errors: 0,
    }
}

/// `pixels` as a grid of one row (the RMS needs only matching shapes).
fn one_row<T>(pixels: impl IntoIterator<Item = T>) -> Grid<T> {
    let pixels: Vec<T> = pixels.into_iter().collect();
    Grid::from_vec(1, pixels.len(), pixels).expect("one row holds every pixel")
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

    #[test]
    fn cases_render_in_the_looks_asked_for() {
        let case = FigureCase::new("look/blank", |fig| {
            fig.subplot();
            Ok(())
        })
        .sized(2.0, 1.5, 100.0);
        let corner = |look| {
            let png = look_png(&case, look).expect("the case renders");
            let image = plt_codec::png::decode(&png).expect("a PNG");
            assert_eq!((image.width(), image.height()), (200, 150));
            image.to_rgba8()[..3].to_vec()
        };
        assert_eq!(corner(FigureLook::Modern), [255, 255, 255]);
        assert_eq!(corner(FigureLook::ModernDark), [0x1e, 0x20, 0x24]);
    }
}
