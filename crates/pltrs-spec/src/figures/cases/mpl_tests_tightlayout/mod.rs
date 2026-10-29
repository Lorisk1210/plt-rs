//! `figures/cases_mpl_tests_tightlayout.py`: matplotlib's non-image tests
//! from `test_tightlayout.py` that lay a figure out and then check where
//! the axes went, or only draw, in the test suite's default style: ticks
//! pointing out, boxes anchored around the axes, very long labels, grids
//! tight layout declines, the figure title, a hand-placed colorbar and the
//! engine's padding.

use pltrs::axes::{Axes, MappedColors, ScatterOptions, TickDirection, TickLevel, TickParams};
use pltrs::colorbar::{ColorbarAxes, ColorbarOptions};
use pltrs::layout::{LayoutEngine, TightLayout};
use pltrs::offsetbox::{
    AnchorLoc, AnchoredBox, AnchoredOptions, BboxToAnchor, BoxItem, DrawingArea,
};
use pltrs::patches::{PatchProps, Rectangle};
use pltrs::prelude::*;
use pltrs::text::AnnotateOptions;
use pltrs::ticker::FixedLocator;
use pltrs::transforms::Transform;

use super::mpl_tests::classic;
use crate::figures::{CaseResult, FigureCase};

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    vec![
        classic("mpl_tests_tightlayout/outward_ticks", outward_ticks),
        classic("mpl_tests_tightlayout/tight_layout_offsetboxes_0", |fig| {
            offsetboxes(fig, Boxes::None)
        }),
        classic("mpl_tests_tightlayout/tight_layout_offsetboxes_1", |fig| {
            offsetboxes(fig, Boxes::Visible)
        }),
        classic("mpl_tests_tightlayout/tight_layout_offsetboxes_2", |fig| {
            offsetboxes(fig, Boxes::Hidden)
        }),
        classic("mpl_tests_tightlayout/verybig_decorators_xlabel", |fig| {
            fig.subplot().set_xlabel("a".repeat(100));
            Ok(())
        })
        .sized(3.0, 2.0, 100.0),
        classic("mpl_tests_tightlayout/verybig_decorators_ylabel", |fig| {
            fig.subplot().set_ylabel("a".repeat(100));
            Ok(())
        })
        .sized(3.0, 2.0, 100.0),
        classic("mpl_tests_tightlayout/big_decorators_horizontal", |fig| {
            let axs = fig.subplots(1, 2)?;
            axs[(0, 0)].set_xlabel("a".repeat(30));
            axs[(0, 1)].set_xlabel("b".repeat(30));
            Ok(())
        })
        .sized(3.0, 2.0, 100.0),
        classic("mpl_tests_tightlayout/big_decorators_vertical", |fig| {
            let axs = fig.subplots(2, 1)?;
            axs[(0, 0)].set_ylabel("a".repeat(20));
            axs[(1, 0)].set_ylabel("b".repeat(20));
            Ok(())
        })
        .sized(3.0, 2.0, 100.0),
        classic("mpl_tests_tightlayout/badsubplotgrid", |fig| {
            fig.add_subplot_spec(&fig.add_gridspec(4, 5)?.cell(0, 0)?)?;
            fig.add_subplot_spec(&fig.add_gridspec(5, 5)?.span(0..5, 3..5)?)?;
            // Grids of 4 and 5 rows do not combine: tight layout declines
            // (matplotlib warns) and leaves the figure alone.
            let _ = fig.tight_layout(TightLayout::default());
            Ok(())
        }),
        classic("mpl_tests_tightlayout/collapsed", |fig| {
            fig.set_layout_engine(LayoutEngine::tight())?;
            let ax = fig.subplot();
            ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 1.0)?;
            let options = AnnotateOptions {
                xytext: Some((10.5, 1.75).into()),
                annotation_clip: Some(false),
                ..AnnotateOptions::default()
            };
            ax.annotate("BIG LONG STRING", (1.25, 2.0), options)?;
            // The annotation leaves the axes no width: tight layout
            // declines both times (matplotlib warns).
            let _ = fig.tight_layout(TightLayout::default());
            let _ = fig.tight_layout(TightLayout {
                rect: Some([0.0, 0.0, 0.8, 0.8]),
                ..TightLayout::default()
            });
            Ok(())
        }),
        classic("mpl_tests_tightlayout/suptitle", |fig| {
            fig.set_layout_engine(LayoutEngine::tight())?;
            let ax = fig.subplot();
            fig.suptitle("foo");
            ax.set_title("bar");
            Ok(())
        }),
        classic("mpl_tests_tightlayout/manual_colorbar", manual_colorbar),
        classic("mpl_tests_tightlayout/tight_pads", |fig| {
            fig.set_layout_engine(LayoutEngine::Tight(TightLayout {
                pad: 0.15,
                ..TightLayout::default()
            }))?;
            fig.subplot();
            Ok(())
        }),
    ]
}

