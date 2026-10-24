//! Removing a colorbar (matplotlib's `Colorbar.remove`, which removing
//! the colorbar's axes runs as well).

use super::placement::place;
use super::{Colorbar, colorbar_in};
use crate::artist::ArtistHandle;
use crate::axes::{Axes, AxesState};

impl Colorbar {
    /// Removes the colorbar and its axes from the figure (matplotlib's
    /// `remove`; removing [`Colorbar::ax`] does the same). The axes the
    /// mappable is drawn in gets back the grid cell the colorbar's grid
    /// subdivided (`use_gridspec`), or else is drawn at its original
    /// position until it is next laid out.
    ///
    /// ```
    /// use plt::axes::ImshowOptions;
    /// use plt::colorbar::ColorbarOptions;
    /// use plt::numeric::Grid;
    /// use plt::prelude::*;
    /// let fig = Figure::new();
    /// // Without a layout engine, the colorbar takes room from the grid cell.
    /// fig.set_layout_engine(None);
    /// let ax = fig.subplot();
    /// let image = ax.imshow(Grid::from_fn(2, 2, |r, c| (r + c) as f64), ImshowOptions::default())?;
    /// let colorbar = fig.colorbar(&image, ColorbarOptions::default())?;
    /// // The image's axes moved into a grid nested in its cell, ...
    /// assert!(ax.subplot_spec().is_some_and(|spec| spec.gridspec().parent().is_some()));
    /// colorbar.remove();
    /// // ... and is back in its cell.
    /// assert!(ax.subplot_spec().is_some_and(|spec| spec.gridspec().parent().is_none()));
    /// assert_eq!(fig.axes(), [ax]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn remove(&self) {
        self.ax.remove();
    }
}

/// What the removal of the axes `ax` does when it holds a colorbar, just
/// before `ax` goes: the layout forgets the colorbar, and the axes of its
/// mappable gets back the grid cell `ax`'s grid was nested in, or its
/// original position.
pub(crate) fn release(ax: &Axes) {
    let figure = ax.raw().figure_state();
    let link = {
        let state = figure.borrow();
        let Some(colorbar) = colorbar_in(&state.store, ax.id()) else {
            return;
        };
        colorbar.state.link
    };
    figure
        .borrow_mut()
        .layout
        .colorbars
        .retain(|colorbar| colorbar.cax != ax.id());
    let Some(host) = link.and_then(|link| link.host_axes(figure)) else {
        return;
    };
    let cell = ax
        .subplot_spec()
        .and_then(|spec| spec.gridspec().parent().cloned());
    let position = cell
        .as_ref()
        .and_then(|cell| cell.position(&host.figure().subplot_params()).ok());
    match (cell, position) {
        (Some(cell), Some(position)) => place(&host, position, Some(cell)),
        _ => host.raw().with_mut::<AxesState, _>(|axes| {
            axes.aspect.set_active(axes.position, axes.position);
        }),
    }
}

#[cfg(test)]
mod tests {
    use plt_core::Bbox;

    use super::*;
    use crate::axes::{ScatterColors, ScatterOptions};
    use crate::colorbar::{ColorbarAxes, ColorbarOptions};
    use crate::colors::{ScalarMappable, colormaps};
    use crate::figure::Figure;
    use crate::gridspec::{GridSpec, SubplotOverrides};
    use crate::layout::LayoutEngine;

    /// A scatter coloured by value in `ax`.
    fn scatter(ax: &Axes) -> crate::collections::Collection {
        let options = ScatterOptions {
            colors: ScatterColors::from(crate::axes::MappedColors::new(vec![5.0, 6.0])),
            ..ScatterOptions::default()
        };
        ax.scatter_with(vec![1.0, 2.0], vec![3.0, 4.0], options)
            .expect("a scatter")
    }

    fn assert_close(actual: Bbox, expected: Bbox) {
        let (a, e) = (actual.extents(), expected.extents());
        let close = |x: f64, y: f64| (x - y).abs() < 1e-12;
        assert!(
            close(a.0, e.0) && close(a.1, e.1) && close(a.2, e.2) && close(a.3, e.3),
            "{a:?} != {e:?}"
        );
    }

    #[test]
    fn removing_restores_the_nested_cell() {
        // matplotlib's test_remove_from_figure with nested grids.
        let fig = Figure::new();
        let outer = GridSpec::new(2, 2).unwrap();
        let inner =
            GridSpec::nested(&outer.cell(1, 1).unwrap(), 2, 2, None, None, (None, None)).unwrap();
        let ax = fig.add_subplot_spec(&inner.cell(1, 1).unwrap()).unwrap();
        let points = scatter(&ax);
        let before = ax.position();
        let colorbar = fig.colorbar(&points, ColorbarOptions::default()).unwrap();
        fig.subplots_adjust(&SubplotOverrides::default()).unwrap();
        assert!(ax.position().x1() < before.x1());
        colorbar.remove();
        fig.subplots_adjust(&SubplotOverrides::default()).unwrap();
        assert_close(ax.position(), before);
        assert_eq!(fig.axes(), [ax]);
    }

    #[test]
    fn removing_the_axes_removes_the_colorbar() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let points = scatter(&ax);
        let before = ax.position();
        let colorbar = fig.colorbar(&points, ColorbarOptions::default()).unwrap();
        colorbar.ax().remove();
        assert_close(ax.position(), before);
        assert_eq!(fig.raw().figure_state().borrow().layout.colorbars, []);
        // The scatter recolours without a colorbar to follow.
        points.set_clim(Some(0.0), Some(1.0));
        assert!(fig.render_raster().is_ok());
    }

    #[test]
    fn constrained_layouts_take_the_room_back() {
        // matplotlib's test_remove_from_figure_cl.
        let fig = Figure::new();
        fig.set_layout_engine(LayoutEngine::constrained());
        let ax = fig.subplot();
        let points = scatter(&ax);
        fig.layout();
        let before = ax.position();
        fig.colorbar(&points, ColorbarOptions::default())
            .unwrap()
            .remove();
        fig.layout();
        assert_close(ax.position(), before);
    }

    #[test]
    fn bare_mappables_leave_no_axes_to_restore() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let mappable = ScalarMappable::new(colormaps::get("viridis").unwrap());
        let options = ColorbarOptions {
            axes: ColorbarAxes::Cax(ax),
            ..ColorbarOptions::default()
        };
        fig.colorbar(&mappable, options).unwrap().remove();
        assert_eq!(fig.axes(), []);
    }
}
