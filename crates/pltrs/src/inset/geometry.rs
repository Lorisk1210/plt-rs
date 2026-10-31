//! Where an inset indicator's parts go: its rectangle following the view
//! limits of the inset it indicates, its connectors' ends (matplotlib's
//! `InsetIndicator._update_connectors`), and which connectors are drawn.

use pltrs_core::{Bbox, Point};

use super::artist::IndicatorArtist;
use crate::artist::{Artist, ArtistId, LayoutContext};
use crate::axes::AxesState;
use crate::coords::{CoordOrigin, CoordUnit, Coords};
use crate::figure::FigureState;
use crate::patches::{ConnectionEnd, ConnectionPatch, PatchArtist, Rectangle};
use crate::transforms::Transform;

/// The corners of the inset axes the connectors start from, in its axes
/// fractions: lower left, upper left, lower right, upper right.
pub(super) const INSET_CORNERS: [(f64, f64); 4] = [(0.0, 0.0), (0.0, 1.0), (1.0, 0.0), (1.0, 1.0)];

/// The rectangle `[x, y, width, height]` of `rectangle`, in its own
/// coordinates.
pub(super) fn bounds(rectangle: &PatchArtist<Rectangle>) -> [f64; 4] {
    let shape = &rectangle.shape;
    [shape.xy.x, shape.xy.y, shape.width, shape.height]
}

/// The transform of the rectangle's coordinates (matplotlib's
/// `rectangle.get_data_transform()`).
pub(super) fn data_transform(rectangle: &PatchArtist<Rectangle>) -> Transform {
    rectangle
        .base()
        .transform
        .clone()
        .unwrap_or(Transform::Display)
}

/// The rectangle `[x, y, width, height]` as matplotlib measures it to
/// choose the connectors: `Bbox.from_bounds(x, y, width, height)` mapped by
/// the rectangle's `get_transform()`, whose patch transform maps the unit
/// square onto the bounds once more, so the corners compared are
/// `(x + x·width, y + y·height)` and `(x + (x + width)·width, y + (y +
/// height)·height)` in data coordinates. Not normalised, like matplotlib's
/// `Bbox.transformed`: on inverted axes the first corner may lie right of
/// or above the second.
fn measured_rectangle(
    layout: LayoutContext<'_>,
    rectangle: &PatchArtist<Rectangle>,
) -> Option<Bbox> {
    let transform = layout.resolve(&data_transform(rectangle))?;
    let [x, y, width, height] = bounds(rectangle);
    let corner = |(cx, cy): (f64, f64)| {
        transform.transform_point(Point::new(x + cx * width, y + cy * height))
    };
    Some(Bbox::new(corner((x, y)), corner((x + width, y + height))))
}

/// Which of the connectors to the inset's lower-left, upper-left,
/// lower-right and upper-right corners are drawn, so that they do not
/// cross the indicated rectangle (both boxes in display pixels).
pub(super) fn visible_connectors(rect: &Bbox, inset: &Bbox) -> [bool; 4] {
    let x0 = rect.x0() < inset.x0();
    let x1 = rect.x1() < inset.x1();
    let y0 = rect.y0() < inset.y0();
    let y1 = rect.y1() < inset.y1();
    [x0 ^ y0, x0 == y1, x1 == y0, x1 ^ y1]
}

impl IndicatorArtist {
    /// The connectors matplotlib would make visible for the indicator as
    /// laid out in `layout`; `None` without an inset or a layout.
    pub(super) fn automatic_visibility(&self, layout: LayoutContext<'_>) -> Option<[bool; 4]> {
        let inset_box = layout.axes_extent(self.inset?)?;
        let rectangle = layout
            .store()
            .get_as::<PatchArtist<Rectangle>>(self.rectangle)?;
        Some(visible_connectors(
            &measured_rectangle(layout, rectangle)?,
            &inset_box,
        ))
    }
}

/// What an update gives one indicator: its rectangle's bounds when it
/// follows the inset, and its connectors' ends.
struct Placement {
    bounds: Option<[f64; 4]>,
    ends: Vec<(ArtistId, ConnectionEnd, ConnectionEnd)>,
}

