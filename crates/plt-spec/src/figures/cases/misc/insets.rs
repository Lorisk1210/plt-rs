//! Inset indicator cases of `figures/cases_axes_misc.py`.

use plt::axes::ImshowOptions;
use plt::image::{Extent, Origin};
use plt::prelude::*;

use super::{field, wave};
use crate::figures::FigureCase;

/// `imshow(_field(40), extent=(-3, 3, -3, 3), origin="lower")`.
fn show_field(ax: &Axes) -> Result<(), plt::image::ImageError> {
    let options = ImshowOptions {
        extent: Some(Extent::new(-3.0, 3.0, -3.0, 3.0)),
        origin: Some(Origin::Lower),
        ..ImshowOptions::default()
    };
    ax.imshow(field(40), options).map(drop)
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("misc/indicate_inset_zoom", |fig| {
        let ax = fig.subplot();
        show_field(&ax)?;
        let inset = ax.inset_axes([0.6, 0.6, 0.37, 0.37]);
        inset.set_xlim(-1.5, -0.5)?.set_ylim(-0.5, 0.5)?;
        show_field(&inset)?;
        ax.indicate_inset_zoom(&inset, PatchProps::default().edgecolor("black"));
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("misc/indicate_inset_bounds", |fig| {
        let ax = fig.subplot();
        ax.plot_y(wave(50));
        ax.indicate_inset(
            [10.0, 0.0, 15.0, 1.0],
            None,
            PatchProps::default().edgecolor("tab:red").alpha(1.0),
        );
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
];
