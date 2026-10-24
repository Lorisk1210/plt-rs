//! What several targets share: the output formats and rendering to them.

use plt::figure::{Figure, SaveFormat, SaveOptions};

use crate::harness::Gen;

/// Every format a figure can be saved in.
pub(crate) const FORMATS: [SaveFormat; 12] = [
    SaveFormat::Png,
    SaveFormat::Svg,
    SaveFormat::Svgz,
    SaveFormat::Pdf,
    SaveFormat::Ps,
    SaveFormat::Eps,
    SaveFormat::Pgf,
    SaveFormat::Jpeg,
    SaveFormat::Tiff,
    SaveFormat::Gif,
    SaveFormat::Webp,
    SaveFormat::Raw,
];

/// A format to render a fuzzed figure to: PNG half the time (the raster
/// path is the most intricate), otherwise any.
pub(crate) fn format(g: &mut Gen) -> SaveFormat {
    if g.chance(0.5) {
        SaveFormat::Png
    } else {
        g.pick(&FORMATS)
    }
}

/// Renders `fig` to `format`; errors are fine, the output is dropped.
pub(crate) fn render(fig: &Figure, format: SaveFormat) {
    let _ = fig.to_format(format, SaveOptions::default());
}
