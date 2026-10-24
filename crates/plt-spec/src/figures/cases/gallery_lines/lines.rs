//! The lines of the section: infinite lines, cap and join styles, event
//! collections and plots, dash control, the line style table, ticked
//! strokes, masked data, multicoloured lines, the simplest plot and
//! vertical and horizontal line families.

use std::f64::consts::PI;

use plt::axes::{Axes, EventplotOptions, PlotError, TickParams};
use plt::collections::{CollectionProps, CollectionSpec, ColorMapping, EdgeColors};
use plt::colorbar::ColorbarOptions;
use plt::colors::{ScalarMappable, colormaps};
use plt::coords::Coords;
use plt::figure::Share;
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::layout::LayoutEngine;
use plt::legend::LegendOptions;
use plt::lines::AxLineThrough;
use plt::orientation::Orientation;
use plt::patheffects::{PathEffect, TickedStroke};
use plt::prelude::*;
use plt::text::{AnnotateOptions, HAlign};
use plt::transforms::Point;
use plt_render::{CapStyle, JoinStyle};

use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// A black line in `style`.
fn black(style: LineStyle) -> LineProps {
    LineProps::default().color("black").linestyle(style)
}

fn axline_1(fig: &Figure) -> CaseResult {
    let t = linspace(-10.0, 10.0, 100);
    let sig = map(&t, |t| 1.0 / (1.0 + (-t).exp()));
    let ax = fig.subplot();
    ax.axhline(0.0, black(LineStyle::Dashed));
    ax.axhline(0.5, black(LineStyle::Dotted));
    ax.axhline(1.0, black(LineStyle::Dashed));
    ax.axvline(0.0, LineProps::default().color("grey"));
    ax.axline(
        Point::new(0.0, 0.5),
        AxLineThrough::Slope(0.25),
        black(dashes(&[5.0, 5.0])),
    )?;
    let sigmoid = LineProps::default()
        .linewidth(2.0)
        .label(r"$\sigma(t) = \frac{1}{1 + e^{-t}}$");
    ax.plot_with(&t, &sig, sigmoid)?;
    ax.set_xlim(-10.0, 10.0);
    ax.set_xlabel("t");
    ax.legend(LegendOptions::default().fontsize(14.0))?;
    Ok(())
}

fn axline_2(fig: &Figure) -> Result<(), PlotError> {
    let ax = fig.subplot();
    for pos in linspace(-2.0, 1.0, 10) {
        let props = LineProps::default().color("k").transform(ax.trans_axes());
        ax.axline(Point::new(pos, 0.0), AxLineThrough::Slope(0.5), props)?;
    }
    ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
    Ok(())
}

/// The thick blue stroke of the cap and join style demos, drawn over by
/// the thin black centre line.
fn thick_and_thin(ax: &Axes, x: &[f64], y: &[f64], thick: LineProps) -> CaseResult {
    ax.plot_with(x, y, thick.linewidth(12.0).color("tab:blue"))?;
    ax.plot_with(x, y, LineProps::default().linewidth(1.0).color("black"))?;
    Ok(())
}

/// Small red dots at the points.
fn red_dots(ax: &Axes, x: &[f64], y: &[f64]) -> CaseResult {
    ax.plot_with(x, y, LineProps::fmt("o")?.color("tab:red").markersize(3.0))?;
    Ok(())
}

fn capstyle(fig: &Figure) -> CaseResult {
    let ax = fig.add_axes([0.0, 0.0, 1.0, 0.8]);
    ax.set_title("Cap style");
    let styles = [
        ("butt", CapStyle::Butt),
        ("round", CapStyle::Round),
        ("projecting", CapStyle::Projecting),
    ];
    for (x, (name, style)) in (0_i32..).map(f64::from).zip(styles) {
        ax.text(x + 0.25, 0.85, name).set_halign(HAlign::Center);
        let (xx, yy) = ([x, x + 0.5], [0.0, 0.0]);
        thick_and_thin(&ax, &xx, &yy, LineProps::default().solid_capstyle(style))?;
        red_dots(&ax, &xx, &yy)?;
    }
    ax.set_ylim(-0.5, 1.5);
    ax.set_axis_off();
    Ok(())
}

