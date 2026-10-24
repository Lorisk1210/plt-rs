//! Probe operations for the `mpl_backend_pdf` area: matplotlib's own tests
//! of its PDF backend that read the document (see
//! `spec/reference/probes/test_mpl_backend_pdf.py`, which documents every
//! op).

mod document;

use plt::artist::ArtistHandle;
use plt::axes::{ImshowOptions, MappedColors, ScatterOptions};
use plt::figure::{Figure, Metadata, PdfPages, SaveFormat, SaveOptions};
use plt::image::{Extent, FigimageOptions};
use plt::numeric::Grid;
use plt_font::{FontChain, Hinting, ShapeOptions};
use plt_render::{Timestamp, Trapped};

use super::mpl_tests::in_test_style;
use super::text_shaping::{bundled_font, database};
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};
use crate::rng::SpecRng;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("PdfPages.get_pagecount", page_counts),
    Op::new("PdfPages.close", finalized),
    Op::new("PdfPages.__exit__", kept),
    Op::new("Axes.imshow", image_count),
    Op::new("Figure.figimage", indexed_image),
    Op::new("savefig.pdf.metadata", metadata),
    Op::new("Text.set_url", links),
    Op::new("Line2D.set_rasterized", empty_rasterized),
    Op::new("get_glyphs_subset", glyphs_subset),
    Op::new("Axes.scatter", scatter_sizes),
];

/// The PDF document of `fig` saved with `options`.
fn pdf(fig: &Figure, options: SaveOptions) -> Result<Vec<u8>, ProbeError> {
    fig.to_format(SaveFormat::Pdf, options)
        .map_err(ProbeError::rejected)
}

/// The page count of a new document, after one figure and after the
/// current figure again.
fn page_counts(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let mut pages = PdfPages::new().map_err(ProbeError::rejected)?;
        let mut counts = vec![pages.page_count()];
        let fig = plt::pyplot::figure();
        fig.subplot()
            .plot([0.0, 1.0, 2.0], [1.0, 2.0, 3.0])
            .map_err(ProbeError::rejected)?;
        pages.savefig(&fig).map_err(ProbeError::rejected)?;
        counts.push(pages.page_count());
        pages
            .savefig(&plt::pyplot::gcf())
            .map_err(ProbeError::rejected)?;
        counts.push(pages.page_count());
        Ok(Json::Array(
            counts
                .into_iter()
                .map(|n| Json::Integer(n as i64))
                .collect(),
        ))
    })
}

/// How often a document of ten titled pages says `startxref`, and
/// whether it is small.
fn finalized(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let mut pages = PdfPages::new().map_err(ProbeError::rejected)?;
        for _ in 0..10 {
            let fig = Figure::new();
            fig.subplot().set_title("This is a long title");
            pages.savefig(&fig).map_err(ProbeError::rejected)?;
        }
        let document = pages.finish().map_err(ProbeError::rejected)?;
        let startxref = document
            .windows(b"startxref".len())
            .filter(|w| *w == b"startxref")
            .count();
        Ok(Json::Object(vec![
            ("startxref".to_owned(), Json::Integer(startxref as i64)),
            ("small".to_owned(), Json::Bool(document.len() < 40_000)),
        ]))
    })
}

/// Whether a document of `pages` empty figures saved to a file exists.
fn kept(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let pages_wanted = args.i64("pages")?;
        let dir = std::env::temp_dir().join(format!(
            "plt-spec-mpl-backend-pdf-{}-{pages_wanted}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).map_err(ProbeError::rejected)?;
        let path = dir.join("a.pdf");
        let mut pages = PdfPages::new().map_err(ProbeError::rejected)?;
        for _ in 0..pages_wanted {
            pages
                .savefig(&plt::pyplot::figure())
                .map_err(ProbeError::rejected)?;
        }
        let saved = pages.save(&path).map_err(ProbeError::rejected);
        let exists = path.exists();
        std::fs::remove_dir_all(&dir).map_err(ProbeError::rejected)?;
        saved.map(|()| Json::Bool(exists))
    })
}

/// How many images a page of two images in one axes holds.
fn image_count(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let rows: Vec<f64> = (-5..5_i32)
            .flat_map(|y| (-5..5).map(move |_| f64::from(y * y).sin()))
            .collect();
        let z = Grid::from_vec(10, 10, rows).map_err(ProbeError::rejected)?;
        let flipped = Grid::from_fn(10, 10, |row, col| {
            z.get(9 - row, col).copied().unwrap_or_default()
        });
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(0.0, 3.0).map_err(ProbeError::rejected)?;
        for (data, left) in [(z, 0.0), (flipped, 2.0)] {
            let options = ImshowOptions {
                extent: Some(Extent::new(left, left + 1.0, 0.0, 1.0)),
                ..ImshowOptions::default()
            };
            ax.imshow(data, options).map_err(ProbeError::rejected)?;
        }
        let mut pages = PdfPages::new().map_err(ProbeError::rejected)?;
        pages.savefig(&fig).map_err(ProbeError::rejected)?;
        let doc = document::parse(&pages.finish().map_err(ProbeError::rejected)?)?;
        Ok(Json::Integer(document::image_count(&doc) as i64))
    })
}

