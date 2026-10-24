//! Inset locators, RGB axes and `axisartist` axis lines of
//! `figures/cases_toolkits.py`.

use plt::axes::{HistPlotOptions, ImshowOptions};
use plt::axes_grid1::{
    Corner, InsetOptions, RgbAxes, SizeSpec, inset_axes, mark_inset, zoomed_inset_axes,
};
use plt::axisartist::{self, AxislineStyle};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::{Color, Colorize, ColorizerOptions, colormaps};
use plt::figure::Figure;
use plt::image::{Extent, Origin};
use plt::lines::LineProps;
use plt::numeric::{Grid, linspace};
use plt::offsetbox::{AnchorLoc, BboxToAnchor};
use plt::patches::PatchProps;
use plt::prelude::ArtistHandle;
use plt::stats::{Bins, HistOptions};

use super::field;
use crate::figures::CaseResult;

/// `imshow(field, extent=(-3, 3, -3, 3), cmap=cmap)`, origin `origin`.
fn field_image(
    cmap: &str,
    origin: Option<Origin>,
) -> Result<ImshowOptions, Box<dyn std::error::Error>> {
    Ok(ImshowOptions {
        colorizer: ColorizerOptions::default().cmap(colormaps::get(cmap)?),
        extent: Some(Extent::new(-3.0, 3.0, -3.0, 3.0)),
        origin,
        ..ImshowOptions::default()
    })
}

pub(super) fn inset_locator(fig: &Figure) -> CaseResult {
    let ax = fig.add_axes([0.1, 0.1, 0.65, 0.8])?;
    let im = ax.imshow(field(40, 0.0), field_image("coolwarm", None)?)?;
    let upper = inset_axes(
        &ax,
        SizeSpec::Percent(30.0),
        SizeSpec::Percent(40.0),
        InsetOptions::default(),
    );
    upper.plot_y(
        linspace(0.0, 1.0, 20)
            .into_iter()
            .map(|v| v * v)
            .collect::<Vec<_>>(),
    );
    let lower = inset_axes(
        &ax,
        1.0.into(),
        0.6.into(),
        InsetOptions {
            loc: AnchorLoc::LowerLeft,
            borderpad: (1.5, 1.5),
            ..InsetOptions::default()
        },
    );
    let histogram = HistPlotOptions {
        layout: HistOptions {
            bins: Some(Bins::Count(20)),
            ..HistOptions::default()
        },
        color: vec![Color::from("0.4")],
        ..HistPlotOptions::default()
    };
    lower.hist(&[field(40, 0.0).as_slice().to_vec()], histogram)?;
    lower.set_xticks(Vec::<f64>::new());
    lower.set_yticks(Vec::<f64>::new());
    let padded = inset_axes(
        &ax,
        SizeSpec::Percent(25.0),
        SizeSpec::Percent(20.0),
        InsetOptions {
            loc: AnchorLoc::LowerRight,
            borderpad: (3.0, 0.5),
            ..InsetOptions::default()
        },
    );
    padded.set_facecolor("lightyellow");
    let cax = inset_axes(
        &ax,
        SizeSpec::Percent(5.0),
        SizeSpec::Percent(100.0),
        InsetOptions {
            loc: AnchorLoc::LowerLeft,
            bbox_to_anchor: Some(BboxToAnchor::Bounds(1.05, 0.0, 1.0, 1.0)),
            bbox_transform: Some(ax.trans_axes()),
            borderpad: (0.0, 0.0),
        },
    );
    fig.colorbar(
        &im,
        ColorbarOptions {
            axes: ColorbarAxes::Cax(cax),
            ..ColorbarOptions::default()
        },
    )?;
    Ok(())
}

pub(super) fn zoomed_marked(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.imshow(field(60, 0.0), field_image("magma", Some(Origin::Lower))?)?;
    let zoomed = zoomed_inset_axes(
        &ax,
        2.5,
        InsetOptions {
            loc: AnchorLoc::UpperLeft,
            ..InsetOptions::default()
        },
    );
    zoomed.imshow(field(60, 0.0), field_image("magma", Some(Origin::Lower))?)?;
    zoomed.set_xlim(0.5, 1.5)?.set_ylim(-0.8, 0.2)?;
    zoomed.set_xticks(Vec::<f64>::new());
    zoomed.set_yticks(Vec::<f64>::new());
    mark_inset(
        &ax,
        &zoomed,
        Corner::UpperLeft,
        Corner::LowerRight,
        PatchProps::default().facecolor("none").edgecolor("0.8"),
    );
    let lines = zoomed_inset_axes(
        &ax,
        0.6,
        InsetOptions {
            loc: AnchorLoc::LowerRight,
            borderpad: (1.0, 1.0),
            ..InsetOptions::default()
        },
    );
    lines.plot_with(
        [0.0, 1.0, 2.0],
        [1.0, 0.0, 1.0],
        LineProps::default().color("w"),
    )?;
    lines.set_facecolor("0.2");
    Ok(())
}

pub(super) fn rgb(fig: &Figure) -> CaseResult {
    let axes = RgbAxes::new(fig, [0.1, 0.1, 0.8, 0.8], 0.0)?;
    let x = linspace(-1.0, 1.0, 24);
    let r = Grid::from_fn(24, 24, |i, j| {
        (1.0 - x[j] * x[j] - x[i] * x[i]).clamp(0.0, 1.0)
    });
    let g = Grid::from_fn(24, 24, |_, j| f64::midpoint(x[j], 1.0));
    let b = Grid::from_fn(24, 24, |i, _| f64::midpoint(x[i], 1.0));
    axes.imshow_rgb(&r, &g, &b, || ImshowOptions {
        origin: Some(Origin::Lower),
        ..ImshowOptions::default()
    })?;
    Ok(())
}

pub(super) fn axisline_styles(fig: &Figure) -> CaseResult {
    let ax = axisartist::Axes::zero(fig.subplot());
    let xzero = ax.axis("xzero").ok_or("no xzero axis")?;
    xzero.set_axisline_style(Some(AxislineStyle::SimpleArrow { size: 1.5 }));
    let yzero = ax.axis("yzero").ok_or("no yzero axis")?;
    yzero.set_axisline_style(Some(AxislineStyle::FilledArrow {
        size: 2.0,
        facecolor: None,
    }));
    for name in ["xzero", "yzero"] {
        ax.axis(name).ok_or("no such axis")?.set_visible(true);
    }
    for name in ["left", "right", "bottom", "top"] {
        ax.axis(name).ok_or("no such axis")?.set_visible(false);
    }
    let x = linspace(-1.0, 1.0, 50);
    let y: Vec<f64> = x.iter().map(|x| x.powi(3) - 0.5 * x).collect();
    ax.ax().plot(x, y)?;
    ax.ax().set_xlim(-1.2, 1.2)?.set_ylim(-0.8, 0.8)?;
    Ok(())
}
