//! The AFM files pltrs ships, matplotlib's two directories of them: in
//! `fonts/pdfcorefonts` the metrics of the 14 PDF core fonts, with Adobe's
//! `readme.txt` beside them, and in `fonts/afm` the 47 files of
//! matplotlib's `mpl-data/fonts/afm` (the 35 standard PostScript fonts,
//! Helvetica Light, Utopia and Computer Modern), with the `readme.txt`
//! carrying Adobe's notice for them. Each file is parsed once, on first
//! use.

use std::sync::OnceLock;

use super::error::AfmError;
use super::metrics::AfmMetrics;
use crate::db::AfmDirectory;

/// `(file name, contents)` of the files `names` of the font directory
/// `dir`.
macro_rules! afm_files {
    ($dir:literal: $($name:literal)*) => {
        [$(($name, include_bytes!(concat!("../../fonts/", $dir, "/", $name)) as &[u8])),*]
    };
}

/// The bundled files of `fonts/pdfcorefonts`.
static PDF_CORE: [BundledFile; 14] = afm_files!("pdfcorefonts":
    "Courier-Bold.afm" "Courier-BoldOblique.afm" "Courier-Oblique.afm" "Courier.afm"
    "Helvetica-Bold.afm" "Helvetica-BoldOblique.afm" "Helvetica-Oblique.afm"
    "Helvetica.afm" "Symbol.afm" "Times-Bold.afm" "Times-BoldItalic.afm"
    "Times-Italic.afm" "Times-Roman.afm" "ZapfDingbats.afm"
);

/// The bundled files of `fonts/afm`.
static POSTSCRIPT: [BundledFile; 47] = afm_files!("afm":
    "cmex10.afm" "cmmi10.afm" "cmr10.afm" "cmsy10.afm" "cmti10.afm" "cmtt10.afm"
    "pagd8a.afm" "pagdo8a.afm" "pagk8a.afm" "pagko8a.afm"
    "pbkd8a.afm" "pbkdi8a.afm" "pbkl8a.afm" "pbkli8a.afm"
    "pcrb8a.afm" "pcrbo8a.afm" "pcrr8a.afm" "pcrro8a.afm"
    "phvb8a.afm" "phvb8an.afm" "phvbo8a.afm" "phvbo8an.afm" "phvl8a.afm" "phvlo8a.afm"
    "phvr8a.afm" "phvr8an.afm" "phvro8a.afm" "phvro8an.afm"
    "pncb8a.afm" "pncbi8a.afm" "pncr8a.afm" "pncri8a.afm"
    "pplb8a.afm" "pplbi8a.afm" "pplr8a.afm" "pplri8a.afm" "psyr.afm"
    "ptmb8a.afm" "ptmbi8a.afm" "ptmr8a.afm" "ptmri8a.afm"
    "putb8a.afm" "putbi8a.afm" "putr8a.afm" "putri8a.afm" "pzcmi8a.afm" "pzdr.afm"
);

/// `(file name, contents)` of a bundled file.
type BundledFile = (&'static str, &'static [u8]);

/// The metrics of a bundled file, parsed on first use.
type Parsed = OnceLock<Result<AfmMetrics, AfmError>>;

/// The parsed metrics of every bundled file of `fonts/pdfcorefonts`.
static PDF_CORE_PARSED: [Parsed; 14] = [const { OnceLock::new() }; 14];

/// The parsed metrics of every bundled file of `fonts/afm`.
static POSTSCRIPT_PARSED: [Parsed; 47] = [const { OnceLock::new() }; 47];

/// The files of `directory` and the cells of their parsed metrics.
fn directory_files(directory: AfmDirectory) -> (&'static [BundledFile], &'static [Parsed]) {
    match directory {
        AfmDirectory::PdfCore => (&PDF_CORE, &PDF_CORE_PARSED),
        AfmDirectory::PostScript => (&POSTSCRIPT, &POSTSCRIPT_PARSED),
    }
}

/// The file name and contents of every bundled AFM file of `directory`
/// (for writing them out where a file path is needed).
pub fn bundled_afm_files(
    directory: AfmDirectory,
) -> impl Iterator<Item = (&'static str, &'static [u8])> {
    directory_files(directory).0.iter().copied()
}

/// The metrics of the bundled AFM file `file` (the names of the two
/// directories' files differ).
///
/// # Errors
///
/// When no such file is bundled, or (never for the files shipped) it
/// does not parse.
pub fn bundled_metrics(file: &str) -> Result<&'static AfmMetrics, AfmError> {
    let (data, cell) = AfmDirectory::ALL
        .into_iter()
        .find_map(|directory| {
            let (files, parsed) = directory_files(directory);
            let index = files.iter().position(|(name, _)| *name == file)?;
            Some((files[index].1, &parsed[index]))
        })
        .ok_or_else(|| AfmError::NotBundled(file.to_owned()))?;
    cell.get_or_init(|| AfmMetrics::parse(data))
        .as_ref()
        .map_err(Clone::clone)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_file_parses() {
        for (file, _) in bundled_afm_files(AfmDirectory::PdfCore) {
            let afm = bundled_metrics(file).expect("bundled files parse");
            assert_eq!(
                format!("{}.afm", afm.font_name().expect("named")),
                file,
                "the core fonts' files are named after their fonts"
            );
        }
        for (file, _) in bundled_afm_files(AfmDirectory::PostScript) {
            let afm = bundled_metrics(file).expect("bundled files parse");
            assert!(afm.font_name().is_ok() && afm.family_name().is_ok());
        }
        assert_eq!(
            bundled_metrics("Helvetica.ttf"),
            Err(AfmError::NotBundled("Helvetica.ttf".to_owned()))
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

    #[test]
    fn standard_font_metrics_match_matplotlib() {
        // matplotlib: AFM(fonts/afm/pagd8a.afm).
        let avant_garde = bundled_metrics("pagd8a.afm").expect("bundled");
        assert_eq!(avant_garde.font_name(), Ok("AvantGarde-Demi"));
        assert_eq!(avant_garde.family_name(), Ok("ITC Avant Garde Gothic"));
        assert_eq!(avant_garde.weight(), Ok("Demi"));
        let bbox = avant_garde.str_bbox_and_descent("AVATAR").expect("known");
        assert_eq!(
            (
                bbox.left,
                bbox.bottom,
                bbox.width,
                bbox.height,
                bbox.descent
            ),
            (0.0, 0.0, 3666.0, 740.0, 0.0)
        );
        assert_eq!(avant_garde.kern_dist_from_name("A", "V"), -102.0);
        // The Helvetica of `fonts/afm` is an older one than the core
        // font's: it does not kern f-eacute.
        let helvetica = bundled_metrics("phvr8a.afm").expect("bundled");
        assert_eq!(helvetica.font_name(), Ok("Helvetica"));
        assert_eq!(helvetica.kern_dist_from_name("f", "eacute"), 0.0);
        let core = bundled_metrics("Helvetica.afm").expect("bundled");
        assert_eq!(core.kern_dist_from_name("f", "eacute"), -30.0);
        let cmr10 = bundled_metrics("cmr10.afm").expect("bundled");
        assert_eq!(cmr10.family_name(), Ok("Computer Modern"));
        let bbox = cmr10.str_bbox_and_descent("AVATAR").expect("known");
        assert_eq!(
            (bbox.bottom, bbox.width, bbox.height, bbox.descent),
            (-22.0, 4069.445, 738.0, 22.0)
        );
    }
}
