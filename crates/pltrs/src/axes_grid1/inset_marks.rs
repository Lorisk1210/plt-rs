//! Marks joining an inset axes to the area of its parent it shows: an
//! outline of the area and lines between corners (matplotlib's
//! `mark_inset`).

use super::bbox_patches::{BboxConnector, BboxPatch, BboxSource, Corner};
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::patches::{Patch, PatchProps};

/// The marks [`mark_inset`] adds.
#[derive(Clone, Debug, PartialEq)]
pub struct InsetMarks {
    /// The outline of the area, in the parent.
    pub area: Patch<BboxPatch>,
    /// The lines from the two corners, in the inset.
    pub connectors: [Patch<BboxConnector>; 2],
}

/// Outlines the area of `parent` that `inset` shows and joins corners
/// `first` and `second` of the inset to the same corners of the area,
/// styled by `props` (matplotlib's `mark_inset`). Unless `props` say
/// whether to fill them, the marks are filled only when `props` give a
/// face colour or a colour, as in matplotlib; the outline is clipped to
/// the parent, the lines are not clipped.
///
/// ```
/// use pltrs::axes_grid1::{Corner, InsetOptions, mark_inset, zoomed_inset_axes};
/// use pltrs::prelude::*;
/// let fig = Figure::new();
/// let ax = fig.subplot();
/// let zoomed = zoomed_inset_axes(&ax, 2.0, InsetOptions::default());
/// zoomed.set_xlim(0.2, 0.3)?.set_ylim(0.2, 0.3)?;
/// let marks = mark_inset(&ax, &zoomed, Corner::UpperLeft, Corner::LowerRight, PatchProps::default());
/// assert_eq!(marks.area.zorder(), 1.0);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn mark_inset(
    parent: &Axes,
    inset: &Axes,
    first: Corner,
    second: Corner,
    props: PatchProps,
) -> InsetMarks {
    let fill = props
        .fill
        .unwrap_or(props.is_set("facecolor") || props.is_set("color"));
    let styled = PatchProps {
        fill: Some(fill),
        ..props
    };
    let shown = BboxSource::view_limits(inset, parent.trans_data());
    let area = parent.add_patch(BboxPatch::new(shown.clone()));
    area.update_props(styled.clone());
    let connectors = [first, second].map(|corner| {
        let line = inset.add_patch(BboxConnector::new(
            BboxSource::axes(inset),
            shown.clone(),
            corner,
            None,
        ));
        line.update_props(styled.clone());
        line.set_clip_on(false);
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
    use pltrs_core::Point;

    #[test]
    fn marks_outline_the_area_and_join_the_corners() {
        let fig = Figure::with_size(4.0, 4.0, 100.0);
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        let options = InsetOptions {
            borderpad: (0.0, 0.0),
            ..InsetOptions::default()
        };
        let zoomed = zoomed_inset_axes(&ax, 2.0, options);
        zoomed
            .set_xlim(1.0, 2.0)
            .and_then(|ax| ax.set_ylim(1.0, 2.0))
            .expect("finite limits");
        let marks = mark_inset(
            &ax,
            &zoomed,
            Corner::UpperLeft,
            Corner::LowerRight,
            PatchProps::default().edgecolor("0.5"),
        );
        assert_eq!(marks.connectors[1].zorder(), 1.0);
        assert!(
            ax.data_lim().bbox.is_null(),
            "the marks have no data limits"
        );
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

    #[test]
    fn a_face_colour_fills_the_marks_unless_told_otherwise() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let zoomed = zoomed_inset_axes(&ax, 2.0, InsetOptions::default());
        let mark = |props: PatchProps| {
            let marks = mark_inset(&ax, &zoomed, Corner::UpperLeft, Corner::LowerRight, props);
            (marks.area.style().fill, marks.connectors[0].style().fill)
        };
        assert_eq!(mark(PatchProps::default()), (false, false));
        assert_eq!(mark(PatchProps::default().facecolor("g")), (true, true));
        assert_eq!(mark(PatchProps::default().color("g")), (true, true));
        let unfilled = PatchProps::default().facecolor("g").fill(false);
        assert_eq!(mark(unfilled), (false, false));
    }
}
