//! Probe operations for the `mpl_backend_ps` area: matplotlib's own tests
//! of its PostScript backend that read the document (see
//! `spec/reference/probes/test_mpl_backend_ps.py`, which documents every
//! op).

use std::collections::HashMap;

use plt::figure::{Figure, PageOrientation, Paper, SaveOptions, Share};
use plt::lines::LineStyle;
use plt::rc::FontType;

use super::mpl_tests::in_test_style;
use crate::json::Json;
use crate::probes::encode::numbers;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("Figure.savefig", placed),
    Op::new("savefig.eps.bbox", bounding_box),
    Op::new("rcParams.ps.fonttype", font_type),
    Op::new("Line2D.set_linestyle", dashed_line),
    Op::new("Figure.text", empty_lines),
    Op::new("Figure.suptitle", most_definitions),
    Op::new("Figure.savefig.papertype", papertype),
    Op::new("rcParams.ps.papersize", papersize),
];

/// DSC comments that name the library, the date or the file.
const UNSTABLE: [&str; 3] = ["%%Creator", "%%CreationDate", "%%Title"];

/// The document of `fig` as PostScript or Encapsulated PostScript.
fn document(fig: &Figure, format: &str, options: SaveOptions) -> Result<String, ProbeError> {
    match format {
        "ps" => fig.to_ps_with(options),
        "eps" => fig.to_eps_with(options),
        other => return Err(ProbeError::BadArgs(format!("unknown format {other}"))),
    }
    .map_err(ProbeError::rejected)
}

/// The text between the first `start` after `from` and the first `end`
/// after it.
fn section<'a>(document: &'a str, from: &str, start: &str, end: &str) -> Option<&'a str> {
    let rest = &document[document.find(from)?..];
    let rest = &rest[rest.find(start)?..];
    rest.find(end).map(|stop| &rest[..stop])
}

/// The header comments and the placement on the page of a titled line
/// saved with an orientation and a paper type.
fn placed(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        if args.bool("log")? {
            ax.set_yscale("log").map_err(ProbeError::rejected)?;
        }
        ax.plot([1.0, 2.0], [1.0, 2.0])
            .map_err(ProbeError::rejected)?;
        ax.set_title("Déjà vu \u{2212}\u{20ac}");
        let options = SaveOptions {
            orientation: Some(
                args.str("orientation")?
                    .parse::<PageOrientation>()
                    .map_err(ProbeError::rejected)?,
            ),
            papertype: Some(
                args.str("papertype")?
                    .parse::<Paper>()
                    .map_err(ProbeError::rejected)?,
            ),
            ..SaveOptions::default()
        };
        let document = document(&fig, args.str("format")?, options)?;
        let missing = || ProbeError::rejected("the document misses a section");
        let head = &document[..document.find("%%EndComments").ok_or_else(missing)?];
        let header: Vec<Json> = head
            .lines()
            .filter(|line| !UNSTABLE.iter().any(|prefix| line.starts_with(prefix)))
            .map(Json::from)
            .collect();
        let body =
            section(&document, "%%EndProlog", "mpldict begin", "rectclip").ok_or_else(missing)?;
        let lines: Vec<&str> = body.lines().collect();
        let placement = lines
            .get(1..lines.len().saturating_sub(1))
            .unwrap_or_default()
            .iter()
            .map(|&line| Json::from(line))
            .collect();
        Ok(Json::Object(vec![
            ("header".to_owned(), Json::Array(header)),
            ("placement".to_owned(), Json::Array(placement)),
        ]))
    })
}

/// The numbers of the comment `name` of a document.
fn comment_numbers(document: &str, name: &str) -> Result<Vec<f64>, ProbeError> {
    let line = document
        .lines()
        .find_map(|line| line.strip_prefix(name))
        .ok_or_else(|| ProbeError::rejected(format!("no {name} comment")))?;
    line.split_whitespace()
        .map(|word| word.parse().map_err(ProbeError::rejected))
        .collect()
}

/// The bounding box comments of an EPS figure.
fn bounding_box(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        fig.subplot();
        let document = document(&fig, "eps", SaveOptions::default())?;
        Ok(Json::Object(vec![
            (
                "bbox".to_owned(),
                numbers(comment_numbers(&document, "%%BoundingBox: ")?),
            ),
            (
                "hires".to_owned(),
                numbers(comment_numbers(&document, "%%HiResBoundingBox: ")?),
            ),
        ]))
    })
}

/// Whether the PostScript of a text declares fonts of `ps.fonttype`.
fn font_type(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        fig.subplot()
            .text(0.25, 0.5, "Forty-two is the answer to everything!");
        let document = document(&fig, "ps", SaveOptions::default())?;
        let number = match plt::rc::params().ps_fonttype {
            FontType::Type3 => 3,
            FontType::TrueType => 42,
        };
        Ok(Json::Bool(
            document.contains(&format!("/FontType {number} def")),
        ))
    })
}

/// Whether the PostScript of a dashed line is not empty.
fn dashed_line(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        fig.subplot()
            .plot([0.0, 1.0], [0.0, 1.0])
            .map_err(ProbeError::rejected)?
            .set_linestyle(LineStyle::Dashed);
        Ok(Json::Bool(
            !document(&fig, "ps", SaveOptions::default())?.is_empty(),
        ))
    })
}

/// Whether a text of empty lines saves as EPS and PostScript.
fn empty_lines(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        fig.text(0.5, 0.5, "\nfoo\n\n");
        let eps = document(&fig, "eps", SaveOptions::default())?;
        let ps = document(&fig, "ps", SaveOptions::default())?;
        Ok(Json::Bool(!eps.is_empty() && !ps.is_empty()))
    })
}

/// The most times one name is defined in the EPS of sixteen polar axes.
fn most_definitions(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let axs = fig
            .polar_subplots(4, 4, Share::None, Share::None)
            .map_err(ProbeError::rejected)?;
        for ax in axs.as_slice() {
            ax.set_xticks(Vec::<f64>::new())
                .set_yticks(Vec::<f64>::new());
            ax.plot([0.0, 1.0], [1.0, 2.0])
                .map_err(ProbeError::rejected)?;
        }
        fig.suptitle("hello, world");
        let document = document(&fig, "eps", SaveOptions::default())?;
        let mut counts: HashMap<&str, i64> = HashMap::new();
        for line in document.lines().filter(|line| line.starts_with('/')) {
            let word = line.split(' ').next().unwrap_or(line);
            *counts.entry(word).or_default() += 1;
        }
        Ok(Json::Integer(counts.into_values().max().unwrap_or(0)))
    })
}

/// Whether an EPS figure saves with the paper type.
fn papertype(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let paper: Paper = args
            .str("papertype")?
            .parse()
            .map_err(ProbeError::rejected)?;
        let options = SaveOptions {
            papertype: Some(paper),
            ..SaveOptions::default()
        };
        document(&Figure::new(), "eps", options)?;
        Ok(Json::Bool(true))
    })
}

/// The value `ps.papersize` takes.
fn papersize(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        plt::rc::set("ps.papersize", args.str("value")?).map_err(ProbeError::rejected)?;
        Ok(Json::from(plt::rc::params().ps_papersize.to_string()))
    })
}
