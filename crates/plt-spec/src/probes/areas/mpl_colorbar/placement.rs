//! Where colorbars go: the parent's anchor, the placement and shape of
//! colorbar axes, what removing a colorbar gives back, and the offset
//! text and title above the extensions.

use plt::artist::ArtistHandle;
use plt::axes::{Axes, Mesh, PcolorOptions, TitleLocation};
use plt::collections::Collection;
use plt::colorbar::{ColorbarAxes, ColorbarOptions, ParentAnchor};
use plt::colors::{Extend, ScalarMappable, colormaps};
use plt::contour::{ContourOptions, Levels, QuadField};
use plt::figure::Figure;
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::layout::LayoutEngine;
use plt::numeric::Grid;
use plt::orientation::Orientation;
use plt_core::Anchor;

use super::super::mpl_tests::{in_test_style, numbers};
use super::{extents, image, side};
use crate::json::Json;
use crate::probes::encode::object;
use crate::probes::{Args, Outcome, ProbeError};
use crate::rng::SpecRng;

/// `test_colorbar_single_ax_panchor_false` and `_east`: the anchor of an
/// axes anchored `N` once a colorbar of its image took space from it.
pub(super) fn parent_anchor(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        if let Some(layout) = args.opt_str("layout")? {
            fig.set_layout_engine(
                layout
                    .parse::<LayoutEngine>()
                    .map_err(ProbeError::rejected)?,
            );
        }
        let ax = fig.subplot();
        ax.set_anchor(Anchor::N);
        let image = image(&ax, vec![vec![0.0, 1.0]])?;
        let panchor = match args.json("panchor")? {
            Json::Bool(false) => ParentAnchor::Unchanged,
            Json::String(code) => ParentAnchor::At(
                Anchor::from_code(code)
                    .ok_or_else(|| ProbeError::BadArgs(format!("bad anchor {code}")))?,
            ),
            other => return Err(ProbeError::BadArgs(format!("bad panchor {other}"))),
        };
        let options = ColorbarOptions {
            panchor,
            ..ColorbarOptions::default()
        };
        fig.colorbar(&image, options)
            .map_err(ProbeError::rejected)?;
        let (x, y) = ax.anchor().coefficients();
        Ok(numbers([x, y]))
    })
}

/// `test_anchored_cbar_position_using_specgrid`, `test_aspects` and
/// `test_inset_colorbar_layout`: where colorbar axes are placed.
pub(super) fn positions(args: Args<'_>) -> Outcome {
    in_test_style(args, || match args.str("scene")? {
        "anchored" => anchored(args),
        "aspects" => aspects(),
        "inset" => inset(),
        other => Err(ProbeError::BadArgs(format!("unknown scene {other}"))),
    })
}

/// The extents of an axes holding the colour-filled contours of
/// `arange(1200)` and of the colorbar made in its grid cell at
/// `location`, `anchor` and `shrink`, before drawing.
fn anchored(args: Args<'_>) -> Result<Json, ProbeError> {
    let fig = Figure::new();
    let ax = fig.subplot();
    let options = ContourOptions {
        levels: Levels::Values((0..=6).map(|i| f64::from(i) * 200.0).collect()),
        ..ContourOptions::default()
    };
    let ramp = Grid::from_fn(30, 40, |r, c| (r * 40 + c) as f64);
    let contours = ax
        .contourf(&QuadField::new(ramp), options)
        .map_err(ProbeError::rejected)?;
    let anchor = match args.f64_vec("anchor")?.as_slice() {
        &[x, y] => Anchor::Fraction(x, y),
        _ => return Err(ProbeError::BadArgs("an anchor has two numbers".into())),
    };
    let options = ColorbarOptions {
        axes: ColorbarAxes::Parent(ax.clone()),
        location: Some(side(args.str("location")?)?),
        anchor: Some(anchor),
        shrink: args.f64("shrink")?,
        ..ColorbarOptions::default()
    };
    let colorbar = fig
        .colorbar(&contours, options)
        .map_err(ProbeError::rejected)?;
    // matplotlib's `get_position` of an axes with a locator reads the
    // position it was last placed at: before a draw, the original one.
    Ok(Json::Array(vec![
        extents(ax.position()),
        extents(colorbar.ax().original_position()),
    ]))
}

