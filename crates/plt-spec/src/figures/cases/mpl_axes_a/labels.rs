//! An invisible axes and axis labels at the ends of their axes (by `loc`
//! and by the `xaxis.labellocation`/`yaxis.labellocation` rc parameters),
//! including the colorbar's.

use plt::axes::LabelLoc;
use plt::colorbar::{Colorbar, ColorbarOptions};
use plt::legend::LegendOptions;
use plt::orientation::Orientation;
use plt::prelude::*;

use crate::figures::CaseResult;

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn invisible_axes(fig: &Figure) -> CaseResult {
    fig.subplot().set_visible(false);
    Ok(())
}

/// `_labelled_scatter`: two colour-mapped points with a legend, and their
/// colorbar in `orientation`.
fn labelled_scatter(
    fig: &Figure,
    orientation: Orientation,
) -> Result<(Axes, Colorbar), Box<dyn std::error::Error>> {
    let ax = fig.subplot();
    let options = ScatterOptions {
        colors: MappedColors::new(vec![1.0, 2.0]).into(),
        ..ScatterOptions::default()
    };
    let points = ax.scatter_with([1.0, 2.0], [1.0, 2.0], options)?;
    points.set_label("scatter");
    ax.legend(LegendOptions::default())?;
    let colorbar = fig.colorbar(
        &points,
        ColorbarOptions {
            orientation: Some(orientation),
            ..ColorbarOptions::default()
        },
    )?;
    Ok((ax, colorbar))
}

/// The axis labels at `loc` and the colorbar label at `colorbar_loc`.
fn labels_at(ax: &Axes, colorbar: &Colorbar, loc: LabelLoc, colorbar_loc: Option<LabelLoc>) {
    ax.set_ylabel("Y Label").yaxis().set_label_loc(loc);
    ax.set_xlabel("X Label").xaxis().set_label_loc(loc);
    colorbar.set_label("Z Label", colorbar_loc);
}

pub(super) fn label_loc_vertical(fig: &Figure) -> CaseResult {
    let (ax, colorbar) = labelled_scatter(fig, Orientation::Vertical)?;
    labels_at(&ax, &colorbar, LabelLoc::End, Some(LabelLoc::End));
    Ok(())
}

pub(super) fn label_loc_horizontal(fig: &Figure) -> CaseResult {
    let (ax, colorbar) = labelled_scatter(fig, Orientation::Horizontal)?;
    labels_at(&ax, &colorbar, LabelLoc::Start, Some(LabelLoc::Start));
    Ok(())
}

pub(super) fn label_loc_rc(fig: &Figure) -> CaseResult {
    plt::rc::context(|| {
        plt::rc::set("xaxis.labellocation", "right")?;
        plt::rc::set("yaxis.labellocation", "top")?;
        let (ax, colorbar) = labelled_scatter(fig, Orientation::Horizontal)?;
        ax.set_ylabel("Y Label").set_xlabel("X Label");
        colorbar.set_label("Z Label", None);
        Ok(())
    })
}
