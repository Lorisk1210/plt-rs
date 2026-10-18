//! Adobe Font Metrics: matplotlib's `_afm` parser and the queries its PDF
//! and PostScript backends make when they measure and draw text in the
//! standard PostScript fonts instead of embedding TrueType fonts
//! (`pdf.use14corefonts`, `ps.useafm`).
//!
//! * [`AfmMetrics`] — one parsed AFM file: header entries, character
//!   metrics by code and by glyph name, kerning pairs and composites, and
//!   `get_str_bbox_and_descent`.
//! * [`type1_name`] — the glyph name a character is looked up by
//!   (matplotlib's `uni2type1`).
//! * [`bundled_metrics`] — the metrics of the 14 PDF core fonts plt-rs
//!   ships.
//!
//! AFM data is untrusted: parsing and every query that can fail return an
//! [`AfmError`] where matplotlib raises.

mod bundled;
mod error;
mod metrics;
mod names;
mod parse;

pub use bundled::bundled_metrics;
pub use error::{AfmError, AfmSection};
pub use metrics::{AfmMetrics, CharMetrics, CompositePart, StrBbox};
pub use names::type1_name;

#[cfg(test)]
mod tests {
    use super::*;

    /// Parses `data`, expecting an error.
    fn error(data: &[u8]) -> AfmError {
        AfmMetrics::parse(data).expect_err("malformed")
    }

    const HEAD: &str = "StartFontMetrics 4.1\nFontName X\nStartCharMetrics 1\n";
    const CHARS: &str = "C 65 ; WX 700 ; N A ; B 0 0 1 1 ;\nEndCharMetrics\n";

    fn with_tail(tail: &str) -> Vec<u8> {
        format!("{HEAD}{CHARS}{tail}").into_bytes()
    }

    #[test]
    fn headers_must_come_first_and_end() {
        assert_eq!(error(b""), AfmError::UnexpectedEnd(AfmSection::Header));
        assert_eq!(error(b"FontName X\n"), AfmError::NotAfm);
        assert_eq!(error(b"\nStartFontMetrics 2\n"), AfmError::NotAfm);
        assert!(AfmMetrics::parse(format!("Comment first\n{HEAD}{CHARS}").as_bytes()).is_ok());
        assert_eq!(
            error(b"StartFontMetrics 2\nFontName X\n"),
            AfmError::UnexpectedEnd(AfmSection::Header)
        );
        // A count that does not read as a number does not end the header.
        assert_eq!(
            error(format!("StartFontMetrics 2\nStartCharMetrics x\n{CHARS}").as_bytes()),
            AfmError::UnexpectedEnd(AfmSection::Header)
        );
        let overflow = AfmError::BadLine {
            section: AfmSection::Header,
            line: 2,
        };
        assert_eq!(error(b"StartFontMetrics 2\nFontBBox 0 0 inf 1\n"), overflow);
        assert_eq!(error(b"StartFontMetrics 2\nCharacters -inf\n"), overflow);
        let lenient = b"StartFontMetrics 2\nFontBBox 0 x\nCharacters nan\n\
                        FontName \xff\nAscender ?\nUnknown 1\nStartCharMetrics 0\nEndCharMetrics\n";
        let afm = AfmMetrics::parse(lenient).expect("unreadable entries are skipped");
        assert!(afm.font_name().is_err() && afm.ascender().is_err());
    }

    #[test]
    fn character_metrics_must_be_complete() {
        assert_eq!(
            error(HEAD.as_bytes()),
            AfmError::UnexpectedEnd(AfmSection::CharMetrics)
        );
        let bad_line = |line| AfmError::BadLine {
            section: AfmSection::CharMetrics,
            line,
        };
        assert_eq!(
            error(format!("{HEAD}C 65 ; WX 1\n").as_bytes()),
            bad_line(4)
        );
        assert_eq!(
            error(format!("{HEAD}\nEndCharMetrics\n").as_bytes()),
            bad_line(4)
        );
        assert_eq!(
            error(format!("{HEAD}C 65 ; WX 1 ; N \u{e9} ; B 0 0 0 0\n").as_bytes()),
            AfmError::UnexpectedEnd(AfmSection::CharMetrics),
            "UTF-8 names are fine"
        );
        let mut latin1 = HEAD.as_bytes().to_vec();
        latin1.extend_from_slice(b"C 65 ; WX 1 ; N \xe9 ; B 0 0 0 0\nEndCharMetrics\n");
        assert_eq!(error(&latin1), bad_line(4));
    }

    #[test]
    fn kerning_pairs_must_be_well_formed() {
        let bad_line = |line| AfmError::BadLine {
            section: AfmSection::KernPairs,
            line,
        };
        let end = AfmError::UnexpectedEnd(AfmSection::KernPairs);
        assert_eq!(error(&with_tail("StartKernData\n")), end);
        assert_eq!(
            error(&with_tail("StartKernData\nStartTrackKern 1\n")),
            bad_line(7)
        );
        assert_eq!(error(&with_tail("StartKernData\nStartKernPairs 1\n")), end);
        assert_eq!(
            error(&with_tail("StartKernData\nStartKernPairs 1\nKPY A A 1\n")),
            bad_line(8)
        );
        assert_eq!(
            error(&with_tail(
                "StartKernData\nStartKernPairs 1\nEndKernPairs\n"
            )),
            end,
            "the line after EndKernPairs is required"
        );
        let afm = AfmMetrics::parse(&with_tail(
            "StartKernData\nStartKernPairs 1\nKPX A A -5\nEndKernPairs\nanything\n",
        ))
        .expect("valid");
        assert_eq!(afm.kern_dist_from_name("A", "A"), -5.0);
    }

    #[test]
    fn composites_must_be_well_formed() {
        assert_eq!(
            error(&with_tail("StartComposites 1\n")),
            AfmError::UnexpectedEnd(AfmSection::Composites)
        );
        assert_eq!(
            error(&with_tail("StartComposites 1\nCC Aacute 2 ; PCC A 0 0 ;\n")),
            AfmError::BadLine {
                section: AfmSection::Composites,
                line: 7,
            }
        );
        let afm = AfmMetrics::parse(&with_tail(
            "Unknown section\nStartComposites 1\n\nCC Aring 1 ; PCC A 0 0 ;\nEndComposites\n",
        ))
        .expect("valid");
        assert_eq!(afm.composite("Aring").map(<[_]>::len), Some(1));
    }
}
