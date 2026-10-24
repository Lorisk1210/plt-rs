//! The showcase section: the anatomy of a figure, the Firefox logo from
//! an SVG path, overlapping axes, stock prices over three decades and an
//! xkcd bar chart (`anatomy`, `firefox`, `pan_zoom_overlap`,
//! `stock_prices` and `xkcd`).

use plt::axes::{AxisName, GridOptions, TickLevel, TickParams, TickPosition};
use plt::dates::iso::Datetime64;
use plt::fonts::{FontSize, FontStyle, FontWeight, RelativeSize};
use plt::layout::LayoutEngine;
use plt::legend::LegendOptions;
use plt::offsetbox::AnchorLoc;
use plt::patches::{Circle, PathShape};
use plt::prelude::*;
use plt::rc::{Aspect, Cycler};
use plt::spines::SpineSide;
use plt::text::{HAlign, VAlign};
use plt::ticker::{AutoMinorLocator, MinorDivisions, MultipleLocator, StrMethodFormatter};
use plt_core::{Path, PathCode, Point};

use crate::figures::cases::common::{
    GALLERY_SEED, datetime64, hide_top_right, map, tickless, with_stroke, xkcd,
};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The circle, name and code of one element of `anatomy`.
fn anatomy_label(ax: &Axes, (x, y): (f64, f64), text: &str, code: &str) {
    let royal_blue = [0.0, 20.0 / 256.0, 82.0 / 256.0];
    let halo = || with_stroke(7.0, "white");
    ax.add_artist_with(
        Circle::new((x, y), 0.15),
        PatchProps::default()
            .clip_on(false)
            .zorder(10.0)
            .linewidth(2.5)
            .edgecolor([royal_blue[0], royal_blue[1], royal_blue[2], 0.6])
            .facecolor("none")
            .path_effects(halo()),
    );
    // The halos and the coloured texts are drawn separately, so that a
    // halo cannot cover another text.
    for haloed in [true, false] {
        let name = ax.text(x, y - 0.2, text);
        name.set_zorder(100.0);
        name.set_halign(HAlign::Center)
            .set_valign(VAlign::Top)
            .set_weight(FontWeight::BOLD)
            .set_style(FontStyle::Italic)
            .set_family(["monospace"]);
        let call = ax.text(x, y - 0.33, code);
        call.set_zorder(100.0);
        call.set_halign(HAlign::Center)
            .set_valign(VAlign::Top)
            .set_weight(FontWeight::NORMAL)
            .set_family(["monospace"])
            .set_fontsize(RelativeSize::Medium);
        if haloed {
            name.set_color("white").set_path_effects(halo());
            call.set_color("white").set_path_effects(halo());
        } else {
            name.set_color(royal_blue);
            call.set_color("black");
        }
    }
}

/// The elements `anatomy` names: where their circle goes, their name and
/// the code making them.
const ANATOMY_LABELS: [((f64, f64), &str, &str); 16] = [
    (
        (3.5, -0.13),
        "Minor tick label",
        "ax.xaxis.set_minor_formatter",
    ),
    ((-0.03, 1.0), "Major tick", "ax.yaxis.set_major_locator"),
    ((0.00, 3.75), "Minor tick", "ax.yaxis.set_minor_locator"),
    (
        (-0.15, 3.00),
        "Major tick label",
        "ax.yaxis.set_major_formatter",
    ),
    ((1.68, -0.39), "xlabel", "ax.set_xlabel"),
    ((-0.38, 1.67), "ylabel", "ax.set_ylabel"),
    ((1.52, 4.15), "Title", "ax.set_title"),
    ((1.75, 2.80), "Line", "ax.plot"),
    ((2.25, 1.54), "Markers", "ax.scatter"),
    ((3.00, 3.00), "Grid", "ax.grid"),
    ((3.60, 3.58), "Legend", "ax.legend"),
    ((2.5, 0.55), "Axes", "fig.subplots"),
    ((4.0, 4.5), "Figure", "plt.figure"),
    ((0.65, 0.01), "x Axis", "ax.xaxis"),
    ((0.0, 0.36), "y Axis", "ax.yaxis"),
    ((4.0, 0.7), "Spine", "ax.spines"),
];

