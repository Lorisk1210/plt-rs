//! `test_figure.py`, figure-level parts: aligned labels and titles,
//! figure legends and titles, the figure background, figure artists,
//! transparent saving, tight boxes and the `figure.*` font rcParams.

use plt::axes::{AxisName, TickParams, TitleLocation};
use plt::figure::Share;
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::layout::{LayoutEngine, TightLayout};
use plt::legend::LegendOptions;
use plt::mplot3d::Axes3DOptions;
use plt::patches::{Circle, CirclePolygon};
use plt::prelude::*;
use plt::spines::SpineSide;
use plt::transforms::Transform;
use plt_core::Rgba;

use super::{Fallible, TEAL, TIGHT_BBOX, classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_figure/align_labels", align_labels),
    FigureCase::new("mpl_figure/align_titles_1", |fig| align_titles(fig, false)),
    FigureCase::new("mpl_figure/align_titles_2", |fig| align_titles(fig, true)),
    FigureCase::new("mpl_figure/figure", |fig| {
        // The named figure "today"; the red line goes to another figure.
        let ax = fig.subplot();
        ax.set_title("today");
        ax.plot_y(arange(0.0, 5.0, 1.0));
        Ok(())
    }),
    FigureCase::new("mpl_figure/figure_legend", |fig| {
        let axs = fig.subplots(2, 1)?;
        let lines: [(usize, [f64; 2], &str, &str); 5] = [
            (0, [1.0, 0.0], "x", "g"),
            (0, [0.0, 1.0], "y", "r"),
            (0, [0.5, 0.5], "y", "k"),
            (1, [1.0, 0.0], "_y", "r"),
            (1, [0.0, 1.0], "z", "b"),
        ];
        for (row, y, label, color) in lines {
            axs[(row, 0)].plot_with(
                [0.0, 1.0],
                y,
                LineProps::default().label(label).color(color),
            )?;
        }
        fig.legend(LegendOptions::default())?;
        Ok(())
    }),
    FigureCase::new("mpl_figure/suptitle", |fig| {
        fig.subplot();
        fig.suptitle("hello").set_color("r");
        fig.suptitle("title").set_color("g").set_rotation(30.0);
        Ok(())
    }),
    classic("mpl_figure/alpha", |fig| {
        fig.set_facecolor(Rgba::new(0.0, 1.0, 0.4, 1.0));
        // `fig.patch.set_alpha(0.4)`, kept through `savefig`'s facecolor.
        fig.set_alpha(Some(0.4));
        // Appended to `fig.patches` as it is: in display pixels.
        let circle = fig.add_patch(CirclePolygon::new((20.0, 20.0), 15.0));
        circle.set_facecolor("red").set_alpha(Some(0.6));
        circle.set_transform(Transform::Display);
        Ok(())
    })
    .sized(2.0, 1.0, 100.0)
    .with_rc_setup(|rc| {
        rc.set("savefig.facecolor", "(0, 1, 0.4)")?;
        rc.set("savefig.edgecolor", "none")?;
        Ok(())
    }),
    classic("mpl_figure/savefig_transparent", savefig_transparent)
        .with_rc_setup(|rc| Ok(rc.set("savefig.transparent", "True")?)),
    classic("mpl_figure/tightlayout_autolayout_deconflict", |fig| {
        fig.subplots(1, 2)?;
        fig.tight_layout(TightLayout {
            w_pad: Some(10.0),
            ..TightLayout::default()
        })?;
        Ok(())
    }),
    classic("mpl_figure/add_artist", add_artist),
    FigureCase::new("mpl_figure/tightbbox_box_aspect", |fig| {
        let gs = fig.add_gridspec(1, 2)?;
        let ax1 = fig.add_subplot_spec(&gs.cell(0, 0)?)?;
        let ax2 = fig.add_subplot_spec_3d(&gs.cell(0, 1)?, Axes3DOptions::default())?;
        ax1.set_box_aspect(Some(0.5))?;
        ax2.set_box_aspect(Some([2.0, 1.0, 1.0]), 1.0)?;
        remove_ticks_and_titles(fig);
        Ok(())
    })
    .with_rc(&[TIGHT_BBOX, TEAL]),
    classic("mpl_figure/animated_with_canvas_change", |fig| {
        let line = fig.subplot().plot_y(arange(0.0, 5.0, 1.0));
        line.set_animated(true);
        Ok(())
    }),
    classic("mpl_figure/rcparams", |fig| {
        fig.supxlabel("xlabel");
        fig.supylabel("ylabel");
        fig.suptitle("Title");
        Ok(())
    })
    .with_rc_setup(|rc| {
        rc.set("figure.labelweight", "bold")?;
        rc.set("figure.labelsize", "15")?;
        rc.set("figure.titleweight", "light")?;
        rc.set("figure.titlesize", "20")?;
        Ok(())
    }),
];

/// Rotates the x tick labels of `ax` by 90° (setting the rotation of
/// each label of `ax.get_xticklabels()`).
fn rotate_xticklabels(ax: &Axes) {
    ax.tick_params(TickParams {
        axis: Some(AxisName::X),
        labelrotation: Some(90.0),
        ..TickParams::default()
    });
}

