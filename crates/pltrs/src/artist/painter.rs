//! Artists the program draws in place of pltrs (matplotlib's `Artist`
//! subclasses overriding `draw`): a [`Painter`] set on any artist.
//!
//! An artist with a painter ([`RawHandle::set_painter`]) is drawn by the
//! painter wherever pltrs draws it, at its place in the z-order. The
//! painter draws through the context's renderer, and may draw the artist
//! as pltrs draws it ([`DrawContext::draw_unpainted`], matplotlib's
//! `super().draw(renderer)`), or other artists. Before a figure's
//! artists are drawn, with the figure laid out and not borrowed, every
//! painter the draw reaches is [prepared](ArtistPainter::prepare), in
//! drawing order: it may change the figure then, as matplotlib's `draw`
//! methods change their artists, and record what it will draw
//! ([`Drawing`](super::Drawing)). Before a figure is laid out or its
//! tight box computed, painters [measure](ArtistPainter::measure) their
//! artists, which layouts then take as they measured them. Figures without
//! painters cost a scan of their artists' properties per layout and per
//! draw.
//!
//! ```
//! use pltrs::artist::{ArtistId, ArtistPainter, DrawContext, Painter};
//! use pltrs::prelude::*;
//! use pltrs::path::Path;
//! use pltrs::transforms::{Affine2D, Point};
//!
//! /// Draws a line, then a cross at the origin of the canvas.
//! #[derive(Debug)]
//! struct Crossed;
//!
//! impl ArtistPainter for Crossed {
//!     fn draw(&self, ctx: &mut DrawContext<'_>, artist: ArtistId) {
//!         ctx.draw_unpainted(artist);
//!         let gc = pltrs::artist::GraphicsContext::default();
//!         let stroke = Path::polyline(vec![Point::new(0.0, 0.0), Point::new(10.0, 10.0)]);
//!         ctx.renderer().draw_path(&gc, &stroke, &Affine2D::IDENTITY, None);
//!     }
//! }
//!
//! let fig = Figure::new();
//! let line = fig.subplot().plot([0.0, 1.0], [0.0, 1.0])?;
//! line.raw().set_painter(Some(Painter::new(Crossed)));
//! fig.to_png()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::any::Any;
use std::fmt;
use std::rc::Rc;

use pltrs_core::Bbox;
use pltrs_core::archive::{Archive, ArchiveError, Decoder, Encoder};
use pltrs_render::Renderer;

use super::{Artist, ArtistId, ArtistLookup, DrawContext, LayoutContext, RawHandle};
use crate::axes::AxesState;
use crate::figure::FigureState;

/// What the coming draw of a painted artist draws with (see
/// [`ArtistPainter::prepare`]).
#[derive(Clone, Copy)]
pub struct DrawTarget<'a> {
    /// The renderer the figure is drawn with.
    pub renderer: &'a dyn Renderer,
    /// How many rasterized artists the artist is drawn in, itself
    /// included when its painter [rasterizes](ArtistPainter::rasterizes)
    /// it (matplotlib's `renderer._raster_depth` while it draws).
    pub raster_depth: usize,
}

impl fmt::Debug for DrawTarget<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DrawTarget")
            .field("raster_depth", &self.raster_depth)
            .finish_non_exhaustive()
    }
}

