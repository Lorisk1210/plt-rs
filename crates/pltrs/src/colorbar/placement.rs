//! Making room for a colorbar: stealing space from its parent axes in
//! figure coordinates or inside the parent's grid cell (matplotlib's
//! `make_axes`, `make_axes_gridspec` and
//! `_normalize_location_orientation`).

use std::rc::Rc;

use pltrs_core::{Anchor, Bbox};

use super::{ColorbarError, ColorbarOptions, ParentAnchor};
use crate::artist::{ArtistHandle, ArtistId, RawHandle};
use crate::axes::{Axes, AxesState, AxisName};
use crate::figure::{Figure, FigureCell};
use crate::gridspec::{GridSpec, SubplotSpec};
use crate::layout::ColorbarLayout;
use crate::mplot3d::Axes3D;
use crate::orientation::Orientation;
use crate::rc::Aspect;
use crate::spines::SpineSide;
use crate::transforms::bbox_transform;

/// The orientation of a colorbar on `location`.
pub(crate) fn orientation_of(location: SpineSide) -> Orientation {
    if location.axis() == AxisName::Y {
        Orientation::Vertical
    } else {
        Orientation::Horizontal
    }
}

/// The default tick side of a colorbar of `orientation`.
pub(crate) fn default_ticklocation(orientation: Orientation) -> SpineSide {
    match orientation {
        Orientation::Vertical => SpineSide::Right,
        Orientation::Horizontal => SpineSide::Bottom,
    }
}

/// The defaults of a colorbar location.
#[derive(Clone, Copy, Debug, PartialEq)]
struct LocationDefaults {
    location: SpineSide,
    orientation: Orientation,
    anchor: Anchor,
    panchor: Anchor,
    pad: f64,
}

impl LocationDefaults {
    /// matplotlib's `_normalize_location_orientation`.
    fn new(
        location: Option<SpineSide>,
        orientation: Option<Orientation>,
    ) -> Result<Self, ColorbarError> {
        let location =
            location.unwrap_or_else(|| default_ticklocation(orientation.unwrap_or_default()));
        let (anchor, panchor, pad) = match location {
            SpineSide::Left => ((1.0, 0.5), (0.0, 0.5), 0.10),
            SpineSide::Right => ((0.0, 0.5), (1.0, 0.5), 0.05),
            SpineSide::Top => ((0.5, 0.0), (0.5, 1.0), 0.05),
            SpineSide::Bottom => ((0.5, 1.0), (0.5, 0.0), 0.15),
        };
        let located = orientation_of(location);
        if let Some(orientation) = orientation
            && orientation != located
        {
            return Err(ColorbarError::LocationOrientation {
                location,
                orientation,
            });
        }
        Ok(Self {
            location,
            orientation: located,
            anchor: Anchor::Fraction(anchor.0, anchor.1),
            panchor: Anchor::Fraction(panchor.0, panchor.1),
            pad,
        })
    }
}

/// An axes a colorbar takes its space from: a 2-D axes, or a 3-D axes
/// (whose projection plane draws the mappable).
#[derive(Clone, Debug)]
pub(crate) enum Parent {
    /// A 2-D axes.
    Axes(Axes),
    /// A 3-D axes.
    Axes3D(Axes3D),
}

impl Parent {
    /// The parent for a mappable drawn in `ax`: the 3-D axes when `ax` is
    /// the projection plane of one.
    pub(crate) fn of(ax: Axes) -> Self {
        Axes3D::of_plane(&ax).map_or(Self::Axes(ax), Self::Axes3D)
    }

    fn raw(&self) -> &RawHandle {
        match self {
            Self::Axes(ax) => ax.raw(),
            Self::Axes3D(ax) => ax.raw(),
        }
    }

    fn id(&self) -> ArtistId {
        self.raw().id()
    }

    /// The state of the figure the parent belongs to.
    pub(crate) fn figure_state(&self) -> &Rc<FigureCell> {
        self.raw().figure_state()
    }

    /// The grid cell of the parent, if it was placed on a grid.
    pub(crate) fn subplot_spec(&self) -> Option<SubplotSpec> {
        match self {
            Self::Axes(ax) => ax.subplot_spec(),
            Self::Axes3D(ax) => ax.subplot_spec(),
        }
    }

    fn original_position(&self) -> Bbox {
        match self {
            Self::Axes(ax) => ax.original_position(),
            Self::Axes3D(ax) => ax.original_position(),
        }
    }

    /// Replaces the parent's original position, keeping its corners
    /// exactly (matplotlib's `_set_position`), and its grid cell when
    /// `spec` is given.
    fn place(&self, position: Bbox, spec: Option<SubplotSpec>) {
        match self {
            Self::Axes(ax) => place(ax, position, spec),
            Self::Axes3D(ax) => ax.place(position, spec),
        }
    }

