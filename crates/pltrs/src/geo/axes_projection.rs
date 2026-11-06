//! Geographic axes as a [`Projection`]: the map inside an ellipse or a
//! circle with fixed limits and linear scales, gridlines every so many
//! degrees, and the set-up of fresh axes (matplotlib's `GeoAxes` and its
//! `clear`).

use std::error::Error;
use std::f64::consts::{FRAC_PI_2, PI};

use super::api::{format_lonlat, install_grid};
use super::transform::outline;
use super::{
    DEFAULT_LATITUDE_GRID, DEFAULT_LONGITUDE_GRID, DEFAULT_LONGITUDE_GRID_ENDS, GeoProjection,
    GeoState, LATITUDE_EDGE, LONGITUDE_EDGE,
};
use crate::artist::ArtistHandle;
use crate::axes::{Adjustable, Axes, TickParams, TickPosition};
use crate::projections::{AxesOutline, Projection, ProjectionView, TickLabelPlacement};
use crate::rc::Aspect;
use crate::spines::SpineSide;
use crate::ticker::{AxisName, NullFormatter, NullLocator};
use crate::transforms::{Anchor, ResolvedTransform};

/// The aspect of the elliptical maps: a full turn of longitude is twice
/// as wide as a half turn of latitude is high.
const ELLIPSE_ASPECT: f64 = 0.5;

impl Projection for GeoState {
    fn name(&self) -> &'static str {
        self.projection.kind().name()
    }

    fn data_to_axes(&self, _: &ProjectionView<'_>) -> ResolvedTransform {
        self.to_axes()
    }

    /// matplotlib's `GeoAxes.clear` and the aspect its subclasses set.
    fn set_up(&mut self, axes: &Axes) -> Result<(), Box<dyn Error>> {
        self.longitude_cap = DEFAULT_LONGITUDE_GRID_ENDS.to_radians();
        axes.set_xlim(-PI, PI)?.set_ylim(-FRAC_PI_2, FRAC_PI_2)?;
        for spine in axes.spines().iter() {
            // matplotlib's circular spine ignores `axes.spines.*`.
            spine.raw().with_base_mut(|base| base.visible = true);
        }
        install_grid(&axes.xaxis(), DEFAULT_LONGITUDE_GRID, LONGITUDE_EDGE);
        install_grid(&axes.yaxis(), DEFAULT_LATITUDE_GRID, LATITUDE_EDGE);
        for axis in [axes.xaxis(), axes.yaxis()] {
            axis.set_minor_locator(NullLocator);
            axis.set_ticks_position(TickPosition::None)?;
        }
        axes.yaxis().set_tick_params(TickParams {
            labelleft: Some(true),
            ..TickParams::default()
        });
        let lambert = matches!(self.projection, GeoProjection::Lambert { .. });
        if lambert {
            axes.yaxis().set_major_formatter(NullFormatter);
        }
        axes.grid(crate::rc::params().axes_grid);
        axes.set_aspect(if lambert {
            Aspect::Equal
        } else {
            Aspect::Ratio(ELLIPSE_ASPECT)
        })?;
        axes.store_adjustable(Adjustable::Box).set_anchor(Anchor::C);
        Ok(())
    }

    fn axis_transform(&self, _: &ProjectionView<'_>, axis: AxisName) -> Option<ResolvedTransform> {
        Some(self.gridline_transform(axis))
    }

    fn tick_label(
        &self,
        _: &ProjectionView<'_>,
        side: SpineSide,
        loc: f64,
    ) -> Option<TickLabelPlacement> {
        Some(self.label_placement(side, loc))
    }

    fn outline(&self) -> AxesOutline {
        AxesOutline::Path(outline())
    }

    fn fixed_limits(&self) -> bool {
        true
    }

    fn linear_only(&self) -> bool {
        true
    }

    fn data_ratio(&self, _: &ProjectionView<'_>) -> Option<f64> {
        Some(1.0)
    }

    fn fixes_data_ratio(&self) -> bool {
        true
    }

    fn can_zoom(&self) -> bool {
        false
    }

    fn can_pan(&self) -> bool {
        false
    }

    fn format_coord(&self, x: f64, y: f64) -> Option<String> {
        Some(format_lonlat(x, y))
    }
}
