//! `figures/cases_layout.py`.

use plt::figure::Share;
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::prelude::*;

use crate::figures::FigureCase;

/// `T = np.linspace(0, 10, 40)`.
fn t() -> Vec<f64> {
    linspace(0.0, 10.0, 40)
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

pub(crate) const CASES: &[FigureCase] = &[
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
];
