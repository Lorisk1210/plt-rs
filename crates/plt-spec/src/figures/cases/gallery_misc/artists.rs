//! Artists put together by hand: anchored boxes, lines tested against a
//! box, histograms drawn as one compound path, links in vector output,
//! scatter data given by name, artists found by type, rc parameters
//! changed on the fly, offset transforms and a table under stacked bars
//! (`anchored_artists`, `bbox_intersect`, `histogram_path`, `hyperlinks`,
//! `keyword_plotting`, `findobj_demo`, `customize_rc`, `transoffset` and
//! `table_demo`).

use plt::artist::RawHandle;
use plt::axes::{
    ImshowOptions, MappedColors, ScatterColors, ScatterOptions, TickLabelProps, TickLevel,
    TickParams,
};
use plt::colors::colormaps;
use plt::fonts::FontStyle;
use plt::image::{Extent, Interpolation, Origin};
use plt::legend::LegendOptions;
use plt::offsetbox::{
    AnchorLoc, AnchoredBox, AnchoredOptions, AuxTransformBox, BoxItem, DrawingArea, OffsetBox,
    PackAlign, Packer, TextArea,
};
use plt::patches::{BoxStyle, Circle, Ellipse, PathShape, Rectangle};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::stats::{Bins, HistogramOptions, histogram};
use plt::table::{TableLoc, TableOptions};
use plt::text::{HAlign, Text, TextProps, VAlign};
use plt::transforms::OffsetUnits;
use plt_core::{Bbox, Path, PathCode, Point};

use crate::figures::cases::common::{GALLERY_SEED, image_field};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

fn anchored_artists(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_aspect(Aspect::Ratio(1.0))?;
    let label = ax.add_anchored_box(AnchoredBox::new(
        TextArea::from_text("Figure 1a", TextProps::default()),
        AnchoredOptions::new(AnchorLoc::UpperLeft),
    ));
    let rounded: BoxStyle = "round,pad=0.,rounding_size=0.2".parse()?;
    label.patch().update_shape(|frame| frame.boxstyle = rounded);
    let circles = DrawingArea::new(40.0, 20.0, 0.0, 0.0)
        .with_artist(BoxItem::patch(
            Circle::new((10.0, 10.0), 10.0),
            PatchProps::default().facecolor("tab:blue"),
        ))
        .with_artist(BoxItem::patch(
            Circle::new((30.0, 10.0), 5.0),
            PatchProps::default().facecolor("tab:red"),
        ));
    ax.add_anchored_box(AnchoredBox::new(
        circles,
        AnchoredOptions {
            pad: 0.0,
            frameon: false,
            ..AnchoredOptions::new(AnchorLoc::UpperRight)
        },
    ));
    let ellipse = AuxTransformBox::new(ax.trans_data()).with_artist(BoxItem::patch(
        Ellipse::new((0.0, 0.0), 0.1, 0.15),
        PatchProps::default(),
    ));
    ax.add_anchored_box(AnchoredBox::new(
        ellipse,
        AnchoredOptions::new(AnchorLoc::LowerLeft),
    ));
    let sizebar = AuxTransformBox::new(ax.trans_data()).with_artist(BoxItem::line(
        vec![0.0, 0.1],
        vec![0.0, 0.0],
        LineProps::default().color("black"),
    )?);
    let packer = Packer::new(
        0.0,
        5.0,
        PackAlign::Center,
        vec![
            sizebar.into(),
            TextArea::from_text(r"1$^{\prime}$", TextProps::default()).into(),
        ],
    );
    ax.add_anchored_box(AnchoredBox::new(
        OffsetBox::VPacker(packer),
        AnchoredOptions {
            pad: 0.1,
            borderpad: (0.5, 0.5),
            frameon: false,
            ..AnchoredOptions::new(AnchorLoc::LowerCenter)
        },
    ));
    Ok(())
}