fn joinstyle(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    ax.set_title("Join style");
    let styles = [
        ("miter", JoinStyle::Miter),
        ("round", JoinStyle::Round),
        ("bevel", JoinStyle::Bevel),
    ];
    for (x, (name, style)) in (0_i32..).map(f64::from).zip(styles) {
        ax.text(x, 5.0, name);
        for (y, angle) in (0_i32..)
            .map(f64::from)
            .zip([20.0_f64, 45.0, 60.0, 90.0, 120.0])
        {
            let phi = angle.to_radians();
            let xx = [x + 0.5, x, x + 0.5 * phi.cos()];
            let yy = [y, y, y + 0.5 * phi.sin()];
            thick_and_thin(&ax, &xx, &yy, LineProps::default().solid_joinstyle(style))?;
            red_dots(&ax, &xx[1..2], &yy[1..2])?;
            if x == 0.0 {
                ax.text(-1.3, y, format!("{angle} degrees"));
            }
        }
    }
    ax.set_xlim(-1.5, 2.75).set_ylim(-0.5, 5.5);
    ax.set_axis_off();
    Ok(())
}

/// `n` uniform values, sorted.
fn sorted_uniform(rng: &mut SpecRng, n: usize) -> Vec<f64> {
    let mut values = rng.uniform(n, 0.0, 1.0);
    values.sort_by(f64::total_cmp);
    values
}

fn eventcollection_demo(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(19_680_801);
    let xdata1 = sorted_uniform(&mut rng, 10);
    let xdata2 = sorted_uniform(&mut rng, 10);
    let ydata1 = map(&xdata1, |x| x.powi(2));
    let ydata2 = map(&xdata2, |x| 1.0 - x.powi(3));
    let ax = fig.add_subplot(1, 1, 1)?;
    ax.plot_with(&xdata1, &ydata1, LineProps::default().color("tab:blue"))?;
    ax.plot_with(&xdata2, &ydata2, LineProps::default().color("tab:orange"))?;
    let events = [
        (&xdata1, "tab:blue", Orientation::Horizontal),
        (&xdata2, "tab:orange", Orientation::Horizontal),
        (&ydata1, "tab:blue", Orientation::Vertical),
        (&ydata2, "tab:orange", Orientation::Vertical),
    ];
    for (positions, color, orientation) in events {
        let spec = CollectionSpec::events(positions, orientation, 0.0, 0.05)
            .with_props(CollectionProps::default().color(color));
        ax.add_collection(spec);
    }
    ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
    ax.set_title("line plot with data points");
    Ok(())
}

/// `rows × 50` values as one row per dataset.
fn datasets(values: &[f64]) -> Vec<Vec<f64>> {
    values.chunks(50).map(<[f64]>::to_vec).collect()
}

fn eventplot_demo(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(19_680_801);
    let data1 = datasets(&rng.uniform(6 * 50, 0.0, 1.0));
    let axs = fig.subplots(2, 2)?;
    let colored = EventplotOptions {
        colors: (0..6)
            .map(|i| Color::from(format!("C{i}").as_str()))
            .collect(),
        lineoffsets: vec![-15.0, -3.0, 1.0, 1.5, 6.0, 10.0],
        linelengths: vec![5.0, 2.0, 1.0, 1.0, 3.0, 1.5],
        ..EventplotOptions::default()
    };
    let data2 = datasets(&rng.gamma(60 * 50, 4));
    let black = EventplotOptions {
        colors: vec![Color::from("black")],
        ..EventplotOptions::default()
    };
    for (column, (data, options)) in [(data1, colored), (data2, black)].into_iter().enumerate() {
        for (row, orientation) in [Orientation::Horizontal, Orientation::Vertical]
            .into_iter()
            .enumerate()
        {
            let options = EventplotOptions {
                orientation,
                ..options.clone()
            };
            axs[(row, column)].eventplot(&data, options)?;
        }
    }
    Ok(())
}

/// The on-off `pattern` from the start (matplotlib's `dashes=pattern`).
fn dashes(pattern: &[f64]) -> LineStyle {
    LineStyle::Dashes {
        offset: 0.0,
        pattern: pattern.to_vec(),
    }
}

fn line_demo_dash_control(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 10.0, 500);
    let y = map(&x, f64::sin);
    let ax = fig.subplot();
    let line1 = ax.plot_with(
        &x,
        &y,
        LineProps::default().label("Using set_dashes() and set_dash_capstyle()"),
    )?;
    line1.set_linestyle(dashes(&[2.0, 2.0, 10.0, 2.0]));
    line1.set_dash_capstyle(CapStyle::Round);
    let dashed = LineProps::default()
        .linestyle(dashes(&[6.0, 2.0]))
        .label("Using the dashes parameter");
    ax.plot_with(&x, map(&y, |y| y - 0.2), dashed)?;
    let gapped = LineProps::default()
        .linestyle(dashes(&[4.0, 4.0]))
        .gapcolor("tab:pink")
        .label("Using the dashes and gapcolor parameters");
    ax.plot_with(&x, map(&y, |y| y - 0.4), gapped)?;
    ax.legend(LegendOptions::default().handlelength(4.0))?;
    Ok(())
}

