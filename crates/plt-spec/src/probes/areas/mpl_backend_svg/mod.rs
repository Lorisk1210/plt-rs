//! Probe operations for the `mpl_backend_svg` area: matplotlib's own tests
//! of its SVG backend that read the document (see
//! `spec/reference/probes/test_mpl_backend_svg.py`, which documents every
//! op).

mod gids;
mod metadata;

use plt::artist::ArtistHandle;
use plt::axes::{ErrorSize, ErrorbarFormat, ErrorbarOptions, ImshowOptions, TickLabelProps};
use plt::figure::{BboxInches, Figure, Metadata, SaveFormat, SaveOptions};
use plt::lines::LineStyle;
use plt::markers::Marker;
use plt::numeric::Grid;
use plt::transforms::Transform;
use plt_core::{Affine2D, Path};
use plt_render::Timestamp;

use super::mpl_tests::in_test_style;
use crate::figures::vector::xml::{self, Element};
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("Artist.set_visible", hidden_bars_parse),
    Op::new("Text.set_url", suptitle_url),
    Op::new("Artist.set_rasterized", bitmaps),
    Op::new("Text.set_fontstretch", stretched_text),
    Op::new("Artist.set_gid", gids::missing_gids),
    Op::new("Artist.set_clip_path", clip_path_ids),
    Op::new("Figure.savefig", gzipped),
    Op::new("Collection.set_urls", urls),
    Op::new("Tick.set_url", tick_urls),
    Op::new("savefig.svg.metadata", metadata::metadata),
    Op::new("Text.set_gid", escaped_gid),
    Op::new("AnnotationBbox.set_gid", gids::annotation_box_gid),
    Op::new("savefig.svg.id", root_id),
];

/// The `SOURCE_DATE_EPOCH` of the tests that compare documents or dates.
const EPOCH: Timestamp = Timestamp(19_680_801);

/// The SVG document of `fig`.
fn svg(fig: &Figure) -> Result<String, ProbeError> {
    fig.to_svg().map_err(ProbeError::rejected)
}

/// The root element of an SVG document.
fn root(document: &str) -> Result<Element, ProbeError> {
    xml::parse(document).map_err(ProbeError::rejected)
}

/// Whether the SVG of an error bar plot with hidden bar lines parses.
fn hidden_bars_parse(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        let x: Vec<f64> = (0..50)
            .map(|i| 4.0 * std::f64::consts::PI * f64::from(i) / 49.0)
            .collect();
        let y: Vec<f64> = x.iter().map(|x| x.sin()).collect();
        let options = ErrorbarOptions {
            yerr: Some(ErrorSize::Symmetric(vec![1.0; 50])),
            fmt: ErrorbarFormat::Format("ko".parse().map_err(ProbeError::rejected)?),
            ..ErrorbarOptions::default()
        };
        let bars = ax.errorbar(x, y, options).map_err(ProbeError::rejected)?;
        for bar in &bars.barlinecols {
            bar.set_visible(false);
        }
        root(&svg(&fig)?)?;
        Ok(Json::Bool(true))
    })
}

/// Whether a suptitle's link wraps it in the SVG.
fn suptitle_url(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let url = "http://test_text_urls.matplotlib.org";
        fig.suptitle("test_text_urls").set_url(url);
        let expected = format!("<a xlink:href=\"{url}\" target=\"_blank\">");
        Ok(Json::Bool(svg(&fig)?.contains(&expected)))
    })
}

/// How many images and paths the SVG of `test_count_bitmaps`' figure
/// holds.
fn bitmaps(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let figure = args.i64("figure")?;
        let fig = Figure::new();
        if figure == 5 {
            fig.set_suppress_composite(Some(true));
        }
        let ax = fig.add_subplot(1, 1, 1).map_err(ProbeError::rejected)?;
        ax.set_axis_off();
        if figure == 4 {
            ax.set_rasterized(true);
        }
        for n in 0..5_u32 {
            let n = f64::from(n);
            if figure == 3 || figure == 4 {
                ax.plot([0.0, 20.0], [n, 0.0])
                    .map_err(ProbeError::rejected)?
                    .set_color("b");
            }
            ax.plot([0.0, 20.0], [0.0, n])
                .map_err(ProbeError::rejected)?
                .set_color("b")
                .set_rasterized(figure != 1);
        }
        let document = svg(&fig)?;
        let count = |tag: &str| Json::Integer(document.matches(tag).count() as i64);
        Ok(Json::Object(vec![
            ("image".to_owned(), count("<image")),
            ("path".to_owned(), count("<path")),
        ]))
    })
}

/// Whether a text written as text, condensed, appears in the SVG with
/// its stretch.
fn stretched_text(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let expected = "Unlikely to appear by chance";
        let date = |text| plt::dates::datestr2num(text).map_err(ProbeError::rejected);
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.text(date("2019-06-30")?, 1.0, expected);
        ax.set_xlim(date("2019-01-01")?, date("2019-12-31")?)
            .set_ylim(0.0, 2.0);
        let document = svg(&fig)?;
        Ok(Json::from(vec![
            document.contains(expected),
            document.contains("condensed"),
        ]))
    })
}