    fn set_anchor(&self, anchor: Anchor) {
        match self {
            Self::Axes(ax) => {
                ax.set_anchor(anchor);
            }
            Self::Axes3D(ax) => {
                ax.set_anchor(anchor);
            }
        }
    }
}

/// A colorbar axes made by stealing space from parent axes, with what
/// the colorbar needs to know about it (what matplotlib's `make_axes` and
/// `make_axes_gridspec` return: the axes, and the `orientation` and
/// `ticklocation` keywords they add for the colorbar).
#[derive(Clone, Debug)]
pub struct ColorbarCax {
    /// The new colorbar axes.
    pub cax: Axes,
    /// The colorbar's orientation.
    pub orientation: Orientation,
    /// The side the colorbar sits on, which is also its tick side.
    pub location: SpineSide,
}

/// Makes an axes for a colorbar beside `parents`, all axes of one
/// figure, shrinking them within their bounding box to free its space
/// (matplotlib's `colorbar.make_axes`): the location or orientation,
/// fraction, shrink, aspect, pad, anchor and parents' anchor of `options`
/// say where and how; the padding defaults to 0.05 under constrained
/// layout. The new axes is the figure's current axes.
///
/// ```
/// use pltrs::colorbar::{ColorbarOptions, make_axes};
/// use pltrs::prelude::*;
/// use pltrs::spines::SpineSide;
/// let fig = Figure::new();
/// fig.set_layout_engine(None)?;
/// let ax = fig.subplot();
/// let made = make_axes(&[ax.clone()], &ColorbarOptions::default())?;
/// assert_eq!(made.location, SpineSide::Right);
/// assert!(made.cax.original_position().x0() > ax.original_position().x1());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When there are no parents, they are axes of different figures, or the
/// options contradict each other.
pub fn make_axes(
    parents: &[Axes],
    options: &ColorbarOptions,
) -> Result<ColorbarCax, ColorbarError> {
    let first = parents.first().ok_or(ColorbarError::NoAxes)?;
    let figure = first.parent_figure();
    let constrained = matches!(
        figure.layout_engine(),
        Some(crate::layout::LayoutEngine::Constrained(_))
    );
    let parents: Vec<Parent> = parents.iter().cloned().map(Parent::of).collect();
    make_axes_in(&figure, &parents, options, constrained)
}

/// Makes an axes for a colorbar in the grid cell of `parent`, which it
/// subdivides into the parent's new cell and the colorbar's (matplotlib's
/// `colorbar.make_axes_gridspec`), as `options` say (see [`make_axes`]).
///
/// ```
/// use pltrs::colorbar::{ColorbarOptions, make_axes_gridspec};
/// use pltrs::prelude::*;
/// let fig = Figure::new();
/// let ax = fig.subplot();
/// let made = make_axes_gridspec(&ax, &ColorbarOptions::default())?;
/// assert!(made.cax.subplot_spec().is_some());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When `parent` was not placed on a grid, or the options contradict each
/// other.
pub fn make_axes_gridspec(
    parent: &Axes,
    options: &ColorbarOptions,
) -> Result<ColorbarCax, ColorbarError> {
    let spec = parent.subplot_spec().ok_or(ColorbarError::NoSubplotSpec)?;
    make_axes_gridspec_in(
        &parent.parent_figure(),
        Parent::of(parent.clone()),
        &spec,
        options,
    )
}

/// The settings shared by both ways of making room.
struct Request<'a> {
    defaults: LocationDefaults,
    anchor: Anchor,
    panchor: Option<Anchor>,
    options: &'a ColorbarOptions,
}

impl<'a> Request<'a> {
    fn new(options: &'a ColorbarOptions) -> Result<Self, ColorbarError> {
        let defaults = LocationDefaults::new(options.location, options.orientation)?;
        Ok(Self {
            defaults,
            anchor: options.anchor.unwrap_or(defaults.anchor),
            panchor: match options.panchor {
                ParentAnchor::Default => Some(defaults.panchor),
                ParentAnchor::Unchanged => None,
                ParentAnchor::At(anchor) => Some(anchor),
            },
            options,
        })
    }

    /// The box aspect of the colorbar axes (height over width).
    fn box_aspect(&self) -> f64 {
        match self.defaults.orientation {
            Orientation::Vertical => self.options.aspect,
            Orientation::Horizontal => 1.0 / self.options.aspect,
        }
    }

