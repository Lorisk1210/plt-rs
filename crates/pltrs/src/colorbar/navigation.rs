//! Navigating a colorbar: zooming and panning its axes change the limits
//! of the norm it shows, which recolours its mappable, rather than the
//! limits of the axes (matplotlib's interactive colorbar: the `_get_view`,
//! `_set_view`, `_set_view_from_bbox` and `drag_pan` that `Colorbar`
//! installs on its axes).

use super::Colorbar;
use super::mappable::Source;
use super::state::ColorbarArtist;
use crate::artist::DrawError;
use crate::axes::Axes;
use crate::colors::NormKind;

impl Colorbar {
    /// The colorbar drawn in `ax`, if `ax` is a colorbar's axes
    /// (matplotlib's `ax._colorbar`).
    pub(crate) fn in_axes(ax: &Axes) -> Option<Self> {
        let raw = ax.children_of::<ColorbarArtist>().into_iter().next()?;
        Self::from_raw(raw)
    }

    /// Whether the navigation toolbar may change the colorbar's norm: not
    /// for a `BoundaryNorm` or a `NoNorm`, nor for the levels of a contour
    /// set (matplotlib turns navigation off on their axes).
    pub(crate) fn navigable(&self) -> bool {
        self.read(|state| {
            state.contours.is_none()
                && !matches!(
                    state.mapping.norm().kind(),
                    NormKind::Boundary(_) | NormKind::Identity
                )
        })
    }

    /// The norm limits a view of the colorbar consists of, once they are
    /// known (matplotlib's `Colorbar._get_view`).
    pub(crate) fn view(&self) -> Option<(f64, f64)> {
        self.read(|state| {
            let norm = state.mapping.norm();
            Some((norm.vmin()?, norm.vmax()?))
        })
    }

    /// Makes `(vmin, vmax)` the limits of the norm shown, recolouring the
    /// mappable followed and moving the long axis with them (matplotlib's
    /// `Colorbar._set_view`, which sets the norm it shares with its
    /// mappable). Problems are reported with the next draw.
    pub(crate) fn set_view(&self, (vmin, vmax): (f64, f64)) {
        let figure = self.raw.figure_state();
        if let Some(link) = self.read(|state| state.link) {
            link.share_limits(figure, vmin, vmax);
            return;
        }
        let redrawn = self.edit(|state| {
            let mut mapping = state.mapping.clone();
            mapping.set_clim(Some(vmin), Some(vmax));
            let source = Source {
                mapping,
                alpha: state.alpha,
                contours: state.contours.clone(),
                data: None,
            };
            state.follow(&self.ax, source, false)
        });
        if let Err(err) = redrawn {
            figure
                .borrow_mut()
                .pending_errors
                .push(DrawError::new(self.raw.id(), err.to_string()));
        }
    }
}
