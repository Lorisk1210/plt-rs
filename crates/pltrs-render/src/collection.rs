//! Batched drawing of many paths with per-item properties.

use std::rc::Rc;

use pltrs_core::{Affine2D, Path, Point, Rgba};

use crate::gc::{Dashes, GraphicsContext};

/// The arguments of matplotlib's `draw_path_collection`.
///
/// Every per-item sequence is *cycled* to the item count (see
/// [`ItemCycle`]); an empty sequence means "use the base graphics context"
/// (or, for colours, "do not fill/stroke").
#[derive(Clone, Debug, Default)]
pub struct PathCollection<'a> {
    /// Applied after each item's own transform.
    pub master_transform: Affine2D,
    /// Item shapes.
    pub paths: &'a [Path],
    /// Per-item transforms applied before `master_transform`.
    pub path_transforms: &'a [Affine2D],
    /// Per-item translations in display coordinates.
    pub offsets: &'a [Point],
    /// Face colours; empty = no fills.
    pub facecolors: &'a [Rgba],
    /// Edge colours; empty = no strokes.
    pub edgecolors: &'a [Rgba],
    /// Hatch colours.
    pub hatchcolors: &'a [Rgba],
    /// Edge line widths in points.
    pub linewidths: &'a [f64],
    /// Edge dash patterns.
    pub dashes: &'a [Dashes],
    /// Per-item anti-aliasing.
    pub antialiased: &'a [bool],
    /// Per-item hyperlinks.
    pub urls: &'a [Option<String>],
}

/// How the items of a [`PathCollection`] cycle through its shapes and
/// their transforms, which matplotlib's backends do two ways.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ItemCycle {
    /// `RendererBase._iter_collection`, which the vector backends use:
    /// `max(max(paths, path_transforms), offsets)` items, item `i` drawing
    /// the raw path (a shape with its transform, see
    /// [`PathCollection::raw_paths`]) `i mod max(paths, path_transforms)`.
    #[default]
    RawPaths,
    /// Agg's `draw_path_collection`: `max(paths, offsets)` items, item `i`
    /// drawing shape `i mod paths` with transform `i mod
    /// path_transforms`, so that transforms beyond the items are never
    /// drawn.
    Agg,
}

/// One resolved item of a [`PathCollection`].
#[derive(Clone, Debug)]
pub struct CollectionItem<'a> {
    /// The shape.
    pub path: &'a Path,
    /// Index of the shape in [`PathCollection::raw_paths`] (in `paths`
    /// for items in [`ItemCycle::Agg`]).
    pub path_index: usize,
    /// Translation of the item in display coordinates.
    pub offset: Point,
    /// Full transform to display coordinates (offset included).
    pub transform: Affine2D,
    /// The graphics context with per-item overrides applied, shared with
    /// the items before it until an override changes it.
    pub gc: Rc<GraphicsContext>,
    /// Face colour, or `None` when the item is not filled.
    pub fill: Option<Rgba>,
}

impl<'a> PathCollection<'a> {
    /// Number of items that will be drawn (before skipping non-finite offsets).
    pub fn len(&self) -> usize {
        if self.paths.is_empty() {
            return 0;
        }
        self.paths
            .len()
            .max(self.path_transforms.len())
            .max(self.offsets.len())
    }

    /// Number of distinct shapes: `max(paths, path_transforms)`.
    fn raw_path_count(&self) -> usize {
        if self.paths.is_empty() {
            0
        } else {
            self.paths.len().max(self.path_transforms.len())
        }
    }

    /// Every shape with its transform before the offset (matplotlib's
    /// `_iter_collection_raw_paths`); items refer to them by
    /// [`CollectionItem::path_index`].
    pub fn raw_paths(&self) -> impl Iterator<Item = (&'a Path, Affine2D)> + '_ {
        (0..self.raw_path_count()).map(|i| {
            let own = cycled(self.path_transforms, i).copied().unwrap_or_default();
            (
                &self.paths[i % self.paths.len()],
                own.then(self.master_transform),
            )
        })
    }

    /// How often each raw path is used, rounded up (matplotlib's
    /// `_iter_collection_uses_per_path`); zero when nothing is filled or
    /// stroked.
    pub fn uses_per_path(&self) -> usize {
        let raw = self.raw_path_count();
        if raw == 0 || (self.facecolors.is_empty() && self.edgecolors.is_empty()) {
            return 0;
        }
        self.len().div_ceil(raw)
    }

    /// Returns `true` when nothing would be drawn.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
            || (self.facecolors.is_empty()
                && self.edgecolors.is_empty()
                && self.hatchcolors.is_empty())
    }