/// The square axes of `anatomy`: ticks every unit with four minor
/// intervals, large tick labels and a dashed grid.
fn anatomy_axes(fig: &Figure) -> Result<Axes, Box<dyn std::error::Error>> {
    let ax = fig.add_axes([0.2, 0.17, 0.68, 0.7])?;
    ax.set_aspect(Aspect::Ratio(1.0))?;
    for axis in [ax.xaxis(), ax.yaxis()] {
        axis.set_major_locator(MultipleLocator::new(1.0)?)
            .set_minor_locator(AutoMinorLocator::new(Some(MinorDivisions::Count(4))));
    }
    ax.xaxis()
        .set_minor_formatter(StrMethodFormatter::new("{x:.2f}")?);
    ax.set_xlim(0.0, 4.0)?.set_ylim(0.0, 4.0)?;
    ax.tick_params(&TickParams {
        which: TickLevel::Major,
        width: Some(1.0),
        length: Some(10.0),
        labelsize: Some(FontSize::Points(14.0)),
        ..TickParams::default()
    });
    ax.tick_params(&TickParams {
        which: TickLevel::Minor,
        width: Some(1.0),
        length: Some(5.0),
        labelsize: Some(FontSize::Points(10.0)),
        labelcolor: Some("0.25".into()),
        ..TickParams::default()
    });
    ax.grid_with(
        true,
        GridOptions {
            linestyle: Some(LineStyle::Dashed),
            linewidth: Some(0.5),
            color: Some(".25".into()),
            ..GridOptions::default()
        },
    );
    Ok(ax)
}

fn anatomy(fig: &Figure) -> CaseResult {
    let x = linspace(0.5, 3.5, 100);
    let y1 = map(&x, |x| 3.0 + x.cos());
    let y2 = map(&x, |x| 1.0 + (1.0 + x / 0.75).cos() / 2.0);
    let u = SpecRng::new(GALLERY_SEED).uniform(x.len(), 0.0, 1.0);
    let y3: Vec<f64> = (0..x.len())
        .map(|i| y1[i] + (y2[i] - y1[i]) * u[i])
        .collect();
    let ax = anatomy_axes(fig)?;
    ax.plot_with(
        &x,
        &y1,
        LineProps::default()
            .color("C0")
            .linewidth(2.5)
            .label("Blue signal")
            .zorder(10.0),
    )?;
    ax.plot_with(
        &x,
        &y2,
        LineProps::default()
            .color("C1")
            .linewidth(2.5)
            .label("Orange signal"),
    )?;
    let every_third = |values: &[f64]| values.iter().step_by(3).copied().collect::<Vec<_>>();
    ax.plot_with(
        every_third(&x),
        every_third(&y3),
        LineProps::default()
            .linewidth(0.0)
            .markersize(9.0)
            .marker("s".parse::<Marker>()?)
            .markerfacecolor(Color::from("none"))
            .markeredgecolor(Color::from("C4"))
            .markeredgewidth(2.5),
    )?;
    ax.set_title("Anatomy of a figure")
        .set_fontsize(20.0)
        .set_valign(VAlign::Bottom);
    ax.set_xlabel("x Axis label").set_ylabel("y Axis label");
    for name in [AxisName::X, AxisName::Y] {
        ax.axis_of(name).set_label_fontsize(FontSize::Points(14.0));
    }
    ax.legend(LegendOptions {
        fontsize: Some(FontSize::Points(14.0)),
        ..LegendOptions::default().loc(AnchorLoc::UpperRight)
    })?;
    for (xy, text, code) in ANATOMY_LABELS {
        anatomy_label(&ax, xy, text, code);
    }
    fig.set_linewidth(4.0);
    fig.set_edgecolor(Color::from("0.5").resolve()?);
    Ok(())
}

