//! The cases from `test_lines.py`.

use plt::axes::Axes;
use plt::colors::{Color, colormaps};
use plt::contour::{ContourOptions, QuadField};
use plt::figure::Figure;
use plt::legend::LegendOptions;
use plt::lines::{DrawStyle, LineProps, LineStyle, MarkEvery};
use plt::markers::{FillStyle, Marker};
use plt::numeric::{Grid, linspace};
use plt::rc::{ColorOrAuto, RcParams};
use plt::transforms::{Affine2D, Transform};

use super::super::mpl_tests::{classic, remove_ticks_and_titles};
use super::plot_y;
use crate::figures::cases::common::indices;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `test_line_dashes`: a thick line with a custom dash pattern.
fn line_dashes(fig: &Figure) -> CaseResult {
    let dashes = LineStyle::Dashes {
        offset: 0.0,
        pattern: Some(vec![3.0, 3.0]),
    };
    let props = LineProps::default().linestyle(dashes).linewidth(5.0);
    plot_y(&fig.subplot(), indices(10), props)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_drawstyle_variants`: every draw style of a long line, zoomed in.
fn drawstyle_variants(fig: &Figure) -> CaseResult {
    let styles = [
        DrawStyle::Default,
        DrawStyle::StepsMid,
        DrawStyle::StepsPre,
        DrawStyle::StepsPost,
        DrawStyle::Steps,
        DrawStyle::Default,
    ];
    let axs = fig.subplots(6, 1)?;
    for (row, style) in styles.into_iter().enumerate() {
        let ax = &axs[(row, 0)];
        plot_y(ax, indices(2000), LineProps::default().drawstyle(style))?;
        ax.set_xlim(0.0, 2.0)?.set_ylim(0.0, 2.0)?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_no_subslice_with_transform`: a long line shifted by a transform.
fn no_subslice_with_transform(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = indices(2000);
    let shifted =
        Transform::Affine(Affine2D::identity().translate(2000.0, 0.0)).then(ax.trans_data());
    ax.plot_with(&x, &x, LineProps::default().transform(shifted))?;
    Ok(())
}

/// `test_set_line_coll_dash_image`: dashed contours of noise.
fn line_collection_dashes(fig: &Figure) -> CaseResult {
    let noise = SpecRng::new(0).normal(600, 0.0, 1.0);
    let z = Grid::from_vec(20, 30, noise)?;
    let options = ContourOptions {
        linestyles: Some(vec![LineStyle::Dashes {
            offset: 0.0,
            pattern: Some(vec![3.0, 3.0]),
        }]),
        ..ContourOptions::default()
    };
    fig.subplot().contour(&QuadField::new(z), options)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_marker_fill_styles`: every fill style of fifteen markers.
fn marker_fill_styles(fig: &Figure) -> CaseResult {
    let colors = ["#0000ff", "g", "#ff0000", "c", "m", "y", "#000000"];
    let mut colors = colors.iter().cycle();
    let fillstyles = [
        FillStyle::Full,
        FillStyle::Left,
        FillStyle::Right,
        FillStyle::Bottom,
        FillStyle::Top,
        FillStyle::None,
    ];
    let ax = fig.subplot();
    for (j, marker) in "ov^<>8sp*hHDdPX".chars().enumerate() {
        for (i, fillstyle) in fillstyles.into_iter().enumerate() {
            let color = Color::from(*colors.next().expect("the colours cycle"));
            let x = (j * 10) as f64;
            let y = 1.0 + i as f64 + 0.5 * (j % 2) as f64;
            let props = LineProps::default()
                .marker(marker.to_string().parse::<Marker>()?)
                .markersize(20.0)
                .markerfacecoloralt(Color::from("lightgreen"))
                .fillstyle(fillstyle)
                .linewidth(5.0)
                .color(color.clone())
                .markeredgecolor(ColorOrAuto::Color(color))
                .markeredgewidth(2.0);
            ax.plot_with([x, x + 9.0], [y, y], props)?;
        }
    }
    ax.set_ylim(0.0, 7.5)?.set_xlim(-5.0, 155.0)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_lw_scaling`: dashed, dotted and dash-dotted lines of ten widths.
fn scaled_lines(fig: &Figure) -> CaseResult {
    let th = linspace(0.0, 32.0, 50);
    let ax = fig.subplot();
    let styles = [LineStyle::Dashed, LineStyle::Dotted, LineStyle::DashDot];
    for (j, style) in styles.into_iter().enumerate() {
        for lw in linspace(0.5, 10.0, 10) {
            let y = vec![j as f64 + 0.1 * lw; 50];
            let props = LineProps::default()
                .linestyle(style.clone())
                .linewidth(lw)
                .color(Color::from(format!("C{j}").as_str()));
            ax.plot_with(&th, y, props)?;
        }
    }
    Ok(())
}

/// `test_markevery`'s data: 14 points with random heights.
fn markevery_data() -> (Vec<f64>, Vec<f64>) {
    (
        linspace(0.0, 1.0, 14),
        SpecRng::new(42).uniform(14, 0.0, 1.0),
    )
}

/// `test_markevery`'s `markevery` values, in order.
fn markevery_cases(y: &[f64]) -> Vec<Option<MarkEvery>> {
    let above_half = (0..y.len() as i64)
        .filter(|&i| y[i as usize] > 0.5)
        .collect();
    vec![
        None,
        Some(MarkEvery::Every { start: 0, step: 4 }),
        Some(MarkEvery::Every { start: 2, step: 5 }),
        Some(MarkEvery::Indices(vec![1, 5, 11])),
        Some(MarkEvery::Indices(vec![0, -1])),
        Some(MarkEvery::Slice {
            start: Some(5),
            stop: Some(10),
            step: Some(2),
        }),
        Some(MarkEvery::Indices(above_half)),
        Some(MarkEvery::Distance {
            start: 0.0,
            step: 0.3,
        }),
        Some(MarkEvery::Distance {
            start: 0.3,
            step: 0.4,
        }),
    ]
}

/// `test_markevery[figure]`: lines on the figure itself, with the
/// `markevery` values that do not need an axes.
fn markevery_figure(fig: &Figure) -> CaseResult {
    let (x, y) = markevery_data();
    let cases = markevery_cases(&y);
    for markevery in &cases[..cases.len() - 2] {
        let props = LineProps::default()
            .marker("o".parse::<Marker>()?)
            .markevery(markevery.clone());
        fig.add_line(&x, &y, props)?;
    }
    Ok(())
}

/// `test_markevery[axes]`: one axes per `markevery` value.
fn markevery_axes(fig: &Figure) -> CaseResult {
    let (x, y) = markevery_data();
    let axs = fig.subplots(3, 3)?;
    for (i, markevery) in markevery_cases(&y).into_iter().enumerate() {
        let props = LineProps::fmt("-gD")?.markevery(markevery);
        axs[(i / 3, i % 3)].plot_with(&x, &y, props)?;
    }
    Ok(())
}

/// `test_striped_lines`: dashed lines with gap colours, in a legend.
fn striped_line(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(19_680_801);
    let ax = fig.subplot();
    let blue_in_orange = LineProps::default()
        .color(Color::from("orange"))
        .gapcolor(Color::from("blue"))
        .linestyle(LineStyle::Dashed)
        .linewidth(5.0)
        .label("blue in orange");
    plot_y(&ax, rng.uniform(12, 0.0, 1.0), blue_in_orange)?;
    let black_in_red = LineProps::default()
        .color(Color::from("red"))
        .gapcolor(Color::from("black"))
        .linestyle(LineStyle::Dashes {
            offset: 0.0,
            pattern: Some(vec![2.0, 5.0, 4.0, 2.0]),
        })
        .linewidth(5.0)
        .label("black in red")
        .alpha(0.5);
    plot_y(&ax, rng.uniform(12, 0.0, 1.0), black_in_red)?;
    ax.legend(LegendOptions {
        handlelength: Some(5.0),
        ..LegendOptions::default()
    })?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_markevery_prop_cycle`'s `markevery` values, as cycler text.
const CYCLED_MARKEVERY: &str = "[None, 8, (30, 8), [16, 24, 30], [0, -1], slice(100, 200, 3), \
                                0.1, 0.3, 1.5, (0.0, 0.1), (0.45, 0.1)]";

/// The property cycle of `test_markevery_prop_cycle`: the `markevery`
/// values with colours from `jet`.
fn markevery_cycle(rc: &mut RcParams) -> CaseResult {
    let jet = colormaps::get("jet")?;
    let colors: Vec<String> = linspace(0.2, 0.8, 11)
        .into_iter()
        .map(|v| {
            let c = jet.sample(v);
            format!("({:?}, {:?}, {:?}, {:?})", c.r, c.g, c.b, c.a)
        })
        .collect();
    let cycle = format!(
        "cycler(markevery={CYCLED_MARKEVERY}, color=[{}])",
        colors.join(", ")
    );
    rc.set("axes.prop_cycle", &cycle)?;
    Ok(())
}

/// `test_markevery_prop_cycle`: `markevery` taken from the cycle.
fn markevery_prop_cycle(fig: &Figure) -> CaseResult {
    let x = linspace(-1.0, 1.0, 50);
    let ax: Axes = fig.subplot();
    for i in 0..11 {
        let y: Vec<f64> = x.iter().map(|x| 5.0 * x * x - f64::from(i)).collect();
        plot_y(&ax, y, LineProps::fmt("o-")?)?;
    }
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    classic("mpl_lines/line_dashes", line_dashes),
    classic("mpl_lines/drawstyle_variants", drawstyle_variants),
    classic(
        "mpl_lines/no_subslice_with_transform",
        no_subslice_with_transform,
    ),
    FigureCase::new("mpl_lines/line_collection_dashes", line_collection_dashes),
    classic("mpl_lines/marker_fill_styles", marker_fill_styles),
    FigureCase::new("mpl_lines/scaled_lines", scaled_lines),
    classic("mpl_lines/step_markers", |fig| {
        let props = LineProps::fmt("-o")?.drawstyle(DrawStyle::StepsPre);
        plot_y(&fig.subplot(), [0.0, 1.0], props)?;
        Ok(())
    }),
    classic("mpl_lines/markevery_figure", markevery_figure),
    classic("mpl_lines/markevery_axes", markevery_axes),
    FigureCase::new("mpl_lines/striped_line", striped_line),
    classic("mpl_lines/odd_dashes", |fig| {
        let odd = LineStyle::Dashes {
            offset: 0.0,
            pattern: Some(vec![1.0, 2.0, 3.0]),
        };
        plot_y(
            &fig.subplot(),
            [1.0, 2.0],
            LineProps::default().linestyle(odd),
        )?;
        Ok(())
    }),
    classic("mpl_lines/markevery_prop_cycle", markevery_prop_cycle).with_rc_setup(markevery_cycle),
];