fn bbox_intersect(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(GALLERY_SEED);
    let (left, bottom, width, height) = (-1.0, -1.0, 2.0, 2.0);
    let ax = fig.subplot();
    ax.add_patch_with(
        Rectangle::new((left, bottom), width, height),
        PatchProps::default().facecolor("black").alpha(0.1),
    );
    let bbox = Bbox::from_bounds(left, bottom, width, height);
    for _ in 0..12 {
        let v = rng.uniform(4, 0.0, 1.0);
        let scaled = |value: f64| (value - 0.5) * 6.0;
        let (x, y) = ([scaled(v[0]), scaled(v[2])], [scaled(v[1]), scaled(v[3])]);
        let path = Path::polyline(vec![Point::new(x[0], y[0]), Point::new(x[1], y[1])]);
        let color = if path.intersects_bbox(&bbox, true) {
            "r"
        } else {
            "b"
        };
        ax.plot_with(x, y, LineProps::default().color(color))?;
    }
    Ok(())
}

/// The bars of a histogram of 1000 normal samples in 50 bins, each as
/// its corners: bottom left, top left, top right and bottom right.
fn histogram_bars() -> Result<Vec<[Point; 4]>, Box<dyn std::error::Error>> {
    let data = SpecRng::new(GALLERY_SEED).normal(1000, 0.0, 1.0);
    let hist = histogram(
        &data,
        HistogramOptions {
            bins: Bins::Count(50),
            ..HistogramOptions::default()
        },
    )?;
    Ok(hist
        .edges
        .windows(2)
        .zip(&hist.counts)
        .map(|(edges, &top)| {
            [
                Point::new(edges[0], 0.0),
                Point::new(edges[0], top),
                Point::new(edges[1], top),
                Point::new(edges[1], 0.0),
            ]
        })
        .collect())
}

/// `barpath` as a patch that adds no margin at y = 0, autoscaled.
fn add_histogram_patch(fig: &Figure, barpath: Path) {
    let ax = fig.subplot();
    let patch = ax.add_patch(PathShape(barpath));
    patch.set_sticky_edges(Vec::new(), vec![0.0]);
    ax.autoscale_view(None);
}

fn histogram_path_1(fig: &Figure) -> CaseResult {
    add_histogram_patch(fig, Path::compound_from_polygons(&histogram_bars()?));
    Ok(())
}

fn histogram_path_2(fig: &Figure) -> CaseResult {
    let bars = histogram_bars()?;
    let mut vertices = Vec::with_capacity(bars.len() * 5);
    let mut codes = Vec::with_capacity(bars.len() * 5);
    for corners in bars {
        vertices.extend(corners);
        vertices.push(Point::ORIGIN);
        codes.extend([
            PathCode::MoveTo,
            PathCode::LineTo,
            PathCode::LineTo,
            PathCode::LineTo,
            PathCode::ClosePoly,
        ]);
    }
    add_histogram_patch(fig, Path::new(vertices, codes)?);
    Ok(())
}

fn hyperlinks_1(fig: &Figure) -> CaseResult {
    let points = fig.subplot().scatter([1.0, 2.0, 3.0], [4.0, 5.0, 6.0])?;
    points.set_urls(vec![
        Some("https://www.bbc.com/news".into()),
        Some("https://www.google.com/".into()),
        None,
    ]);
    Ok(())
}

fn hyperlinks_2(fig: &Figure) -> CaseResult {
    let options = ImshowOptions {
        origin: Some(Origin::Lower),
        extent: Some(Extent::new(-3.0, 3.0, -3.0, 3.0)),
        interpolation: Some(Interpolation::Bilinear),
        ..ImshowOptions::default()
    }
    .cmap(colormaps::get("gray")?);
    fig.subplot()
        .imshow(image_field(), options)?
        .set_url("https://www.google.com/");
    Ok(())
}

fn keyword_plotting(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(GALLERY_SEED);
    let a: Vec<f64> = (0..50).map(f64::from).collect();
    let c: Vec<f64> = rng
        .integers(50, 0, 50)
        .into_iter()
        .map(|v| v as f64)
        .collect();
    let d: Vec<f64> = rng.normal(50, 0.0, 1.0);
    let b: Vec<f64> = a
        .iter()
        .zip(rng.normal(50, 0.0, 1.0))
        .map(|(a, n)| a + 10.0 * n)
        .collect();
    let sizes = d.iter().map(|d| d.abs() * 100.0).collect();
    let ax = fig.subplot();
    ax.scatter_with(
        &a,
        &b,
        ScatterOptions {
            sizes: Some(sizes),
            colors: ScatterColors::from(MappedColors::new(c)),
            ..ScatterOptions::default()
        },
    )?;
    ax.set_xlabel("entry a").set_ylabel("entry b");
    Ok(())
}

