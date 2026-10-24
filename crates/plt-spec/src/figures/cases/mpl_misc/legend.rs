//! The cases from `test_legend.py`.

use plt::axes::{
    Axes, BarAlign, BarOptions, ErrorbarOptions, FillBetweenOptions, ScatterColors, ScatterOptions,
    StackplotOptions,
};
use plt::collections::CollectionProps;
use plt::colors::Color;
use plt::legend::{
    HandlerTuple, LegendHandle, LegendMode, LegendOptions, LegendShadow, LegendShadowStyle,
};
use plt::lines::FormatError;
use plt::numeric::{arange, linspace};
use plt::patches::Rectangle;
use plt::prelude::*;
use plt::transforms::Transform;
use plt_core::Affine2D;

use super::{classic, remove_text};
use crate::figures::{CaseResult, FigureCase};

/// `ax.plot(y, fmt, label=label)`: `y` against its indices.
fn plot_y(ax: &Axes, y: &[f64], props: LineProps) -> CaseResult {
    ax.plot_with(arange(0.0, y.len() as f64, 1.0), y, props)?;
    Ok(())
}

/// A line styled by `fmt` and labelled `label`.
fn styled(fmt: &str, label: &str) -> Result<LineProps, FormatError> {
    Ok(LineProps::fmt(fmt)?.label(label))
}

/// `test_legend_auto1` and `test_legend_expand`: dots at
/// `x = 0, 1, ..., 99` along `y = slope · (x − 50)`, labelled `label`.
fn sloped_dots(ax: &Axes, slope: f64, label: &str) -> CaseResult {
    let x = arange(0.0, 100.0, 1.0);
    let y: Vec<f64> = x.iter().map(|x| slope * (x - 50.0)).collect();
    ax.plot_with(&x, y, styled("o", label)?)?;
    Ok(())
}

/// `test_legend_auto1`.
fn legend_auto1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    sloped_dots(&ax, -1.0, "y=1")?;
    sloped_dots(&ax, 1.0, "y=-1")?;
    ax.legend(LegendOptions::default().loc("best"))?;
    remove_text(fig);
    Ok(())
}

/// `test_legend_auto2`: the legend of two bar series, by their first bar.
fn legend_auto2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = arange(0.0, 100.0, 1.0);
    let bars = |heights: Vec<f64>, color: &str| {
        let options = BarOptions {
            align: BarAlign::Edge,
            color: vec![Color::from(color)],
            ..BarOptions::default()
        };
        ax.bar(&x, heights, options)
    };
    let up = bars(x.clone(), "m")?;
    let down = bars(x.iter().rev().copied().collect(), "g")?;
    ax.legend(
        LegendOptions::default()
            .handles([&up.patches[0], &down.patches[0]])
            .labels(["up", "down"])
            .loc("best"),
    )?;
    remove_text(fig);
    Ok(())
}

/// `test_legend_auto3`.
fn legend_auto3(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = [0.9, 0.1, 0.1, 0.9, 0.9, 0.5];
    let y = [0.95, 0.95, 0.05, 0.05, 0.5, 0.5];
    ax.plot_with(x, y, styled("o-", "line")?)?;
    ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 1.0)?;
    ax.legend(LegendOptions::default().loc("best"))?;
    Ok(())
}

/// `test_various_labels`: a number, accents and a hidden label.
fn various_labels(fig: &Figure) -> CaseResult {
    let ax = fig.add_subplot(1, 2, 1)?;
    plot_y(&ax, &arange(0.0, 4.0, 1.0), styled("o", "1")?)?;
    plot_y(&ax, &linspace(4.0, 4.1, 50), styled("o", "Développés")?)?;
    plot_y(&ax, &arange(4.0, 1.0, -1.0), styled("o", "__nolegend__")?)?;
    ax.legend(LegendOptions::default().numpoints(1_usize).loc("best"))?;
    remove_text(fig);
    Ok(())
}

