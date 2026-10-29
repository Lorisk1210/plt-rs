//! Inset axes anchored in a box of their parent, sized in inches, as a
//! percentage of the box or by zooming the parent's data coordinates
//! (matplotlib's `axes_grid1.inset_locator`).

use pltrs_core::Bbox;

use super::size::{Size, SizeSpec};
use crate::artist::{Artist, ArtistHandle, ArtistId, ArtistLookup};
use crate::axes::{Axes, AxesState};
use crate::figure::{FigureState, make_axes};
use crate::offsetbox::{AnchorLoc, BboxToAnchor, anchored_bbox, transformed_bbox};
use crate::transforms::{Transform, TransformContext};

/// An inset locator of a figure, by its index among the figure's inset
/// locators.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct InsetLocatorId(pub(crate) usize);

/// How big an inset is.
#[derive(Clone, Copy, Debug, PartialEq)]
enum InsetExtent {
    /// A width and a height in inches or percentages of the anchor box
    /// (matplotlib's `AnchoredSizeLocator`).
    Sized { width: SizeSpec, height: SizeSpec },
    /// The inset's view limits measured in the parent's data coordinates,
    /// scaled by `zoom` (matplotlib's `AnchoredZoomLocator`).
    Zoomed { zoom: f64 },
    /// The anchor box itself (matplotlib's `_TransformedBoundsLocator`).
    Bounds,
}

/// Where an inset axes goes at every layout pass (matplotlib's
/// `AnchoredLocatorBase` subclasses).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct InsetLocator {
    extent: InsetExtent,
    loc: AnchorLoc,
    /// The axes the inset belongs to.
    parent: ArtistId,
    /// The box the inset is anchored in, with its coordinates; `None` is
    /// the parent axes.
    anchor: Option<(Bbox, Transform)>,
    /// Padding to the anchor box, horizontally and vertically, in font
    /// sizes.
    borderpad: (f64, f64),
    /// The font size the padding is a fraction of, in points
    /// (`legend.fontsize` when the inset was made).
    fontsize: f64,
}

/// Where [`inset_axes`] and [`zoomed_inset_axes`] put an inset (their
/// keyword arguments).
#[derive(Clone, Debug, PartialEq)]
pub struct InsetOptions {
    /// The position in the anchor box.
    pub loc: AnchorLoc,
    /// The box the inset is anchored in; `None` is the parent axes, or the
    /// unit box of `bbox_transform` when that is set (matplotlib does so
    /// for the axes and figure transforms).
    pub bbox_to_anchor: Option<BboxToAnchor>,
    /// The coordinates of `bbox_to_anchor` (`None` = display pixels).
    pub bbox_transform: Option<Transform>,
    /// Padding between the inset and the anchor box, horizontally and
    /// vertically, in font sizes (`legend.fontsize`).
    pub borderpad: (f64, f64),
}

impl Default for InsetOptions {
    /// matplotlib's defaults: the upper right corner of the parent axes,
    /// half a font size from its edges.
    fn default() -> Self {
        Self {
            loc: AnchorLoc::UpperRight,
            bbox_to_anchor: None,
            bbox_transform: None,
            borderpad: (0.5, 0.5),
        }
    }
}

/// Adds an inset axes `width` × `height` (inches, or percentages of the
/// anchor box) anchored in `parent` as `options` say (matplotlib's
/// `inset_locator.inset_axes`). Unlike [`Axes::inset_axes`], the inset is
/// an axes of the figure, placed anew at every layout pass, with
/// navigation off. Like it, it has all four spines under
/// `pltrs.inset.frame`.
///
/// ```
/// use pltrs::axes_grid1::{InsetOptions, SizeSpec, inset_axes};
/// use pltrs::prelude::*;
/// let fig = Figure::with_size(6.4, 4.8, 100.0);
/// let ax = fig.subplot();
/// let inset = inset_axes(&ax, SizeSpec::Percent(30.0), 1.0.into(), InsetOptions::default());
/// let (_, _, width, height) = inset.position().bounds();
/// let (_, _, parent_width, _) = ax.position().bounds();
/// assert!((width - 0.3 * parent_width).abs() < 1e-12);
/// assert!((height - 1.0 / 4.8).abs() < 1e-12, "one inch of a 4.8 in figure");
/// ```
pub fn inset_axes(parent: &Axes, width: SizeSpec, height: SizeSpec, options: InsetOptions) -> Axes {
    add_inset(parent, InsetExtent::Sized { width, height }, &options)
}