/// The extents of `test_aspects`' six colorbar axes after a draw.
fn aspects() -> Result<Json, ProbeError> {
    let fig = Figure::with_size(8.0, 8.0, plt::rc::params().figure_dpi);
    let axs = fig.subplots(3, 2).map_err(ProbeError::rejected)?;
    let mut colorbars = Vec::new();
    for (column, orientation) in [Orientation::Vertical, Orientation::Horizontal]
        .into_iter()
        .enumerate()
    {
        for (row, (aspect, extend)) in [
            (20.0, Extend::Neither),
            (20.0, Extend::Both),
            (10.0, Extend::Both),
        ]
        .into_iter()
        .enumerate()
        {
            let ax = &axs[(row, column)];
            let mesh = ax
                .pcolormesh(
                    Mesh::new(Grid::from_fn(10, 10, |r, c| (r * 10 + c) as f64)),
                    PcolorOptions::default(),
                )
                .map_err(ProbeError::rejected)?;
            let options = ColorbarOptions {
                orientation: Some(orientation),
                aspect,
                extend: Some(extend),
                ..ColorbarOptions::beside(ax)
            };
            colorbars.push(fig.colorbar(&mesh, options).map_err(ProbeError::rejected)?);
        }
    }
    fig.layout();
    Ok(Json::Array(
        colorbars
            .iter()
            .map(|colorbar| extents(colorbar.ax().position()))
            .collect(),
    ))
}

/// The bounds of a colorbar drawn into an inset axes of a constrained
/// figure, after a draw, and whether the inset is still a child of its
/// axes.
fn inset() -> Result<Json, ProbeError> {
    let fig = Figure::with_size(3.0, 6.0, plt::rc::params().figure_dpi);
    fig.set_layout_engine(LayoutEngine::constrained());
    let ax = fig.subplot();
    let image = image(
        &ax,
        (0..10)
            .map(|r| (0..10).map(|c| f64::from(r * 10 + c)).collect())
            .collect(),
    )?;
    let cax = ax.inset_axes([1.02, 0.1, 0.03, 0.8]);
    let options = ColorbarOptions {
        axes: ColorbarAxes::Cax(cax),
        ..ColorbarOptions::default()
    };
    let colorbar = fig
        .colorbar(&image, options)
        .map_err(ProbeError::rejected)?;
    fig.layout();
    let (x, y, width, height) = colorbar.ax().position().bounds();
    Ok(object([
        ("bounds", numbers([x, y, width, height])),
        ("child", Json::Bool(ax.child_axes().contains(colorbar.ax()))),
    ]))
}

/// A scatter of `(1, 3)` and `(2, 4)` coloured by `[5, 6]`.
fn scatter(ax: &Axes) -> Result<Collection, ProbeError> {
    let points = ax
        .scatter(vec![1.0, 2.0], vec![3.0, 4.0])
        .map_err(ProbeError::rejected)?;
    points
        .set_array(vec![Some(5.0), Some(6.0)])
        .map_err(ProbeError::rejected)?;
    Ok(points)
}

/// `test_remove_from_figure`, `test_remove_from_figure_cl` and
/// `test_remove_cb_whose_mappable_has_no_figure`.
pub(super) fn remove(args: Args<'_>) -> Outcome {
    in_test_style(args, || match args.str("scene")? {
        "no_figure" => {
            let fig = Figure::new();
            let ax = fig.subplot();
            let before = fig.axes().len();
            let mappable =
                ScalarMappable::new(colormaps::get_cmap(None).map_err(ProbeError::rejected)?);
            let options = ColorbarOptions {
                axes: ColorbarAxes::Cax(ax),
                ..ColorbarOptions::default()
            };
            fig.colorbar(&mappable, options)
                .map_err(ProbeError::rejected)?
                .remove();
            Ok(numbers([before as f64, fig.axes().len() as f64]))
        }
        "constrained" => {
            let fig = Figure::new();
            fig.set_layout_engine(LayoutEngine::constrained());
            let ax = fig.subplot();
            let points = scatter(&ax)?;
            fig.layout();
            let before = extents(ax.position());
            fig.colorbar(&points, ColorbarOptions::default())
                .map_err(ProbeError::rejected)?
                .remove();
            fig.layout();
            Ok(Json::Array(vec![before, extents(ax.position())]))
        }
        scene => remove_from_grid(scene, args.bool("use_gridspec")?),
    })
}