/// The program's drawing of an artist (matplotlib's `draw` of an
/// `Artist` subclass).
pub trait ArtistPainter: Any + fmt::Debug {
    /// Runs before the figure's artists are drawn, once the figure is
    /// laid out, with no borrow of it held, for a draw that will reach
    /// `artist`: the painter may change the figure, the artist included,
    /// and record what [`draw`](Self::draw) is to draw. Does nothing by
    /// default.
    fn prepare(&self, _artist: &RawHandle, _target: DrawTarget<'_>) {}

    /// Draws artist `artist` where pltrs draws it, whether it is visible
    /// or not (matplotlib's `draw` decides): through `ctx`'s renderer,
    /// the artist as pltrs draws it with [`DrawContext::draw_unpainted`].
    fn draw(&self, ctx: &mut DrawContext<'_>, artist: ArtistId);

    /// Whether the artist is rasterized and filtered around the painter's
    /// drawing as pltrs draws its own artists, by its `rasterized` and
    /// `agg_filter` properties (matplotlib's `allow_rasterization` on the
    /// subclass' `draw`); otherwise drawing it only ends the
    /// rasterization of the artists drawn before. `false` by default.
    fn rasterizes(&self) -> bool {
        false
    }

    /// Measures `artist` before the figure is laid out or its tight box
    /// is computed, with no borrow of the figure held: what
    /// [`measured_extent`](Self::measured_extent) gives afterwards. Does
    /// nothing by default.
    fn measure(&self, _artist: &RawHandle) {}

    /// The artist's extent as the painter last measured it (matplotlib's
    /// `get_window_extent` of a subclass), which layouts and tight boxes
    /// take in place of the artist's own; [`MeasuredExtent::Unmeasured`]
    /// by default.
    fn measured_extent(&self) -> MeasuredExtent {
        MeasuredExtent::Unmeasured
    }

    /// Called where a tight box or a layout takes the extent the painter
    /// measured (matplotlib's call of the artist's `get_tightbbox`
    /// there), before [`measured_extent`](Self::measured_extent): a
    /// painter whose measuring failed reports the failure then, after
    /// what matplotlib measures first (an axes' ticks before its
    /// artists), and not for an artist the layout leaves out. Does
    /// nothing by default.
    fn extent_taken(&self) {}

    /// The extent of `artist`, an offset box of the program's laid out by
    /// the box holding it ([`ProgramBox`](crate::offsetbox::ProgramBox)),
    /// relative to its origin, in display pixels (matplotlib's
    /// `get_bbox(renderer)`): asked once the figure is laid out, with no
    /// borrow of it held, before the box holding it is laid out again
    /// with it. `None` (the default) keeps the extent it had.
    fn box_extent(&self, _artist: &RawHandle) -> Option<Bbox> {
        None
    }
}

/// What a painter measured of its artist (see
/// [`ArtistPainter::measured_extent`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum MeasuredExtent {
    /// The painter does not measure: the artist's own extent counts.
    #[default]
    Unmeasured,
    /// The artist occupies no space.
    Empty,
    /// The artist occupies this box, in display coordinates.
    Extent(Bbox),
}

/// The extent of `artist` that tight boxes and layouts take, before its
/// clipping: the one its painter measured, if it measures, else the
/// artist's own ([`Artist::tight_extent`]).
pub(crate) fn tight_extent_of(artist: &dyn Artist, layout: &LayoutContext<'_>) -> Option<Bbox> {
    let measured = artist
        .base()
        .painter
        .as_ref()
        .map_or(MeasuredExtent::Unmeasured, |painter| {
            painter.get().extent_taken();
            painter.get().measured_extent()
        });
    match measured {
        MeasuredExtent::Unmeasured => artist.tight_extent(layout),
        MeasuredExtent::Empty => None,
        MeasuredExtent::Extent(extent) => Some(extent),
    }
}

pltrs_core::archive_kinds!(dyn ArtistPainter {});

/// The painter of an artist (see [`ArtistPainter`]), shared by its
/// clones. A figure with a painter is archived only if the program
/// archives the painter's type (its [`ProgramKinds`]).
///
/// [`ProgramKinds`]: pltrs_core::archive::ProgramKinds
#[derive(Clone, Debug)]
pub struct Painter(Rc<dyn ArtistPainter>);

impl Painter {
    /// The painter `painter`.
    pub fn new(painter: impl ArtistPainter) -> Self {
        Self(Rc::new(painter))
    }