/// Adds an inset axes showing its view limits `zoom` times as large as
/// `parent` shows them, anchored in `parent` as `options` say
/// (matplotlib's `inset_locator.zoomed_inset_axes`); set the inset's
/// limits to choose the area it enlarges.
pub fn zoomed_inset_axes(parent: &Axes, zoom: f64, options: InsetOptions) -> Axes {
    add_inset(parent, InsetExtent::Zoomed { zoom }, &options)
}

/// Adds the axes of an inset of `parent`, placed by a new locator
/// (matplotlib's `_add_inset_axes`).
fn add_inset(parent: &Axes, extent: InsetExtent, options: &InsetOptions) -> Axes {
    let rc = crate::rc::params();
    let anchor = match (options.bbox_to_anchor, &options.bbox_transform) {
        (None, None) => None,
        (anchor, transform) => Some((
            anchor.map_or(Bbox::UNIT, BboxToAnchor::bbox),
            transform.clone().unwrap_or(Transform::Display),
        )),
    };
    let locator = InsetLocator {
        extent,
        loc: options.loc,
        parent: parent.id(),
        anchor,
        borderpad: options.borderpad,
        fontsize: rc.legend_fontsize.to_points(rc.font_size),
    };
    let figure = parent.raw().figure_state();
    let position = parent.position();
    // The inset of a removed axes is removed too.
    let frame = {
        let state = figure.borrow();
        state
            .artist_as::<AxesState>(parent.id())
            .map_or(Some(state.root), |axes| axes.base().parent)
    };
    let inset = make_axes(figure, frame, position, None);
    inset.set_navigate(false);
    inset.frame_inset();
    register(&inset, locator);
    inset
}

/// Places `inset`, an axes inside `parent`, on the box `bounds` in the
/// coordinates of `transform` at every layout pass (matplotlib's
/// `Axes.inset_axes(bounds, transform=...)`).
pub(crate) fn locate_on_bounds(parent: &Axes, inset: &Axes, bounds: Bbox, transform: Transform) {
    let locator = InsetLocator {
        extent: InsetExtent::Bounds,
        loc: AnchorLoc::LowerLeft,
        parent: parent.id(),
        anchor: Some((bounds, transform)),
        borderpad: (0.0, 0.0),
        fontsize: 0.0,
    };
    register(inset, locator);
}

/// Makes `locator` place `inset` from the next layout pass on.
fn register(inset: &Axes, locator: InsetLocator) {
    let mut state = inset.raw().figure_state().borrow_mut();
    state.layout.insets.push(locator);
    let id = InsetLocatorId(state.layout.insets.len() - 1);
    if let Some(axes) = state.artist_as_mut::<AxesState>(inset.id()) {
        axes.aspect.inset = Some(id);
    }
    state.stale = true;
}

