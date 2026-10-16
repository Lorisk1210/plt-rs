//! Figures: the top-level container of everything drawn (matplotlib.figure).

mod frame;
mod mosaic;
mod save;
mod state;
mod subplots;
#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use plt_core::Rgba;
use plt_render::Renderer;

pub(crate) use frame::FigureFrame;
pub use mosaic::{Mosaic, MosaicAxes, MosaicCell, MosaicError, MosaicOptions};
pub use save::SaveError;
pub(crate) use state::FigureState;
pub use subplots::Share;
pub(crate) use subplots::insert_axes;

use crate::artist::{Artist, ArtistHandle, ArtistId, DrawError, RawHandle};
use crate::axes::{Axes, AxesState, PlotError};
use crate::fonts::FontProperties;
use crate::gridspec::{GridSpecError, SubplotOverrides, SubplotParams};
use crate::lines::{Line2D, LineArtist, LineProps};
use crate::patches::{Patch, PatchArtist, PatchShape};
use crate::rc::RcParams;
use crate::text::{Text, TextArtist};
use crate::transforms::Transform;
use plt_core::Point;
use plt_text::{HAlign, VAlign};

/// The problems found while drawing a figure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawErrors(pub Vec<DrawError>);

impl fmt::Display for DrawErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let messages: Vec<String> = self.0.iter().map(ToString::to_string).collect();
        write!(
            f,
            "could not draw {} artist(s): {}",
            self.0.len(),
            messages.join("; ")
        )
    }
}

impl std::error::Error for DrawErrors {}

/// A figure: the canvas, its background and every axes and artist on it.
///
/// `Figure` is a cheap handle; clones refer to the same figure.
///
/// ```
/// use plt::figure::Figure;
/// let fig = Figure::new();
/// let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8]);
/// ax.set_xlim(0.0, 10.0);
/// assert_eq!(ax.xlim(), (0.0, 10.0));
/// ```
#[derive(Clone)]
pub struct Figure {
    state: Rc<RefCell<FigureState>>,
    root: RawHandle,
}

impl fmt::Debug for Figure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.state.borrow();
        f.debug_struct("Figure")
            .field("size_inches", &state.size_inches)
            .field("dpi", &state.dpi)
            .finish_non_exhaustive()
    }
}

impl PartialEq for Figure {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}

impl Default for Figure {
    fn default() -> Self {
        Self::new()
    }
}

impl Figure {
    /// A 6.4 × 4.8 inch figure at 100 dpi coloured by `figure.facecolor`
    /// and `figure.edgecolor`.
    pub fn new() -> Self {
        Self::with_size(6.4, 4.8, 100.0)
    }

    /// A figure of the given size in inches and resolution.
    pub fn with_size(width: f64, height: f64, dpi: f64) -> Self {
        let rc = crate::rc::params();
        let color = |c: &crate::colors::Color| c.resolve().unwrap_or(Rgba::WHITE);
        let state = FigureState::new(
            (width, height),
            dpi,
            color(&rc.figure_facecolor),
            color(&rc.figure_edgecolor),
        );
        let root = state.root;
        let state = Rc::new(RefCell::new(state));
        let root = RawHandle::new(Rc::clone(&state), root);
        Self { state, root }
    }

    /// Canvas size in inches.
    pub fn size_inches(&self) -> (f64, f64) {
        self.state.borrow().size_inches
    }

    /// Resizes the canvas (in inches).
    pub fn set_size_inches(&self, width: f64, height: f64) -> &Self {
        let mut state = self.state.borrow_mut();
        state.size_inches = (width, height);
        state.stale = true;
        self
    }

    /// Canvas resolution in dots per inch.
    pub fn dpi(&self) -> f64 {
        self.state.borrow().dpi
    }

    /// Changes the canvas resolution.
    pub fn set_dpi(&self, dpi: f64) -> &Self {
        let mut state = self.state.borrow_mut();
        state.dpi = dpi;
        state.stale = true;
        self
    }