    /// What paints.
    pub fn get(&self) -> &dyn ArtistPainter {
        &*self.0
    }
}

impl PartialEq for Painter {
    /// Painters are equal when they are the same painter.
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Archive for Painter {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        self.0.encode(encoder)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        Archive::decode(decoder).map(Self)
    }
}

impl RawHandle {
    /// Has `painter` draw the artist wherever pltrs draws it, or pltrs
    /// again with `None` (see [`ArtistPainter`]).
    pub fn set_painter(&self, painter: Option<Painter>) {
        self.with_base_mut(|base| base.painter = painter);
    }

    /// The artist's painter, if it has one.
    pub fn painter(&self) -> Option<Painter> {
        self.with_base(|base| base.painter.clone())
    }

    /// Prepares the painters a draw of the artist through `renderer`
    /// reaches (see [`ArtistPainter::prepare`]): those of the artists it
    /// owns and, with `itself`, its own.
    pub(crate) fn prepare_painters(&self, renderer: &dyn Renderer, itself: bool) {
        let painted = {
            let state = self.figure_state().borrow();
            if !has_painters(&state) {
                return;
            }
            painted_under(&state, self.id(), itself)
        };
        for (id, painter, raster_depth) in painted {
            let target = DrawTarget {
                renderer,
                raster_depth,
            };
            painter.get().prepare(&self.sibling(id), target);
        }
    }
}

/// Whether an artist of the figure, in it or removed, has a painter.
fn has_painters(state: &FigureState) -> bool {
    let painted = |id| {
        state
            .artist(id)
            .is_some_and(|artist| artist.base().painter.is_some())
    };
    state.store.ids().any(painted) || state.detached.ids().any(painted)
}

/// The painted artists a draw of artist `root` reaches, in drawing order
/// (children by z-order, as containers draw them), with their raster
/// depths: those under artists that draw (visible ones, animated ones
/// left out as the figure and its axes leave them out), `root` itself
/// only with `itself`.
fn painted_under(
    state: &FigureState,
    root: ArtistId,
    itself: bool,
) -> Vec<(ArtistId, Painter, usize)> {
    let layout = LayoutContext::new(state);
    let mut found = Vec::new();
    let mut pending = vec![(root, 0)];
    while let Some((id, depth)) = pending.pop() {
        let Some(artist) = state.artist(id) else {
            continue;
        };
        let base = artist.base();
        let painter = base.painter.as_ref().filter(|_| itself || id != root);
        let depth =
            depth + usize::from(base.rasterized && painter.is_none_or(|p| p.get().rasterizes()));
        match painter {
            Some(painter) => found.push((id, painter.clone(), depth)),
            None if !base.visible => continue,
            None => {}
        }
        let in_axes = state.store.get_as::<AxesState>(id).is_some();
        let mut children: Vec<(f64, ArtistId)> = artist
            .children()
            .into_iter()
            .filter(|&child| !layout.skips_animated(child, in_axes))
            .filter_map(|child| Some((state.artist(child)?.base().zorder, child)))
            .collect();
        children.sort_by(|a, b| a.0.total_cmp(&b.0));
        pending.extend(children.into_iter().rev().map(|(_, child)| (child, depth)));
    }
    found
}

impl DrawContext<'_> {
    /// Draws `artist`, of id `id`, through its `painter`, whether it is
    /// visible or not, as [`DrawContext::draw_child`] draws a painted
    /// artist: rasterized and filtered when the painter
    /// [rasterizes](ArtistPainter::rasterizes) it.
    pub(super) fn draw_painted(&mut self, id: ArtistId, artist: &dyn Artist, painter: &Painter) {
        let base = artist.base();
        let painter = painter.get();
        let rasterizes = painter.rasterizes();
        self.draw_artist(rasterizes && base.rasterized, |ctx| {
            match base.agg_filter.as_ref().filter(|_| rasterizes) {
                Some(filter) => {
                    ctx.renderer().start_filter();
                    painter.draw(ctx, id);
                    ctx.renderer().stop_shared_filter(filter.shared());
                }
                None => painter.draw(ctx, id),
            }
            ctx.layout().figure_state().mark_drawn(id, true);
        });
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use pltrs_core::{Affine2D, Path, Point};
    use pltrs_render::GraphicsContext;

    use super::*;
    use crate::artist::{ArtistBase, ArtistHandle};
    use crate::figure::{Figure, SaveOptions};

    /// Records the artists it prepares and draws, drawing each natively.
    #[derive(Debug, Default)]
    struct Recorder {
        prepared: RefCell<Vec<(ArtistId, usize)>>,
        drawn: RefCell<Vec<ArtistId>>,
        rasterizes: bool,
    }

    /// The recorder `painter` paints with.
    fn recorder(painter: &Painter) -> &Recorder {
        let any: &dyn Any = painter.get();
        any.downcast_ref().expect("a recorder")
    }

    impl ArtistPainter for Recorder {
        fn prepare(&self, artist: &RawHandle, target: DrawTarget<'_>) {
            // The figure is not borrowed: the painter may change it.
            artist.set_zorder(artist.zorder());
            self.prepared
                .borrow_mut()
                .push((artist.id(), target.raster_depth));
        }

        fn draw(&self, ctx: &mut DrawContext<'_>, artist: ArtistId) {
            self.drawn.borrow_mut().push(artist);
            ctx.draw_unpainted(artist);
        }

        fn rasterizes(&self) -> bool {
            self.rasterizes
        }
    }

    #[test]
    fn painters_are_prepared_then_draw_at_their_place() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let top = ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        let bottom = ax.plot([0.0, 1.0], [1.0, 0.0]).unwrap();
        bottom.set_zorder(0.5);
        let painter = Painter::new(Recorder::default());
        top.raw().set_painter(Some(painter.clone()));
        bottom.raw().set_painter(Some(painter.clone()));
        fig.to_png().unwrap();
        let order = [bottom.id(), top.id()];
        let prepared: Vec<ArtistId> = recorder(&painter)
            .prepared
            .borrow()
            .iter()
            .map(|&(id, _)| id)
            .collect();
        assert_eq!(prepared, order, "in drawing order");
        assert_eq!(*recorder(&painter).drawn.borrow(), order);
        assert_eq!(top.raw().painter(), Some(painter));
        let archived = top.raw().archive_figure(&pltrs_core::archive::NoProgram);
        assert!(
            matches!(archived, Err(ArchiveError::Unsupported(_))),
            "a painter of a type the program does not archive"
        );
        top.raw().set_painter(None);
        bottom.raw().set_painter(None);
        assert_eq!(top.raw().painter(), None);
        assert!(
            top.raw()
                .archive_figure(&pltrs_core::archive::NoProgram)
                .is_ok()
        );
    }

