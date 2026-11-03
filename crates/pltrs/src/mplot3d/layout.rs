//! What layout engines need from a 3-D axes: its grid cell, its original
//! position and its extent with decorations (matplotlib's
//! `get_subplotspec`, `get_position(original=True)`, `_set_position` and
//! `Axes3D.get_tightbbox`, which constrained layout calls on 3-D axes as
//! on any other).

use pltrs_core::Bbox;

use super::state::{Axes3DState, BoxFit, axes3d_fresh_tight_bbox};
use crate::artist::{Artist, ArtistId, ArtistStore};
use crate::figure::FigureState;
use crate::gridspec::SubplotSpec;

/// The grid cell of the 3-D axes `id` and whether it takes part in
/// layouts; `None` when `id` is not a 3-D axes.
pub(crate) fn axes3d_placement(
    store: &ArtistStore,
    id: ArtistId,
) -> Option<(Option<SubplotSpec>, bool)> {
    let axes = store.get_as::<Axes3DState>(id)?;
    Some((axes.subplot_spec.clone(), axes.base().in_layout))
}

/// The original position of the 3-D axes `id` in the fractions of its
/// frame, and its extent with decorations as layouts measure it, in
/// display pixels, once its ticks are worked out for that position
/// (`None` when it is hidden); `None` when `id` is not a 3-D axes.
pub(crate) fn axes3d_layout_extents(
    figure: &mut FigureState,
    id: ArtistId,
) -> Option<(Bbox, Option<Bbox>)> {
    let position = figure.store.get_as::<Axes3DState>(id)?.position;
    Some((position, axes3d_fresh_tight_bbox(figure, id, true)))
}

/// Moves the 3-D axes `id` to `position` in the fractions of its frame;
/// whether `id` is a 3-D axes.
pub(crate) fn set_axes3d_position(figure: &mut FigureState, id: ArtistId, position: Bbox) -> bool {
    let Some(axes) = figure.store.get_as_mut::<Axes3DState>(id) else {
        return false;
    };
    axes.position = position;
    axes.fit = BoxFit::Placed;
    true
}

#[cfg(test)]
mod tests {
    use crate::figure::Figure;
    use crate::gridspec::GridSpec;
    use crate::layout::LayoutEngine;
    use crate::mplot3d::{Axes3D, Axes3DOptions};

    /// The 3-D axes of `fig.subplots(nrows, ncols, subplot_kw={"projection":
    /// "3d"})` on a constrained figure `width × height` inches large, under
    /// the spec's reference configuration (no text hinting).
    fn constrained_grid(
        width: f64,
        height: f64,
        nrows: usize,
        ncols: usize,
    ) -> (Figure, Vec<Axes3D>) {
        crate::rc::set("text.hinting", "none").expect("valid setting");
        let fig = Figure::with_size(width, height, 100.0);
        fig.set_layout_engine(LayoutEngine::constrained()).unwrap();
        let grid = GridSpec::new(nrows, ncols).unwrap();
        let axs = (0..nrows * ncols)
            .map(|i| {
                let cell = grid.cell(i / ncols, i % ncols).unwrap();
                fig.add_subplot_spec_3d(&cell, Axes3DOptions::default())
                    .unwrap()
            })
            .collect();
        (fig, axs)
    }

    /// Checks the extents of the positions of `axs` against matplotlib's.
    fn assert_positions(axs: &[&Axes3D], expected: &[[f64; 4]]) {
        for (ax, expected) in axs.iter().zip(expected) {
            let position = ax.position();
            let actual = [position.x0(), position.y0(), position.x1(), position.y1()];
            for (a, e) in actual.into_iter().zip(expected) {
                assert!((a - e).abs() < 2e-5, "{actual:?} vs {expected:?}");
            }
        }
    }

    #[test]
    fn constrained_layout_moves_3d_axes_into_their_cells() {
        crate::rc::context(|| {
            let (_fig, axs) = constrained_grid(8.0, 6.0, 2, 3);
            for ax in &axs {
                ax.set_title("orientation=x");
            }
            // matplotlib 3.11 under the spec's reference configuration.
            let expected = [
                [0.009_03, 0.560_26, 0.293_64, 0.939_74],
                [0.342_36, 0.560_26, 0.626_97, 0.939_74],
                [0.675_69, 0.060_26, 0.960_30, 0.439_74],
            ];
            assert_positions(&[&axs[0], &axs[1], &axs[5]], &expected);
        });
    }

    #[test]
    fn layouts_leave_axis_labels_and_title_widths_out() {
        crate::rc::context(|| {
            let (_fig, axs) = constrained_grid(6.4, 4.8, 1, 2);
            for ax in &axs {
                ax.set_xlabel("x label").set_zlabel("a rather long z label");
            }
            axs[0].set_title("a title much wider than the axes it sits above, really");
            let expected = [
                [0.006_511, 0.200_883, 0.455_186, 0.799_117],
                [0.506_511, 0.200_883, 0.955_186, 0.799_117],
            ];
            assert_positions(&[&axs[0], &axs[1]], &expected);
        });
    }

    #[test]
    fn constrained_layout_puts_colorbars_of_3d_axes_beside_them() {
        use crate::colorbar::ColorbarOptions;
        use crate::colors::{Colorize, ColorizerOptions, colormaps};
        use crate::mplot3d::SurfaceOptions;
        use crate::numeric::{Grid, linspace, meshgrid};

        crate::rc::context(|| {
            let (fig, axs) = constrained_grid(6.4, 4.8, 1, 1);
            let (x, y) = meshgrid(&linspace(-1.0, 1.0, 5), &linspace(-1.0, 1.0, 5));
            let z = Grid::from_fn(5, 5, |i, j| x[(i, j)] * y[(i, j)]);
            let options = SurfaceOptions {
                colors: ColorizerOptions::default()
                    .cmap(colormaps::get("viridis").unwrap())
                    .into(),
                ..SurfaceOptions::default()
            };
            let surface = axs[0].plot_surface(&x, &y, &z, options).unwrap();
            let colorbar = ColorbarOptions {
                shrink: 0.6,
                ..ColorbarOptions::default()
            };
            let colorbar = fig.colorbar(&surface, colorbar).unwrap();
            // matplotlib 3.11 under the spec's reference configuration.
            assert_positions(&[&axs[0]], &[[0.094_723, 0.013_034, 0.828_436, 0.991_319]]);
            let cax = colorbar.ax().position();
            let expected = [0.901_911, 0.208_691, 0.923_922, 0.795_662];
            let actual = [cax.x0(), cax.y0(), cax.x1(), cax.y1()];
            for (a, e) in actual.into_iter().zip(expected) {
                assert!((a - e).abs() < 2e-5, "{actual:?} vs {expected:?}");
            }
        });
    }
}
