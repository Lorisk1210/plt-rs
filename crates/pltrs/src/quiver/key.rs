//! A labelled reference arrow for a quiver plot (matplotlib's
//! `QuiverKey`).

use std::rc::Rc;

use pltrs_core::{Affine2D, Path, Point};
use pltrs_text::{HAlign, VAlign};

use super::Pivot;
use super::geometry::{AxesFrame, QuiverGeometry};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, DrawContext, LayoutContext, RawHandle, TypedHandle,
};
use crate::collections::{CollectionArtist, CollectionKind, CollectionProps, EdgeColors};
use crate::colors::Color;
use crate::fonts::FontProperties;
use crate::rc::RcParams;
use crate::text::{Text, TextArtist};
use crate::transforms::Transform;

/// Where a quiver key's label sits relative to its arrow (matplotlib's
/// `labelpos`); it also decides which point of the arrow is placed on the
/// key position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LabelPosition {
    /// Above the arrow, which is centred on the position.
    #[default]
    North,
    /// Below the arrow, which is centred on the position.
    South,
    /// Right of the arrow, whose tip is on the position.
    East,
    /// Left of the arrow, whose tail is on the position.
    West,
}

pltrs_core::archive_enum!(LabelPosition {
    North,
    South,
    East,
    West,
});

impl LabelPosition {
    /// The arrow point on the key position.
    fn pivot(self) -> Pivot {
        match self {
            Self::North | Self::South => Pivot::Middle,
            Self::East => Pivot::Tip,
            Self::West => Pivot::Tail,
        }
    }

    /// The label alignment.
    fn alignment(self) -> (HAlign, VAlign) {
        match self {
            Self::North => (HAlign::Center, VAlign::Bottom),
            Self::South => (HAlign::Center, VAlign::Top),
            Self::East => (HAlign::Left, VAlign::Center),
            Self::West => (HAlign::Right, VAlign::Center),
        }
    }

    /// The label offset from the key position, `sep` away from the arrow.
    fn shift(self, sep: f64) -> (f64, f64) {
        match self {
            Self::North => (0.0, sep),
            Self::South => (0.0, -sep),
            Self::East => (sep, 0.0),
            Self::West => (-sep, 0.0),
        }
    }
}

/// The coordinate system of a quiver key's position (matplotlib's
/// `coordinates`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KeyCoordinates {
    /// Fractions of the axes.
    #[default]
    Axes,
    /// Fractions of the figure (or subfigure) containing the axes.
    Figure,
    /// Data coordinates of the axes.
    Data,
    /// Inches from the lower left corner of the canvas.
    Inches,
}

/// Options of `Axes::quiverkey`.
#[derive(Clone, Debug, PartialEq)]
pub struct QuiverKeyOptions {
    /// Arrow angle in degrees counter-clockwise from the x axis.
    pub angle: f64,
    /// Coordinate system of the key position.
    pub coordinates: KeyCoordinates,
    /// Arrow colour (face and edge) instead of the quiver's.
    pub color: Option<Color>,
    /// Gap between the arrow and the label in inches.
    pub labelsep: f64,
    /// Where the label sits.
    pub labelpos: LabelPosition,
    /// Label colour (`None` = `text.color`).
    pub labelcolor: Option<Color>,
    /// Label font (`None` = the rc defaults).
    pub font: Option<FontProperties>,
    /// Drawing order (`None` = 0.1 above the quiver).
    pub zorder: Option<f64>,
    /// Properties of the arrow's polygon collection in place of the
    /// quiver's (matplotlib's `**kwargs` of `quiverkey`), `color` still
    /// overriding its colours.
    pub props: CollectionProps,
}

impl Default for QuiverKeyOptions {
    fn default() -> Self {
        Self {
            angle: 0.0,
            coordinates: KeyCoordinates::Axes,
            color: None,
            labelsep: 0.1,
            labelpos: LabelPosition::North,
            labelcolor: None,
            font: None,
            zorder: None,
            props: CollectionProps::default(),
        }
    }
}