    /// Canvas size in pixels.
    pub fn size_pixels(&self) -> (f64, f64) {
        self.state.borrow().size_pixels()
    }

    /// Sets the background colour.
    pub fn set_facecolor(&self, color: Rgba) -> &Self {
        self.root
            .with_mut::<FigureFrame, _>(|frame| frame.facecolor = color);
        self
    }

    /// Sets the colour of the frame around the canvas (matplotlib's
    /// `set_edgecolor`); it shows once the frame has a width.
    pub fn set_edgecolor(&self, color: Rgba) -> &Self {
        self.root
            .with_mut::<FigureFrame, _>(|frame| frame.edgecolor = color);
        self
    }

    /// Sets the width of the frame around the canvas in points
    /// (matplotlib's `set_linewidth`).
    pub fn set_linewidth(&self, width: f64) -> &Self {
        self.root
            .with_mut::<FigureFrame, _>(|frame| frame.linewidth = width);
        self
    }

    /// The background colour.
    pub fn facecolor(&self) -> Rgba {
        self.root.with::<FigureFrame, _>(|frame| frame.facecolor)
    }

    /// The margins and spacing of subplot grids.
    pub fn subplot_params(&self) -> SubplotParams {
        self.root
            .with::<FigureFrame, _>(|frame| frame.subplot_params)
    }

    /// Changes the subplot margins and spacing and moves every grid-placed
    /// axes accordingly (`fig.subplots_adjust(...)`).
    pub fn subplots_adjust(&self, overrides: &SubplotOverrides) -> Result<&Self, GridSpecError> {
        let params = self.subplot_params().updated(overrides)?;
        let mut state = self.state.borrow_mut();
        if let Some(frame) = state.store.get_as_mut::<FigureFrame>(self.root.id()) {
            frame.subplot_params = params;
        }
        let ids: Vec<_> = state.store.ids().collect();
        for id in ids {
            if let Some(axes) = state.store.get_as_mut::<AxesState>(id)
                && let Some(spec) = &axes.subplot_spec
            {
                axes.position = spec.position(&params)?;
            }
        }
        state.stale = true;
        Ok(self)
    }

    /// Sets the figure title (matplotlib's `suptitle`) and returns it.
    pub fn suptitle(&self, text: impl Into<String>) -> Text {
        self.super_text(SuperKind::Title, text.into())
    }

    /// Sets the figure-wide x label (`supxlabel`) and returns it.
    pub fn supxlabel(&self, text: impl Into<String>) -> Text {
        self.super_text(SuperKind::XLabel, text.into())
    }

    /// Sets the figure-wide y label (`supylabel`) and returns it.
    pub fn supylabel(&self, text: impl Into<String>) -> Text {
        self.super_text(SuperKind::YLabel, text.into())
    }

