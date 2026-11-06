//! A line's path mapped through a scale of the program's, kept as
//! matplotlib's `TransformedPath` keeps it.
//!
//! matplotlib's `Line2D.draw` maps its path through the non-affine part of
//! its transform with a `TransformedPath`, which maps the path, then its
//! vertices as points (`_revalidate`, two calls of a transform of the
//! program's), and keeps both until the line's data or its transform's
//! non-affine part change; a line drawing a slice of its points
//! (`_subslice`) makes a new one at every draw. A line whose transform
//! maps through a scale of the program's keeps them the same way, so that
//! the program is called as often (a transform of the program's mapping
//! the plane keeps its own, see `PointMap::transform_path`).

use std::cell::RefCell;
use std::rc::Rc;

use pltrs_core::Path;

use super::LineArtist;
use crate::artist::{Artist, DrawContext, LayoutContext};
use crate::transforms::ResolvedTransform;
use crate::transforms::program_calls::same_path;

/// The line's path and its vertices mapped through the non-affine part of
/// a transform, with what they were mapped from.
#[derive(Debug)]
pub(super) struct MappedPaths {
    /// The path mapped.
    given: Path,
    /// The transform it was mapped through.
    through: ResolvedTransform,
    /// The path through the non-affine part (matplotlib's
    /// `transform_path_non_affine`).
    pub(super) path: Path,
    /// Its vertices through it, as points (`transform_non_affine`).
    pub(super) points: Path,
}

/// What a line last mapped through a scale of the program's.
#[derive(Clone, Debug, Default)]
pub(crate) struct KeptPaths(RefCell<Option<Kept>>);

/// The paths a line mapped last, and whether the coming paint of the
/// figure is to draw them: mapped where matplotlib's draw maps them (see
/// `axes::tick_sites`), before pltrs paints the figure.
#[derive(Clone, Debug)]
struct Kept {
    mapped: Rc<MappedPaths>,
    for_paint: bool,
}

impl KeptPaths {
    /// `drawn` mapped through the non-affine part of `resolved` for
    /// painting: as last mapped while the path and the transform's
    /// non-affine part are the same, and the line keeps them (`keep`) or
    /// they were mapped for this paint, else mapped again, the figure free
    /// meanwhile.
    pub(super) fn mapped(
        &self,
        ctx: &mut DrawContext<'_>,
        resolved: &ResolvedTransform,
        drawn: &Path,
        keep: bool,
    ) -> Rc<MappedPaths> {
        if let Some(kept) = self.0.borrow_mut().as_mut()
            && kept.mapped.maps(resolved, drawn)
            && (keep || kept.for_paint)
        {
            kept.for_paint = false;
            return Rc::clone(&kept.mapped);
        }
        ctx.free_for(resolved);
        self.map(resolved, drawn, false)
    }

    /// Whether what was mapped last stands for `drawn` through `resolved`
    /// at a draw, the line keeping it (`keep`).
    fn kept(&self, resolved: &ResolvedTransform, drawn: &Path, keep: bool) -> bool {
        keep && (self.0.borrow().as_ref()).is_some_and(|kept| kept.mapped.maps(resolved, drawn))
    }

    /// `drawn` mapped through the non-affine part of `resolved`, the path
    /// then its points, and kept, `for_paint` when the figure's coming
    /// paint is to draw them.
    fn map(&self, resolved: &ResolvedTransform, drawn: &Path, for_paint: bool) -> Rc<MappedPaths> {
        let (path, _) = resolved.split_path(drawn);
        let (points, _) = resolved.split_points(drawn);
        let mapped = Rc::new(MappedPaths {
            given: drawn.clone(),
            through: resolved.clone(),
            path: path.into_owned(),
            points: points.into_owned(),
        });
        self.0.replace(Some(Kept {
            mapped: Rc::clone(&mapped),
            for_paint,
        }));
        mapped
    }
}

impl MappedPaths {
    /// Whether these are `drawn` mapped through the non-affine part of
    /// `resolved`.
    fn maps(&self, resolved: &ResolvedTransform, drawn: &Path) -> bool {
        same_path(&self.given, drawn) && self.through.same_steps(resolved)
    }
}

impl LineArtist {
    /// What matplotlib's `Line2D.draw` maps through a scale of the
    /// program's, the figure laid out as `layout` says: the mapping, to
    /// run with the figure free, whose paths the figure's coming paint
    /// draws, or `None` when it maps nothing (a hidden line, paths kept, a
    /// transform calling no scale of the program's).
    pub(crate) fn program_mapping(&self, layout: LayoutContext<'_>) -> Option<impl FnOnce() + '_> {
        if !self.base().visible {
            return None;
        }
        let resolved = self.drawn_transform(layout).ok()??;
        if !resolved.only_scales_call_program() {
            return None;
        }
        let subslice = self.subslice(layout);
        let keep = subslice.is_none();
        let range = subslice.filter(|range| *range != (0..self.xdata.len()));
        let drawn = self.drawn_path(range).ok()?.into_owned();
        if self.kept_paths.kept(&resolved, &drawn, keep) {
            return None;
        }
        Some(move || {
            self.kept_paths.map(&resolved, &drawn, true);
        })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use pltrs_core::{Affine2D, Path, Point};

    use super::KeptPaths;
    use crate::figure::Figure;
    use crate::transforms::{AxisMap, ResolvedTransform, Step};

    /// Doubles values, counting its calls, as a map of the program's.
    #[derive(Debug, Default)]
    struct Counted(Cell<usize>);

    impl AxisMap for Counted {
        fn forward(&self, value: f64) -> f64 {
            self.0.set(self.0.get() + 1);
            2.0 * value
        }

        fn inverse(&self, value: f64) -> f64 {
            value / 2.0
        }

        fn forward_all(&self, values: &mut [f64]) {
            self.0.set(self.0.get() + 1);
            for v in values.iter_mut() {
                *v *= 2.0;
            }
        }

        fn calls_program(&self) -> bool {
            true
        }
    }

    #[test]
    fn paths_are_mapped_twice_and_kept_while_path_and_transform_stay() {
        let map = Rc::new(Counted::default());
        let shared: Rc<dyn AxisMap> = Rc::clone(&map) as Rc<dyn AxisMap>;
        let through = |shift: f64| {
            ResolvedTransform::with_step(
                Step::Separable(None, Some(Rc::clone(&shared))),
                Affine2D::translation(shift, 0.0),
            )
        };
        let line = Path::polyline(vec![Point::new(0.0, 1.0), Point::new(1.0, 2.0)]);
        let kept = KeptPaths::default();
        let fig = Figure::new();
        let cell = Rc::clone(fig.state_cell());
        let mut renderer = crate::testing::RecordingRenderer::new(100.0, 100.0, 72.0);
        cell.paint(&mut renderer, |ctx| {
            let mapped = kept.mapped(ctx, &through(0.0), &line, true);
            assert_eq!(mapped.path.vertices()[1], Point::new(1.0, 4.0));
            assert_eq!(map.0.get(), 2, "the path, then its points");
            kept.mapped(ctx, &through(5.0), &line, true);
            assert_eq!(map.0.get(), 2, "an affine change keeps them");
            kept.mapped(ctx, &through(5.0), &line, false);
            assert_eq!(map.0.get(), 4, "a slice of a line is mapped anew");
            let moved = Path::polyline(vec![Point::new(0.0, 3.0)]);
            kept.mapped(ctx, &through(5.0), &moved, true);
            assert_eq!(map.0.get(), 6, "new data");
        });
    }
}