/// `ax.xaxis.set_label_position("top")` and `ax.xaxis.tick_top()`.
fn x_on_top(ax: &Axes) -> Fallible<()> {
    ax.xaxis().set_label_position(SpineSide::Top)?;
    ax.xaxis().tick_to(SpineSide::Top)?;
    Ok(())
}

/// `ax.yaxis.set_label_position("right")` and `ax.yaxis.tick_right()`.
fn y_on_right(ax: &Axes) -> Fallible<()> {
    ax.yaxis().set_label_position(SpineSide::Right)?;
    ax.yaxis().tick_to(SpineSide::Right)?;
    Ok(())
}

fn align_labels(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::tight())?;
    let gs = GridSpec::new(3, 3)?;

    let ax = fig.add_subplot_spec(&gs.span(0..1, 0..2)?)?;
    ax.plot_y(arange(0.0, 1e6, 1000.0));
    ax.set_ylabel("Ylabel0 0");
    let ax = fig.add_subplot_spec(&gs.cell(0, 2)?)?;
    ax.plot_y(arange(0.0, 1e4, 100.0));

    for i in 0..3 {
        let ax = fig.add_subplot_spec(&gs.cell(1, i)?)?;
        ax.set_ylabel(format!("YLabel1 {i}"))
            .set_xlabel(format!("XLabel1 {i}"));
        if i != 1 {
            x_on_top(&ax)?;
        }
        if i == 0 {
            rotate_xticklabels(&ax);
        }
        if i == 2 {
            y_on_right(&ax)?;
        }
    }

    for i in 0..3 {
        let ax = fig.add_subplot_spec(&gs.cell(2, i)?)?;
        ax.set_xlabel(format!("XLabel2 {i}"))
            .set_ylabel(format!("YLabel2 {i}"));
        if i == 2 {
            ax.plot_y(arange(0.0, 1e4, 10.0));
            y_on_right(&ax)?;
            rotate_xticklabels(&ax);
        }
    }

    fig.align_labels(None);
    Ok(())
}

/// `test_align_titles` under tight (`1`) or constrained (`2`) layout.
fn align_titles(fig: &Figure, constrained_layout: bool) -> CaseResult {
    if constrained_layout {
        fig.set_layout_engine(LayoutEngine::constrained())?;
    } else {
        fig.set_layout_engine(LayoutEngine::tight())?;
    }
    let gs = GridSpec::with_ratios(
        1,
        2,
        Some(vec![2.0, 1.0]),
        None,
        SubplotOverrides::default(),
    )?;
    let axs = fig.subplots_on(&gs, Share::None, Share::None)?;

    let ax = &axs[(0, 0)];
    ax.plot_y(arange(0.0, 1e6, 1000.0));
    for (location, title) in [
        (TitleLocation::Left, "Title0 left"),
        (TitleLocation::Center, "Title0 center"),
        (TitleLocation::Right, "Title0 right"),
    ] {
        ax.set_title_at(location, title);
    }

    let ax = &axs[(0, 1)];
    ax.plot_y(arange(0.0, 1e4, 100.0));
    ax.set_title("Title1");
    ax.set_xlabel("Xlabel0");
    x_on_top(ax)?;
    rotate_xticklabels(ax);

    fig.align_titles(None);
    Ok(())
}

fn savefig_transparent(fig: &Figure) -> CaseResult {
    let gs1 = GridSpec::with_ratios(
        3,
        3,
        None,
        None,
        SubplotOverrides {
            left: Some(0.05),
            wspace: Some(0.05),
            ..SubplotOverrides::default()
        },
    )?;
    let f1 = fig.add_subfigure(&gs1.span(0..3, 0..3)?);
    let f2 = f1.add_subfigure(&gs1.cell(0, 0)?);

    let ax12 = f2.add_subplot_spec(&gs1.span(0..3, 0..3)?)?;

    let ax1 = f1.add_subplot_spec(&gs1.span(0..2, 0..3)?)?;
    let iax1 = ax1.inset_axes([0.1, 0.2, 0.3, 0.4]);
    let iax2 = iax1.inset_axes([0.1, 0.2, 0.3, 0.4]);

    let ax2 = fig.add_subplot_spec(&gs1.span(2..3, 0..2)?)?;
    let ax3 = fig.add_subplot_spec(&gs1.cell(2, 2)?)?;

    for ax in [ax12, ax1, iax1, iax2, ax2, ax3] {
        ax.set_xticks(Vec::<f64>::new())
            .set_yticks(Vec::<f64>::new());
        for spine in ax.spines().iter() {
            spine.set_visible(false);
        }
    }
    Ok(())
}

fn add_artist(fig: &Figure) -> CaseResult {
    fig.subplot();
    fig.add_line(&[0.2, 0.7], &[0.7, 0.7], LineProps::default())?;
    let l2 = fig.add_line(&[0.2, 0.7], &[0.8, 0.8], LineProps::default())?;
    // `transform=None`: the identity, display pixels.
    fig.add_patch(Circle::new((20.0, 20.0), 100.0))
        .set_transform(Transform::Display);
    fig.add_patch(Circle::new((0.7, 0.5), 0.05));
    let r3 = fig.add_patch(Circle::new((4.5, 0.8), 0.55));
    r3.set_facecolor("crimson");
    r3.set_transform(Transform::Inches);
    l2.remove();
    Ok(())
}