/// A line style of the table with the Python spelling the example prints.
struct NamedStyle {
    name: &'static str,
    style: LineStyle,
    repr: String,
}

/// A named line style (`linestyle="dotted"`).
fn named(name: &'static str, style: LineStyle) -> NamedStyle {
    NamedStyle {
        name,
        style,
        repr: format!("'{name}'"),
    }
}

/// A parametrized line style (`linestyle=(offset, (on, off, ...))`).
fn parametrized(name: &'static str, offset: u32, pattern: &[u32]) -> NamedStyle {
    let lengths: Vec<String> = pattern.iter().map(u32::to_string).collect();
    NamedStyle {
        name,
        style: LineStyle::Dashes {
            offset: f64::from(offset),
            pattern: pattern.iter().copied().map(f64::from).collect(),
        },
        repr: format!("({offset}, ({}))", lengths.join(", ")),
    }
}

/// The example's `plot_linestyles`: one black line per style, labelled
/// with its name and annotated with its spelling.
fn plot_linestyles(ax: &Axes, linestyles: &[NamedStyle], title: &str) -> CaseResult {
    let x = linspace(0.0, 100.0, 10);
    for (i, style) in (0_i32..).map(f64::from).zip(linestyles) {
        let props = LineProps::default()
            .linestyle(style.style.clone())
            .linewidth(1.5)
            .color("black");
        ax.plot_with(&x, vec![i; 10], props)?;
    }
    ax.set_title(title);
    let n = linestyles.len() as f64;
    ax.set_ylim(-0.5, n - 0.5);
    ax.set_yticks(arange(0.0, n, 1.0))
        .set_yticklabels(linestyles.iter().map(|style| style.name))?;
    ax.tick_params(&TickParams {
        left: Some(false),
        bottom: Some(false),
        labelbottom: Some(false),
        ..TickParams::default()
    });
    for spine in ax.spines().iter() {
        spine.set_visible(false);
    }
    for (i, style) in (0_i32..).map(f64::from).zip(linestyles) {
        let options = AnnotateOptions {
            xycoords: Coords::Transform(ax.trans_yaxis()),
            xytext: Some(Point::new(-6.0, -12.0)),
            textcoords: Some("offset points".parse()?),
            ..AnnotateOptions::default()
        };
        ax.annotate(&style.repr, Point::new(0.0, i), options)?
            .set_color("blue")
            .set_fontsize(8.0)
            .set_halign(HAlign::Right)
            .set_family(["monospace"]);
    }
    Ok(())
}

fn linestyles(fig: &Figure) -> CaseResult {
    let linestyle_str = [
        named("solid", LineStyle::Solid),
        named("dotted", LineStyle::Dotted),
        named("dashed", LineStyle::Dashed),
        named("dashdot", LineStyle::DashDot),
    ];
    let linestyle_tuple = [
        parametrized("loosely dotted", 0, &[1, 10]),
        parametrized("dotted", 0, &[1, 5]),
        parametrized("densely dotted", 0, &[1, 1]),
        parametrized("long dash with offset", 5, &[10, 3]),
        parametrized("loosely dashed", 0, &[5, 10]),
        parametrized("dashed", 0, &[5, 5]),
        parametrized("densely dashed", 0, &[5, 1]),
        parametrized("loosely dashdotted", 0, &[3, 10, 1, 10]),
        parametrized("dashdotted", 0, &[3, 5, 1, 5]),
        parametrized("densely dashdotted", 0, &[3, 1, 1, 1]),
        parametrized("dashdotdotted", 0, &[3, 5, 1, 5, 1, 5]),
        parametrized("loosely dashdotdotted", 0, &[3, 10, 1, 10, 1, 10]),
        parametrized("densely dashdotdotted", 0, &[3, 1, 1, 1, 1, 1]),
    ];
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let grid = GridSpec::with_ratios(
        2,
        1,
        None,
        Some(vec![1.0, 3.0]),
        SubplotOverrides::default(),
    )?;
    let axs = fig.subplots_on(&grid, Share::None, Share::None)?;
    let (mut named_styles, mut tuple_styles) = (linestyle_str, linestyle_tuple);
    named_styles.reverse();
    tuple_styles.reverse();
    plot_linestyles(&axs[(0, 0)], &named_styles, "Named linestyles")?;
    plot_linestyles(&axs[(1, 0)], &tuple_styles, "Parametrized linestyles")?;
    Ok(())
}