fn findobj_demo(fig: &Figure) -> CaseResult {
    let a = arange(0.0, 3.0, 0.02);
    let c: Vec<f64> = a.iter().map(|v| v.exp()).collect();
    let d: Vec<f64> = c.iter().rev().copied().collect();
    let total: Vec<f64> = c.iter().zip(&d).map(|(c, d)| c + d).collect();
    let ax = fig.subplot();
    ax.plot_fmt(&a, &c, "k--")?;
    ax.plot_fmt(&a, &d, "k:")?;
    ax.plot_fmt(&a, &total, "k")?;
    ax.legend(LegendOptions {
        labels: Some(
            ["Model length", "Data length", "Total message length"]
                .map(String::from)
                .to_vec(),
        ),
        shadow: Some(true.into()),
        ..LegendOptions::default().loc(AnchorLoc::UpperCenter)
    })?;
    ax.set_ylim(-1.0, 20.0)?;
    ax.grid(false);
    ax.set_xlabel("Model complexity --->")
        .set_ylabel("Message length --->");
    ax.set_title("Minimum Message Length");
    // Lines and texts, matched by type.
    let artists = fig.findobj(|_| true);
    for line in artists.iter().filter_map(RawHandle::downcast::<Line2D>) {
        line.set_color("blue");
    }
    for text in artists.iter().filter_map(RawHandle::downcast::<Text>) {
        text.set_color("blue").set_style(FontStyle::Italic);
    }
    // Tick lines and labels and the axis labels are parts of their axis
    // in plt-rs, not artists of their own: they take the same styling
    // through the axis.
    ax.tick_params(TickParams {
        labelcolor: Some("blue".into()),
        ..TickParams::default()
    });
    for axis in [ax.xaxis(), ax.yaxis()] {
        axis.set_tick_line_color("blue");
        axis.set_ticklabel_props(
            TickLevel::Both,
            TickLabelProps {
                fontstyle: Some(FontStyle::Italic),
                ..TickLabelProps::default()
            },
        );
        axis.set_label_color("blue")
            .set_label_style(FontStyle::Italic);
    }
    Ok(())
}

fn customize_rc(fig: &Figure) -> CaseResult {
    let ramp = [1.0, 2.0, 3.0];
    fig.add_subplot(3, 1, 1)?.plot_y(ramp);
    // `plt.rcdefaults()` returns to the reference configuration here.
    let reference = plt::rc::params();
    for (key, value) in [
        ("font.weight", "bold"),
        ("xtick.major.size", "5"),
        ("xtick.major.pad", "7"),
        ("xtick.labelsize", "15"),
        ("grid.color", "0.5"),
        ("grid.linestyle", "-"),
        ("grid.linewidth", "5"),
        ("lines.linewidth", "2"),
        ("lines.color", "g"),
    ] {
        plt::rc::set(key, value)?;
    }
    let ax = fig.add_subplot(3, 1, 2)?;
    ax.plot_y(ramp);
    ax.grid(true);
    plt::rc::replace((*reference).clone());
    let ax = fig.add_subplot(3, 1, 3)?;
    ax.plot_y(ramp);
    ax.grid(true);
    Ok(())
}