/// The reference vector of a key.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct KeyVector {
    /// Where the key sits, in `transform`.
    pub(crate) position: Point,
    /// The vector length in the quiver's data units.
    pub(crate) length: f64,
    /// The vector angle in degrees.
    pub(crate) angle: f64,
}

pltrs_core::archive_struct!(KeyVector {
    position,
    length,
    angle
});

/// State of a quiver key: the reference arrow drawn with the quiver's
/// current scale and style, and its label (a child text artist).
#[derive(Debug)]
pub(crate) struct QuiverKeyArtist {
    base: ArtistBase,
    quiver: ArtistId,
    vector: KeyVector,
    /// Coordinates of the key position.
    transform: Transform,
    color: Option<Color>,
    /// The arrow's own properties (see [`QuiverKeyOptions::props`]).
    props: CollectionProps,
    labelpos: LabelPosition,
    label: ArtistId,
    rc: Rc<RcParams>,
}

pltrs_core::archive_struct!(QuiverKeyArtist {
    base,
    quiver,
    vector,
    transform,
    color,
    props,
    labelpos,
    label,
    rc
});

impl QuiverKeyArtist {
    /// The key of the quiver `quiver`, labelled by the text artist
    /// `label`, at z-order 0 (set it after creation).
    pub(crate) fn new(
        quiver: ArtistId,
        vector: KeyVector,
        transform: Transform,
        options: &QuiverKeyOptions,
        label: ArtistId,
    ) -> Self {
        Self {
            base: ArtistBase::default(),
            quiver,
            vector,
            transform,
            color: options.color.clone(),
            props: options.props.clone(),
            labelpos: options.labelpos,
            label,
            rc: crate::rc::params(),
        }
    }

    /// The label text artist of a key at `vector.position` in `transform`,
    /// `labelsep` inches away from the arrow.
    pub(crate) fn label_artist(
        vector: &KeyVector,
        transform: &Transform,
        text: String,
        options: &QuiverKeyOptions,
    ) -> TextArtist {
        let (dx, dy) = options.labelpos.shift(options.labelsep);
        let mut label = TextArtist::new(vector.position, text, crate::rc::params());
        (label.halign, label.valign) = options.labelpos.alignment();
        if let Some(color) = &options.labelcolor {
            label.color = color.clone();
        }
        if let Some(font) = &options.font {
            label.font = font.clone();
        }
        label.base_mut().transform = Some(transform.clone().offset_inches(dx, dy));
        label
    }

    /// The quiver of the key and the geometry of its arrows, in the layout
    /// `layout`.
    fn quiver<'a>(
        &self,
        layout: LayoutContext<'a>,
    ) -> Result<(&'a CollectionArtist, &'a QuiverGeometry), String> {
        let gone = || "its quiver no longer exists".to_owned();
        let quiver = layout
            .store()
            .get_as::<CollectionArtist>(self.quiver)
            .ok_or_else(gone)?;
        let geometry = quiver
            .draw_time_state::<QuiverGeometry>()
            .ok_or_else(gone)?;
        Ok((quiver, geometry))
    }

    /// The arrow as a one-item collection styled like the quiver
    /// (matplotlib's `QuiverKey.vector`), in the layout `layout`.
    fn vector(&self, layout: LayoutContext<'_>) -> Result<CollectionArtist, String> {
        let (quiver, geometry) = self.quiver(layout)?;
        let frame = AxesFrame::new(geometry.axes(), layout)?;
        let radians = self.vector.angle.to_radians();
        let uv = (
            self.vector.length * radians.cos(),
            self.vector.length * radians.sin(),
        );
        let (outline, trans_scale) = geometry.key_arrow(&frame, uv, self.labelpos.pivot());
        // The key's polygon collection closes its outline.
        let arrow = Path::polygon(outline);
        let mut vector =
            CollectionArtist::new(CollectionKind::Poly, vec![arrow], Rc::clone(&self.rc));
        vector.facecolors.clone_from(&quiver.facecolors);
        vector.edgecolors = quiver.edgecolors.clone();
        vector.linewidths.clone_from(&quiver.linewidths);
        vector.linestyles.clone_from(&quiver.linestyles);
        vector.antialiased.clone_from(&quiver.antialiased);
        vector.hatch.clone_from(&quiver.hatch);
        vector.base_mut().alpha = quiver.base().alpha;
        self.props.clone().apply(&mut vector);
        if let Some(color) = &self.color {
            vector.facecolors = Some(vec![color.clone()]);
            vector.edgecolors = EdgeColors::Colors(vec![color.clone()]);
        }
        vector.base_mut().transform = Some(Transform::Affine(Affine2D::scaling(
            trans_scale,
            trans_scale,
        )));
        vector.offsets = vec![self.vector.position];
        vector.offset_transform = Some(self.transform.clone());
        Ok(vector)
    }
}

