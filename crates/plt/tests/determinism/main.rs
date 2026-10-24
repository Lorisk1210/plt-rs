//! Reproducible output, ported from matplotlib's `tests/test_determinism.py`:
//! the same figure drawn by separate processes gives byte-identical PNG,
//! SVG, PDF and PS documents, `SOURCE_DATE_EPOCH` fixes the creation
//! dates the vector formats record, and `svg.hashsalt` fixes the ids of
//! SVG documents.
//!
//! As upstream, the documents are drawn by child processes: this test
//! binary runs itself with only [`draw_documents`] selected and
//! `PLT_DETERMINISM_OUT` naming the directory it writes them to, so
//! anything that varies between runs of a program (hash seeds, addresses,
//! the clock) shows up as a difference.

mod figure;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use plt::figure::{SaveFormat, SaveOptions};
use plt_render::{Metadata, Timestamp};

use figure::{Failure, save_figure};

/// The environment variable that turns [`draw_documents`] on.
const OUT: &str = "PLT_DETERMINISM_OUT";

/// 2000-01-01 00:00:00 UTC, the upstream `SOURCE_DATE_EPOCH`.
const EPOCH: &str = "946684800";

/// The upstream cases (TeX ones aside) and a PNG of every object kind:
/// the objects drawn and the format.
const CASES: [(&str, SaveFormat, &str); 9] = [
    ("", SaveFormat::Pdf, "pdf"),
    ("m", SaveFormat::Pdf, "pdf"),
    ("h", SaveFormat::Pdf, "pdf"),
    ("i", SaveFormat::Pdf, "pdf"),
    ("mhip", SaveFormat::Pdf, "pdf"),
    ("mhip", SaveFormat::Ps, "ps"),
    ("p", SaveFormat::Svg, "svg"),
    ("mhip", SaveFormat::Svg, "svg"),
    ("mhip", SaveFormat::Png, "png"),
];

/// The file a case's document is written to.
fn document_path(dir: &Path, objects: &str, extension: &str) -> PathBuf {
    dir.join(format!("objects-{objects}.{extension}"))
}

/// Draws every case into the directory `PLT_DETERMINISM_OUT` names, and
/// does nothing without it (the parent tests run it in child processes).
#[test]
fn draw_documents() -> Result<(), Failure> {
    let Some(dir) = std::env::var_os(OUT) else {
        return Ok(());
    };
    let dir = PathBuf::from(dir);
    for (objects, format, extension) in CASES {
        std::fs::write(
            document_path(&dir, objects, extension),
            save_figure(objects, format)?,
        )?;
    }
    Ok(())
}

/// The documents of one child process, by case.
fn documents_of_child(name: &str) -> Result<Vec<Vec<u8>>, Failure> {
    let dir = std::env::temp_dir().join(format!("plt-determinism-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let status = Command::new(std::env::current_exe()?)
        .args(["--exact", "draw_documents", "--test-threads=1", "--quiet"])
        .env(OUT, &dir)
        .env("SOURCE_DATE_EPOCH", EPOCH)
        .stdout(Stdio::null())
        .status()?;
    assert!(status.success(), "the child drawing the documents failed");
    let documents = CASES
        .iter()
        .map(|&(objects, _, extension)| std::fs::read(document_path(&dir, objects, extension)))
        .collect::<Result<Vec<_>, _>>()?;
    std::fs::remove_dir_all(&dir)?;
    Ok(documents)
}

/// Whether `haystack` holds `needle`.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// `test_determinism_check` and `test_determinism_source_date_epoch`:
/// three processes write the same bytes, dated `SOURCE_DATE_EPOCH`.
#[test]
fn documents_are_reproducible_and_dated_by_source_date_epoch() -> Result<(), Failure> {
    let first = documents_of_child("first")?;
    for run in ["second", "third"] {
        let again = documents_of_child(run)?;
        for ((objects, _, extension), (a, b)) in CASES.iter().zip(first.iter().zip(&again)) {
            assert!(
                a == b,
                "{objects:?} as {extension} differs between processes"
            );
        }
    }
    let document = |objects: &str, extension: &str| {
        CASES
            .iter()
            .position(|&(o, _, e)| o == objects && e == extension)
            .map(|index| &first[index])
            .expect("a listed case")
    };
    assert!(contains(
        document("", "pdf"),
        b"/CreationDate (D:20000101000000Z)"
    ));
    assert!(contains(
        document("mhip", "ps"),
        b"%%CreationDate: Sat Jan 01 00:00:00 2000"
    ));
    assert!(contains(
        document("p", "svg"),
        b"<dc:date>2000-01-01T00:00:00+00:00</dc:date>"
    ));
    Ok(())
}

/// The ids of an SVG document: the values of its `id="..."` attributes.
fn svg_ids(svg: &str) -> Vec<&str> {
    svg.split(" id=\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .collect()
}

/// `svg.hashsalt` decides the ids derived from content: the same salt
/// gives the same document, another salt other ids.
#[test]
fn svg_hashsalt_fixes_the_ids() -> Result<(), Failure> {
    let draw = |salt: &str| -> Result<String, Failure> {
        plt::rc::context(|| {
            plt::rc::set("svg.hashsalt", salt)?;
            let fig = plt::figure::Figure::new();
            let ax = fig.subplot();
            ax.plot_with(
                vec![0.0, 1.0, 2.0],
                vec![1.0, 0.0, 1.0],
                plt::lines::LineProps::fmt("o-")?,
            )?;
            let options = SaveOptions {
                metadata: Some(Metadata {
                    date: Some(Timestamp(0)),
                    ..Metadata::none()
                }),
                ..SaveOptions::default()
            };
            Ok(fig.to_svg_with(options)?)
        })
    };
    let (salted, again, other) = (draw("asdf")?, draw("asdf")?, draw("qwerty")?);
    assert_eq!(salted, again);
    let derived = |svg: &str| -> Vec<String> {
        svg_ids(svg)
            .into_iter()
            .filter(|id| id.starts_with('m') || id.starts_with('p'))
            .map(str::to_owned)
            .collect()
    };
    assert!(
        !derived(&salted).is_empty(),
        "markers and clip paths have ids"
    );
    assert_ne!(derived(&salted), derived(&other));
    Ok(())
}
