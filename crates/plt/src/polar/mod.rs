//! Polar axes (matplotlib's `projections.polar`): data given as angle θ
//! (radians, the x axis) and radius r (the y axis) drawn inside a circle
//! or an annular sector.
//!
//! * [`PolarAxes`] — the handle returned by
//!   [`Figure::polar_subplot`](crate::figure::Figure::polar_subplot); it
//!   dereferences to [`Axes`] for plotting and adds the polar settings
//!   (θ orientation and limits, radial origin, label positions, ...).
//! * [`ThetaFormatter`] — the θ axis' labels in degrees, for unit
//!   converters of angles to give polar axes.
//! * The projection itself (`PolarTransform`, `PolarAffine`,
//!   `_WedgeBbox`), the angular and radial locators, and the painting of
//!   the circular grid live in private submodules.

mod api;
mod create;
mod paint;
mod ticker;
mod transform;

use std::f64::consts::PI;

pub use api::{PolarAxes, PolarError};
pub(crate) use paint::{GRIDLINE_INTERPOLATION_STEPS, draw_tick_lines, tick_label_anchor};
pub use ticker::ThetaFormatter;
pub(crate) use ticker::{RadialLocator, ThetaLocator, theta_tickers};
pub(crate) use transform::{PolarMap, PolarView};

use crate::artist::ArtistId;
use crate::axes::{Axes, AxesState};
use crate::figure::FigureState;
use crate::spines::SpineSide;

/// The way θ increases (matplotlib's `set_theta_direction`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ThetaDirection {
    /// Anticlockwise, matplotlib's default (`1`).
    #[default]
    Counterclockwise,
    /// Clockwise (`-1`).
    Clockwise,
}

impl ThetaDirection {
    /// `1` or `-1`, the factor applied to θ.
    pub const fn sign(self) -> f64 {
        match self {
            Self::Counterclockwise => 1.0,
            Self::Clockwise => -1.0,
        }
    }
}

/// Compass points for the location of θ = 0 (matplotlib's
/// `set_theta_zero_location`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Compass {
    /// North (up).
    N,
    /// North-west.
    NW,
    /// West (left).
    W,
    /// South-west.
    SW,
    /// South (down).
    S,
    /// South-east.
    SE,
    /// East (right), matplotlib's default.
    E,
    /// North-east.
    NE,
}

impl Compass {
    /// The direction in radians, anticlockwise from east.
    pub fn angle(self) -> f64 {
        match self {
            Self::N => PI * 0.5,
            Self::NW => PI * 0.75,
            Self::W => PI,
            Self::SW => PI * 1.25,
            Self::S => PI * 1.5,
            Self::SE => PI * 1.75,
            Self::E => 0.0,
            Self::NE => PI * 0.25,
        }
    }
}

/// The outline of polar axes: their background, the clip path of what
/// they draw and the outer spine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PolarFrame {
    /// A circle (or the wedge of a sector), matplotlib's `PolarAxes`.
    #[default]
    Circle,
    /// A regular polygon with this many vertices, the first one at the
    /// top of the axes, as in matplotlib's radar chart example: the
    /// gridlines follow it too, with one straight piece between
    /// consecutive vertex directions.
    Polygon(usize),
}

/// The polar settings of an axes besides its view limits.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PolarState {
    /// Where θ = 0 points, in radians anticlockwise from east.
    pub(crate) theta_offset: f64,
    /// The way θ increases.
    pub(crate) direction: ThetaDirection,
    /// The radius drawn at the centre; `None` follows the lower radial
    /// limit (matplotlib's locked `_originViewLim.y0`).
    pub(crate) rorigin: Option<f64>,
    /// The angle of the radial tick labels in radians.
    pub(crate) rlabel_position: f64,
    /// The outline of the axes.
    pub(crate) frame: PolarFrame,
}

impl Default for PolarState {
    /// matplotlib's defaults: θ = 0 east, anticlockwise, radial labels
    /// at 22.5°.
    fn default() -> Self {
        Self {
            theta_offset: 0.0,
            direction: ThetaDirection::Counterclockwise,
            rorigin: None,
            rlabel_position: 22.5_f64.to_radians(),
            frame: PolarFrame::Circle,
        }
    }
}

impl Axes {
    /// This axes as polar axes, if it is one.
    pub fn as_polar(&self) -> Option<PolarAxes> {
        PolarAxes::new(self.clone())
    }
}

/// Shows the start and end spines of a sector and the inner spine around
/// a hole, and hides them otherwise, whatever their visibility was
/// (matplotlib's `PolarAxes.draw` sets it before every draw).
pub(crate) fn update_spine_visibility(state: &mut FigureState, axes: ArtistId) {
    let Some(axes_state) = state.store.get_as::<AxesState>(axes) else {
        return;
    };
    let Some(view) = axes_state.polar_view() else {
        return;
    };
    let updates: Vec<(ArtistId, bool)> = SpineSide::ALL
        .iter()
        .zip(&axes_state.spines)
        .filter(|(side, _)| **side != SpineSide::Top)
        .map(|(&side, &id)| (id, view.spine_shown(side)))
        .collect();
    for (id, visible) in updates {
        if let Some(spine) = state.store.get_mut(id) {
            spine.base_mut().visible = visible;
        }
    }
}