fn lines_with_ticks_demo(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let line_ticks = TickedStroke {
        spacing: 7.0,
        angle: 135.0,
        ..TickedStroke::default()
    };
    let line = LineProps::default()
        .label("Line")
        .path_effects(vec![PathEffect::with_ticked_stroke(line_ticks)]);
    ax.plot_with([0.0, 1.0], [0.0, 1.0], line)?;
    let x = linspace(0.0, 1.0, 101);
    let y = map(&x, |x| 0.3 * (x * 8.0).sin() + 0.4);
    let curve =
        LineProps::default()
            .label("Curve")
            .path_effects(vec![
                PathEffect::with_ticked_stroke(TickedStroke::default()),
            ]);
    ax.plot_with(&x, &y, curve)?;
    ax.legend(LegendOptions::default())?;
    Ok(())
}

fn masked_demo(fig: &Figure) -> CaseResult {
    let x = linspace(-PI / 2.0, PI / 2.0, 31);
    let y = map(&x, |x| x.cos().powi(3));
    let (x2, y2): (Vec<f64>, Vec<f64>) = x.iter().zip(&y).filter(|(_, y)| **y <= 0.7).unzip();
    // Masked values and NaNs both leave gaps.
    let gapped = map(&y, |y| if y > 0.7 { f64::NAN } else { y });
    let ax = fig.subplot();
    let styled = |label: &str| LineProps::fmt("o-").map(|props| props.label(label));
    ax.plot_with(
        map(&x, |x| x * 0.1),
        &y,
        styled("No mask")?.color("lightgrey"),
    )?;
    ax.plot_with(map(&x2, |x| x * 0.4), &y2, styled("Points removed")?)?;
    ax.plot_with(map(&x, |x| x * 0.7), &gapped, styled("Masked values")?)?;
    ax.plot_with(&x, &gapped, styled("NaN values")?)?;
    ax.legend(LegendOptions::default())?;
    ax.set_title("Masked and NaN data");
    Ok(())
}

/// A line collection coloured by `values` through the colormap `cmap`
/// with an autoscaled norm.
fn mapped_lines(
    segments: Vec<Vec<Point>>,
    values: &[f64],
    cmap: &str,
    linewidth: f64,
) -> Result<CollectionSpec, Box<dyn std::error::Error>> {
    Ok(CollectionSpec::lines(segments)
        .with_props(CollectionProps::default().linewidth(linewidth))
        .with_mapping(ColorMapping {
            values: values.iter().copied().map(Some).collect(),
            mappable: ScalarMappable::new(colormaps::get(cmap)?),
        }))
}

/// The example's `colored_line`: a line coloured by `c` at each point,
/// from segments joining each point to the midpoints with its neighbours.
fn colored_line(x: &[f64], y: &[f64]) -> Vec<Vec<Point>> {
    let xy: Vec<Point> = x.iter().zip(y).map(|(&x, &y)| Point::new(x, y)).collect();
    let mut mid = vec![xy[0]];
    mid.extend(
        xy.windows(2)
            .map(|w| Point::new(f64::midpoint(w[0].x, w[1].x), f64::midpoint(w[0].y, w[1].y))),
    );
    mid.push(xy[xy.len() - 1]);
    xy.iter()
        .enumerate()
        .map(|(i, &p)| vec![mid[i], p, mid[i + 1]])
        .collect()
}

fn multicolored_line_1(fig: &Figure) -> CaseResult {
    let t = linspace(-7.4, -0.5, 200);
    let x = map(&t, |t| 0.9 * t.sin());
    let y = map(&t, |t| 0.9 * (1.6 * t).cos());
    let color = linspace(0.0, 2.0, 200);
    let ax1 = fig.subplot();
    let lines = ax1.add_collection(mapped_lines(colored_line(&x, &y), &color, "plasma", 10.0)?);
    fig.colorbar(&lines, ColorbarOptions::default())?;
    ax1.set_title("Color at each point");
    Ok(())
}

fn multicolored_line_2(fig: &Figure) -> CaseResult {
    let x = [0.0, 1.0, 2.0, 3.0, 4.0];
    let y = [0.0, 1.0, 2.0, 1.0, 1.0];
    let c = [1.0, 2.0, 3.0, 4.0, 5.0];
    let ax = fig.subplot();
    let options = ScatterOptions {
        colors: MappedColors::new(c.to_vec())
            .cmap(colormaps::get("rainbow")?)
            .into(),
        ..ScatterOptions::default()
    };
    ax.scatter_with(x, y, options)?;
    let width = plt::rc::params().lines_linewidth;
    ax.add_collection(mapped_lines(colored_line(&x, &y), &c, "rainbow", width)?);
    Ok(())
}

