//! The texts of axis artists drawn on their own (matplotlib's `LabelBase`,
//! `AxisLabel` and `TickLabels` added to an axes with `add_artist`): texts
//! at a distance from their reference points, in the direction of each,
//! turned by that direction, as the parts of an
//! [`AxisArtist`](super::AxisArtist) place theirs.

use std::rc::Rc;

use pltrs_core::{Bbox, Point};
use pltrs_text::{HAlign, VAlign};

use super::labels::{LabelKind, LabelState, PlacedTickLabels, TickLabel, axis_label, tick_labels};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, DrawContext, LayoutContext, RawHandle, TypedHandle,
};
use crate::axes::{Axes, DataUpdate, PlacedText};
use crate::colors::Color;
use crate::fonts::FontWeight;
use crate::rc::RcParams;
use crate::spines::SpineSide;
use crate::text::draw_laid_out;
use crate::transforms::Transform;

/// The z-order of the texts (matplotlib's `Text`).
const ZORDER: f64 = 3.0;

/// How far texts of [`AxisTexts`] are from their reference points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AxisTextsKind {
    /// `radius` pixels (matplotlib's `LabelBase` with its
    /// `_offset_radius`).
    Label {
        /// The distance in pixels.
        radius: f64,
    },
    /// The pad in points (matplotlib's `AxisLabel`, whose draw sets the
    /// distance from the pad).
    AxisLabel,
    /// The pad in points beyond the room the labels' alignment leaves
    /// between their anchors and the points; blank texts are not drawn
    /// (matplotlib's `TickLabels`).
    TickLabels,
}

pltrs_core::archive_enum!(AxisTextsKind {
    Label { radius },
    AxisLabel,
    TickLabels,
});

impl AxisTextsKind {
    /// The vertical and horizontal alignment and the angle texts of this
    /// kind take facing axis direction `direction` (matplotlib's
    /// `set_axis_direction`, the angle relative to 90° plus the direction
    /// to the text); plain labels follow no convention (`None`).
    ///
    /// ```
    /// use pltrs::axisartist::AxisTextsKind;
    /// use pltrs::spines::SpineSide;
    /// use pltrs::text::{HAlign, VAlign};
    /// let left = AxisTextsKind::TickLabels.direction_defaults(SpineSide::Left);
    /// assert_eq!(left, Some((VAlign::Center, HAlign::Right, 90.0)));
    /// assert_eq!(AxisTextsKind::Label { radius: 0.0 }.direction_defaults(SpineSide::Left), None);
    /// ```
    pub fn direction_defaults(self, direction: SpineSide) -> Option<(VAlign, HAlign, f64)> {
        let kind = match self {
            Self::Label { .. } => return None,
            Self::AxisLabel => LabelKind::AxisLabel,
            Self::TickLabels => LabelKind::TickLabels,
        };
        let (valign, halign) = kind.alignment(direction);
        Some((valign, halign, kind.angle(direction)))
    }
}

/// Texts placed at a distance from their reference points, in the
/// direction of each, turned by that direction, as the parts of an
/// [`AxisArtist`](super::AxisArtist) place theirs (matplotlib's
/// `LabelBase`, `AxisLabel` and `TickLabels` added to an axes).
#[derive(Clone, Debug, PartialEq)]
pub struct AxisTexts {
    /// How far the texts are from their points.
    pub kind: AxisTextsKind,
    /// The texts: each with its reference point in data coordinates and
    /// the direction from it to the text in degrees (0 along +x).
    pub texts: Vec<(Point, f64, String)>,
    /// The internal pad in points (matplotlib's `set_pad`).
    pub pad: f64,
    /// The text colour.
    pub color: Color,
    /// The font size in points.
    pub size: f64,
    /// The font weight.
    pub weight: FontWeight,
    /// The text angle in degrees, relative to 90° plus the direction to
    /// the text.
    pub rotation: f64,
    /// The vertical alignment around the placed point.
    pub valign: VAlign,
    /// The horizontal alignment around the placed point.
    pub halign: HAlign,
    /// The axis direction whose conventions tick labels measure the room
    /// their alignment leaves along (matplotlib's `_axis_direction`).
    pub direction: SpineSide,
}