/// `test_labels_first`: labels left of the handles.
fn labels_first(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    plot_y(&ax, &arange(0.0, 10.0, 1.0), styled("-o", "1")?)?;
    plot_y(&ax, &[5.0; 10], styled(":x", "x")?)?;
    plot_y(&ax, &arange(20.0, 10.0, -1.0), styled("d", "diamond")?)?;
    ax.legend(LegendOptions::default().loc("best").markerfirst(false))?;
    remove_text(fig);
    Ok(())
}

/// `test_multiple_keys`: entries of several handles side by side.
fn multiple_keys(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let p1 = ax.plot_with([0.0, 1.0, 2.0], [1.0, 2.0, 3.0], LineProps::fmt("-o")?)?;
    let p2 = ax.plot_with([0.0, 1.0, 2.0], [2.0, 3.0, 4.0], LineProps::fmt("-x")?)?;
    let p3 = ax.plot_with([0.0, 1.0, 2.0], [3.0, 4.0, 5.0], LineProps::fmt("-d")?)?;
    let (p1, p2) = (LegendHandle::from(&p1), LegendHandle::from(&p2));
    let divided = |pad| HandlerTuple { ndivide: None, pad };
    let handles = [
        LegendHandle::tuple([p1.clone(), p2.clone()], divided(None)),
        LegendHandle::tuple([p2, p1], divided(Some(0.0))),
        LegendHandle::from(&p3),
    ];
    ax.legend(
        LegendOptions::default()
            .handles(handles)
            .labels(["two keys", "pad=0", "one key"])
            .numpoints(1_usize),
    )?;
    remove_text(fig);
    Ok(())
}

/// `test_alpha_rgba` and `test_alpha_rcparam`: a half-transparent red
/// frame over a thick line.
fn red_frame(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    plot_y(
        &ax,
        &arange(0.0, 10.0, 1.0),
        LineProps::default().linewidth(5.0),
    )?;
    let legend = ax.legend(
        LegendOptions::default()
            .labels(["Longlabel that will go away"])
            .loc("center"),
    )?;
    legend.frame().set_facecolor([1.0, 0.0, 0.0, 0.5]);
    remove_text(fig);
    Ok(())
}

/// `test_fancy`: a two-column legend with a title and a shadow outside
/// the axes.
fn fancy(fig: &Figure) -> CaseResult {
    let ax = fig.add_subplot(1, 2, 1)?;
    plot_y(&ax, &[5.0; 10], styled("o--", "XX")?)?;
    let x = arange(0.0, 10.0, 1.0);
    ax.scatter(&x, arange(10.0, 0.0, -1.0))?.set_label("XX\nXX");
    let errors = ErrorbarOptions {
        xerr: Some(0.5.into()),
        yerr: Some(0.5.into()),
        line: LineProps::default().label("XX"),
        ..ErrorbarOptions::default()
    };
    ax.errorbar(&x, &x, errors)?;
    ax.legend(
        LegendOptions::default()
            .loc("center left")
            .bbox_to_anchor((1.0, 0.5))
            .ncols(2)
            .shadow(true)
            .title("My legend")
            .numpoints(1_usize),
    )?;
    remove_text(fig);
    Ok(())
}

/// `test_framealpha`.
fn framealpha(fig: &Figure) -> CaseResult {
    let ax = fig.add_subplot(1, 1, 1)?;
    let x = linspace(1.0, 100.0, 100);
    ax.plot_with(
        &x,
        &x,
        LineProps::default().label("mylabel").linewidth(10.0),
    )?;
    ax.legend(LegendOptions::default().framealpha(0.5).loc("upper right"))?;
    remove_text(fig);
    Ok(())
}

/// `test_rc`: a scatter labelled `label` with its legend outside on the
/// right.
fn scatter_legend(fig: &Figure, label: &str) -> CaseResult {
    let ax = fig.add_subplot(1, 2, 1)?;
    ax.scatter(arange(0.0, 10.0, 1.0), arange(10.0, 0.0, -1.0))?
        .set_label(label);
    ax.legend(
        LegendOptions::default()
            .loc("center left")
            .bbox_to_anchor((1.0, 0.5))
            .title("My legend"),
    )?;
    remove_text(fig);
    Ok(())
}