/// Whether a figure image of 256 reds is written indexed, its palette
/// size and whether its samples decode to the image.
fn indexed_image(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let reds: Vec<[u8; 3]> = (0..=255).map(|red| [red, 0, 0]).collect();
        let data = Grid::from_vec(256, 1, reds.clone()).map_err(ProbeError::rejected)?;
        let fig = Figure::new();
        let options = FigimageOptions {
            resize: true,
            ..FigimageOptions::default()
        };
        fig.figimage(data, options).map_err(ProbeError::rejected)?;
        let save = SaveOptions {
            dpi: Some(fig.dpi()),
            ..SaveOptions::default()
        };
        let doc = document::parse(&pdf(&fig, save)?)?;
        let Some(image) = document::indexed_image(&doc)? else {
            return Ok(Json::Object(vec![
                ("indexed".to_owned(), Json::Bool(false)),
                ("colors".to_owned(), Json::Null),
                ("pixels".to_owned(), Json::Null),
            ]));
        };
        let pixels: Option<Vec<[u8; 3]>> = image
            .indices
            .iter()
            .map(|&index| image.palette.get(usize::from(index)).copied())
            .collect();
        let same = (image.width, image.height) == (1, 256) && pixels.as_ref() == Some(&reds);
        Ok(Json::Object(vec![
            ("indexed".to_owned(), Json::Bool(true)),
            (
                "colors".to_owned(),
                Json::Integer(image.palette.len() as i64),
            ),
            ("pixels".to_owned(), Json::Bool(same)),
        ]))
    })
}

/// The plt-rs metadata of the probe's matplotlib `metadata` dictionary,
/// dated by the tests' `SOURCE_DATE_EPOCH` of 0.
fn decode_metadata(entries: &[(String, Json)]) -> Result<Metadata, ProbeError> {
    let mut metadata = Metadata {
        date: Some(Timestamp(0)),
        ..Metadata::default()
    };
    for (key, value) in entries {
        let text = || {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| ProbeError::BadArgs(format!("bad {key} {value}")))
        };
        match key.as_str() {
            "Author" => metadata.author = Some(text()?),
            "Title" => metadata.title = Some(text()?),
            "Subject" => metadata.subject = Some(text()?),
            "Keywords" => metadata.keywords = Some(text()?),
            "ModDate" => {
                let iso = value.get("datetime").and_then(Json::as_str).unwrap_or("");
                let seconds =
                    plt::dates::datestr2num(iso).map_err(ProbeError::rejected)? * 86_400.0;
                metadata.modified = Some(Timestamp(seconds.round() as i64));
            }
            "Trapped" => {
                metadata.trapped = Some(match text()?.as_str() {
                    "True" => Trapped::True,
                    "False" => Trapped::False,
                    _ => Trapped::Unknown,
                });
            }
            other => return Err(ProbeError::BadArgs(format!("unknown key {other}"))),
        }
    }
    Ok(metadata)
}

/// The information dictionary of a figure saved with metadata, alone or
/// as pages of a document.
fn metadata(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let entries = args.json("metadata")?.as_object().unwrap_or(&[]);
        let metadata = decode_metadata(entries)?;
        let fig = Figure::new();
        fig.subplot()
            .plot([0.0, 1.0, 2.0, 3.0, 4.0], [0.0, 1.0, 2.0, 3.0, 4.0])
            .map_err(ProbeError::rejected)?;
        let bytes = match args.get("pages").and_then(Json::as_i64) {
            None => pdf(
                &fig,
                SaveOptions {
                    metadata: Some(metadata),
                    ..SaveOptions::default()
                },
            )?,
            Some(count) => {
                let mut pages = PdfPages::with_metadata(metadata).map_err(ProbeError::rejected)?;
                for _ in 0..count {
                    pages.savefig(&fig).map_err(ProbeError::rejected)?;
                }
                pages.finish().map_err(ProbeError::rejected)?
            }
        };
        let defaults = Metadata::default();
        let names: Vec<&str> = [&defaults.creator, &defaults.producer]
            .into_iter()
            .flatten()
            .map(String::as_str)
            .collect();
        document::info(&document::parse(&bytes)?, &names)
    })
}

