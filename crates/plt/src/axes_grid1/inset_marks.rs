//! Marks joining an inset axes to the area of its parent it shows: an
//! outline of the area and lines between corners (matplotlib's
//! `BboxPatch`, `BboxConnector` and `mark_inset`).

use plt_core::{Bbox, Path, Point};

use crate::artist::{ArtistBase, ArtistHandle, ArtistId, LayoutContext};
use crate::axes::{Axes, AxesState};
use crate::offsetbox::transformed_bbox;
use crate::patches::{
    DisplayPath, Patch, PatchError, PatchProps, PatchShape, PatchStyle, ShapeFrame,
};
use crate::transforms::Transform;

/// A corner of a box (matplotlib's codes 1 to 4 of `mark_inset`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Corner {
    /// `1`.
    UpperRight,
    /// `2`.
    UpperLeft,
    /// `3`.
    LowerLeft,
    /// `4`.
    LowerRight,
}

impl Corner {
    /// The corner with matplotlib's code `1` to `4`.
    pub fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            1 => Self::UpperRight,
            2 => Self::UpperLeft,
            3 => Self::LowerLeft,
            4 => Self::LowerRight,
            _ => return None,
        })
    }

    /// The corner of `bbox`, read from its stored points like
    /// matplotlib's `get_bbox_edge_pos` (a flipped box swaps corners).
    fn of(self, bbox: &Bbox) -> Point {
        let (p0, p1) = (bbox.p0, bbox.p1);
        match self {
            Self::UpperRight => p1,
            Self::UpperLeft => Point::new(p0.x, p1.y),
            Self::LowerLeft => p0,
            Self::LowerRight => Point::new(p1.x, p0.y),
        }
    }
}

/// The area inset axes `inset` shows, in display pixels: its view limits
/// through the data transform of `parent` (matplotlib's `TransformedBbox`
/// of `mark_inset`).
fn shown_area(layout: LayoutContext<'_>, parent: ArtistId, inset: ArtistId) -> Option<Bbox> {
    let view = layout.store().get_as::<AxesState>(inset)?.view_lim;
    Some(transformed_bbox(
        &view,
        &layout.resolve(&Transform::Data(parent))?,
    ))
}

/// The outline of the area an inset axes shows, drawn in the inset's
/// parent (matplotlib's `BboxPatch` of `mark_inset`).
#[derive(Clone, Debug, PartialEq)]
pub struct InsetArea {
    inset: ArtistId,
}

impl PatchShape for InsetArea {
    fn path(&self) -> Path {
        Path::polyline(Vec::new())
    }

    fn skips_autoscale(&self) -> bool {
        true
    }

    fn display_paths(&self, frame: &ShapeFrame<'_>) -> Result<Vec<DisplayPath>, PatchError> {
        let Some(area) = frame
            .parent
            .and_then(|parent| shown_area(frame.layout, parent, self.inset))
        else {
            return Ok(Vec::new());
        };
        let corners = [
            Corner::LowerLeft,
            Corner::LowerRight,
            Corner::UpperRight,
            Corner::UpperLeft,
        ]
        .map(|corner| corner.of(&area));
        Ok(vec![DisplayPath::display(Path::polygon(corners), true)])
    }
}

/// A line from a corner of an inset axes to the same corner of the area
/// it shows, drawn in the inset (matplotlib's `BboxConnector` of
/// `mark_inset`).
#[derive(Clone, Debug, PartialEq)]
pub struct InsetConnector {
    parent: ArtistId,
    corner: Corner,
}

impl PatchShape for InsetConnector {
    fn path(&self) -> Path {
        Path::polyline(Vec::new())
    }

    fn skips_autoscale(&self) -> bool {
        true
    }

    fn init_style(&self, _style: &mut PatchStyle, base: &mut ArtistBase) {
        base.clip_on = false;
    }

    fn display_paths(&self, frame: &ShapeFrame<'_>) -> Result<Vec<DisplayPath>, PatchError> {
        let ends = frame.parent.and_then(|inset| {
            let inset_box = frame.layout.axes_extent(inset)?;
            let area = shown_area(frame.layout, self.parent, inset)?;
            Some([self.corner.of(&inset_box), self.corner.of(&area)])
        });
        Ok(ends.map_or_else(Vec::new, |ends| {
            vec![DisplayPath::display(Path::polyline(ends.to_vec()), false)]
        }))
    }
}