/// `test_legend_expand`: three legends per axes, without and with
/// `mode="expand"`.
fn legend_expand(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(2, 1)?;
    for (ax, mode) in axes
        .as_slice()
        .iter()
        .zip([LegendMode::Fit, LegendMode::Expand])
    {
        let legend = |loc: &str| LegendOptions::default().loc(loc).mode(mode);
        sloped_dots(ax, -1.0, "y=1")?;
        ax.keep_legend(&ax.legend(legend("upper left"))?);
        sloped_dots(ax, 1.0, "y=-1")?;
        ax.keep_legend(&ax.legend(legend("right"))?);
        ax.legend(legend("lower left").ncols(2))?;
    }
    remove_text(fig);
    Ok(())
}

/// `test_hatching`: hatched patches and areas, filled and unfilled, in
/// default and explicit colours.
fn hatching(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    for (y, fill, filled) in [(0.0, true, "filled"), (0.4, false, "unfilled")] {
        for (x, hatch, edge, color) in [
            (0.0, "xx", None, "default"),
            (0.33, "||", Some("C1"), "explicit"),
        ] {
            let mut props = PatchProps::default()
                .hatch(hatch)
                .fill(fill)
                .label(format!("Patch\n{color} color\n{filled}"));
            props.edgecolor = edge.map(Color::from);
            ax.add_patch_with(Rectangle::new((x, y), 0.3, 0.3), props);
        }
    }
    for (x, edge, color) in [
        ([0.0, 0.15, 0.3], None, "default"),
        ([0.33, 0.48, 0.63], Some("C2"), "explicit"),
    ] {
        let mut props = CollectionProps::default()
            .hatch("+")
            .label(format!("Path\n{color} color"));
        if let Some(edge) = edge {
            props = props.edgecolor(edge);
        }
        let band = FillBetweenOptions {
            props,
            ..FillBetweenOptions::default()
        };
        ax.fill_between(x, [0.8, 0.8, 0.8], [0.9, 1.0, 0.9], band)?;
    }
    ax.set_xlim(-0.01, 1.1)?.set_ylim(-0.01, 1.1)?;
    ax.legend(LegendOptions::default().handlelength(4.0).handleheight(4.0))?;
    remove_text(fig);
    Ok(())
}

/// `test_reverse_legend_display`.
fn reverse_legend_display(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    plot_y(&ax, &[1.0], styled("ro", "first")?)?;
    plot_y(&ax, &[2.0], styled("bx", "second")?)?;
    ax.legend(LegendOptions::default().reverse(true))?;
    Ok(())
}

/// `test_legend_stackplot`.
fn legend_stackplot(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = linspace(0.0, 10.0, 10);
    let layers: Vec<Vec<f64>> = [(1.0, 0.0), (2.0, 1.0), (3.0, 2.0)]
        .iter()
        .map(|(slope, offset)| x.iter().map(|x| slope * x + offset).collect())
        .collect();
    let stacked = ax.stackplot(&x, &layers, StackplotOptions::default())?;
    for (layer, label) in stacked.iter().zip(["y1", "y2", "y3"]) {
        layer.set_label(label);
    }
    ax.set_xlim(0.0, 10.0)?.set_ylim(0.0, 70.0)?;
    ax.legend(LegendOptions::default().loc("best"))?;
    Ok(())
}

/// `test_not_covering_scatter`.
fn not_covering_scatter(fig: &Figure) -> CaseResult {
    let ax = fig.add_subplot(1, 1, 1)?;
    for (n, color) in ["b", "g", "r"].into_iter().enumerate() {
        let options = ScatterOptions {
            colors: ScatterColors::Single(Color::from(color)),
            ..ScatterOptions::default()
        };
        ax.scatter_with([n as f64], [n as f64], options)?;
    }
    ax.legend(
        LegendOptions::default()
            .labels(["foo", "foo", "foo"])
            .loc("best"),
    )?;
    ax.set_xlim(-0.5, 2.2)?.set_ylim(-0.5, 2.2)?;
    Ok(())
}