/// How many clip paths six images clipped to two circles define, and
/// whether their ids are unique.
fn clip_path_ids(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let circle =
            |radius: f64| Path::unit_circle().transformed(&Affine2D::scaling(radius, radius));
        let images = (0..5_u32).map(|i| (f64::from(i), 10.0)).chain([(0.0, 1.0)]);
        for (value, radius) in images {
            let image = fig
                .subplot()
                .imshow(
                    Grid::from_vec(1, 1, vec![value]).map_err(ProbeError::rejected)?,
                    ImshowOptions::default(),
                )
                .map_err(ProbeError::rejected)?;
            image.set_clip_path(circle(radius), Transform::Display);
        }
        let ids: Vec<String> = descendants(&root(&svg(&fig)?)?)
            .filter(|element| element.name == "clipPath")
            .filter_map(|element| element.attr("id").map(str::to_owned))
            .collect();
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        Ok(Json::Object(vec![
            ("ids".to_owned(), Json::Integer(ids.len() as i64)),
            ("unique".to_owned(), Json::Bool(unique.len() == ids.len())),
        ]))
    })
}

/// Every element under `element`, depth first.
fn descendants(element: &Element) -> impl Iterator<Item = &Element> {
    let mut pending: Vec<&Element> = element.children.iter().rev().collect();
    std::iter::from_fn(move || {
        let next = pending.pop()?;
        pending.extend(next.children.iter().rev());
        Some(next)
    })
}

/// Whether the current figure saved as `format` with `bbox_inches` starts
/// with the gzip signature.
fn gzipped(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let format: SaveFormat = args.str("format")?.parse().map_err(ProbeError::rejected)?;
        let bbox_inches = match args.str("bbox_inches")? {
            "tight" => BboxInches::Tight,
            other => return Err(ProbeError::BadArgs(format!("bad bbox_inches {other}"))),
        };
        let options = SaveOptions {
            bbox_inches: Some(bbox_inches),
            ..SaveOptions::default()
        };
        let document = plt::pyplot::gcf()
            .to_format(format, options)
            .map_err(ProbeError::rejected)?;
        Ok(Json::Bool(document.starts_with(&[0x1f, 0x8b])))
    })
}

/// The links of a scatter, a line and a marker line found in the SVG.
fn urls(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.scatter([1.0, 2.0, 3.0], [4.0, 5.0, 6.0])
            .map_err(ProbeError::rejected)?
            .set_urls(vec![
                Some("https://example.com/foo".to_owned()),
                Some("https://example.com/bar".to_owned()),
                None,
            ]);
        ax.plot([2.0, 3.0, 4.0], [4.0, 5.0, 6.0])
            .map_err(ProbeError::rejected)?
            .set_url("https://example.com/baz");
        let markers = ax
            .plot([3.0, 4.0, 5.0], [4.0, 5.0, 6.0])
            .map_err(ProbeError::rejected)?;
        markers
            .set_linestyle(LineStyle::None)
            .set_marker("x".parse::<Marker>().map_err(ProbeError::rejected)?);
        markers.set_url("https://example.com/quux");
        let document = svg(&fig)?;
        Ok(Json::Array(
            ["foo", "bar", "baz", "quux"]
                .into_iter()
                .filter(|v| document.contains(&format!("https://example.com/{v}")))
                .map(Json::from)
                .collect(),
        ))
    })
}

/// Whether the links of the y tick labels appear in the SVG, and whether
/// the documents linking the ticks and their labels are equal.
fn tick_urls(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let linked = || -> Result<(String, usize), ProbeError> {
            let fig = Figure::new();
            let ax = fig.subplot();
            ax.scatter([1.0, 2.0, 3.0], [4.0, 5.0, 6.0])
                .map_err(ProbeError::rejected)?;
            let ticks = ax.yaxis().ticklocs(false).map_err(ProbeError::rejected)?;
            for i in 0..ticks.len() {
                let props = TickLabelProps {
                    url: Some(format!("https://example.com/{i}")),
                    ..TickLabelProps::default()
                };
                ax.yaxis().set_major_ticklabel_props(i, props);
            }
            let options = SaveOptions {
                metadata: Some(Metadata {
                    date: Some(EPOCH),
                    ..Metadata::default()
                }),
                ..SaveOptions::default()
            };
            let document = fig.to_svg_with(options).map_err(ProbeError::rejected)?;
            Ok((document, ticks.len()))
        };
        let (first, count) = linked()?;
        let (second, _) = linked()?;
        let urls = (0..count)
            .map(|i| first.contains(&format!("https://example.com/{i}")))
            .collect::<Vec<_>>();
        Ok(Json::Object(vec![
            ("urls".to_owned(), Json::from(urls)),
            ("same".to_owned(), Json::Bool(first == second)),
        ]))
    })
}

/// Whether a text and group id needing escapes are escaped.
fn escaped_gid(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        fig.text(0.5, 0.5, "<'\"&>").set_gid("<'\"&>");
        Ok(Json::Bool(
            svg(&fig)?.contains("&lt;&apos;&quot;&amp;&gt;\""),
        ))
    })
}

/// The `id` of the document's root with `svg.id` set.
fn root_id(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        fig.subplot()
            .plot([1.0, 2.0, 3.0], [3.0, 2.0, 1.0])
            .map_err(ProbeError::rejected)?;
        let id = args.opt_str("svg_id")?.unwrap_or("None");
        plt::rc::context(|| {
            plt::rc::set("svg.id", id).map_err(ProbeError::rejected)?;
            let document = svg(&fig)?;
            Ok(root(&document)?.attr("id").map_or(Json::Null, Json::from))
        })
    })
}