/// The Firefox logo as an SVG path (from Raphaël's icon set).
const FIREFOX: &str = "M28.4,22.469c0.479-0.964,0.851-1.991,1.095-3.066c0.953-3.661,0.666-6.854,0.666-6.854l-0.327,2.104c0,0-0.469-3.896-1.044-5.353c-0.881-2.231-1.273-2.214-1.274-2.21c0.542,1.379,0.494,2.169,0.483,2.288c-0.01-0.016-0.019-0.032-0.027-0.047c-0.131-0.324-0.797-1.819-2.225-2.878c-2.502-2.481-5.943-4.014-9.745-4.015c-4.056,0-7.705,1.745-10.238,4.525C5.444,6.5,5.183,5.938,5.159,5.317c0,0-0.002,0.002-0.006,0.005c0-0.011-0.003-0.021-0.003-0.031c0,0-1.61,1.247-1.436,4.612c-0.299,0.574-0.56,1.172-0.777,1.791c-0.375,0.817-0.75,2.004-1.059,3.746c0,0,0.133-0.422,0.399-0.988c-0.064,0.482-0.103,0.971-0.116,1.467c-0.09,0.845-0.118,1.865-0.039,3.088c0,0,0.032-0.406,0.136-1.021c0.834,6.854,6.667,12.165,13.743,12.165l0,0c1.86,0,3.636-0.37,5.256-1.036C24.938,27.771,27.116,25.196,28.4,22.469zM16.002,3.356c2.446,0,4.73,0.68,6.68,1.86c-2.274-0.528-3.433-0.261-3.423-0.248c0.013,0.015,3.384,0.589,3.981,1.411c0,0-1.431,0-2.856,0.41c-0.065,0.019,5.242,0.663,6.327,5.966c0,0-0.582-1.213-1.301-1.42c0.473,1.439,0.351,4.17-0.1,5.528c-0.058,0.174-0.118-0.755-1.004-1.155c0.284,2.037-0.018,5.268-1.432,6.158c-0.109,0.07,0.887-3.189,0.201-1.93c-4.093,6.276-8.959,2.539-10.934,1.208c1.585,0.388,3.267,0.108,4.242-0.559c0.982-0.672,1.564-1.162,2.087-1.047c0.522,0.117,0.87-0.407,0.464-0.872c-0.405-0.466-1.392-1.105-2.725-0.757c-0.94,0.247-2.107,1.287-3.886,0.233c-1.518-0.899-1.507-1.63-1.507-2.095c0-0.366,0.257-0.88,0.734-1.028c0.58,0.062,1.044,0.214,1.537,0.466c0.005-0.135,0.006-0.315-0.001-0.519c0.039-0.077,0.015-0.311-0.047-0.596c-0.036-0.287-0.097-0.582-0.19-0.851c0.01-0.002,0.017-0.007,0.021-0.021c0.076-0.344,2.147-1.544,2.299-1.659c0.153-0.114,0.55-0.378,0.506-1.183c-0.015-0.265-0.058-0.294-2.232-0.286c-0.917,0.003-1.425-0.894-1.589-1.245c0.222-1.231,0.863-2.11,1.919-2.704c0.02-0.011,0.015-0.021-0.008-0.027c0.219-0.127-2.524-0.006-3.76,1.604C9.674,8.045,9.219,7.95,8.71,7.95c-0.638,0-1.139,0.07-1.603,0.187c-0.05,0.013-0.122,0.011-0.208-0.001C6.769,8.04,6.575,7.88,6.365,7.672c0.161-0.18,0.324-0.356,0.495-0.526C9.201,4.804,12.43,3.357,16.002,3.356z";

/// The numbers of one SVG command, separated by commas or by the sign
/// starting the next number.
fn svg_numbers(values: &str) -> Result<Vec<f64>, std::num::ParseFloatError> {
    let mut numbers = Vec::new();
    let mut start = 0;
    for (i, c) in values.char_indices() {
        if c == ',' || ((c == '-' || c == '+') && i > start) {
            if i > start {
                numbers.push(values[start..i].parse()?);
            }
            start = if c == ',' { i + 1 } else { i };
        }
    }
    if start < values.len() {
        numbers.push(values[start..].parse()?);
    }
    Ok(numbers)
}

/// The example's `svg_parse`: an SVG path of `M`, `L`, `Q`, `C` and `Z`
/// commands (lower case: relative to the previous point) as a path.
fn svg_parse(path: &str) -> Result<Path, Box<dyn std::error::Error>> {
    let mut vertices: Vec<Point> = Vec::new();
    let mut codes = Vec::new();
    let starts: Vec<usize> = path
        .char_indices()
        .filter(|(_, c)| c.is_ascii_alphabetic())
        .map(|(i, _)| i)
        .collect();
    for (k, &start) in starts.iter().enumerate() {
        let end = starts.get(k + 1).copied().unwrap_or(path.len());
        let command = path[start..].chars().next().ok_or("empty command")?;
        let values = svg_numbers(&path[start + 1..end])?;
        let mut points: Vec<Point> = if values.is_empty() {
            vec![Point::ORIGIN]
        } else {
            values
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&[x, y]| Point::new(x, y))
                .collect()
        };
        if command.is_ascii_lowercase() {
            let last = *vertices.last().ok_or("relative command first")?;
            for point in &mut points {
                *point = Point::new(point.x + last.x, point.y + last.y);
            }
        }
        let code = match command.to_ascii_uppercase() {
            'M' => vec![PathCode::MoveTo],
            'L' => vec![PathCode::LineTo],
            'Q' => vec![PathCode::Curve3; 2],
            'C' => vec![PathCode::Curve4; 3],
            'Z' => vec![PathCode::ClosePoly],
            other => return Err(format!("unsupported SVG command {other:?}").into()),
        };
        codes.extend(code);
        vertices.extend(points);
    }
    Ok(Path::new(vertices, codes)?)
}