/// The placement of the indicator `id` in `state` as laid out now.
fn placement(state: &FigureState, id: ArtistId) -> Option<Placement> {
    let store = &state.store;
    let indicator = store.get_as::<IndicatorArtist>(id)?;
    let rectangle = store.get_as::<PatchArtist<Rectangle>>(indicator.rectangle)?;
    let inset = indicator.inset;
    let view = |axes: ArtistId| store.get_as::<AxesState>(axes).map(|axes| axes.view_lim);
    let bounds = if indicator.follows_inset {
        let limits = view(inset?)?;
        Some([limits.x0(), limits.y0(), limits.width(), limits.height()])
    } else {
        None
    };
    let [x, y, width, height] = bounds.unwrap_or_else(|| self::bounds(rectangle));
    let axes = indicator.base().parent;
    let (x_inverted, y_inverted) = axes.and_then(view).map_or((false, false), |limits| {
        (limits.x1() < limits.x0(), limits.y1() < limits.y0())
    });
    let coords = Coords::Transform(data_transform(rectangle));
    let inset_fraction = Coords::Relative {
        origin: CoordOrigin::Axes,
        unit: CoordUnit::Fraction,
    };
    let ends = indicator
        .connectors
        .iter()
        .zip(INSET_CORNERS)
        .map(|(&connector, corner)| {
            let ex = if x_inverted { 1.0 - corner.0 } else { corner.0 };
            let ey = if y_inverted { 1.0 - corner.1 } else { corner.1 };
            (
                connector,
                ConnectionEnd::new(corner, inset_fraction.clone(), inset),
                ConnectionEnd::new((x + ex * width, y + ey * height), coords.clone(), axes),
            )
        })
        .collect();
    Some(Placement { bounds, ends })
}

/// Places the parts of every inset indicator of `state` as it is laid out
/// now: the rectangle of one following its inset takes the inset's view
/// limits, and the connectors join the inset's corners to the rectangle's.
pub(crate) fn place_indicators(state: &mut FigureState) {
    let indicators: Vec<ArtistId> = state
        .store
        .ids()
        .filter(|&id| state.store.get_as::<IndicatorArtist>(id).is_some())
        .collect();
    for id in indicators {
        let Some(placement) = placement(state, id) else {
            continue;
        };
        let store = &mut state.store;
        if let Some([x, y, width, height]) = placement.bounds {
            let rectangle = store
                .get_as::<IndicatorArtist>(id)
                .map(|indicator| indicator.rectangle);
            if let Some(shape) = rectangle
                .and_then(|rectangle| store.get_as_mut::<PatchArtist<Rectangle>>(rectangle))
                .map(|rectangle| &mut rectangle.shape)
            {
                shape.xy = Point::new(x, y);
                shape.width = width;
                shape.height = height;
            }
        }
        for (connector, a, b) in placement.ends {
            if let Some(patch) = store.get_as_mut::<PatchArtist<ConnectionPatch>>(connector) {
                patch.shape.a = a;
                patch.shape.b = b;
            }
        }
    }
}

/// Chooses which connectors of the indicators of `state` that have not
/// chosen yet are visible, as matplotlib does when it makes them (at the
/// first draw, or when the program first asks for them).
pub(crate) fn choose_connectors(state: &mut FigureState) {
    let layout = LayoutContext::new(state);
    let choices: Vec<(ArtistId, [bool; 4])> = state
        .store
        .ids()
        .filter_map(|id| {
            let indicator = state.store.get_as::<IndicatorArtist>(id)?;
            if indicator.chosen {
                return None;
            }
            Some((id, indicator.automatic_visibility(layout)?))
        })
        .collect();
    for (id, visibility) in choices {
        let Some(indicator) = state.store.get_as_mut::<IndicatorArtist>(id) else {
            continue;
        };
        indicator.chosen = true;
        let connectors = indicator.connectors.clone();
        for (connector, visible) in connectors.into_iter().zip(visibility) {
            if let Some(artist) = state.store.get_mut(connector) {
                artist.base_mut().visible = visible;
            }
        }
    }
}