fn transoffset(fig: &Figure) -> CaseResult {
    let points: Vec<(f64, f64)> = (0..7).map(|x| (f64::from(x), f64::from(x * x))).collect();
    let ax = fig.add_subplot(2, 1, 1)?;
    let shifted = ax.trans_data().offset_inches(0.05, 0.10);
    for &(x, y) in &points {
        ax.plot_fmt([x], [y], "ro")?;
        ax.text(x, y, format!("{x}, {y}"))
            .set_transform(shifted.clone());
    }
    let polar = fig.add_polar_subplot(2, 1, 2)?;
    let raised = polar.trans_data().offset_copy(0.0, 6.0, OffsetUnits::Dots);
    for &(x, y) in &points {
        polar.plot_fmt([x], [y], "ro")?;
        polar
            .text(x, y, format!("{x}, {y}"))
            .set_transform(raised.clone())
            .set_halign(HAlign::Center)
            .set_valign(VAlign::Bottom);
    }
    Ok(())
}

fn table_demo(fig: &Figure) -> CaseResult {
    let data = [
        [66386.0, 174_296.0, 75131.0, 577_908.0, 32015.0],
        [58230.0, 381_139.0, 78045.0, 99308.0, 160_454.0],
        [89135.0, 80552.0, 152_558.0, 497_981.0, 603_535.0],
        [78415.0, 81858.0, 150_656.0, 193_263.0, 69638.0],
        [139_361.0, 331_509.0, 343_164.0, 781_380.0, 52269.0],
    ];
    let columns = ["Freeze", "Wind", "Flood", "Quake", "Hail"].map(String::from);
    let rows = [100, 50, 20, 10, 5].map(|years| format!("{years} year"));
    let bupu = colormaps::get("BuPu")?;
    let colors: Vec<Color> = linspace(0.0, 0.5, rows.len())
        .into_iter()
        .map(|x| Color::from(bupu.sample(x)))
        .collect();
    let index: Vec<f64> = (0..columns.len()).map(|i| i as f64 + 0.3).collect();
    let mut y_offset = vec![0.0; columns.len()];
    let ax = fig.subplot();
    let mut cell_text = Vec::new();
    for (row, color) in data.iter().zip(&colors) {
        ax.bar(
            index.as_slice(),
            row,
            BarOptions {
                thickness: 0.4.into(),
                base: y_offset.clone().into(),
                color: vec![color.clone()],
                ..BarOptions::default()
            },
        )?;
        for (offset, value) in y_offset.iter_mut().zip(row) {
            *offset += value;
        }
        cell_text.push(
            y_offset
                .iter()
                .map(|v| format!("{:.1}", v / 1000.0))
                .collect::<Vec<_>>(),
        );
    }
    cell_text.reverse();
    ax.table(TableOptions {
        row_labels: Some(rows.to_vec()),
        row_colours: Some(colors.into_iter().rev().collect()),
        col_labels: Some(columns.to_vec()),
        loc: TableLoc::Bottom,
        ..TableOptions::with_text(cell_text)
    })?;
    fig.subplots_adjust(plt::gridspec::SubplotOverrides {
        left: Some(0.2),
        bottom: Some(0.2),
        ..plt::gridspec::SubplotOverrides::default()
    })?;
    ax.set_ylabel("Loss in $1000's");
    let values: Vec<f64> = (0..5).map(|i| f64::from(i) * 500.0).collect();
    ax.set_yticks(values.iter().map(|v| v * 1000.0).collect::<Vec<_>>());
    ax.set_yticklabels(values.iter().map(|v| format!("{v}")))?;
    ax.set_xticks(Vec::<f64>::new());
    ax.set_title("Loss by Disaster");
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_misc/anchored_artists", anchored_artists),
    FigureCase::new("gallery_misc/bbox_intersect", bbox_intersect),
    FigureCase::new("gallery_misc/histogram_path_1", histogram_path_1),
    FigureCase::new("gallery_misc/histogram_path_2", histogram_path_2),
    FigureCase::new("gallery_misc/hyperlinks_1", hyperlinks_1),
    FigureCase::new("gallery_misc/hyperlinks_2", hyperlinks_2),
    FigureCase::new("gallery_misc/keyword_plotting", keyword_plotting),
    FigureCase::new("gallery_misc/findobj_demo", findobj_demo),
    FigureCase::new("gallery_misc/customize_rc", customize_rc),
    FigureCase::new("gallery_misc/transoffset", transoffset).sized(5.0, 10.0, 100.0),
    FigureCase::new("gallery_misc/table_demo", table_demo),
];