impl Artist for QuiverKeyArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    /// matplotlib's `QuiverKey.draw`, whose `_init` runs the quiver's
    /// (which sizes its arrows) before the key's arrow is made.
    fn draw(&self, ctx: &mut DrawContext<'_>) {
        let layout = *ctx.layout();
        if let Err(problem) = self
            .quiver(layout)
            .and_then(|(_, geometry)| {
                AxesFrame::new(geometry.axes(), layout).map(|frame| geometry.record_sizes(&frame))
            })
            .and_then(|_| self.vector(layout))
            .and_then(|vector| vector.render(ctx))
        {
            ctx.report(self.base.id(), problem);
        }
        ctx.draw_child(self.label);
    }

    fn children(&self) -> Vec<ArtistId> {
        vec![self.label]
    }
}

/// A handle to a quiver key (matplotlib's `QuiverKey`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuiverKey {
    raw: RawHandle,
}

impl ArtistHandle for QuiverKey {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl TypedHandle for QuiverKey {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<QuiverKeyArtist>()
            .then(|| Self::from_raw(raw.clone()))
    }
}

impl QuiverKey {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    /// The label.
    pub fn text(&self) -> Text {
        let label = self.raw.with::<QuiverKeyArtist, _>(|key| key.label);
        Text::from_raw(RawHandle::new(Rc::clone(self.raw.figure_state()), label))
    }

    /// The outline of the key's arrow, in arrow-width units scaled to
    /// pixels, in the figure's current layout (laid out first): the paths
    /// of the one-item collection the key draws (matplotlib's
    /// `QuiverKey.vector.get_paths()` after a draw). Empty once the key or
    /// its quiver was removed.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::quiver::{Angles, QuiverKeyOptions, QuiverOptions, VectorField};
    /// let ax = Figure::new().subplot();
    /// let field = VectorField::new([0.0, 1.0], [0.0, 1.0], [1.0], [1.0]);
    /// let options = QuiverOptions { angles: Angles::Degrees(vec![1.0]), ..QuiverOptions::default() };
    /// let q = ax.quiver(field, options)?;
    /// let key = ax.quiverkey(&q, (1.0, 1.0), 2.0, "Label", QuiverKeyOptions::default());
    /// assert_eq!(key.vector_paths().len(), 1);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn vector_paths(&self) -> Vec<Path> {
        let state = self.raw.figure_state().prepared();
        let layout = LayoutContext::new(&state);
        state
            .store
            .get_as::<QuiverKeyArtist>(self.raw.id())
            .and_then(|key| key.vector(layout).ok())
            .map(|vector| vector.paths)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_positions_place_arrow_and_text() {
        assert_eq!(LabelPosition::East.pivot(), Pivot::Tip);
        assert_eq!(LabelPosition::West.pivot(), Pivot::Tail);
        assert_eq!(LabelPosition::South.pivot(), Pivot::Middle);
        assert_eq!(
            LabelPosition::West.alignment(),
            (HAlign::Right, VAlign::Center)
        );
        assert_eq!(LabelPosition::South.shift(0.1), (0.0, -0.1));
    }
}
