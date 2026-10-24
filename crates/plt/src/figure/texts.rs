//! Figure-level texts: free texts placed in figure fractions and the
//! figure-wide title and labels (matplotlib's `Figure.text`, `suptitle`,
//! `supxlabel`, `supylabel` and `Figure.texts`).

use std::rc::Rc;

use plt_core::Point;
use plt_text::{HAlign, VAlign};

use super::{Figure, FigureFrame, SuperText};
use crate::artist::{Artist, ArtistHandle, ArtistId, ArtistLookup, RawHandle};
use crate::fonts::FontProperties;
use crate::rc::RcParams;
use crate::text::{Text, TextArtist};
use crate::transforms::Transform;

impl Figure {
    /// Sets the figure title (matplotlib's `suptitle`) and returns it.
    ///
    /// Like matplotlib's, each call puts the title back in its default
    /// place, alignment and `figure.title*` font size and weight, and
    /// hands its placement back to the layout engine; place it by hand
    /// afterwards with [`Figure::set_suptitle_y`].
    pub fn suptitle(&self, text: impl Into<String>) -> Text {
        self.super_text(SuperKind::Title, text.into())
    }

    /// Sets the figure-wide x label (`supxlabel`) and returns it, reset
    /// as [`Figure::suptitle`] resets the title; see
    /// [`Figure::set_supxlabel_y`].
    pub fn supxlabel(&self, text: impl Into<String>) -> Text {
        self.super_text(SuperKind::XLabel, text.into())
    }

    /// Sets the figure-wide y label (`supylabel`) and returns it, reset
    /// as [`Figure::suptitle`] resets the title; see
    /// [`Figure::set_supylabel_x`].
    pub fn supylabel(&self, text: impl Into<String>) -> Text {
        self.super_text(SuperKind::YLabel, text.into())
    }

    /// Places the figure title at height `y` in figure fractions, where
    /// constrained layout leaves it without making room for it
    /// (matplotlib's `suptitle(..., y=y)`). Moving it sideways with
    /// [`Text::set_x`] leaves it to the layout.
    ///
    /// ```
    /// use plt::layout::LayoutEngine;
    /// use plt::prelude::*;
    /// let fig = Figure::new();
    /// fig.set_layout_engine(LayoutEngine::constrained())?;
    /// let ax = fig.subplot();
    /// fig.layout();
    /// let top = ax.position().y1();
    /// let title = fig.suptitle("Suptitle");
    /// fig.set_suptitle_y(0.5);
    /// fig.layout();
    /// assert_eq!(ax.position().y1(), top, "no room is made");
    /// assert_eq!(title.position().y, 0.5);
    /// # Ok::<(), plt::layout::LayoutEngineError>(())
    /// ```
    pub fn set_suptitle_y(&self, y: f64) -> &Self {
        self.place_super_text(SuperKind::Title, y)
    }

    /// Places the figure-wide x label at height `y` in figure fractions,
    /// where constrained layout leaves it without making room for it
    /// (matplotlib's `supxlabel(..., y=y)`).
    pub fn set_supxlabel_y(&self, y: f64) -> &Self {
        self.place_super_text(SuperKind::XLabel, y)
    }

    /// Places the figure-wide y label at `x` in figure fractions, where
    /// constrained layout leaves it without making room for it
    /// (matplotlib's `supylabel(..., x=x)`).
    pub fn set_supylabel_x(&self, x: f64) -> &Self {
        self.place_super_text(SuperKind::YLabel, x)
    }

    /// The figure title (matplotlib's `get_suptitle`; empty when unset).
    pub fn suptitle_text(&self) -> String {
        self.super_text_of(SuperKind::Title)
    }

    /// The figure-wide x label (matplotlib's `get_supxlabel`; empty when
    /// unset).
    pub fn supxlabel_text(&self) -> String {
        self.super_text_of(SuperKind::XLabel)
    }

    /// The figure-wide y label (matplotlib's `get_supylabel`; empty when
    /// unset).
    pub fn supylabel_text(&self) -> String {
        self.super_text_of(SuperKind::YLabel)
    }

    /// Adds text at `(x, y)` in figure fractions (matplotlib's `fig.text`).
    pub fn text(&self, x: f64, y: f64, text: impl Into<String>) -> Text {
        let rc = crate::rc::params();
        let mut artist = TextArtist::new(Point::new(x, y), text, rc);
        artist.base_mut().transform = Some(Transform::Figure(self.root.id()));
        let id = self
            .state
            .borrow_mut()
            .adopt(self.root.id(), Box::new(artist));
        Text::from_raw(RawHandle::new(Rc::clone(&self.state), id))
    }

