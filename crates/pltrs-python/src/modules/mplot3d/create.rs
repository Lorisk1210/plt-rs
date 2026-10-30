//! Making 3-D axes in a figure (`add_subplot(projection='3d')`,
//! `add_axes(rect, projection='3d')`) and listing them with the figure's
//! other axes (`fig.axes`).

use pltrs::artist::{ArtistHandle, RawHandle};
use pltrs::axes::Axes;
use pltrs::figure::Figure;
use pltrs::gridspec::SubplotSpec;
use pltrs::mplot3d::{Axes3D, Axes3DOptions, ProjType, ViewAngles};

use super::axis3;
use crate::binding::prelude::*;

/// The camera of a new 3-D axes: `(elev, azim, roll)`, the vertical axis,
/// the projection (`"persp"` or `"ortho"`) and the perspective focal
/// length (`None`: 1).
type Camera = ((f64, f64, f64), String, String, Option<f64>);

/// The axes a new 3-D axes shares its x, y and z axes and its view with.
type Shares = (
    Option<Axes3D>,
    Option<Axes3D>,
    Option<Axes3D>,
    Option<Axes3D>,
);

/// The options of a new 3-D axes: its camera, box aspect, whether
/// collections are drawn in depth order, and the axes it shares with
/// (matplotlib's `Axes3D` keyword arguments, checked in Python).
pub(super) struct Options(Axes3DOptions);

impl FromPy<'_> for Options {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let (camera, box_aspect, computed_zorder, shares): (
            Camera,
            Option<(f64, f64, f64)>,
            bool,
            Shares,
        ) = object.extract()?;
        let ((elev, azim, roll), vertical, proj_type, focal_length) = camera;
        let (sharex, sharey, sharez, shareview) = shares;
        Ok(Self(Axes3DOptions {
            view: ViewAngles { elev, azim, roll },
            vertical_axis: axis3(&vertical)?,
            proj_type: proj_type_of(&proj_type, focal_length)?,
            box_aspect: box_aspect.map(|(x, y, z)| [x, y, z]),
            computed_zorder,
            sharex,
            sharey,
            sharez,
            shareview,
        }))
    }
}

/// The projection called `name` with `focal_length` (`None`: 1).
pub(super) fn proj_type_of(name: &str, focal_length: Option<f64>) -> PyResult<ProjType> {
    match name {
        "persp" => Ok(ProjType::Persp {
            focal_length: focal_length.unwrap_or(1.0),
        }),
        "ortho" => Ok(ProjType::Ortho),
        _ => Err(PyErr::value_error(format!(
            "{name:?} is not a valid value for proj_type; supported values are 'persp', 'ortho'"
        ))),
    }
}

/// A 3-D axes on the grid block `spec` (`add_subplot(...,
/// projection='3d')`).
fn add_subplot(figure: Figure, spec: SubplotSpec, options: Options) -> PyResult<Axes3D> {
    Ok(figure.add_subplot_spec_3d(&spec, options.0)?)
}

/// A 3-D axes at `rect`, `(left, bottom, width, height)` in figure
/// fractions (`add_axes(rect, projection='3d')`).
fn add_axes(figure: Figure, rect: (f64, f64, f64, f64), options: Options) -> PyResult<Axes3D> {
    let (left, bottom, width, height) = rect;
    Ok(figure.add_axes_3d([left, bottom, width, height], options.0)?)
}

/// The projection plane of the 3-D axes: the 2-D axes it is as an `Axes`.
fn plane(axes: Axes3D) -> PyResult<Axes> {
    Ok(axes.projection_plane())
}

/// Every axes of the figure, 2-D and 3-D, in the order they were made,
/// each with whether it is a 3-D axes (`fig.axes`).
fn figure_axes(figure: Figure) -> PyResult<Vec<(RawHandle, bool)>> {
    let mut axes: Vec<(RawHandle, bool)> = figure
        .axes()
        .iter()
        .map(|ax| (ax.raw().clone(), false))
        .chain(figure.axes3d().iter().map(|ax| (ax.raw().clone(), true)))
        .collect();
    axes.sort_by_key(|(raw, _)| raw.id());
    Ok(axes)
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("add_subplot", add_subplot)?;
    module.add("add_axes", add_axes)?;
    module.add("plane", plane)?;
    module.add("figure_axes", figure_axes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn figure_axes_list_2d_and_3d_axes_in_order() {
        let fig = Figure::new();
        let flat = fig.add_subplot(1, 2, 1).unwrap();
        let deep = fig
            .add_axes_3d([0.5, 0.0, 0.5, 1.0], Axes3DOptions::default())
            .unwrap();
        let axes = figure_axes(fig.clone()).unwrap();
        assert_eq!(axes.len(), 2);
        assert_eq!((axes[0].0.id(), axes[0].1), (flat.id(), false));
        assert_eq!((axes[1].0.id(), axes[1].1), (deep.id(), true));
        assert_eq!(plane(deep.clone()).unwrap(), deep.projection_plane());
        assert!(proj_type_of("fisheye", None).is_err());
        assert_eq!(proj_type_of("ortho", None).unwrap(), ProjType::Ortho);
    }
}