    /// Anchors the parents, shapes the colorbar axes and records where
    /// the colorbar took its space for layout engines (matplotlib's
    /// `_colorbar_info`).
    fn finish(self, cax: Axes, parents: &[Parent], pad: f64) -> Result<ColorbarCax, ColorbarError> {
        if let Some(panchor) = self.panchor {
            for parent in parents {
                parent.set_anchor(panchor);
            }
        }
        cax.set_anchor(self.anchor);
        cax.set_box_aspect(Some(self.box_aspect()))
            .map_err(|_| ColorbarError::InvalidAspect(self.options.aspect))?;
        cax.set_aspect(Aspect::Auto)
            .expect("the auto aspect is always valid");
        let layout = ColorbarLayout {
            cax: cax.id(),
            parents: parents.iter().map(Parent::id).collect(),
            location: self.defaults.location,
            fraction: self.options.fraction,
            shrink: self.options.shrink,
            aspect: self.options.aspect,
            pad,
            anchor: self.anchor,
        };
        cax.raw()
            .figure_state()
            .borrow_mut()
            .layout
            .colorbars
            .push(layout);
        Ok(ColorbarCax {
            cax,
            orientation: self.defaults.orientation,
            location: self.defaults.location,
        })
    }
}

/// Replaces an axes' original position, keeping its corners exactly
/// (matplotlib's `_set_position`), and its grid cell when `spec` is
/// given.
pub(super) fn place(ax: &Axes, position: Bbox, spec: Option<SubplotSpec>) {
    ax.raw().with_mut::<AxesState, _>(|axes| {
        axes.position = position;
        if spec.is_some() {
            axes.subplot_spec = spec;
        }
    });
}

/// matplotlib's `make_axes`: shrinks the parents within their bounding box
/// and puts a new colorbar axes in the space freed on `location`.
pub(crate) fn make_axes_in(
    figure: &Figure,
    parents: &[Parent],
    options: &ColorbarOptions,
    constrained: bool,
) -> Result<ColorbarCax, ColorbarError> {
    let request = Request::new(options)?;
    if parents
        .iter()
        .any(|ax| !Rc::ptr_eq(ax.raw().figure_state(), figure.raw().figure_state()))
    {
        return Err(ColorbarError::MixedFigures);
    }
    let pad0 = if constrained {
        0.05
    } else {
        request.defaults.pad
    };
    let pad = options.pad.unwrap_or(pad0);
    let originals: Vec<Bbox> = parents.iter().map(Parent::original_position).collect();
    let bounds = Bbox::union_all(&originals).ok_or(ColorbarError::NoAxes)?;
    let (fraction, shrink) = (options.fraction, options.shrink);
    let anchor = request.anchor;
    let (parent_box, colorbar_box) = match request.defaults.location {
        SpineSide::Left => {
            let [cb, _, rest] = split3(&bounds.split_x(&[fraction, fraction + pad]));
            (rest, cb.shrunk(1.0, shrink).anchored(anchor, &cb))
        }
        SpineSide::Right => {
            let [rest, _, cb] = split3(&bounds.split_x(&[1.0 - fraction - pad, 1.0 - fraction]));
            (rest, cb.shrunk(1.0, shrink).anchored(anchor, &cb))
        }
        SpineSide::Bottom => {
            let [cb, _, rest] = split3(&bounds.split_y(&[fraction, fraction + pad]));
            (rest, cb.shrunk(shrink, 1.0).anchored(anchor, &cb))
        }
        SpineSide::Top => {
            let [rest, _, cb] = split3(&bounds.split_y(&[1.0 - fraction - pad, 1.0 - fraction]));
            (rest, cb.shrunk(shrink, 1.0).anchored(anchor, &cb))
        }
    };
    let shrinking = bbox_transform(&bounds, &parent_box);
    for (parent, original) in parents.iter().zip(&originals) {
        parent.place(
            Bbox::new(
                shrinking.transform_point(original.p0),
                shrinking.transform_point(original.p1),
            ),
            None,
        );
    }
    let (left, bottom, width, height) = colorbar_box.bounds();
    let cax = figure.add_axes([left, bottom, width, height])?;
    place(&cax, colorbar_box, None);
    request.finish(cax, parents, pad)
}

/// The three pieces of a two-way split.
fn split3(pieces: &[Bbox]) -> [Bbox; 3] {
    [pieces[0], pieces[1], pieces[2]]
}