    /// The texts placed in this figure (or subfigure) itself, the title
    /// and super-labels included, in the order they were made
    /// (matplotlib's `Figure.texts`).
    ///
    /// ```
    /// use plt::artist::ArtistHandle;
    /// use plt::figure::Figure;
    /// let fig = Figure::new();
    /// let title = fig.suptitle("title");
    /// fig.text(0.5, 0.5, "note");
    /// let texts: Vec<String> = fig.texts().iter().map(|t| t.text()).collect();
    /// assert_eq!(texts, ["title", "note"]);
    /// title.remove();
    /// assert_eq!(fig.texts().len(), 1);
    /// assert_eq!(fig.suptitle_text(), "");
    /// ```
    pub fn texts(&self) -> Vec<Text> {
        let children = self
            .root
            .with::<FigureFrame, _>(|frame| frame.children.clone());
        let state = self.state.borrow();
        children
            .into_iter()
            .filter(|&id| state.artist_as::<TextArtist>(id).is_some())
            .map(|id| Text::from_raw(RawHandle::new(Rc::clone(&self.state), id)))
            .collect()
    }

    /// The text of the figure-wide text `kind`, empty when there is none.
    fn super_text_of(&self, kind: SuperKind) -> String {
        self.root
            .with::<FigureFrame, _>(|frame| kind.slot(frame))
            .map(|known| Text::from_raw(RawHandle::new(Rc::clone(&self.state), known.id)).text())
            .unwrap_or_default()
    }

    /// The figure-wide text `kind` showing `text`, made on first use and
    /// otherwise reset to its defaults, placed by the layout engine (the
    /// part of matplotlib's `_suplabels` that its calls without `x`, `y`
    /// or text properties do).
    fn super_text(&self, kind: SuperKind, text: String) -> Text {
        let handle = self.super_text_handle(kind, true);
        let rc = crate::rc::params();
        handle.raw().with_mut::<TextArtist, _>(|artist| {
            kind.reset(artist, &rc);
            artist.text = text;
        });
        handle
    }

    /// Moves the figure-wide text `kind` (made empty if missing) to `at`
    /// across the edge it belongs to, out of the layout engine's hands.
    fn place_super_text(&self, kind: SuperKind, at: f64) -> &Self {
        let handle = self.super_text_handle(kind, false);
        handle.raw().with_mut::<TextArtist, _>(|artist| match kind {
            SuperKind::Title | SuperKind::XLabel => artist.position.y = at,
            SuperKind::YLabel => artist.position.x = at,
        });
        self
    }

    /// The figure-wide text `kind`, made with its defaults if missing,
    /// now placed by the layout engine or not as `auto_placed` says.
    fn super_text_handle(&self, kind: SuperKind, auto_placed: bool) -> Text {
        let existing = self.root.with::<FigureFrame, _>(|frame| kind.slot(frame));
        let id = existing.map_or_else(
            || {
                let artist = kind.artist(self.root.id(), &crate::rc::params());
                self.state
                    .borrow_mut()
                    .adopt(self.root.id(), Box::new(artist))
            },
            |known| known.id,
        );
        let placed = SuperText { id, auto_placed };
        self.root
            .with_mut::<FigureFrame, _>(|frame| *kind.slot_mut(frame) = Some(placed));
        Text::from_raw(RawHandle::new(Rc::clone(&self.state), id))
    }
}

/// The figure-wide texts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SuperKind {
    Title,
    XLabel,
    YLabel,
}

impl SuperKind {
    /// The text of this kind `frame` has, if any.
    fn slot(self, frame: &FigureFrame) -> Option<SuperText> {
        let texts = &frame.super_texts;
        match self {
            Self::Title => texts.title,
            Self::XLabel => texts.xlabel,
            Self::YLabel => texts.ylabel,
        }
    }

    /// Where `frame` keeps its text of this kind.
    fn slot_mut(self, frame: &mut FigureFrame) -> &mut Option<SuperText> {
        let texts = &mut frame.super_texts;
        match self {
            Self::Title => &mut texts.title,
            Self::XLabel => &mut texts.xlabel,
            Self::YLabel => &mut texts.ylabel,
        }
    }

    /// A new, empty text of this kind in `frame`.
    fn artist(self, frame: ArtistId, rc: &Rc<RcParams>) -> TextArtist {
        let mut text = TextArtist::new(Point::new(0.0, 0.0), "", Rc::clone(rc));
        text.font = FontProperties::from_rc(rc);
        self.reset(&mut text, rc);
        text.base_mut().transform = Some(Transform::Figure(frame));
        text
    }