/// `test_outward_ticks`: four axes whose ticks point out, in, both ways
/// and out again, 16 to 32 points long, then tight layout.
fn outward_ticks(fig: &Figure) -> CaseResult {
    let ticks = |direction, length, which| TickParams {
        direction: Some(direction),
        length: Some(length),
        width: Some(3.0),
        which,
        ..TickParams::default()
    };
    let ax = fig.add_subplot(2, 2, 1)?;
    let minor = TickParams {
        bottom: Some(true),
        left: Some(true),
        ..ticks(TickDirection::Out, 32.0, TickLevel::Minor)
    };
    for axis in [ax.xaxis(), ax.yaxis()] {
        axis.set_tick_params(ticks(TickDirection::Out, 16.0, TickLevel::Major))
            .set_tick_params(minor.clone())
            .set_minor_locator(FixedLocator::new(vec![0.0]));
    }
    for (index, direction) in [
        (2, TickDirection::In),
        (3, TickDirection::InOut),
        (4, TickDirection::Out),
    ] {
        let ax = fig.add_subplot(2, 2, index)?;
        for axis in [ax.xaxis(), ax.yaxis()] {
            axis.set_tick_params(ticks(direction, 32.0, TickLevel::Major));
        }
    }
    fig.tight_layout(TightLayout::default())?;
    Ok(())
}

/// Whether and how `test_tight_layout_offsetboxes` surrounds its axes
/// with boxes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Boxes {
    /// No boxes.
    None,
    /// Eight boxes around each axes.
    Visible,
    /// The boxes, hidden.
    Hidden,
}

/// `test_tight_layout_offsetboxes`: four axes with a diagonal line each,
/// surrounded by boxes as `boxes` says, then tight layout.
fn offsetboxes(fig: &Figure, boxes: Boxes) -> CaseResult {
    let axs = fig.subplots(2, 2)?;
    for (ax, color) in axs
        .as_slice()
        .iter()
        .zip(["red", "blue", "green", "yellow"])
    {
        ax.plot_with(
            vec![0.0, 1.0],
            vec![0.0, 1.0],
            LineProps::default().color(color),
        )?;
        if boxes != Boxes::None {
            add_offsetboxes(ax, 20.0, color, boxes == Boxes::Visible);
        }
    }
    fig.tight_layout(TightLayout::default())?;
    Ok(())
}

/// The test's `add_offsetboxes`: squares of `size` points anchored by
/// their centres at the corners and edge midpoints of `ax`, a tenth of
/// its size outside it.
fn add_offsetboxes(ax: &Axes, size: f64, color: &str, visible: bool) {
    let (m, mp) = (0.1, 1.1);
    let anchor_points = [
        (-m, -m),
        (-m, 0.5),
        (-m, mp),
        (0.5, mp),
        (mp, mp),
        (mp, 0.5),
        (mp, -m),
        (0.5, -m),
    ];
    for (x, y) in anchor_points {
        let background = BoxItem::patch(
            Rectangle::new((0.0, 0.0), size, size),
            PatchProps::default()
                .facecolor(color)
                .edgecolor(Color::none())
                .linewidth(0.0)
                .antialiased(false),
        );
        let area = DrawingArea::new(size, size, 0.0, 0.0).with_artist(background);
        let options = AnchoredOptions {
            pad: 0.0,
            borderpad: (0.0, 0.0),
            frameon: false,
            bbox_to_anchor: Some(BboxToAnchor::Point(x, y)),
            bbox_transform: Some(Transform::Axes(ax.id())),
            ..AnchoredOptions::new(AnchorLoc::Center)
        };
        ax.add_anchored_box(AnchoredBox::new(area, options))
            .set_visible(visible);
    }
}

/// `test_manual_colorbar`: a scatter's colorbar in an axes placed by hand
/// beside it, which tight layout leaves out (matplotlib warns).
fn manual_colorbar(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(1, 2)?;
    let ax = &axes[(0, 1)];
    let pts = ax.scatter_with(
        vec![0.0, 1.0],
        vec![0.0, 1.0],
        ScatterOptions {
            colors: MappedColors::new(vec![1.0, 5.0]).into(),
            ..ScatterOptions::default()
        },
    )?;
    let rect = ax.position();
    let cax = fig.add_axes([rect.x1() + 0.005, rect.y0(), 0.015, rect.height()])?;
    fig.colorbar(
        &pts,
        ColorbarOptions {
            axes: ColorbarAxes::Cax(cax),
            ..ColorbarOptions::default()
        },
    )?;
    fig.tight_layout(TightLayout::default())?;
    Ok(())
}