    /// Resolves every drawable item, exactly as matplotlib's
    /// `RendererBase._iter_collection` does: items with non-finite offsets
    /// are skipped, fully transparent faces become unfilled, and fully
    /// transparent edges disable stroking.
    ///
    /// The context is copied only when an override changes it while an
    /// earlier item still holds it, so items dropped before the next is
    /// taken share one context.
    pub fn items(&self, base: &GraphicsContext) -> impl Iterator<Item = CollectionItem<'a>> + '_ {
        self.items_cycled(base, ItemCycle::RawPaths)
    }

    /// [`PathCollection::items`], cycling through the shapes and their
    /// transforms as `cycle` says.
    pub fn items_cycled(
        &self,
        base: &GraphicsContext,
        cycle: ItemCycle,
    ) -> impl Iterator<Item = CollectionItem<'a>> + '_ {
        let count = match cycle {
            _ if self.is_empty() => 0,
            ItemCycle::RawPaths => self.len(),
            ItemCycle::Agg => self.paths.len().max(self.offsets.len()),
        };
        let raw_count = self.raw_path_count();
        let mut shared = Rc::new(base.clone());
        if self.edgecolors.is_empty() {
            Rc::make_mut(&mut shared).linewidth = 0.0;
        }
        (0..count).filter_map(move |i| {
            let offset = cycled(self.offsets, i).copied().unwrap_or(Point::ORIGIN);
            if !offset.is_finite() {
                return None;
            }
            if let Some(&edge) = cycled(self.edgecolors, i) {
                let gc = Rc::make_mut(&mut shared);
                if let Some(&width) = cycled(self.linewidths, i) {
                    gc.linewidth = width;
                }
                if let Some(dashes) = cycled(self.dashes, i) {
                    gc.dashes.clone_from(dashes);
                }
                if edge.is_transparent() {
                    gc.linewidth = 0.0;
                } else {
                    gc.foreground = edge;
                }
            }
            if let Some(&aa) = cycled(self.antialiased, i) {
                Rc::make_mut(&mut shared).antialiased = aa;
            }
            if let Some(url) = cycled(self.urls, i) {
                Rc::make_mut(&mut shared).url.clone_from(url);
            }
            if let Some(&color) = cycled(self.hatchcolors, i)
                && let Some(hatch) = Rc::make_mut(&mut shared).hatch.as_mut()
            {
                hatch.color = color;
            }
            let fill = cycled(self.facecolors, i)
                .copied()
                .filter(|c| !c.is_transparent());
            let (path_index, shape, own) = match cycle {
                // Items cycle through the raw paths, as matplotlib's
                // `_iter_collection` cycles through the path ids.
                ItemCycle::RawPaths => {
                    let raw = wrap(i, raw_count);
                    let shape = wrap(raw, self.paths.len());
                    (raw, shape, cycled(self.path_transforms, raw))
                }
                ItemCycle::Agg => {
                    let shape = wrap(i, self.paths.len());
                    (shape, shape, cycled(self.path_transforms, i))
                }
            };
            // Agg composes the item's transform with the master's in C++
            // (`trans *= master_transform`), the vector backends' iteration
            // with numpy.
            let own = own.copied().unwrap_or_default();
            let transform = match cycle {
                ItemCycle::RawPaths => own.then(self.master_transform),
                ItemCycle::Agg => own.agg_then(self.master_transform),
            }
            .translate(offset.x, offset.y);
            Some(CollectionItem {
                path: &self.paths[shape],
                path_index,
                offset,
                transform,
                gc: Rc::clone(&shared),
                fill,
            })
        })
    }
}

/// The arguments of a [`PathCollection`] owned rather than borrowed, for
/// a drawing kept until it is drawn (what a program drew through a
/// renderer of its own, see `pltrs::artist::Drawing`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PathCollectionData {
    /// Applied after each item's own transform.
    pub master_transform: Affine2D,
    /// Item shapes.
    pub paths: Vec<Path>,
    /// Per-item transforms applied before `master_transform`.
    pub path_transforms: Vec<Affine2D>,
    /// Per-item translations in display coordinates.
    pub offsets: Vec<Point>,
    /// Face colours; empty = no fills.
    pub facecolors: Vec<Rgba>,
    /// Edge colours; empty = no strokes.
    pub edgecolors: Vec<Rgba>,
    /// Hatch colours.
    pub hatchcolors: Vec<Rgba>,
    /// Edge line widths in points.
    pub linewidths: Vec<f64>,
    /// Edge dash patterns.
    pub dashes: Vec<Dashes>,
    /// Per-item anti-aliasing.
    pub antialiased: Vec<bool>,
    /// Per-item hyperlinks.
    pub urls: Vec<Option<String>>,
}

