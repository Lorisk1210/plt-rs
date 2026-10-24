//! `figures/cases_layout.py`, split by section.

mod constrained;
mod insets;
mod tight;

pub(crate) use constrained::CASES as CONSTRAINED;
pub(crate) use insets::CASES as INSETS;
pub(crate) use tight::CASES as TIGHT;

use plt::figure::{Mosaic, MosaicAxes, MosaicCell, MosaicOptions, Share, SubfigureOptions};
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::numeric::Grid;
use plt::prelude::*;
use plt::text::Text;

use crate::figures::FigureCase;

/// `T = np.linspace(0, 10, 40)`.
fn t() -> Vec<f64> {
    linspace(0.0, 10.0, 40)
}

/// `GRID = np.add.outer(np.sin(np.linspace(0, 3, 12)),
/// np.cos(np.linspace(0, 4, 16)))`.
fn grid() -> Grid<f64> {
    let rows = linspace(0.0, 3.0, 12);
    let cols = linspace(0.0, 4.0, 16);
    Grid::from_fn(12, 16, |r, c| rows[r].sin() + cols[c].cos())
}

/// `_fill(ax, i)`: `ax.plot(T, np.sin(T + i))`.
fn fill(ax: &Axes, i: usize) -> Result<(), plt::axes::PlotError> {
    let shift = i as f64;
    ax.plot(
        t(),
        t().into_iter()
            .map(|t| (t + shift).sin())
            .collect::<Vec<_>>(),
    )
    .map(drop)
}