    #[test]
    fn invisible_painted_artists_are_still_painted_but_not_below_hidden_ones() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let line = ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        let painter = Painter::new(Recorder::default());
        line.raw().set_painter(Some(painter.clone()));
        line.set_visible(false);
        fig.to_png().unwrap();
        assert_eq!(recorder(&painter).drawn.borrow().len(), 1);
        ax.set_visible(false);
        fig.to_png().unwrap();
        assert_eq!(recorder(&painter).prepared.borrow().len(), 1);
        assert_eq!(recorder(&painter).drawn.borrow().len(), 1);
    }

    #[test]
    fn raster_depths_count_rasterized_containers() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_rasterized(true);
        let line = ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        line.set_rasterized(true);
        let painter = Painter::new(Recorder {
            rasterizes: true,
            ..Recorder::default()
        });
        line.raw().set_painter(Some(painter.clone()));
        fig.to_png().unwrap();
        assert_eq!(
            *recorder(&painter).prepared.borrow(),
            [(line.id(), 2)],
            "the axes and the line"
        );
    }

    /// Draws a cross over the canvas without drawing its artist.
    #[derive(Debug)]
    struct Cross(Cell<usize>);

    impl ArtistPainter for Cross {
        fn draw(&self, ctx: &mut DrawContext<'_>, _artist: ArtistId) {
            self.0.set(self.0.get() + 1);
            let gc = GraphicsContext {
                linewidth: 4.0,
                ..GraphicsContext::default()
            };
            let (width, height) = ctx.layout().canvas_size();
            let stroke = Path::polyline(vec![Point::ORIGIN, Point::new(width, height)]);
            ctx.renderer()
                .draw_path(&gc, &stroke, &Affine2D::IDENTITY, None);
        }
    }

    /// An artist drawing nothing of its own.
    #[derive(Clone, Debug, Default)]
    struct Plain {
        base: ArtistBase,
    }

    impl Artist for Plain {
        fn base(&self) -> &ArtistBase {
            &self.base
        }

        fn base_mut(&mut self) -> &mut ArtistBase {
            &mut self.base
        }

        fn draw(&self, _ctx: &mut DrawContext<'_>) {}
    }

    #[test]
    fn a_painter_replaces_the_artists_own_drawing() {
        let fig = Figure::with_size(1.0, 1.0, 20.0);
        let plain = fig.add_custom_artist(Plain::default());
        let cross = Painter::new(Cross(Cell::new(0)));
        plain.raw().set_painter(Some(cross.clone()));
        let image = fig
            .render_raster_with(SaveOptions::default())
            .unwrap()
            .to_image();
        assert!(image.pixel(10, 9).is_some_and(|p| p[0] < 64), "the cross");
        let any: &dyn Any = cross.get();
        let calls = any.downcast_ref::<Cross>().map(|cross| cross.0.get());
        assert_eq!(calls, Some(1));
    }

    /// Measures its artist as a fixed box, counting its measures.
    #[derive(Debug)]
    struct Measured(Cell<usize>);

    impl ArtistPainter for Measured {
        fn draw(&self, _ctx: &mut DrawContext<'_>, _artist: ArtistId) {}

        fn measure(&self, _artist: &RawHandle) {
            self.0.set(self.0.get() + 1);
        }

        fn measured_extent(&self) -> MeasuredExtent {
            MeasuredExtent::Extent(Bbox::from_extents(-50.0, -50.0, 10.0, 10.0))
        }
    }

    #[test]
    fn tight_boxes_take_what_painters_measure() {
        let fig = Figure::with_size(2.0, 2.0, 100.0);
        let plain = fig.add_custom_artist(Plain::default());
        let measured = Painter::new(Measured(Cell::new(0)));
        plain.raw().set_painter(Some(measured.clone()));
        assert_eq!(fig.tight_bbox(), Bbox::from_extents(-0.5, -0.5, 0.1, 0.1));
        let any: &dyn Any = measured.get();
        let count = any.downcast_ref::<Measured>().map(|m| m.0.get());
        assert_eq!(count, Some(1), "measured before the tight box");
    }

    #[test]
    fn draws_measure_painters_for_engines_measuring_alone() {
        let fig = Figure::with_size(2.0, 2.0, 100.0);
        // An artist of an axes: layouts measure the axes, not the figure's
        // own artists.
        let plain = fig.subplot().add_custom_artist(Plain::default());
        let measured = Painter::new(Measured(Cell::new(0)));
        plain.raw().set_painter(Some(measured.clone()));
        let count = || {
            let any: &dyn Any = measured.get();
            any.downcast_ref::<Measured>().map(|m| m.0.get())
        };
        fig.layout();
        assert_eq!(count(), Some(0), "no engine asks for tight boxes");
        fig.set_layout_engine(crate::layout::LayoutEngine::tight())
            .unwrap();
        fig.layout();
        assert_eq!(count(), Some(1), "a tight layout measures");
        fig.set_layout_engine(None).unwrap();
        fig.layout();
        assert_eq!(count(), Some(1), "its placeholder does not");
    }
}