fn firefox(fig: &Figure) -> CaseResult {
    let path = svg_parse(FIREFOX)?;
    let bounds = path.vertices().iter().fold(
        (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |(x0, y0, x1, y1), p| (x0.min(p.x), y0.min(p.y), x1.max(p.x), y1.max(p.y)),
    );
    let (xmin, ymin, xmax, ymax) = (
        bounds.0 - 1.0,
        bounds.1 - 1.0,
        bounds.2 + 1.0,
        bounds.3 + 1.0,
    );
    fig.set_facecolor(Color::from("0.75").resolve()?);
    let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0])?;
    ax.set_frame_on(false);
    ax.set_aspect(Aspect::Ratio(1.0))?;
    ax.set_xlim(xmin, xmax)?.set_ylim(ymax, ymin)?;
    tickless(&ax);
    ax.add_patch_with(
        PathShape(path.clone()),
        PatchProps::default()
            .facecolor("none")
            .edgecolor("w")
            .linewidth(6.0),
    );
    ax.add_patch_with(
        PathShape(path),
        PatchProps::default()
            .facecolor("orange")
            .edgecolor("k")
            .linewidth(2.0),
    );
    Ok(())
}

/// A text centred in `ax`'s box.
fn centred_note(ax: &Axes, text: &str) {
    ax.text(0.5, 0.5, text)
        .set_halign(HAlign::Center)
        .set_valign(VAlign::Center)
        .set_transform(ax.trans_axes());
}

fn pan_zoom_overlap(fig: &Figure) -> CaseResult {
    fig.suptitle("Showcase for pan/zoom events on overlapping axes.");
    let ax = fig.add_axes([0.05, 0.05, 0.9, 0.9])?;
    // `ax.patch.set_color(".75")`: face and edge.
    ax.set_facecolor(".75").set_patch_edgecolor(".75");
    let _twin = ax.twinx();
    let ax1 = fig.add_subplot(2, 2, 1)?;
    let _twin1 = ax1.twinx();
    centred_note(
        &ax1,
        "Visible patch\n\nPan/zoom events are NOT\nforwarded to axes below",
    );
    let ax11 = fig.add_subplot(2, 2, 3)?;
    ax11.share_x(&ax1)?.share_y(&ax1)?;
    ax11.set_forward_navigation_events(Some(true));
    centred_note(
        &ax11,
        "Visible patch\n\nOverride capture behavior:\n\nax.set_forward_navigation_events(True)",
    );
    let ax2 = fig.add_subplot(2, 2, 2)?;
    let _twin2 = ax2.twinx();
    ax2.set_patch_visible(false);
    centred_note(
        &ax2,
        "Invisible patch\n\nPan/zoom events are\nforwarded to axes below",
    );
    let ax22 = fig.add_subplot(2, 2, 4)?;
    ax22.share_x(&ax2)?.share_y(&ax2)?;
    ax22.set_patch_visible(false);
    ax22.set_forward_navigation_events(Some(false));
    centred_note(
        &ax22,
        "Invisible patch\n\nOverride capture behavior:\n\nax.set_forward_navigation_events(False)",
    );
    Ok(())
}

/// The tickers of `stock_prices`, their names, label nudges in points
/// and, for their synthetic prices, the first month quoted, the first
/// price and the mean monthly log return.
const STOCKS: [(&str, f64, usize, f64, f64); 10] = [
    ("IBM", 5.0, 0, 11.0, 0.0066),
    ("Apple", -5.0, 0, 0.24, 0.0167),
    ("Microsoft", 0.0, 0, 0.4, 0.0167),
    ("Xerox", 0.0, 0, 11.2, 0.001),
    ("Amazon", -6.0, 89, 0.08, 0.0245),
    ("Dell", 0.0, 320, 13.3, 0.015),
    ("Alphabet", 0.0, 176, 65.0, 0.0175),
    ("Adobe", 0.0, 0, 1.4, 0.016),
    ("S&P 500", 0.0, 0, 330.0, 0.0065),
    ("NASDAQ", 0.0, 0, 416.0, 0.0087),
];

/// The first days of the months from January 1990 to June 2022.
fn months() -> Result<Vec<Datetime64>, Box<dyn std::error::Error>> {
    (1990..=2022)
        .flat_map(|year| (1..=12).map(move |month| (year, month)))
        .take_while(|&(year, month)| year < 2022 || month <= 6)
        .map(|(year, month)| datetime64(&format!("{year}-{month:02}-01")))
        .collect()
}