/// The link annotations of figure texts with URLs.
fn links(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let size = args.f64_vec("figsize")?;
        let [width, height] = size[..] else {
            return Err(ProbeError::BadArgs("figsize needs two numbers".into()));
        };
        let fig = Figure::with_size(width, height, plt::rc::params().figure_dpi);
        for text in args.json("texts")?.as_array().unwrap_or(&[]) {
            let bad = || ProbeError::BadArgs(format!("bad text {text}"));
            let [x, y, s, rotation, url] = text.as_array().ok_or_else(bad)? else {
                return Err(bad());
            };
            let number = |value: &Json| value.as_f64().ok_or_else(bad);
            fig.text(number(x)?, number(y)?, s.as_str().ok_or_else(bad)?)
                .set_rotation(number(rotation)?)
                .set_url(url.as_str().ok_or_else(bad)?);
        }
        Ok(document::links(&document::parse(&pdf(
            &fig,
            SaveOptions::default(),
        )?)?))
    })
}

/// Whether a rasterized empty line saves as PDF.
fn empty_rasterized(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        fig.subplot()
            .plot(Vec::<f64>::new(), Vec::<f64>::new())
            .map_err(ProbeError::rejected)?
            .set_rasterized(true);
        Ok(Json::Bool(
            pdf(&fig, SaveOptions::default())?.starts_with(b"%PDF"),
        ))
    })
}

/// The characters the subset of a bundled font keeping the glyphs of
/// `chars` maps, whether it maps fewer than the font, and whether both
/// lay `chars` out in as many glyphs.
fn glyphs_subset(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let chars = args.str("chars")?;
        let db = database();
        let whole = db
            .face(bundled_font(args.str("font")?)?)
            .map_err(ProbeError::rejected)?;
        let glyphs = chars.chars().filter_map(|c| whole.glyph_index(c));
        let subset = plt_font::subset(&whole, glyphs).map_err(ProbeError::rejected)?;
        let part = plt_font::Face::parse(&subset.data, 0).map_err(ProbeError::rejected)?;
        let mut mapped: Vec<char> = part
            .mappings()
            .into_iter()
            .filter_map(|(code, _)| char::from_u32(code))
            .collect();
        mapped.sort_unstable();
        let laid_out = |face: &plt_font::Face<'_>| {
            FontChain::new(vec![face.clone()])
                .layout(chars, 12.0, 72.0, &ShapeOptions::default(), Hinting::None)
                .map(|layout| layout.glyphs.len())
                .map_err(ProbeError::rejected)
        };
        Ok(Json::Object(vec![
            (
                "chars".to_owned(),
                Json::from(mapped.into_iter().collect::<String>()),
            ),
            (
                "fewer".to_owned(),
                Json::Bool(part.mappings().len() < whole.mappings().len()),
            ),
            (
                "glyphs".to_owned(),
                Json::Bool(laid_out(&part)? == laid_out(&whole)?),
            ),
        ]))
    })
}

/// Whether coloured markers off the view save within 5000 bytes of an
/// empty scatter, and markers in view at least 15000 bytes more.
fn scatter_sizes(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let mut rng = SpecRng::new(19_680_801);
        let scaled = |values: Vec<f64>| values.into_iter().map(|v| v * 10.0).collect::<Vec<_>>();
        let x = scaled(rng.uniform(1000, 0.0, 1.0));
        let y = scaled(rng.uniform(1000, 0.0, 1.0));
        let c = rng.uniform(1000, 0.0, 1.0);
        let size = |x: Vec<f64>, y: Vec<f64>, c: Option<Vec<f64>>| -> Result<usize, ProbeError> {
            let fig = Figure::new();
            let ax = fig.subplot();
            let options = ScatterOptions {
                colors: c.map_or_else(Default::default, |c| MappedColors::new(c).into()),
                ..ScatterOptions::default()
            };
            ax.scatter_with(x, y, options)
                .map_err(ProbeError::rejected)?;
            ax.set_xlim(20.0, 30.0)
                .and_then(|ax| ax.set_ylim(20.0, 30.0))
                .map_err(ProbeError::rejected)?;
            Ok(pdf(&fig, SaveOptions::default())?.len())
        };
        let shifted = |values: &[f64]| values.iter().map(|v| v + 20.0).collect::<Vec<_>>();
        let offaxis = size(x.clone(), y.clone(), Some(c.clone()))?;
        let empty = size(Vec::new(), Vec::new(), None)?;
        let visible = size(shifted(&x), shifted(&y), Some(c))?;
        Ok(Json::Object(vec![
            ("offaxis".to_owned(), Json::Bool(offaxis < empty + 5_000)),
            ("visible".to_owned(), Json::Bool(visible > empty + 15_000)),
            (
                "visible_vs_offaxis".to_owned(),
                Json::Bool(visible > offaxis + 15_000),
            ),
        ]))
    })
}
