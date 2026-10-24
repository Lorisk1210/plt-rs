//! Pie charts: the tests' exploded frog/hog/dog/log pie in its variants,
//! shadows and hatches.

use plt::axes::{Axes, AxisMode, PieOptions, PieShadow};
use plt::colors::Color;
use plt::figure::Figure;
use plt::legend::LegendOptions;
use plt::patches::{ArrowStyle, BoxStyle, ConnectionStyle, PatchProps};
use plt::text::{AnnotateOptions, ArrowProps, FancyArrowProps, TextBoxProps};
use plt::transforms::Point;

use super::super::mpl_tests::classic;
use crate::figures::{CaseResult, FigureCase};
use plt::ticker::LabelFormat;

const SIZES: [f64; 4] = [15.0, 30.0, 45.0, 10.0];
/// Only the second slice is exploded.
const EXPLODE: [f64; 4] = [0.0, 0.1, 0.0, 0.0];

/// The tests' slice colours.
fn colors() -> Vec<Color> {
    ["yellowgreen", "gold", "lightskyblue", "lightcoral"]
        .map(Color::from)
        .to_vec()
}

/// The tests' slice labels.
fn labels(names: [&str; 4]) -> Vec<String> {
    names.map(String::from).to_vec()
}

/// The exploded, coloured pie starting at 90° without labels.
fn plain() -> PieOptions {
    PieOptions {
        explode: EXPLODE.to_vec(),
        colors: colors(),
        startangle: 90.0,
        ..PieOptions::default()
    }
}

/// The tests' pie: labelled, with percentages and a shadow.
fn frogs() -> PieOptions {
    PieOptions {
        labels: labels(["Frogs", "Hogs", "Dogs", "Logs"]),
        autopct: Some(LabelFormat::Template("%1.1f%%".into())),
        shadow: Some(PieShadow::default()),
        ..plain()
    }
}

/// Wedges without edges (`wedgeprops={"linewidth": 0}`).
fn edgeless(options: PieOptions) -> PieOptions {
    PieOptions {
        wedgeprops: PatchProps::default().linewidth(0.0),
        ..options
    }
}

/// The pie on `gca` with equal aspect (`plt.pie`, `plt.axis("equal")`).
fn equal_pie(fig: &Figure, options: &PieOptions) -> Result<Axes, Box<dyn std::error::Error>> {
    let ax = fig.gca();
    ax.pie(&SIZES, options.clone())?;
    ax.axis_mode(AxisMode::Equal)?;
    Ok(ax)
}

/// `test_pie_center_radius`: a pie off the origin with an annotated
/// centre.
fn pie_center_radius(fig: &Figure) -> CaseResult {
    let ax = fig.gca();
    let options = PieOptions {
        center: (1.0, 2.0),
        radius: 1.5,
        ..edgeless(frogs())
    };
    ax.pie(&SIZES, options)?;
    let arrow = FancyArrowProps {
        connectionstyle: "arc3".parse::<ConnectionStyle>()?,
        ..FancyArrowProps::new("->".parse::<ArrowStyle>()?)
    };
    let annotation = ax.annotate(
        "Center point",
        Point::new(1.0, 2.0),
        AnnotateOptions {
            xytext: Some(Point::new(1.0, 1.3)),
            arrowprops: Some(ArrowProps::Fancy(arrow)),
            ..AnnotateOptions::default()
        },
    )?;
    annotation.set_bbox(Some(
        TextBoxProps::new("square".parse::<BoxStyle>()?)
            .with_patch(PatchProps::default().facecolor("lightgrey")),
    ));
    ax.axis_mode(AxisMode::Equal)?;
    Ok(())
}

/// `test_pie_frame_grid`: three framed pies side by side.
fn pie_frame_grid(fig: &Figure) -> CaseResult {
    let ax = fig.gca();
    let framed = |center| PieOptions {
        frame: true,
        center,
        ..edgeless(frogs())
    };
    ax.pie(&SIZES, framed((2.0, 2.0)))?;
    let mut reversed = SIZES;
    reversed.reverse();
    ax.pie(&reversed, framed((5.0, 2.0)))?;
    let mut explode = EXPLODE;
    explode.reverse();
    let options = PieOptions {
        explode: explode.to_vec(),
        ..framed((3.0, 5.0))
    };
    ax.pie(&SIZES, options)?;
    ax.axis_mode(AxisMode::Equal)?;
    Ok(())
}

/// `test_pie_shadow`: no labels; default, no, and customised shadows.
fn pie_shadow(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(2, 2)?;
    let with_shadow = |shadow| PieOptions {
        shadow,
        ..edgeless(plain())
    };
    axes[(0, 0)].pie(&SIZES, with_shadow(Some(PieShadow::default())))?;
    axes[(0, 1)].pie(&SIZES, with_shadow(None))?;
    let dark = PieShadow {
        ox: -0.05,
        oy: -0.05,
        shade: 0.9,
        props: PatchProps::default().edgecolor("none"),
    };
    axes[(1, 0)].pie(&SIZES, with_shadow(Some(dark)))?;
    let light = PieShadow {
        ox: 0.05,
        shade: 0.2,
        props: PatchProps::default().linewidth(2.0),
        ..PieShadow::default()
    };
    axes[(1, 1)].pie(&SIZES, with_shadow(Some(light)))?;
    Ok(())
}

/// A pie of three values hatched with `hatches`.
fn hatched(fig: &Figure, hatches: &[&str]) -> CaseResult {
    let options = PieOptions {
        hatches: hatches.iter().map(|h| (*h).to_owned()).collect(),
        ..PieOptions::default()
    };
    fig.subplot().pie(&[0.3, 0.3, 0.1], options)?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_axes_d/pie_default", |fig| {
        fig.subplot().pie(&SIZES, frogs())?;
        Ok(())
    })
    .sized(8.0, 6.0, 100.0),
    FigureCase::new("mpl_axes_d/pie_linewidth_0", |fig| {
        equal_pie(fig, &edgeless(frogs()))?;
        Ok(())
    }),
    FigureCase::new("mpl_axes_d/pie_center_radius", pie_center_radius),
    FigureCase::new("mpl_axes_d/pie_linewidth_2", |fig| {
        let options = PieOptions {
            wedgeprops: PatchProps::default().linewidth(2.0),
            ..frogs()
        };
        equal_pie(fig, &options)?;
        Ok(())
    }),
    FigureCase::new("mpl_axes_d/pie_ccw_true", |fig| {
        equal_pie(fig, &frogs())?;
        Ok(())
    }),
    FigureCase::new("mpl_axes_d/pie_frame_grid", pie_frame_grid),
    FigureCase::new("mpl_axes_d/pie_rotatelabels_true", |fig| {
        let options = PieOptions {
            labels: labels(["Hogwarts", "Frogs", "Dogs", "Logs"]),
            rotatelabels: true,
            ..frogs()
        };
        equal_pie(fig, &options)?;
        Ok(())
    }),
    FigureCase::new("mpl_axes_d/pie_nolabel_but_legend", |fig| {
        let options = PieOptions {
            labeldistance: None,
            rotatelabels: true,
            ..frogs()
        };
        let ax = equal_pie(fig, &options)?;
        ax.set_ylim(-1.2, 1.2)?;
        ax.legend(LegendOptions::default())?;
        Ok(())
    }),
    FigureCase::new("mpl_axes_d/pie_shadow", pie_shadow),
    classic("mpl_axes_d/pie_hatch_single", |fig| hatched(fig, &["+"])),
    classic("mpl_axes_d/pie_hatch_multi", |fig| {
        hatched(fig, &["/", "+", "."])
    }),
];