pltrs_core::archive_struct!(AxisTexts {
    kind,
    texts,
    pad,
    color,
    size,
    weight,
    rotation,
    valign,
    halign,
    direction
});

impl AxisTexts {
    /// The look of the texts as the parts of an axis artist keep it.
    fn label_state(&self) -> LabelState {
        LabelState {
            kind: if self.kind == AxisTextsKind::TickLabels {
                LabelKind::TickLabels
            } else {
                LabelKind::AxisLabel
            },
            visible: true,
            pad: self.pad,
            color: Some(self.color.clone()),
            size: self.size,
            weight: self.weight,
            rotation: self.rotation,
            valign: self.valign,
            halign: self.halign,
            direction: self.direction,
        }
    }
}

/// A handle to texts drawn on their own (see [`Axes::add_axis_texts`]).
#[derive(Clone, Debug, PartialEq)]
pub struct AxisTextsArtist {
    raw: RawHandle,
}

impl ArtistHandle for AxisTextsArtist {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl TypedHandle for AxisTextsArtist {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<AxisTextsState>()
            .then(|| Self { raw: raw.clone() })
    }
}

impl AxisTextsArtist {
    /// Replaces the texts and their look.
    pub fn set_texts(&self, texts: AxisTexts) -> &Self {
        self.raw
            .with_mut::<AxisTextsState, _>(|state| state.texts = texts);
        self
    }

    /// The texts and their look.
    pub fn texts(&self) -> AxisTexts {
        self.raw
            .with::<AxisTextsState, _>(|state| state.texts.clone())
    }
}

/// Texts of an axes placed away from their points.
#[derive(Debug)]
pub(crate) struct AxisTextsState {
    base: ArtistBase,
    axes: ArtistId,
    texts: AxisTexts,
    rc: Rc<RcParams>,
}

pltrs_core::archive_struct!(AxisTextsState {
    base,
    axes,
    texts,
    rc
});

impl AxisTextsState {
    /// The texts laid out in the figure's current layout.
    fn placed(&self, layout: LayoutContext<'_>) -> Result<Vec<PlacedText>, String> {
        let transform = layout
            .resolve(&Transform::Data(self.axes))
            .ok_or_else(|| "the axes is gone".to_owned())?;
        let state = self.texts.label_state();
        let style = state.text_style(&self.rc, &self.texts.color);
        let radius = match self.texts.kind {
            AxisTextsKind::Label { radius } => radius,
            AxisTextsKind::AxisLabel => layout.points_to_pixels(self.texts.pad),
            AxisTextsKind::TickLabels => {
                let labels: Vec<TickLabel> = self
                    .texts
                    .texts
                    .iter()
                    .map(|(loc, angle, text)| TickLabel {
                        loc: *loc,
                        angle: *angle,
                        text: text.clone(),
                    })
                    .collect();
                let PlacedTickLabels { texts, .. } =
                    tick_labels(layout, &labels, &transform, &state, 0.0, &style, &self.rc)?;
                return Ok(texts);
            }
        };
        self.texts
            .texts
            .iter()
            .map(|(loc, angle, text)| {
                let reference = (transform.transform_point(*loc), *angle);
                axis_label(layout, text, reference, radius, &state, &style, &self.rc)
            })
            .collect()
    }
}

impl Artist for AxisTextsState {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        let id = self.base.id();
        let placed = match self.placed(*ctx.layout()) {
            Ok(placed) => placed,
            Err(problem) => return ctx.report(id, problem),
        };
        for text in &placed {
            let mut gc = ctx.base_gc(&self.base);
            gc.foreground = text.color;
            gc.antialiased = self.rc.text_antialiased;
            if let Err(err) = draw_laid_out(ctx, &gc, &text.laid_out, text.anchor, text.rotation) {
                ctx.report(id, err);
            }
        }
    }

    /// The box around the texts (matplotlib's `get_window_extent` of
    /// each).
    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        self.placed(*ctx)
            .ok()?
            .iter()
            .map(PlacedText::bbox)
            .reduce(|a, b| a.union(&b))
    }
}