/// The marks [`mark_inset`] adds.
#[derive(Clone, Debug, PartialEq)]
pub struct InsetMarks {
    /// The outline of the area, in the parent.
    pub area: Patch<InsetArea>,
    /// The lines from the two corners, in the inset.
    pub connectors: [Patch<InsetConnector>; 2],
}

/// Outlines the area of `parent` that `inset` shows and joins corners
/// `first` and `second` of the inset to the same corners of the area,
/// styled by `props` (matplotlib's `mark_inset`). The marks are filled
/// only when `props` give a face colour or a colour, as in matplotlib;
/// the outline is clipped to the parent, the lines are not clipped.
///
/// ```
/// use plt::axes_grid1::{Corner, InsetOptions, mark_inset, zoomed_inset_axes};
/// use plt::prelude::*;
/// let fig = Figure::new();
/// let ax = fig.subplot();
/// let zoomed = zoomed_inset_axes(&ax, 2.0, &InsetOptions::default());
/// zoomed.set_xlim(0.2, 0.3).set_ylim(0.2, 0.3);
/// let marks = mark_inset(&ax, &zoomed, Corner::UpperLeft, Corner::LowerRight, PatchProps::default());
/// assert_eq!(marks.area.zorder(), 1.0);
/// ```
pub fn mark_inset(
    parent: &Axes,
    inset: &Axes,
    first: Corner,
    second: Corner,
    props: PatchProps,
) -> InsetMarks {
    let fill = props.is_set("facecolor") || props.is_set("color");
    let styled = PatchProps {
        fill: Some(fill),
        ..props
    };
    let area = parent.add_artist(InsetArea { inset: inset.id() });
    area.update_props(styled.clone());
    let connectors = [first, second].map(|corner| {
        let line = inset.add_artist(InsetConnector {
            parent: parent.id(),
            corner,
        });
        line.update_props(styled.clone());
        line
    });
    InsetMarks { area, connectors }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes_grid1::{InsetOptions, zoomed_inset_axes};
    use crate::figure::Figure;
    use crate::testing::{Call, RecordingRenderer};

    #[test]
    fn corners_follow_matplotlib_codes() {
        let bbox = Bbox::from_bounds(1.0, 2.0, 3.0, 4.0);
        assert_eq!(
            Corner::from_code(1).map(|c| c.of(&bbox)),
            Some(Point::new(4.0, 6.0))
        );
        assert_eq!(
            Corner::from_code(3).map(|c| c.of(&bbox)),
            Some(Point::new(1.0, 2.0))
        );
        assert_eq!(Corner::from_code(5), None);
    }

    #[test]
    fn marks_outline_the_area_and_join_the_corners() {
        let fig = Figure::with_size(4.0, 4.0, 100.0);
        let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]);
        ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
        let options = InsetOptions {
            borderpad: (0.0, 0.0),
            ..InsetOptions::default()
        };
        let zoomed = zoomed_inset_axes(&ax, 2.0, &options);
        zoomed.set_xlim(1.0, 2.0).set_ylim(1.0, 2.0);
        let marks = mark_inset(
            &ax,
            &zoomed,
            Corner::UpperLeft,
            Corner::LowerRight,
            PatchProps::default().edgecolor("0.5"),
        );
        assert_eq!(marks.connectors[1].zorder(), 1.0);
        let mut renderer = RecordingRenderer::new(400.0, 400.0, 100.0);
        fig.draw(&mut renderer).expect("drawable");
        let outlines: Vec<Vec<Point>> = renderer
            .calls
            .into_iter()
            .filter_map(|call| match call {
                Call::Path {
                    points, fill: None, ..
                } => Some(points),
                _ => None,
            })
            .collect();
        // The area spans 40 px from (40, 40); the inset is 80 px wide in
        // the upper right corner.
        assert!(outlines.iter().any(|p| p.contains(&Point::new(40.0, 40.0))));
        assert!(
            outlines
                .iter()
                .any(|p| p == &[Point::new(320.0, 400.0), Point::new(40.0, 80.0)])
        );
    }
}
