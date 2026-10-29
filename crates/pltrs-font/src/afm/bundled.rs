//! The AFM files pltrs ships: matplotlib's `mpl-data/fonts/pdfcorefonts`,
//! the metrics of the 14 PDF core fonts, distributed with Adobe's
//! `readme.txt` beside them in `fonts/pdfcorefonts`. Each file is parsed
//! once, on first use.

use std::sync::OnceLock;

use super::error::AfmError;
use super::metrics::AfmMetrics;

/// `(file name, contents)` of every bundled AFM file.
const BUNDLED_AFM: [(&str, &[u8]); 14] = [
    (
        "Courier-Bold.afm",
        include_bytes!("../../fonts/pdfcorefonts/Courier-Bold.afm"),
    ),
    (
        "Courier-BoldOblique.afm",
        include_bytes!("../../fonts/pdfcorefonts/Courier-BoldOblique.afm"),
    ),
    (
        "Courier-Oblique.afm",
        include_bytes!("../../fonts/pdfcorefonts/Courier-Oblique.afm"),
    ),
    (
        "Courier.afm",
        include_bytes!("../../fonts/pdfcorefonts/Courier.afm"),
    ),
    (
        "Helvetica-Bold.afm",
        include_bytes!("../../fonts/pdfcorefonts/Helvetica-Bold.afm"),
    ),
    (
        "Helvetica-BoldOblique.afm",
        include_bytes!("../../fonts/pdfcorefonts/Helvetica-BoldOblique.afm"),
    ),
    (
        "Helvetica-Oblique.afm",
        include_bytes!("../../fonts/pdfcorefonts/Helvetica-Oblique.afm"),
    ),
    (
        "Helvetica.afm",
        include_bytes!("../../fonts/pdfcorefonts/Helvetica.afm"),
    ),
    (
        "Symbol.afm",
        include_bytes!("../../fonts/pdfcorefonts/Symbol.afm"),
    ),
    (
        "Times-Bold.afm",
        include_bytes!("../../fonts/pdfcorefonts/Times-Bold.afm"),
    ),
    (
        "Times-BoldItalic.afm",
        include_bytes!("../../fonts/pdfcorefonts/Times-BoldItalic.afm"),
    ),
    (
        "Times-Italic.afm",
        include_bytes!("../../fonts/pdfcorefonts/Times-Italic.afm"),
    ),
    (
        "Times-Roman.afm",
        include_bytes!("../../fonts/pdfcorefonts/Times-Roman.afm"),
    ),
    (
        "ZapfDingbats.afm",
        include_bytes!("../../fonts/pdfcorefonts/ZapfDingbats.afm"),
    ),
];

/// The parsed metrics of every bundled file, filled in on first use.
static PARSED: [OnceLock<Result<AfmMetrics, AfmError>>; 14] = [const { OnceLock::new() }; 14];

/// The metrics of the bundled AFM file `file`.
///
/// # Errors
///
/// When no such file is bundled, or (never for the files shipped) it
/// does not parse.
pub fn bundled_metrics(file: &str) -> Result<&'static AfmMetrics, AfmError> {
    let index = BUNDLED_AFM
        .iter()
        .position(|(name, _)| *name == file)
        .ok_or_else(|| AfmError::NotBundled(file.to_owned()))?;
    PARSED[index]
        .get_or_init(|| AfmMetrics::parse(BUNDLED_AFM[index].1))
        .as_ref()
        .map_err(Clone::clone)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_file_parses() {
        for (file, _) in BUNDLED_AFM {
            let afm = bundled_metrics(file).expect("bundled files parse");
            assert_eq!(
                format!("{}.afm", afm.font_name().expect("named")),
                file,
                "the files are named after their fonts"
            );
        }
        assert_eq!(
            bundled_metrics("phvr8a.afm"),
            Err(AfmError::NotBundled("phvr8a.afm".to_owned()))
        );
    }

    #[test]
    fn core_font_metrics_match_matplotlib() {
        let helvetica = bundled_metrics("Helvetica.afm").expect("bundled");
        // matplotlib: AFM(Helvetica.afm).get_str_bbox_and_descent('AVATAR').
        let bbox = helvetica.str_bbox_and_descent("AVATAR").expect("known");
        assert_eq!(
            (
                bbox.left,
                bbox.bottom,
                bbox.width,
                bbox.height,
                bbox.descent
            ),
            (0.0, 0.0, 3611.0, 718.0, 0.0)
        );
        assert_eq!(helvetica.kern_dist_from_name("A", "V"), -70.0);
        assert_eq!(helvetica.width_char(128), Some(556.0), "the Euro");
        assert_eq!(helvetica.width_from_char_name("minus"), Some(584.0));
        assert_eq!(helvetica.ascender(), Ok(718.0));
        assert_eq!(helvetica.descender(), Ok(-207.0));
        let symbol = bundled_metrics("Symbol.afm").expect("bundled");
        assert!(symbol.ascender().is_err(), "Symbol has no ascender");
        let dingbats = bundled_metrics("ZapfDingbats.afm").expect("bundled");
        assert!(
            dingbats.str_bbox_and_descent("a").is_err(),
            "no question mark"
        );
    }
}