impl PathCollectionData {
    /// The collection these arguments describe.
    pub fn view(&self) -> PathCollection<'_> {
        PathCollection {
            master_transform: self.master_transform,
            paths: &self.paths,
            path_transforms: &self.path_transforms,
            offsets: &self.offsets,
            facecolors: &self.facecolors,
            edgecolors: &self.edgecolors,
            hatchcolors: &self.hatchcolors,
            linewidths: &self.linewidths,
            dashes: &self.dashes,
            antialiased: &self.antialiased,
            urls: &self.urls,
        }
    }
}

/// Item `i` of `items` repeated cyclically, or `None` when there are none.
fn cycled<T>(items: &[T], i: usize) -> Option<&T> {
    (!items.is_empty()).then(|| &items[wrap(i, items.len())])
}

/// `i % len`, without the division in the usual case of an index in range
/// (collections draw thousands of items per call).
fn wrap(i: usize, len: usize) -> usize {
    if i < len { i } else { i % len }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles_properties_and_skips_non_finite_offsets() {
        let paths = [Path::unit_rectangle()];
        let offsets = [
            Point::new(1.0, 1.0),
            Point::new(f64::NAN, 0.0),
            Point::new(3.0, 3.0),
        ];
        let faces = [Rgba::BLACK, Rgba::TRANSPARENT];
        let collection = PathCollection {
            paths: &paths,
            offsets: &offsets,
            facecolors: &faces,
            ..PathCollection::default()
        };
        let items: Vec<_> = collection.items(&GraphicsContext::default()).collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].fill, Some(Rgba::BLACK));
        assert_eq!(
            items[1].fill,
            Some(Rgba::BLACK),
            "third item cycles back to the first face"
        );
        assert_eq!(items[1].transform.translation_part(), Point::new(3.0, 3.0));
        assert_eq!(
            items[0].gc.linewidth, 0.0,
            "no edge colours means no stroke"
        );
    }

    #[test]
    fn transparent_edges_disable_stroke() {
        let paths = [Path::unit_rectangle(), Path::unit_circle()];
        let edges = [Rgba::BLACK, Rgba::TRANSPARENT];
        let widths = [2.0];
        let collection = PathCollection {
            paths: &paths,
            edgecolors: &edges,
            linewidths: &widths,
            ..PathCollection::default()
        };
        let items: Vec<_> = collection.items(&GraphicsContext::default()).collect();
        assert_eq!(items[0].gc.linewidth, 2.0);
        assert_eq!(items[1].gc.linewidth, 0.0);
        assert_eq!(items[1].fill, None);
    }

    #[test]
    fn agg_items_leave_extra_transforms_undrawn() {
        let paths = [Path::unit_rectangle(), Path::unit_circle()];
        let transforms = [
            Affine2D::scaling(2.0, 2.0),
            Affine2D::scaling(3.0, 3.0),
            Affine2D::scaling(4.0, 4.0),
        ];
        let offsets = [Point::ORIGIN; 4];
        let faces = [Rgba::BLACK];
        let collection = PathCollection {
            paths: &paths[..1],
            path_transforms: &transforms,
            facecolors: &faces,
            ..PathCollection::default()
        };
        let gc = GraphicsContext::default();
        assert_eq!(collection.items(&gc).count(), 3);
        let agg: Vec<_> = collection.items_cycled(&gc, ItemCycle::Agg).collect();
        assert_eq!(agg.len(), 1);
        assert_eq!(agg[0].transform, transforms[0]);
        // With more offsets than raw paths, Agg cycles the shapes and the
        // transforms apart; the raw paths cycle together.
        let collection = PathCollection {
            paths: &paths,
            offsets: &offsets,
            ..collection
        };
        let shapes = |cycle| {
            collection
                .items_cycled(&gc, cycle)
                .map(|item| (item.path.vertices().len(), item.transform))
                .collect::<Vec<_>>()
        };
        let (rect, circle) = (paths[0].vertices().len(), paths[1].vertices().len());
        assert_eq!(
            shapes(ItemCycle::Agg),
            [
                (rect, transforms[0]),
                (circle, transforms[1]),
                (rect, transforms[2]),
                (circle, transforms[0]),
            ]
        );
        assert_eq!(shapes(ItemCycle::RawPaths)[3], (rect, transforms[0]));
    }

    #[test]
    fn empty_collections_draw_nothing() {
        let collection = PathCollection::default();
        assert!(collection.is_empty());
        assert_eq!(collection.items(&GraphicsContext::default()).count(), 0);
        assert!(PathCollectionData::default().view().is_empty());
    }

    #[test]
    fn owned_arguments_describe_the_same_collection() {
        let data = PathCollectionData {
            paths: vec![Path::unit_rectangle()],
            offsets: vec![Point::new(1.0, 2.0), Point::new(3.0, 4.0)],
            facecolors: vec![Rgba::BLACK],
            ..PathCollectionData::default()
        };
        let items: Vec<_> = data.view().items(&GraphicsContext::default()).collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[1].offset, Point::new(3.0, 4.0));
    }
}