fn multicolored_line_3(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 3.0 * PI, 500);
    let y = map(&x, f64::sin);
    let dydx: Vec<f64> = x
        .windows(2)
        .map(|w| f64::midpoint(w[0], w[1]).cos())
        .collect();
    let segments = x
        .windows(2)
        .zip(y.windows(2))
        .map(|(x, y)| vec![Point::new(x[0], y[0]), Point::new(x[1], y[1])])
        .collect();
    let ax2 = fig.subplot();
    let line = ax2.add_collection(mapped_lines(segments, &dydx, "viridis", 2.0)?);
    let options = ColorbarOptions {
        label: "dy/dx".to_owned(),
        ..ColorbarOptions::beside(&ax2)
    };
    fig.colorbar(&line, options)?;
    ax2.set_xlim(x[0], x[499]).set_ylim(-1.1, 1.1);
    ax2.set_title("Color between points");
    Ok(())
}

fn simple_plot(fig: &Figure) -> CaseResult {
    let t = arange(0.0, 2.0, 0.01);
    let s = map(&t, |t| 1.0 + (2.0 * PI * t).sin());
    let ax = fig.subplot();
    ax.plot(&t, s)?;
    ax.set_xlabel("time (s)");
    ax.set_ylabel("voltage (mV)");
    ax.set_title("About as simple as it gets, folks");
    ax.grid(true);
    Ok(())
}

fn vline_hline_demo(fig: &Figure) -> CaseResult {
    let t = arange(0.0, 5.0, 0.1);
    let s = map(&t, |t| (-t).exp() + (2.0 * PI * t).sin() + 1.0);
    let noise = SpecRng::new(19_680_801).normal(t.len(), 0.0, 0.3);
    let noisy: Vec<f64> = s.iter().zip(&noise).map(|(s, n)| s + n * s).collect();
    let axs = fig.subplots(1, 2)?;
    let (vax, hax) = (&axs[(0, 0)], &axs[(0, 1)]);
    vax.plot_fmt(&t, &noisy, "^")?;
    vax.vlines(&t, [0.0], &s)?;
    vax.vlines_in([1.0, 2.0], [0.0], [1.0], vax.trans_xaxis())?
        .set_edgecolors(EdgeColors::Colors(vec![Color::from("r")]));
    vax.set_xlabel("time (s)");
    vax.set_title("Vertical lines demo");
    hax.plot_fmt(&noisy, &t, "^")?;
    hax.hlines(&t, [0.0], &s)?.set_linewidths(vec![2.0]);
    hax.set_xlabel("time (s)");
    hax.set_title("Horizontal lines demo");
    Ok(())
}

/// Every case of the module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_lines/axline_1", axline_1),
    FigureCase::new("gallery_lines/axline_2", |fig| Ok(axline_2(fig)?)),
    FigureCase::new("gallery_lines/capstyle", capstyle).sized(4.0, 1.2, 100.0),
    FigureCase::new("gallery_lines/joinstyle", joinstyle).sized(5.0, 4.0, 100.0),
    FigureCase::new("gallery_lines/eventcollection_demo", eventcollection_demo),
    FigureCase::new("gallery_lines/eventplot_demo", eventplot_demo)
        .with_rc(&[("font.size", "8.0")]),
    FigureCase::new(
        "gallery_lines/line_demo_dash_control",
        line_demo_dash_control,
    )
    .with_rc(&[("lines.linewidth", "2.5")]),
    FigureCase::new("gallery_lines/linestyles", linestyles).sized(7.0, 8.0, 100.0),
    FigureCase::new("gallery_lines/lines_with_ticks_demo", lines_with_ticks_demo)
        .sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_lines/masked_demo", masked_demo),
    FigureCase::new("gallery_lines/multicolored_line_1", multicolored_line_1),
    FigureCase::new("gallery_lines/multicolored_line_2", multicolored_line_2),
    FigureCase::new("gallery_lines/multicolored_line_3", multicolored_line_3),
    FigureCase::new("gallery_lines/simple_plot", simple_plot),
    FigureCase::new("gallery_lines/vline_hline_demo", vline_hline_demo).sized(12.0, 6.0, 100.0),
];