    /// Puts `text` back where matplotlib's `_suplabels` places this kind
    /// of text, aligned and rotated as it does, in the `figure.title*` or
    /// `figure.label*` font size and weight.
    fn reset(self, text: &mut TextArtist, rc: &RcParams) {
        let (position, halign, valign, size, weight) = match self {
            Self::Title => (
                (0.5, 0.98),
                HAlign::Center,
                VAlign::Top,
                rc.figure_titlesize,
                rc.figure_titleweight,
            ),
            Self::XLabel => (
                (0.5, 0.01),
                HAlign::Center,
                VAlign::Bottom,
                rc.figure_labelsize,
                rc.figure_labelweight,
            ),
            Self::YLabel => (
                (0.02, 0.5),
                HAlign::Left,
                VAlign::Center,
                rc.figure_labelsize,
                rc.figure_labelweight,
            ),
        };
        text.position = Point::new(position.0, position.1);
        text.halign = halign;
        text.valign = valign;
        text.font.set_size(size, rc.font_size);
        text.font.weight = weight;
        // matplotlib's `_suplabels` ignores the `rotation_mode` of its
        // settings, so the y label rotates in the default mode.
        text.rotation = if self == Self::YLabel { 90.0 } else { 0.0 };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn super_texts_read_back_and_are_forgotten_once_removed() {
        // matplotlib's test_get_suptitle_supxlabel_supylabel and
        // test_remove_suptitle_supxlabel_supylabel.
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        assert_eq!(
            [
                fig.suptitle_text(),
                fig.supxlabel_text(),
                fig.supylabel_text()
            ],
            ["", "", ""]
        );
        let texts = [
            fig.suptitle("suptitle"),
            fig.supxlabel("supxlabel"),
            fig.supylabel("supylabel"),
        ];
        assert_eq!(
            [
                fig.suptitle_text(),
                fig.supxlabel_text(),
                fig.supylabel_text()
            ],
            ["suptitle", "supxlabel", "supylabel"]
        );
        assert_eq!(fig.texts().len(), 3);
        for text in &texts {
            text.remove();
        }
        assert_eq!(fig.texts(), Vec::<Text>::new());
        assert_eq!(fig.suptitle_text(), "");
        // A new title is made in place of the removed one.
        fig.suptitle("again");
        assert_eq!(fig.suptitle_text(), "again");
    }

    /// Whether the layout engine places the figure-wide text `kind`.
    fn auto_placed(fig: &Figure, kind: SuperKind) -> Option<bool> {
        fig.root
            .with::<FigureFrame, _>(|frame| kind.slot(frame))
            .map(|text| text.auto_placed)
    }

    #[test]
    fn each_call_resets_the_text_and_hands_it_back_to_the_layout() {
        // matplotlib's `_suplabels`: a call without `x`, `y` or text
        // properties puts the text back in its default place, alignment,
        // size and weight.
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let title = fig.suptitle("one");
        let size = title.font().size;
        title
            .set_valign(VAlign::Bottom)
            .set_halign(HAlign::Left)
            .set_fontsize(30.0)
            .set_weight(crate::fonts::FontWeight::BOLD);
        fig.set_suptitle_y(0.5);
        assert_eq!(title.position(), Point::new(0.5, 0.5));
        assert_eq!(auto_placed(&fig, SuperKind::Title), Some(false));
        assert_eq!(fig.suptitle("two"), title);
        assert_eq!(title.text(), "two");
        assert_eq!(title.position(), Point::new(0.5, 0.98));
        assert_eq!(
            (title.halign(), title.valign()),
            (HAlign::Center, VAlign::Top)
        );
        assert_eq!(title.font().size, size);
        assert_eq!(title.font().weight, crate::rc::params().figure_titleweight);
        assert_eq!(auto_placed(&fig, SuperKind::Title), Some(true));
    }

    #[test]
    fn placing_a_label_moves_it_across_its_edge_only() {
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let ylabel = fig.supylabel("y");
        assert_eq!(
            (ylabel.position(), ylabel.rotation()),
            (Point::new(0.02, 0.5), 90.0)
        );
        fig.set_supylabel_x(0.3);
        assert_eq!(ylabel.position(), Point::new(0.3, 0.5));
        assert_eq!(auto_placed(&fig, SuperKind::YLabel), Some(false));
        // Placing a label not made yet makes it, empty.
        fig.set_supxlabel_y(0.2);
        assert_eq!(fig.supxlabel_text(), "");
        assert_eq!(auto_placed(&fig, SuperKind::XLabel), Some(false));
        assert_eq!(auto_placed(&fig, SuperKind::Title), None);
    }

    #[test]
    fn subfigures_list_their_own_texts() {
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let parts = fig
            .subfigures(1, 2, super::super::SubfigureOptions::default())
            .expect("valid grid");
        parts[(0, 0)].suptitle("left");
        fig.text(0.1, 0.1, "figure");
        assert_eq!(parts[(0, 0)].texts().len(), 1);
        assert_eq!(fig.texts().len(), 1);
    }
}