impl FigureState {
    /// The position of inset axes `axes` placed by locator `id`, in its
    /// frame's fractions (matplotlib's `AnchoredLocatorBase.__call__`), or
    /// `None` if its parent, anchor or frame is gone.
    pub(crate) fn locate_inset(&mut self, axes: ArtistId, id: InsetLocatorId) -> Option<Bbox> {
        let locator = self.layout.insets.get(id.0)?.clone();
        if locator.extent == InsetExtent::Bounds {
            // The bounds may be in the parent's data coordinates.
            self.unstale_axes(locator.parent);
        }
        let anchor = match &locator.anchor {
            None => self.axes_bbox(locator.parent)?,
            Some((bbox, transform)) => transformed_bbox(bbox, &transform.resolve(self).ok()?),
        };
        let (width, height) = match locator.extent {
            InsetExtent::Sized { width, height } => {
                let dpi = self.dpi();
                let mut length = |spec: SizeSpec, of: f64| {
                    let (relative, inches) = spec.resolve(&Size::Scaled(1.0)).evaluate(self);
                    of * relative + inches * dpi
                };
                (
                    length(width, anchor.width()),
                    length(height, anchor.height()),
                )
            }
            InsetExtent::Zoomed { zoom } => {
                self.unstale_axes(axes);
                self.unstale_axes(locator.parent);
                let view = self.store.get_as::<AxesState>(axes)?.view_lim;
                let data = Transform::Data(locator.parent).resolve(self).ok()?;
                let shown = transformed_bbox(&view, &data);
                ((shown.width() * zoom).abs(), (shown.height() * zoom).abs())
            }
            InsetExtent::Bounds => (anchor.width(), anchor.height()),
        };
        let fontsize = locator.fontsize * self.dpi() / 72.0;
        let corner = anchored_bbox(
            locator.loc,
            &Bbox::from_bounds(0.0, 0.0, width, height),
            &anchor,
            locator.borderpad.0 * fontsize,
            locator.borderpad.1 * fontsize,
        );
        let parent = self.store.get(axes)?.base().parent?;
        let frame = self.frame_bbox(parent).or_else(|| self.axes_bbox(parent))?;
        // Like matplotlib's inverse frame transform, scale both corners by
        // the reciprocal frame size (dividing the size instead can round
        // the far edge onto the other side of a pixel boundary).
        let (sx, sy) = (1.0 / frame.width(), 1.0 / frame.height());
        let to_frame = |x: f64, y: f64| ((x - frame.x0()) * sx, (y - frame.y0()) * sy);
        let (x0, y0) = to_frame(corner.x, corner.y);
        let (x1, y1) = to_frame(corner.x + width, corner.y + height);
        Some(Bbox::from_extents(x0, y0, x1, y1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    fn bounds(axes: &Axes) -> [f64; 4] {
        let (x, y, w, h) = axes.position().bounds();
        [x, y, w, h]
    }

    fn close(a: [f64; 4], b: [f64; 4]) -> bool {
        a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-9)
    }

    #[test]
    fn sized_insets_sit_in_their_corner() {
        let fig = Figure::with_size(5.0, 4.0, 100.0);
        let ax = fig
            .add_axes([0.1, 0.1, 0.8, 0.8])
            .expect("a valid rectangle");
        let options = InsetOptions {
            loc: AnchorLoc::LowerLeft,
            borderpad: (0.0, 0.0),
            ..InsetOptions::default()
        };
        let inset = inset_axes(&ax, 1.0.into(), SizeSpec::Percent(50.0), options);
        assert!(
            close(bounds(&inset), [0.1, 0.1, 0.2, 0.4]),
            "{:?}",
            bounds(&inset)
        );
        assert_eq!(fig.axes().len(), 2, "an axes of the figure");
    }

    #[test]
    fn far_edges_round_like_matplotlib() {
        // `demo_colorbar_with_inset_locator`: the colorbar's top edge
        // lands on a pixel boundary, 148.50000000000003 px in matplotlib
        // 3.11, which decides the pixel its outline snaps to.
        let fig = Figure::with_size(6.0, 3.0, 100.0);
        let axs = fig.subplots(1, 2).unwrap();
        let ax2 = &axs[(0, 1)];
        let options = InsetOptions {
            loc: AnchorLoc::LowerLeft,
            bbox_to_anchor: Some(BboxToAnchor::Bounds(1.05, 0.0, 1.0, 1.0)),
            bbox_transform: Some(ax2.trans_axes()),
            borderpad: (0.0, 0.0),
        };
        let inset = inset_axes(
            ax2,
            SizeSpec::Percent(5.0),
            SizeSpec::Percent(50.0),
            options,
        );
        assert_eq!(inset.position().p1.y, 0.495_000_000_000_000_05);
    }

    #[test]
    fn anchor_boxes_follow_their_transform() {
        let fig = Figure::with_size(5.0, 4.0, 100.0);
        let ax = fig
            .add_axes([0.1, 0.1, 0.5, 0.8])
            .expect("a valid rectangle");
        let options = InsetOptions {
            loc: AnchorLoc::LowerLeft,
            bbox_to_anchor: Some(BboxToAnchor::Bounds(1.05, 0.0, 1.0, 1.0)),
            bbox_transform: Some(ax.trans_axes()),
            borderpad: (0.0, 0.0),
        };
        let cax = inset_axes(
            &ax,
            SizeSpec::Percent(5.0),
            SizeSpec::Percent(100.0),
            options,
        );
        let expected = [0.1 + 1.05 * 0.5, 0.1, 0.05 * 0.5, 0.8];
        assert!(close(bounds(&cax), expected), "{:?}", bounds(&cax));
    }

    #[test]
    fn zoomed_insets_scale_the_parent_data() {
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
            .set_xlim(0.0, 1.0)
            .and_then(|ax| ax.set_ylim(0.0, 2.0))
            .expect("finite limits");
        // One data unit is 40 px in the parent: 80 px wide, 160 px tall.
        assert!(
            close(bounds(&zoomed), [0.8, 0.6, 0.2, 0.4]),
            "{:?}",
            bounds(&zoomed)
        );
    }
}