    /// The figure title (empty when unset).
    pub fn suptitle_text(&self) -> String {
        let id = self
            .root
            .with::<FigureFrame, _>(|frame| frame.super_texts.title);
        id.map(|id| Text::from_raw(RawHandle::new(Rc::clone(&self.state), id)).text())
            .unwrap_or_default()
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

    /// Adds a line through `(x, y)` in figure fractions, styled from the
    /// rc parameters and `props` without the property cycle (matplotlib's
    /// `fig.add_artist(Line2D(...))`).
    ///
    /// # Errors
    ///
    /// When `x` and `y` differ in length.
    pub fn add_line(&self, x: &[f64], y: &[f64], props: LineProps) -> Result<Line2D, PlotError> {
        if x.len() != y.len() {
            return Err(PlotError::LengthMismatch {
                x: x.len(),
                y: y.len(),
            });
        }
        let mut line = LineArtist::new(x.to_vec(), y.to_vec(), crate::rc::params());
        props.apply(&mut line);
        if line.base().transform.is_none() {
            line.base_mut().transform = Some(Transform::Figure(self.root.id()));
        }
        let id = self
            .state
            .borrow_mut()
            .adopt(self.root.id(), Box::new(line));
        Ok(Line2D::from_raw(RawHandle::new(Rc::clone(&self.state), id)))
    }

    /// Adds a patch in figure fractions (matplotlib's
    /// `fig.add_artist(patch)`); give it another coordinate system with
    /// [`ArtistHandle::set_transform`].
    pub fn add_patch<S: PatchShape>(&self, shape: S) -> Patch<S> {
        let mut patch = PatchArtist::new(shape, crate::rc::params());
        patch.base_mut().transform = Some(Transform::Figure(self.root.id()));
        let id = self
            .state
            .borrow_mut()
            .adopt(self.root.id(), Box::new(patch));
        Patch::from_raw(RawHandle::new(Rc::clone(&self.state), id))
    }

    fn super_text(&self, kind: SuperKind, text: String) -> Text {
        let existing = self.root.with::<FigureFrame, _>(|frame| match kind {
            SuperKind::Title => frame.super_texts.title,
            SuperKind::XLabel => frame.super_texts.xlabel,
            SuperKind::YLabel => frame.super_texts.ylabel,
        });
        let id = existing.unwrap_or_else(|| {
            let artist = kind.artist(self.root.id(), &crate::rc::params());
            let mut state = self.state.borrow_mut();
            let id = state.adopt(self.root.id(), Box::new(artist));
            if let Some(frame) = state.store.get_as_mut::<FigureFrame>(self.root.id()) {
                let slot = match kind {
                    SuperKind::Title => &mut frame.super_texts.title,
                    SuperKind::XLabel => &mut frame.super_texts.xlabel,
                    SuperKind::YLabel => &mut frame.super_texts.ylabel,
                };
                *slot = Some(id);
            }
            id
        });
        let handle = Text::from_raw(RawHandle::new(Rc::clone(&self.state), id));
        handle.set_text(text);
        handle
    }

    /// Every axes of the figure, in creation order.
    pub fn axes(&self) -> Vec<Axes> {
        let state = self.state.borrow();
        let frame = state
            .store
            .get_as::<FigureFrame>(state.root)
            .expect("the root frame lives as long as the figure");
        frame
            .children
            .iter()
            .filter(|&&id| state.store.get_as::<AxesState>(id).is_some())
            .map(|&id| Axes::from_raw(RawHandle::new(Rc::clone(&self.state), id)))
            .collect()
    }

    /// Renders the figure through `renderer`.
    ///
    /// Everything drawable is drawn; artists with invalid properties (for
    /// example an unknown colour name) are skipped and reported.
    pub fn draw(&self, renderer: &mut dyn Renderer) -> Result<(), DrawErrors> {
        self.state.borrow_mut().prepare();
        let errors = self.state.borrow().draw(renderer);
        let mut state = self.state.borrow_mut();
        state.stale = false;
        state.pending_errors.clear();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(DrawErrors(errors))
        }
    }

    /// Runs the layout pass (autoscaling and tick placement) without
    /// drawing, so limits and ticks can be queried.
    pub fn layout(&self) {
        self.state.borrow_mut().prepare();
    }

    /// Whether something changed since the last draw.
    pub fn is_stale(&self) -> bool {
        self.state.borrow().stale
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
    /// A new text placed and styled like matplotlib's `_suplabels`.
    fn artist(self, frame: ArtistId, rc: &Rc<RcParams>) -> TextArtist {
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
        let mut text = TextArtist::new(Point::new(position.0, position.1), "", Rc::clone(rc));
        text.halign = halign;
        text.valign = valign;
        text.font = FontProperties::from_rc(rc)
            .with_size(size, rc.font_size)
            .with_weight(weight);
        // matplotlib's `_suplabels` ignores the `rotation_mode` of its
        // settings, so the y label rotates in the default mode.
        if self == Self::YLabel {
            text.rotation = 90.0;
        }
        text.base_mut().transform = Some(Transform::Figure(frame));
        text
    }
}

impl ArtistHandle for Figure {
    fn raw(&self) -> &RawHandle {
        &self.root
    }
}
