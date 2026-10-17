//! The inset section of `figures/cases_layout.py`.

use plt::prelude::*;

use super::{fill, t};
use crate::figures::FigureCase;

/// `np.sin(3 * T) * np.exp(-T / 5)`.
fn damped() -> Vec<f64> {
    t().into_iter()
        .map(|t| (3.0 * t).sin() * (-t / 5.0).exp())
        .collect()
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("layout/inset_axes", |fig| {
        let ax = fig.subplot();
        fill(&ax, 0)?;
        let inset = ax.inset_axes([0.6, 0.6, 0.35, 0.3]);
        let cosine: Vec<f64> = t().into_iter().map(f64::cos).collect();
        inset.plot_fmt(t(), cosine, "C1")?;
        Ok(())
    }),
    FigureCase::new("layout/indicate_inset_zoom", |fig| {
        let ax = fig.subplot();
        ax.plot(t(), damped())?;
        let inset = ax.inset_axes([0.55, 0.55, 0.4, 0.4]);
        inset.set_xlim(1.0, 2.5).set_ylim(-0.8, 0.6);
        inset.set_xticklabels(Vec::<String>::new())?;
        inset.set_yticklabels(Vec::<String>::new())?;
        inset.plot(t(), damped())?;
        ax.indicate_inset_zoom(&inset, PatchProps::default().edgecolor("black"));
        Ok(())
    }),
];