pub(crate) const GRIDS: &[FigureCase] = &[
    FigureCase::new("layout/subplots_2x2", |fig| {
        for (i, ax) in fig.subplots(2, 2)?.as_slice().iter().enumerate() {
            fill(ax, i)?;
        }
        Ok(())
    }),
    FigureCase::new("layout/subplots_share_col_row", |fig| {
        let axs = fig.subplots_shared(2, 3, Share::Col, Share::Row)?;
        for (i, ax) in axs.as_slice().iter().enumerate() {
            let x_gain = 1.0 + (i % 3) as f64;
            let y_gain = 1.0 + 3.0 * (i / 3) as f64;
            let x: Vec<f64> = t().into_iter().map(|t| t * x_gain).collect();
            let y: Vec<f64> = t().into_iter().map(|t| t.sin() * y_gain).collect();
            ax.plot(x, y)?;
        }
        Ok(())
    }),
    FigureCase::new("layout/subplots_adjust", |fig| {
        for (i, ax) in fig.subplots(2, 2)?.as_slice().iter().enumerate() {
            fill(ax, i)?;
        }
        fig.subplots_adjust(&SubplotOverrides {
            left: Some(0.1),
            right: Some(0.98),
            bottom: Some(0.08),
            top: Some(0.95),
            wspace: Some(0.4),
            hspace: Some(0.6),
        })?;
        Ok(())
    }),
    FigureCase::new("layout/add_subplot_codes", |fig| {
        fill(&fig.add_subplot(2, 2, 1)?, 0)?;
        fill(&fig.add_subplot(2, 2, 2)?, 1)?;
        fill(&fig.add_subplot(2, 1, 2)?, 2)?;
        Ok(())
    }),
    FigureCase::new("layout/gridspec_ratios", |fig| {
        let gs = GridSpec::with_ratios(
            2,
            3,
            Some(vec![1.0, 2.0, 3.0]),
            Some(vec![2.0, 1.0]),
            SubplotOverrides::default(),
        )?;
        for i in 0..6 {
            fill(&fig.add_subplot_spec(&gs.cell(i / 3, i % 3)?)?, i)?;
        }
        Ok(())
    }),
    FigureCase::new("layout/gridspec_spans", |fig| {
        let gs = GridSpec::new(3, 3)?;
        let blocks = [
            (0..1, 0..3),
            (1..2, 0..2),
            (1..3, 2..3),
            (2..3, 0..1),
            (2..3, 1..2),
        ];
        for (i, (rows, cols)) in blocks.into_iter().enumerate() {
            fill(&fig.add_subplot_spec(&gs.span(rows, cols)?)?, i)?;
        }
        Ok(())
    }),
    FigureCase::new("layout/gridspec_spacing_and_extent", |fig| {
        let gs = GridSpec::with_ratios(
            2,
            2,
            None,
            None,
            SubplotOverrides {
                left: Some(0.2),
                right: Some(0.9),
                bottom: Some(0.15),
                top: Some(0.85),
                wspace: Some(0.05),
                hspace: Some(0.05),
            },
        )?;
        let axs = fig.subplots_on(&gs, Share::All, Share::All)?;
        for (i, ax) in axs.as_slice().iter().enumerate() {
            fill(ax, i)?;
        }
        Ok(())
    }),
    FigureCase::new("layout/gridspec_from_subplotspec", |fig| {
        let outer = GridSpec::with_ratios(
            1,
            2,
            None,
            None,
            SubplotOverrides {
                wspace: Some(0.3),
                ..SubplotOverrides::default()
            },
        )?;
        let inner = GridSpec::nested(&outer.cell(0, 1)?, 2, 2, None, None, (Some(0.1), Some(0.1)))?;
        fill(&fig.add_subplot_spec(&outer.cell(0, 0)?)?, 0)?;
        for i in 0..4 {
            fill(&fig.add_subplot_spec(&inner.cell(i / 2, i % 2)?)?, i + 1)?;
        }
        Ok(())
    }),
    FigureCase::new("layout/subplot_mosaic_string", |fig| {
        let axes = fig.subplot_mosaic(&"AAB;C.B".parse::<Mosaic>()?, MosaicOptions::default())?;
        fill_sorted(&axes, None)
    }),
    FigureCase::new("layout/subplot_mosaic_nested_ratios", |fig| {
        let label = |name: &str| MosaicCell::Label(name.into());
        let mosaic = Mosaic::new(vec![
            vec![
                label("main"),
                MosaicCell::Nested("ab;cd".parse::<Mosaic>()?),
            ],
            vec![label("bottom"), label("bottom")],
        ])?;
        let options = MosaicOptions {
            width_ratios: Some(vec![2.0, 1.0]),
            height_ratios: Some(vec![3.0, 1.0]),
            ..MosaicOptions::default()
        };
        fill_sorted(&fig.subplot_mosaic(&mosaic, options)?, Some(8.0))
    }),
    FigureCase::new("layout/subfigures_side_by_side", |fig| {
        let options = SubfigureOptions {
            wspace: Some(0.05),
            width_ratios: Some(vec![2.0, 1.0]),
            ..SubfigureOptions::default()
        };
        let parts = fig.subfigures(1, 2, options)?;
        let (left, right) = (&parts[(0, 0)], &parts[(0, 1)]);
        left.set_facecolor(Color::from("0.92").resolve()?);
        left.suptitle("left subfigure");
        for (i, ax) in left.subplots(2, 1)?.as_slice().iter().enumerate() {
            fill(ax, i)?;
        }
        right.suptitle("right subfigure");
        fill(&right.subplot(), 3)?;
        Ok(())
    }),
    FigureCase::new("layout/add_axes_rects", |fig| {
        fill(&fig.add_axes([0.1, 0.1, 0.8, 0.8])?, 0)?;
        let ax = fig.add_axes([0.55, 0.55, 0.3, 0.25])?;
        ax.set_facecolor("0.9");
        fill(&ax, 2)?;
        ax.set_title("overlay");
        Ok(())
    }),
    FigureCase::new("layout/figure_facecolor_edgecolor_linewidth", |fig| {
        fig.set_facecolor(Color::from("lightsteelblue").resolve()?)
            .set_edgecolor(Color::from("navy").resolve()?)
            .set_linewidth(6.0);
        fill(&fig.subplot(), 0)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("layout/dpi_72", |fig| titled(fig, "72 dpi")).sized(6.4, 4.8, 72.0),
    FigureCase::new("layout/dpi_150", |fig| titled(fig, "150 dpi")).sized(4.0, 3.0, 150.0),
    FigureCase::new("layout/figsize_wide_short", |fig| {
        fill(&fig.subplot(), 0)?;
        Ok(())
    })
    .sized(8.0, 2.0, 100.0),
    FigureCase::new("layout/figsize_tall_narrow", |fig| {
        for (i, ax) in fig.subplots(3, 1)?.as_slice().iter().enumerate() {
            fill(ax, i)?;
        }
        Ok(())
    })
    .sized(2.5, 5.0, 100.0),
];

/// `for i, (name, ax) in enumerate(sorted(axd.items()))`: fills each
/// axes and titles it with its label (at `title_size` points if given).
fn fill_sorted(
    axes: &MosaicAxes,
    title_size: Option<f64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut sorted: Vec<(&str, &Axes)> = axes.iter().collect();
    sorted.sort_by_key(|(name, _)| *name);
    for (i, (name, ax)) in sorted.into_iter().enumerate() {
        fill(ax, i)?;
        let title = ax.set_title(name);
        if let Some(size) = title_size {
            title.set_fontsize(size);
        }
    }
    Ok(())
}

/// One filled subplot with `title`.
fn titled(fig: &Figure, title: &str) -> Result<(), Box<dyn std::error::Error>> {
    let ax = fig.subplot();
    fill(&ax, 0)?;
    let _: Text = ax.set_title(title);
    Ok(())
}