/// The extents of a scatter's axes (on the whole grid or on a cell of a
/// grid nested in a cell of another) before a colorbar, with it and after
/// removing it, each after `subplots_adjust()`.
fn remove_from_grid(scene: &str, use_gridspec: bool) -> Result<Json, ProbeError> {
    let fig = Figure::new();
    let ax = match scene {
        "subplot" => fig.subplot(),
        "nested" => {
            let outer = GridSpec::new(2, 2).and_then(|grid| grid.cell(1, 1));
            let cell = outer
                .and_then(|cell| GridSpec::nested(&cell, 2, 2, None, None, (None, None)))
                .and_then(|grid| grid.cell(1, 1))
                .map_err(ProbeError::rejected)?;
            fig.add_subplot_spec(&cell).map_err(ProbeError::rejected)?
        }
        other => return Err(ProbeError::BadArgs(format!("unknown scene {other}"))),
    };
    let points = scatter(&ax)?;
    let adjust = || {
        fig.subplots_adjust(&SubplotOverrides::default())
            .map(|_| extents(ax.position()))
            .map_err(ProbeError::rejected)
    };
    let mut positions = vec![extents(ax.position())];
    let options = ColorbarOptions {
        use_gridspec,
        ..ColorbarOptions::default()
    };
    let colorbar = fig
        .colorbar(&points, options)
        .map_err(ProbeError::rejected)?;
    positions.push(adjust()?);
    colorbar.remove();
    positions.push(adjust()?);
    Ok(Json::Array(positions))
}

/// `test_axes_remove`: the number of axes with a colorbar and after
/// removing the colorbar's axes, and the parent's extents before, with
/// and after.
pub(super) fn remove_axes(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        let points = ax
            .scatter(vec![1.0, 2.0], vec![3.0, 4.0])
            .map_err(ProbeError::rejected)?;
        let mut positions = vec![extents(ax.position())];
        let colorbar = fig
            .colorbar(&points, ColorbarOptions::default())
            .map_err(ProbeError::rejected)?;
        positions.push(extents(ax.position()));
        let mut counts = vec![fig.axes().len() as f64];
        colorbar.ax().remove();
        counts.push(fig.axes().len() as f64);
        positions.push(extents(ax.position()));
        Ok(object([
            ("axes", numbers(counts)),
            ("positions", Json::Array(positions)),
        ]))
    })
}

/// A mesh of `rng(19680808).normal(100)` times `scale` with a colorbar on
/// the right extended at the top, laid out.
fn extended_colorbar(scale: f64) -> Result<(Figure, Axes, plt::colorbar::Colorbar), ProbeError> {
    let fig = Figure::new();
    let ax = fig.subplot();
    let values = SpecRng::new(19_680_808).normal(100, 0.0, 1.0);
    let mesh = ax
        .pcolormesh(
            Mesh::new(Grid::from_fn(10, 10, |r, c| values[r * 10 + c] * scale)),
            PcolorOptions::default(),
        )
        .map_err(ProbeError::rejected)?;
    let options = ColorbarOptions {
        location: Some(plt::spines::SpineSide::Right),
        extend: Some(Extend::Max),
        ..ColorbarOptions::default()
    };
    let colorbar = fig.colorbar(&mesh, options).map_err(ProbeError::rejected)?;
    Ok((fig, ax, colorbar))
}

/// `test_offset_text_loc`: the extent (pixels) of the offset text of a
/// colorbar extended at the top, and the top of its parent axes.
pub(super) fn offset_text(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let (fig, ax, colorbar) = extended_colorbar(1e6)?;
        fig.layout();
        let offset = colorbar
            .long_axis()
            .offset_text_extent()
            .ok_or_else(|| ProbeError::rejected("no offset text"))?;
        let top = ax
            .window_extent()
            .ok_or_else(|| ProbeError::rejected("the axes has no extent"))?
            .y1();
        Ok(object([
            ("offset", extents(offset)),
            ("top", Json::from(top)),
        ]))
    })
}

/// `test_title_text_loc`: the tops (pixels) of the title of a colorbar
/// extended at the top and of its outline.
pub(super) fn title(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let (fig, _, colorbar) = extended_colorbar(1.0)?;
        colorbar.ax().set_title("Aardvark");
        fig.layout();
        let top = |extent: Option<plt_core::Bbox>| {
            extent
                .map(|extent| Json::from(extent.y1()))
                .ok_or_else(|| ProbeError::rejected("no extent"))
        };
        Ok(Json::Array(vec![
            top(colorbar
                .ax()
                .title_text(TitleLocation::Center)
                .window_extent())?,
            top(colorbar.outline().window_extent())?,
        ]))
    })
}
