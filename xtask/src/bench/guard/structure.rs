//! The structural check of vector documents, through the conformance
//! spec's reduction of SVG, PDF and PostScript to drawing items.

use std::path::Path;

use pltrs_spec::figures::vector::{VectorFormat, compare_items};

use super::{DEVIATION, Verdict};

/// Compares the vector document `actual` with matplotlib's `expected`,
/// both saved as `extension` (`svg`, `pdf`, `ps` or `eps`): the same
/// drawing items, their numbers within [`DEVIATION`]. `None` for a format
/// that is not a vector one.
pub(in super::super) fn compare_documents(
    extension: &str,
    expected: &Path,
    actual: &Path,
) -> Option<Verdict> {
    let format = match extension {
        "svg" => VectorFormat::Svg,
        "pdf" => VectorFormat::Pdf,
        "ps" => VectorFormat::Ps,
        "eps" => VectorFormat::Eps,
        _ => return None,
    };
    let items = |path: &Path| {
        std::fs::read(path)
            .map_err(|err| format!("cannot read {}: {err}", path.display()))
            .and_then(|document| format.items(&document))
    };
    Some(match (items(expected), items(actual)) {
        (Ok(expected), Ok(actual)) => {
            let report = compare_items(&expected, &actual);
            if !report.is_structurally_equal() {
                Verdict::Different(format!("{report}: {}", report.details(2).trim()))
            } else if report.max_deviation() > DEVIATION {
                Verdict::Different(format!(
                    "numbers differ by up to {:.3}",
                    report.max_deviation()
                ))
            } else {
                Verdict::Same(report.max_deviation())
            }
        }
        (Err(why), _) | (_, Err(why)) => Verdict::Incomparable(why),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raster_formats_have_no_structure() {
        let path = Path::new("/nonexistent");
        assert_eq!(compare_documents("png", path, path), None);
        assert!(matches!(
            compare_documents("svg", path, path),
            Some(Verdict::Incomparable(_))
        ));
    }
}