/// `test_not_covering_scatter_transform`: the legend avoids a point drawn
/// 20 pixels left of and above its data position.
fn not_covering_scatter_transform(fig: &Figure) -> CaseResult {
    let ax = fig.add_subplot(1, 1, 1)?;
    let x = linspace(0.0, 30.0, 1000);
    ax.plot(&x, &x)?;
    let offset = Transform::Affine(Affine2D::identity().translate(-20.0, 20.0));
    let shifted = ScatterOptions {
        offset_transform: Some(offset.then(ax.trans_data())),
        ..ScatterOptions::default()
    };
    ax.scatter_with([20.0], [10.0], shifted)?;
    ax.legend(LegendOptions::default().labels(["foo", "bar"]).loc("best"))?;
    Ok(())
}

/// `test_shadow_argument_types`: legends with shadows given as booleans
/// and as dicts of colours, opacity and offsets.
fn shadow_argument_types(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    plot_y(&ax, &[1.0, 2.0, 3.0], LineProps::default().label("test"))?;
    let shaded = |props: PatchProps, ox: f64, oy: f64| LegendShadowStyle {
        ox,
        oy,
        props,
        ..LegendShadowStyle::default()
    };
    let shadows: [(&str, LegendShadow); 5] = [
        ("upper left", true.into()),
        ("upper right", false.into()),
        (
            "center left",
            shaded(PatchProps::default().color("red").alpha(0.1), 2.0, -2.0).into(),
        ),
        (
            "center right",
            shaded(PatchProps::default().color((0.1, 0.2, 0.5)), 2.0, -5.0).into(),
        ),
        (
            "lower left",
            shaded(PatchProps::default().color("tab:cyan"), 10.0, -2.0).into(),
        ),
    ];
    for (loc, shadow) in shadows {
        let legend = ax.legend(LegendOptions::default().loc(loc).shadow(shadow))?;
        ax.keep_legend(&legend);
    }
    ax.legend(LegendOptions::default().loc("lower right"))?;
    remove_text(fig);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_misc/legend_auto1", legend_auto1),
    FigureCase::new("mpl_misc/legend_auto2", legend_auto2),
    FigureCase::new("mpl_misc/legend_auto3", legend_auto3),
    FigureCase::new("mpl_misc/various_labels", various_labels),
    FigureCase::new("mpl_misc/labels_first", labels_first),
    FigureCase::new("mpl_misc/multiple_keys", multiple_keys),
    FigureCase::new("mpl_misc/alpha_rgba", red_frame).with_rc(&[("legend.framealpha", "None")]),
    FigureCase::new("mpl_misc/alpha_rcparam", red_frame).with_rc(&[("legend.framealpha", "0.75")]),
    FigureCase::new("mpl_misc/fancy", fancy),
    FigureCase::new("mpl_misc/framealpha", framealpha),
    FigureCase::new("mpl_misc/rc_1", |fig| scatter_legend(fig, "three"))
        .with_rc(&[("legend.scatterpoints", "3")]),
    FigureCase::new("mpl_misc/rc_2", |fig| scatter_legend(fig, "one"))
        .with_rc(&[("legend.scatterpoints", "1")]),
    FigureCase::new("mpl_misc/legend_expand", legend_expand),
    FigureCase::new("mpl_misc/hatching", hatching),
    classic("mpl_misc/reverse_legend_display", reverse_legend_display),
    FigureCase::new("mpl_misc/legend_stackplot", legend_stackplot),
    FigureCase::new("mpl_misc/not_covering_scatter", not_covering_scatter),
    FigureCase::new(
        "mpl_misc/not_covering_scatter_transform",
        not_covering_scatter_transform,
    ),
    FigureCase::new("mpl_misc/shadow_argument_types", shadow_argument_types),
    classic("mpl_misc/ncol_ncols", |fig| {
        fig.legend(
            LegendOptions::default()
                .labels(["a", "b", "c", "d", "e", "f"])
                .ncols(3),
        )?;
        Ok(())
    }),
];