/// A geometric random walk of monthly prices, NaN before month `first`.
fn price_walk(rng: &mut SpecRng, months: usize, first: usize, start: f64, drift: f64) -> Vec<f64> {
    let steps = rng.normal(months, drift, 0.07);
    let mut log_price = 0.0;
    steps
        .iter()
        .enumerate()
        .map(|(i, step)| {
            if i > first {
                log_price += step;
            }
            if i < first {
                f64::NAN
            } else {
                start * log_price.exp()
            }
        })
        .collect()
}

fn stock_prices(fig: &Figure) -> CaseResult {
    let dates = months()?;
    let mut rng = SpecRng::new(GALLERY_SEED);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    let colors = [
        "#1f77b4", "#aec7e8", "#ff7f0e", "#ffbb78", "#2ca02c", "#98df8a", "#d62728", "#ff9896",
        "#9467bd", "#c5b0d5", "#8c564b", "#c49c94", "#e377c2", "#f7b6d2", "#7f7f7f", "#c7c7c7",
        "#bcbd22", "#dbdb8d", "#17becf", "#9edae5",
    ];
    ax.set_prop_cycle(Some(Cycler::colors_of(colors.map(Color::from))));
    let label_x = ax.xaxis().convert_units(vec![datetime64("2022-10-01")?])[0];
    for (name, nudge, first, start, drift) in STOCKS {
        let prices = price_walk(&mut rng, dates.len(), first, start, drift);
        let (quoted, values): (Vec<Datetime64>, Vec<f64>) = dates
            .iter()
            .zip(&prices)
            .filter(|(_, p)| p.is_finite())
            .map(|(&d, &p)| (d, p))
            .unzip();
        let line = ax.plot_with(quoted, values, LineProps::default().linewidth(2.5))?;
        let last = *prices.last().ok_or("no prices")?;
        ax.text(label_x, last, name)
            .set_color(line.color())
            .set_transform(ax.trans_data().offset_inches(0.0, nudge / 72.0));
    }
    let limits = ax
        .xaxis()
        .convert_units(vec![datetime64("1989-06-01")?, datetime64("2023-01-01")?]);
    ax.set_xlim(limits[0], limits[1])?;
    fig.suptitle("Technology company stocks prices dollars (1990-2022)")
        .set_halign(HAlign::Center);
    for spine in ax.spines().iter() {
        spine.set_visible(false);
    }
    ax.xaxis().tick_to(SpineSide::Bottom)?;
    ax.yaxis().tick_to(SpineSide::Left)?;
    ax.set_yscale("log")?;
    ax.grid_with(
        true,
        GridOptions {
            linestyle: Some(LineStyle::Dashed),
            linewidth: Some(0.5),
            color: Some("k".into()),
            alpha: Some(0.3),
            ..GridOptions::default()
        },
    );
    ax.tick_params(&TickParams {
        which: TickLevel::Both,
        labelsize: Some(FontSize::Relative(RelativeSize::Large)),
        bottom: Some(false),
        top: Some(false),
        labelbottom: Some(true),
        left: Some(false),
        right: Some(false),
        labelleft: Some(true),
        ..TickParams::default()
    });
    Ok(())
}

fn xkcd_2(fig: &Figure) -> CaseResult {
    let ax = fig.add_axes([0.1, 0.2, 0.8, 0.7])?;
    ax.bar(
        [0.0, 1.0],
        [0.0, 100.0],
        BarOptions {
            thickness: vec![0.25],
            ..BarOptions::default()
        },
    )?;
    hide_top_right(&ax);
    ax.xaxis()
        .set_ticks_position(TickPosition::Side(SpineSide::Bottom))?;
    ax.set_xticks([0.0, 1.0]);
    ax.set_xticklabels(["CONFIRMED BY\nEXPERIMENT", "REFUTED BY\nEXPERIMENT"])?;
    ax.set_xlim(-0.5, 1.5)?;
    ax.set_yticks(Vec::<f64>::new());
    ax.set_ylim(0.0, 110.0)?;
    ax.set_title("CLAIMS OF SUPERNATURAL POWERS");
    fig.text(0.5, 0.05, "\"The Data So Far\" from xkcd by Randall Munroe")
        .set_halign(HAlign::Center);
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_misc/anatomy", anatomy).sized(7.5, 7.5, 100.0),
    FigureCase::new("gallery_misc/firefox", firefox).sized(5.0, 5.0, 100.0),
    FigureCase::new("gallery_misc/pan_zoom_overlap", pan_zoom_overlap).sized(11.0, 6.0, 100.0),
    FigureCase::new("gallery_misc/stock_prices", stock_prices).sized(6.0, 8.0, 100.0),
    FigureCase::new("gallery_misc/xkcd_2", xkcd_2).with_rc_setup(xkcd),
];
