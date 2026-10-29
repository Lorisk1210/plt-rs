//! Colorbars of discrete and non-linear norms, divided colours, a bare
//! scalar mappable and a scatter plot.

use pltrs::axes::ImshowOptions;
use pltrs::axes::{MappedColors, ScatterOptions};
use pltrs::colorbar::{ColorbarAxes, ColorbarOptions, Spacing};
use pltrs::colors::{
    BoundaryNorm, Colorize, ColorizerOptions, Colormap, Extend, LogNorm, Normalize, ScalarMappable,
    TwoSlopeNorm, colormaps,
};
use pltrs::figure::Figure;
use pltrs::image::ImageData;
use pltrs::numeric::{Grid, linspace};

use super::data::field;
use crate::figures::CaseResult;
use crate::rng::SpecRng;

/// `data` shown with `colors` and a colorbar beside it.
fn image_colorbar(
    fig: &Figure,
    data: impl Into<ImageData>,
    colorizer: ColorizerOptions,
    options: ColorbarOptions,
) -> CaseResult {
    let ax = fig.subplot();
    let im = ax.imshow(data, ImshowOptions::default().colorizer(colorizer))?;
    let options = ColorbarOptions {
        axes: ColorbarAxes::Parent(ax),
        ..options
    };
    fig.colorbar(&im, options)?;
    Ok(())
}

pub(super) fn boundarynorm_uniform(fig: &Figure) -> CaseResult {
    let cmap = Colormap::listed(
        "from_list",
        vec!["#1b9e77", "#d95f02", "#7570b3", "#e7298a", "#66a61e"],
    )?;
    let norm = BoundaryNorm::new([-1.0, -0.4, -0.1, 0.1, 0.4, 1.0], cmap.n(), Extend::Neither)?;
    let colors = ColorizerOptions::default().cmap(cmap).norm(norm);
    image_colorbar(fig, field(40), colors, ColorbarOptions::default())
}

pub(super) fn boundarynorm_proportional_extend(fig: &Figure) -> CaseResult {
    let norm = BoundaryNorm::new([-0.6, -0.4, -0.1, 0.1, 0.4, 0.6], 256, Extend::Both)?;
    let colors = ColorizerOptions::default()
        .cmap(colormaps::get("viridis")?)
        .norm(norm);
    let options = ColorbarOptions {
        spacing: Spacing::Proportional,
        ..ColorbarOptions::default()
    };
    image_colorbar(fig, field(40), colors, options)
}

pub(super) fn lognorm(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 4.0, 40);
    let data = Grid::from_fn(40, 40, |r, c| 10f64.powf(x[r] + x[c] / 2.0));
    let colors = ColorizerOptions::default().norm(LogNorm::default());
    image_colorbar(fig, data, colors, ColorbarOptions::default())
}

pub(super) fn twoslopenorm(fig: &Figure) -> CaseResult {
    let norm = TwoSlopeNorm::new(0.0, Some(-0.5), Some(2.0))?;
    let colors = ColorizerOptions::default()
        .cmap(colormaps::get("RdBu_r")?)
        .norm(norm);
    let data = field(40).map(|v| v * 3.0);
    image_colorbar(fig, data, colors, ColorbarOptions::default())
}

pub(super) fn drawedges(fig: &Figure) -> CaseResult {
    let cmap = Colormap::listed("from_list", vec!["navy", "teal", "gold", "tomato"])?;
    let options = ColorbarOptions {
        drawedges: true,
        ..ColorbarOptions::default()
    };
    image_colorbar(
        fig,
        field(40),
        ColorizerOptions::default().cmap(cmap),
        options,
    )
}

pub(super) fn scalarmappable_only(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot(vec![0.0, 1.0, 2.0], vec![2.0, 0.0, 1.0])?;
    let mappable = ScalarMappable::with_norm(
        colormaps::get("cividis")?,
        Box::new(Normalize::new(0.0, 10.0)),
    );
    let options = ColorbarOptions {
        axes: ColorbarAxes::Parent(ax),
        label: "speed".into(),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&mappable, options)?;
    Ok(())
}

pub(super) fn scatter(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let mut rng = SpecRng::new(11);
    let x = rng.uniform(60, 0.0, 1.0);
    let y = rng.uniform(60, 0.0, 1.0);
    let c: Vec<f64> = x.iter().zip(&y).map(|(x, y)| x + y).collect();
    let options = ScatterOptions {
        colors: MappedColors::new(c).cmap(colormaps::get("plasma")?).into(),
        ..ScatterOptions::default()
    };
    let points = ax.scatter_with(x, y, options)?;
    let options = ColorbarOptions {
        axes: ColorbarAxes::Parent(ax),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&points, options)?;
    Ok(())
}
