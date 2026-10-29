//! Layout engines and spacing: `demo_constrained_layout`,
//! `demo_tight_layout`, `subplots_adjust`, `auto_subplots_adjust` and
//! `ganged_plots`.

use pltrs::axes::ImshowOptions;
use pltrs::colorbar::{ColorbarAxes, ColorbarOptions};
use pltrs::figure::Share;
use pltrs::fonts::FontSize;
use pltrs::gridspec::{GridSpec, SubplotOverrides};
use pltrs::layout::{LayoutEngine, TightLayout};
use pltrs::numeric::Grid;
use pltrs::prelude::*;
use pltrs::transforms::Bbox;

use super::SEED;
use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The layout examples' `example_plot`: a line with x label, y label and
/// title in the given font sizes.
fn example_plot(ax: &Axes, [xsize, ysize, title_size]: [f64; 3]) {
    ax.plot_y([1.0, 2.0]);
    ax.set_xlabel("x-label").set_ylabel("y-label");
    ax.xaxis().set_label_fontsize(FontSize::Points(xsize));
    ax.yaxis().set_label_fontsize(FontSize::Points(ysize));
    ax.set_title("Title").set_fontsize(title_size);
}

/// `demo_constrained_layout`'s font sizes.
const CONSTRAINED_SIZES: [f64; 3] = [12.0, 12.0, 14.0];

fn demo_constrained_layout_1(fig: &Figure) -> CaseResult {
    for ax in fig.subplots(2, 2)?.as_slice() {
        example_plot(ax, CONSTRAINED_SIZES);
    }
    Ok(())
}

fn demo_constrained_layout_2(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(Some(LayoutEngine::constrained()))?;
    demo_constrained_layout_1(fig)
}

fn demo_constrained_layout_3(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(Some(LayoutEngine::constrained()))?;
    let outer = GridSpec::new(1, 2)?;
    for (col, rows) in [3, 2].into_iter().enumerate() {
        let nested = GridSpec::nested(&outer.cell(0, col)?, rows, 1, None, None, (None, None))?;
        for row in 0..rows {
            example_plot(
                &fig.add_subplot_spec(&nested.cell(row, 0)?)?,
                CONSTRAINED_SIZES,
            );
        }
    }
    Ok(())
}

/// `_tight_plots`: `example_plot` on each axes with font sizes cycling
/// through 8, 16, 24 and 32 from where the example's `calls_before`
/// earlier calls (three sizes each) left the cycle.
fn tight_plots(axes: &[Axes], calls_before: usize) {
    let mut sizes = [8.0, 16.0, 24.0, 32.0]
        .into_iter()
        .cycle()
        .skip(3 * calls_before % 4);
    for ax in axes {
        let mut next = || sizes.next().expect("a cycle never ends");
        example_plot(ax, [next(), next(), next()]);
    }
}

/// `tight_plots` on a fresh grid of subplots, then `fig.tight_layout()`.
fn tight_grid(fig: &Figure, nrows: usize, ncols: usize, calls_before: usize) -> CaseResult {
    tight_plots(fig.subplots(nrows, ncols)?.as_slice(), calls_before);
    fig.tight_layout(TightLayout::default())?;
    Ok(())
}

fn demo_tight_layout_1(fig: &Figure) -> CaseResult {
    tight_grid(fig, 1, 1, 0)
}

fn demo_tight_layout_2(fig: &Figure) -> CaseResult {
    tight_grid(fig, 2, 2, 1)
}

fn demo_tight_layout_3(fig: &Figure) -> CaseResult {
    tight_grid(fig, 2, 1, 5)
}

fn demo_tight_layout_4(fig: &Figure) -> CaseResult {
    tight_grid(fig, 1, 2, 7)
}

fn demo_tight_layout_5(fig: &Figure) -> CaseResult {
    tight_grid(fig, 3, 3, 9)
}

fn demo_tight_layout_6(fig: &Figure) -> CaseResult {
    let axes = [
        fig.add_subplot(2, 2, 1)?,
        fig.add_subplot(2, 2, 3)?,
        fig.add_subplot(1, 2, 2)?,
    ];
    tight_plots(&axes, 18);
    fig.tight_layout(TightLayout::default())?;
    Ok(())
}

fn demo_tight_layout_7(fig: &Figure) -> CaseResult {
    let grid = fig.add_gridspec(3, 3)?;
    let axes = [
        fig.add_subplot_spec(&grid.span(0..1, 0..1)?)?,
        fig.add_subplot_spec(&grid.span(0..1, 1..3)?)?,
        fig.add_subplot_spec(&grid.span(1..3, 0..2)?)?,
        fig.add_subplot_spec(&grid.span(1..3, 2..3)?)?,
    ];
    tight_plots(&axes, 21);
    fig.tight_layout(TightLayout::default())?;
    Ok(())
}

/// A column of `rows` subplots on a grid of their own, plotted by
/// `tight_plots` and tightly laid out into `rect`.
fn tight_column(
    fig: &Figure,
    rows: usize,
    calls_before: usize,
    rect: [f64; 4],
) -> Result<GridSpec, Box<dyn std::error::Error>> {
    let grid = fig.add_gridspec(rows, 1)?;
    let axes = (0..rows)
        .map(|row| fig.add_subplot_spec(&grid.cell(row, 0)?))
        .collect::<Result<Vec<_>, _>>()?;
    tight_plots(&axes, calls_before);
    let params = TightLayout {
        rect: Some(rect),
        ..TightLayout::default()
    };
    grid.tight_layout(fig, params)?;
    Ok(grid)
}