/// matplotlib's `make_axes_gridspec`: subdivides the parent's grid cell
/// into the parent's new cell and the colorbar's.
pub(crate) fn make_axes_gridspec_in(
    figure: &Figure,
    parent: Parent,
    spec: &SubplotSpec,
    options: &ColorbarOptions,
) -> Result<ColorbarCax, ColorbarError> {
    let request = Request::new(options)?;
    let pad = options.pad.unwrap_or(request.defaults.pad);
    let space = 2.0 * pad / (1.0 - pad);
    let (fraction, shrink) = (options.fraction, options.shrink);
    let (ax, ay) = request.anchor.coefficients();
    let rest = 1.0 - fraction - pad;
    let (main, colorbar) = match request.defaults.location {
        location @ (SpineSide::Left | SpineSide::Right) => {
            let heights = vec![(1.0 - ay) * (1.0 - shrink), shrink, ay * (1.0 - shrink)];
            let (widths, main_col, cb_col) = if location == SpineSide::Left {
                (vec![fraction, rest], 1, 0)
            } else {
                (vec![rest, fraction], 0, 1)
            };
            let grid = GridSpec::nested(
                spec,
                3,
                2,
                Some(widths),
                Some(heights),
                (Some(space), Some(0.0)),
            )?;
            (
                grid.span(0..3, main_col..main_col + 1)?,
                grid.cell(1, cb_col)?,
            )
        }
        location @ (SpineSide::Top | SpineSide::Bottom) => {
            let widths = vec![ax * (1.0 - shrink), shrink, (1.0 - ax) * (1.0 - shrink)];
            let (heights, main_row, cb_row) = if location == SpineSide::Top {
                (vec![fraction, rest], 1, 0)
            } else {
                (vec![rest, fraction], 0, 1)
            };
            let grid = GridSpec::nested(
                spec,
                2,
                3,
                Some(widths),
                Some(heights),
                (Some(0.0), Some(space)),
            )?;
            (
                grid.span(main_row..main_row + 1, 0..3)?,
                grid.cell(cb_row, 1)?,
            )
        }
    };
    let position = main.position(&figure.subplot_params())?;
    parent.place(position, Some(main));
    let cax = figure.add_subplot_spec(&colorbar)?;
    request.finish(cax, &[parent], pad)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locations_set_orientation_and_defaults() {
        let right = LocationDefaults::new(None, None).unwrap();
        assert_eq!(right.location, SpineSide::Right);
        assert_eq!(right.pad, 0.05);
        let bottom = LocationDefaults::new(None, Some(Orientation::Horizontal)).unwrap();
        assert_eq!(bottom.location, SpineSide::Bottom);
        assert_eq!(bottom.anchor.coefficients(), (0.5, 1.0));
        let left = LocationDefaults::new(Some(SpineSide::Left), None).unwrap();
        assert_eq!((left.pad, left.orientation), (0.10, Orientation::Vertical));
        assert_eq!(
            LocationDefaults::new(Some(SpineSide::Top), Some(Orientation::Vertical)),
            Err(ColorbarError::LocationOrientation {
                location: SpineSide::Top,
                orientation: Orientation::Vertical
            })
        );
    }

    #[test]
    fn make_axes_steals_from_the_right() {
        let fig = Figure::new();
        let ax = fig
            .add_axes([0.125, 0.11, 0.775, 0.77])
            .expect("a valid rectangle");
        let placed = make_axes_in(
            &fig,
            &[Parent::Axes(ax.clone())],
            &ColorbarOptions::default(),
            false,
        )
        .unwrap();
        let (x, _, w, _) = ax.original_position().bounds();
        assert!((x - 0.125).abs() < 1e-12 && (w - 0.62).abs() < 1e-12);
        let (cx, cy, cw, ch) = placed.cax.original_position().bounds();
        assert!((cx - 0.78375).abs() < 1e-12 && (cw - 0.11625).abs() < 1e-12);
        assert!((cy - 0.11).abs() < 1e-12 && (ch - 0.77).abs() < 1e-12);
        assert_eq!(placed.cax.box_aspect(), Some(20.0));
        assert_eq!(ax.anchor(), Anchor::Fraction(1.0, 0.5));
    }

    #[test]
    fn make_axes_gridspec_splits_the_cell() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let spec = ax.subplot_spec().unwrap();
        let options = ColorbarOptions {
            location: Some(SpineSide::Bottom),
            ..ColorbarOptions::default()
        };
        let placed =
            make_axes_gridspec_in(&fig, Parent::Axes(ax.clone()), &spec, &options).unwrap();
        assert_eq!(placed.orientation, Orientation::Horizontal);
        let (_, y, _, h) = placed.cax.original_position().bounds();
        // The bottom row: 0.15 of the cell height below a 0.15 pad.
        assert!((y - 0.11).abs() < 1e-12 && (h - 0.1155).abs() < 1e-12);
        assert_eq!(placed.cax.box_aspect(), Some(0.05));
        assert!(ax.original_position().y0() > 0.3);
    }
}
