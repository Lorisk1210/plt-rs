//! `loc="best"`: the anchored position where the legend covers the least
//! of what is plotted (matplotlib's `_auto_legend_data` and
//! `_find_best_position`).

use pltrs_core::{Bbox, Path, Point};

use super::artist::LegendParent;
use crate::artist::{Artist, ArtistId, LayoutContext};
use crate::axes::AxesState;
use crate::axes_grid1::parasites_of;
use crate::collections::{CollectionArtist, CollectionKind};
use crate::lines::LineArtist;
use crate::offsetbox::{AnchorLoc, anchored_bbox};
use crate::patches::Rectangle;
use crate::text::TextArtist;
use crate::transforms::Transform;
use crate::{clock, warnings};

/// What a legend should avoid, in display pixels.
#[derive(Debug, Default)]
struct Obstacles {
    /// Extents of rectangles and texts.
    bboxes: Vec<Bbox>,
    /// Lines and the outlines of other patches and of polygons.
    lines: Vec<Path>,
    /// Positions of collection items.
    offsets: Vec<Point>,
}

impl Obstacles {
    /// The plotted artists of `axes` and of its parasites, which a host
    /// draws as its own (matplotlib's `_auto_legend_data`).
    fn of_axes(ctx: LayoutContext<'_>, axes: ArtistId) -> Self {
        let mut obstacles = Self::default();
        let store = ctx.store();
        let owners = std::iter::once(axes).chain(parasites_of(store, axes));
        let plotted = owners
            .filter_map(|owner| store.get_as::<AxesState>(owner))
            .flat_map(|state| state.children.iter().filter(|&&id| !state.is_own_part(id)));
        for &id in plotted {
            let Some(artist) = store.get(id) else {
                continue;
            };
            if let Some(line) = store.get_as::<LineArtist>(id) {
                obstacles.lines.extend(line_path(ctx, line));
            } else if let Some(view) = artist.as_patch() {
                match view.shape.downcast_ref::<Rectangle>() {
                    Some(rect) => obstacles.bboxes.extend(rectangle_extent(ctx, artist, rect)),
                    None => obstacles.lines.extend(
                        artist
                            .outline(&ctx)
                            .map(|outline| outline.path.transformed(&outline.transform)),
                    ),
                }
            } else if let Some(collection) = store.get_as::<CollectionArtist>(id) {
                obstacles.add_collection(ctx, collection);
            } else if store.get_as::<TextArtist>(id).is_some() {
                obstacles.bboxes.extend(artist.window_extent(&ctx));
            }
        }
        obstacles
    }

    /// Polygons count with their outlines, other collections with their
    /// item positions (a collection without offsets has one at the origin
    /// of its offset coordinates, the display origin).
    fn add_collection(&mut self, ctx: LayoutContext<'_>, collection: &CollectionArtist) {
        if collection.kind == CollectionKind::Poly {
            let transform = collection
                .base()
                .transform
                .clone()
                .unwrap_or(Transform::Display);
            if let Some(resolved) = ctx.resolve(&transform) {
                self.lines.extend(
                    collection
                        .shapes()
                        .iter()
                        .map(|p| resolved.transform_path(p)),
                );
            }
            return;
        }
        match &collection.offset_transform {
            Some(transform) => {
                if let Some(resolved) = ctx.resolve(transform) {
                    self.offsets.extend(
                        collection
                            .offsets
                            .iter()
                            .map(|&p| resolved.transform_point(p)),
                    );
                }
            }
            None => self.offsets.push(Point::ORIGIN),
        }
    }

    /// How much a legend at `legend_box` would cover (matplotlib's
    /// badness): line vertices and item positions inside it, boxes
    /// overlapping it and lines crossing it.
    fn badness(&self, legend_box: &Bbox) -> usize {
        let vertices: usize = self
            .lines
            .iter()
            .map(|line| legend_box.count_contains(line.vertices()))
            .sum();
        let crossings = self
            .lines
            .iter()
            .filter(|line| line.intersects_bbox(legend_box, false))
            .count();
        vertices
            + legend_box.count_contains(&self.offsets)
            + legend_box.count_overlaps(&self.bboxes)
            + crossings
    }
}

/// A line's drawn path in display pixels.
fn line_path(ctx: LayoutContext<'_>, line: &LineArtist) -> Option<Path> {
    let transform = line.base().transform.clone().unwrap_or(Transform::Display);
    let resolved = ctx.resolve(&transform)?;
    Some(resolved.transform_path(&Path::polyline(line.drawn_points())))
}

/// A rectangle's unrotated extent in display pixels (matplotlib's
/// `get_bbox().transformed(get_data_transform())`).
fn rectangle_extent(ctx: LayoutContext<'_>, artist: &dyn Artist, rect: &Rectangle) -> Option<Bbox> {
    let transform = artist
        .base()
        .transform
        .clone()
        .unwrap_or(Transform::Display);
    let resolved = ctx.resolve(&transform)?;
    let (x0, y0) = (rect.xy.x, rect.xy.y);
    let (x1, y1) = (x0 + rect.width, y0 + rect.height);
    let lower_left = resolved.transform_point(Point::new(x0, y0));
    let upper_left = resolved.transform_point(Point::new(x0, y1));
    let lower_right = resolved.transform_point(Point::new(x1, y0));
    Some(Bbox::new(
        lower_left,
        Point::new(lower_right.x, upper_left.y),
    ))
}

/// How long a search for the best location may take, in seconds, before
/// it warns (matplotlib's `_find_best_position`).
const SLOW_SEARCH: f64 = 1.0;

/// The lower-left corner of the least crowded anchored position of a
/// `size` legend in `anchor` (matplotlib's `_find_best_position`): the
/// first position covering nothing, else the one covering least, earlier
/// location codes winning ties. Figure legends have nothing to avoid
/// (and never ask: `"best"` is rejected for them).
///
/// As matplotlib does, the search times itself on [`clock`] and warns
/// when it took longer than a second for a legend at the default location
/// (`warn_if_slow`), reading the clock at the end only then.
pub(crate) fn best_position(
    ctx: LayoutContext<'_>,
    parent: LegendParent,
    anchor: &Bbox,
    pad: f64,
    (width, height): (f64, f64),
    warn_if_slow: bool,
) -> Point {
    let start = clock::perf_counter();
    let obstacles = match parent {
        LegendParent::Axes(axes) => Obstacles::of_axes(ctx, axes),
        LegendParent::Figure(_) => Obstacles::default(),
    };
    let bbox = Bbox::from_bounds(0.0, 0.0, width, height);
    let mut best: Option<(usize, Point)> = None;
    for loc in AnchorLoc::ALL {
        let corner = anchored_bbox(loc, &bbox, anchor, pad, pad);
        let badness = obstacles.badness(&Bbox::from_bounds(corner.x, corner.y, width, height));
        if best.is_none_or(|(least, _)| badness < least) {
            best = Some((badness, corner));
        }
        if badness == 0 {
            break;
        }
    }
    if warn_if_slow && clock::perf_counter() - start > SLOW_SEARCH {
        warnings::warn("Creating legend with loc=\"best\" can be slow with large amounts of data.");
    }
    best.map_or(anchor.p0, |(_, corner)| corner)
}