fn demo_tight_layout_8(fig: &Figure) -> CaseResult {
    // `rect=[None, None, 0.45, None]` and `[0.45, None, None, None]`: edges
    // at 0 or 1 leave those margins to the layout, as `None` does.
    let left = tight_column(fig, 3, 25, [0.0, 0.0, 0.45, 1.0])?;
    let right = tight_column(fig, 2, 28, [0.45, 0.0, 1.0, 1.0])?;
    let (left_edges, right_edges) = (left.overrides(), right.overrides());
    let edge = |a: Option<f64>, b: Option<f64>, pick: fn(f64, f64) -> f64| {
        a.zip(b).map(|(a, b)| pick(a, b))
    };
    let matched = SubplotOverrides {
        top: edge(left_edges.top, right_edges.top, f64::min),
        bottom: edge(left_edges.bottom, right_edges.bottom, f64::max),
        ..SubplotOverrides::default()
    };
    left.update(fig, matched)?;
    right.update(fig, matched)?;
    Ok(())
}

fn subplots_adjust(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let mut noise = || Grid::from_vec(100, 100, rng.uniform(100 * 100, 0.0, 1.0));
    fig.add_subplot(2, 1, 1)?
        .imshow(noise()?, ImshowOptions::default())?;
    let image = fig
        .add_subplot(2, 1, 2)?
        .imshow(noise()?, ImshowOptions::default())?;
    fig.subplots_adjust(SubplotOverrides {
        bottom: Some(0.1),
        right: Some(0.8),
        top: Some(0.9),
        ..SubplotOverrides::default()
    })?;
    let cax = fig.add_axes([0.85, 0.1, 0.075, 0.8])?;
    fig.colorbar(
        &image,
        ColorbarOptions {
            axes: ColorbarAxes::Cax(cax),
            ..ColorbarOptions::default()
        },
    )?;
    Ok(())
}

fn auto_subplots_adjust(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot_y(arange(0.0, 10.0, 1.0));
    ax.set_yticks([2.0, 5.0, 7.0])
        .set_yticklabels(["really, really, really", "long", "labels"])?;
    // The example's draw callback: make room for the widest tick label.
    let extents = ax.yaxis().ticklabel_extents();
    let bbox = Bbox::union_all(&extents).ok_or("the tick labels have extents")?;
    let bbox_width = bbox.width() / fig.size_pixels().0;
    if fig.subplot_params().left < bbox_width {
        fig.subplots_adjust(SubplotOverrides {
            left: Some(1.1 * bbox_width),
            ..SubplotOverrides::default()
        })?;
    }
    Ok(())
}

fn ganged_plots(fig: &Figure) -> CaseResult {
    let t = arange(0.0, 2.0, 0.01);
    let s1 = map(&t, |t| (2.0 * std::f64::consts::PI * t).sin());
    let s2 = map(&t, |t| (-t).exp());
    let s3: Vec<f64> = s1.iter().zip(&s2).map(|(a, b)| a * b).collect();
    let axs = fig.subplots_shared(3, 1, Share::All, Share::None)?;
    fig.subplots_adjust(SubplotOverrides {
        hspace: Some(0.0),
        ..SubplotOverrides::default()
    })?;
    let rows = [
        (s1, arange(-0.9, 1.0, 0.4), (-1.0, 1.0)),
        (s2, arange(0.1, 1.0, 0.2), (0.0, 1.0)),
        (s3, arange(-0.9, 1.0, 0.4), (-1.0, 1.0)),
    ];
    for (ax, (s, ticks, (low, high))) in axs.as_slice().iter().zip(rows) {
        ax.plot(&t, s)?;
        ax.set_yticks(ticks).set_ylim(low, high)?;
    }
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "gallery_subplots/demo_constrained_layout_1",
        demo_constrained_layout_1,
    ),
    FigureCase::new(
        "gallery_subplots/demo_constrained_layout_2",
        demo_constrained_layout_2,
    ),
    FigureCase::new(
        "gallery_subplots/demo_constrained_layout_3",
        demo_constrained_layout_3,
    ),
    FigureCase::new("gallery_subplots/demo_tight_layout_1", demo_tight_layout_1),
    FigureCase::new("gallery_subplots/demo_tight_layout_2", demo_tight_layout_2),
    FigureCase::new("gallery_subplots/demo_tight_layout_3", demo_tight_layout_3),
    FigureCase::new("gallery_subplots/demo_tight_layout_4", demo_tight_layout_4),
    FigureCase::new("gallery_subplots/demo_tight_layout_5", demo_tight_layout_5),
    FigureCase::new("gallery_subplots/demo_tight_layout_6", demo_tight_layout_6),
    FigureCase::new("gallery_subplots/demo_tight_layout_7", demo_tight_layout_7),
    FigureCase::new("gallery_subplots/demo_tight_layout_8", demo_tight_layout_8),
    FigureCase::new("gallery_subplots/subplots_adjust", subplots_adjust),
    FigureCase::new(
        "gallery_subplots/auto_subplots_adjust",
        auto_subplots_adjust,
    ),
    FigureCase::new("gallery_subplots/ganged_plots", ganged_plots),
];