impl Axes {
    /// Adds `texts`, clipped to the axes and leaving its data limits
    /// alone (matplotlib's `ax.add_artist` of a `LabelBase`, `AxisLabel`
    /// or `TickLabels`), styled from the rc parameters where `texts`
    /// says nothing.
    ///
    /// ```
    /// use pltrs::axisartist::{AxisTexts, AxisTextsKind};
    /// use pltrs::fonts::FontWeight;
    /// use pltrs::prelude::*;
    /// use pltrs::spines::SpineSide;
    /// use pltrs::text::{HAlign, VAlign};
    /// use pltrs::transforms::Point;
    /// let ax = Figure::new().subplot();
    /// let label = ax.add_axis_texts(AxisTexts {
    ///     kind: AxisTextsKind::Label { radius: 50.0 },
    ///     texts: vec![(Point::new(0.5, 0.5), -90.0, "Test".to_owned())],
    ///     pad: 5.0,
    ///     color: "black".into(),
    ///     size: 10.0,
    ///     weight: FontWeight::NORMAL,
    ///     rotation: -90.0,
    ///     valign: VAlign::Top,
    ///     halign: HAlign::Center,
    ///     direction: SpineSide::Bottom,
    /// });
    /// assert!(label.window_extent().is_some());
    /// ```
    pub fn add_axis_texts(&self, texts: AxisTexts) -> AxisTextsArtist {
        let state = AxisTextsState {
            base: ArtistBase::with_zorder(ZORDER),
            axes: self.id(),
            texts,
            rc: crate::rc::params(),
        };
        AxisTextsArtist {
            raw: self.adopt(state, &DataUpdate::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    fn texts(kind: AxisTextsKind, texts: Vec<(Point, f64, String)>) -> AxisTexts {
        AxisTexts {
            kind,
            texts,
            pad: 10.0,
            color: Color::from("black"),
            size: 10.0,
            weight: FontWeight::NORMAL,
            rotation: 0.0,
            valign: VAlign::Center,
            halign: HAlign::Center,
            direction: SpineSide::Bottom,
        }
    }

    /// An axes over the unit square filling a 100 × 100 px figure at 72
    /// dpi, so that points are pixels.
    fn unit_axes() -> Axes {
        Figure::with_size(100.0 / 72.0, 100.0 / 72.0, 72.0)
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle")
    }

    fn centre(bbox: Bbox) -> Point {
        Point::new(bbox.x0().midpoint(bbox.x1()), bbox.y0().midpoint(bbox.y1()))
    }

    #[test]
    fn labels_sit_their_radius_away_towards_their_angle() {
        let ax = unit_axes();
        let at = vec![(Point::new(0.5, 0.5), 0.0, "x".to_owned())];
        let label = ax.add_axis_texts(texts(AxisTextsKind::Label { radius: 20.0 }, at.clone()));
        let moved = centre(label.window_extent().expect("measured"));
        assert!(
            (moved.x - 70.0).abs() < 1e-6 && (moved.y - 50.0).abs() < 1.0,
            "{moved:?}"
        );
        label.set_texts(texts(AxisTextsKind::AxisLabel, at));
        let padded = centre(label.window_extent().expect("measured"));
        assert!(
            (padded.x - 60.0).abs() < 1e-6,
            "the pad of 10 points: {padded:?}"
        );
        assert_eq!(label.texts().kind, AxisTextsKind::AxisLabel);
    }

    #[test]
    fn tick_labels_skip_blank_texts() {
        let ax = unit_axes();
        let labels = ax.add_axis_texts(texts(
            AxisTextsKind::TickLabels,
            vec![
                (Point::new(0.2, 0.5), -90.0, "0.2".to_owned()),
                (Point::new(0.4, 0.5), -90.0, " ".to_owned()),
            ],
        ));
        let extent = labels.window_extent().expect("measured");
        assert!(extent.x1() < 40.0, "only the first label: {extent:?}");
        let empty = ax.add_axis_texts(texts(AxisTextsKind::TickLabels, Vec::new()));
        assert_eq!(empty.window_extent(), None);
    }
}
