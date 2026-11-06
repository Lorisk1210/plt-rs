//! What the subplot tool shows and asks for ([`SubplotLayout`],
//! [`SubplotChange`]), and how a figure window carries out what it asks
//! on its figure ([`FigureLayout`]).

use crate::figure::Figure;
use crate::gridspec::{SubplotOverrides, SubplotParams};
use crate::layout::{ConstrainedLayout, LayoutEngine, PlaceHolderLayout};

/// Constrained layout's padding and spacing, what the subplot tool adjusts
/// while constrained layout places the subplots (matplotlib's
/// `ConstrainedLayoutEngine.set(h_pad=..., w_pad=..., hspace=...,
/// wspace=...)`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstrainedSpacing {
    /// Padding above and below the decorations, in inches.
    pub h_pad: f64,
    /// Padding left and right of the decorations, in inches.
    pub w_pad: f64,
    /// Space between rows, as a fraction of the figure height.
    pub hspace: f64,
    /// Space between columns, as a fraction of the figure width.
    pub wspace: f64,
}

impl ConstrainedSpacing {
    /// The spacing of `layout`.
    pub fn of(layout: &ConstrainedLayout) -> Self {
        Self {
            h_pad: layout.h_pad,
            w_pad: layout.w_pad,
            hspace: layout.hspace,
            wspace: layout.wspace,
        }
    }

    /// `layout` with this spacing.
    fn applied_to(self, layout: ConstrainedLayout) -> ConstrainedLayout {
        ConstrainedLayout {
            h_pad: self.h_pad,
            w_pad: self.w_pad,
            hspace: self.hspace,
            wspace: self.wspace,
            ..layout
        }
    }
}

/// How a figure's subplots are placed, as the subplot tool shows it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SubplotLayout {
    /// By the figure's subplot parameters (`figure.subplotpars`).
    Params(SubplotParams),
    /// By constrained layout, with this spacing.
    Constrained(ConstrainedSpacing),
}

impl SubplotLayout {
    /// How `figure` places its subplots.
    pub fn of(figure: &Figure) -> Self {
        match figure.layout_engine() {
            Some(LayoutEngine::Constrained(layout)) => {
                Self::Constrained(ConstrainedSpacing::of(&layout))
            }
            _ => Self::Params(figure.subplot_params()),
        }
    }
}

/// What the subplot tool asks of the figure it adjusts. The program
/// carries it out, then shows the figure's layout in the tool
/// ([`WindowHost::show_subplot_layout`](crate::gui::WindowHost::show_subplot_layout)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SubplotChange {
    /// Place the subplots by these parameters (`subplots_adjust`, with
    /// all six, as matplotlib's Qt tool calls it). A layout engine that
    /// would place them otherwise at every draw (constrained or tight
    /// layout) gives way to a placeholder that lets the parameters be
    /// (matplotlib's `PlaceHolderLayoutEngine(adjust_compatible=True,
    /// colorbar_gridspec=...)`, keeping the engine's way of making
    /// colorbars).
    Adjust(SubplotParams),
    /// Place the subplots by constrained layout with this spacing: the
    /// engine's `set(...)` under constrained layout, else constrained
    /// layout again (as when the tool opened, if it was on then).
    Constrain(ConstrainedSpacing),
    /// Lay the subplots out tightly, once (`figure.tight_layout()`, the
    /// Qt tool's "Tight layout"), after constrained layout gave way as
    /// for [`SubplotChange::Adjust`].
    TightLayout,
    /// Put back the layout engine and subplot parameters the figure had
    /// when the tool was opened or last raised (the Qt tool's "Reset").
    Reset,
}

/// A figure's layout engine and subplot parameters, as the subplot tool
/// found them, applying the tool's changes to the figure: what a figure
/// window does with them.
#[derive(Clone, Copy, Debug)]
pub(in crate::gui) struct FigureLayout {
    engine: Option<LayoutEngine>,
    params: SubplotParams,
}

impl FigureLayout {
    /// The layout `figure` has now.
    pub(in crate::gui) fn of(figure: &Figure) -> Self {
        Self {
            engine: figure.layout_engine(),
            params: figure.subplot_params(),
        }
    }

    /// Carries out `change` on `figure` (which had this layout when the
    /// tool was opened).
    ///
    /// # Errors
    ///
    /// Why the figure refused the change, for the tool to show.
    pub(in crate::gui) fn apply(
        &self,
        figure: &Figure,
        change: SubplotChange,
    ) -> Result<(), String> {
        match change {
            SubplotChange::Adjust(params) => {
                let_params_be(figure)?;
                figure
                    .subplots_adjust(overrides(params))
                    .map_err(|err| err.to_string())?;
            }
            SubplotChange::Constrain(spacing) => self.constrain(figure, spacing)?,
            SubplotChange::TightLayout => {
                let_params_be(figure)?;
                figure
                    .tight_layout(crate::layout::TightLayout::default())
                    .map_err(|err| format!("Tight layout failed: {err}"))?;
            }
            SubplotChange::Reset => self.reset(figure)?,
        }
        Ok(())
    }

    /// Lays `figure` out by constrained layout with `spacing`, keeping the
    /// rest of its constrained layout, or of the one it had when the tool
    /// opened.
    fn constrain(&self, figure: &Figure, spacing: ConstrainedSpacing) -> Result<(), String> {
        let base = match (figure.layout_engine(), self.engine) {
            (Some(LayoutEngine::Constrained(layout)), _)
            | (_, Some(LayoutEngine::Constrained(layout))) => layout,
            _ => ConstrainedLayout::default(),
        };
        figure
            .set_layout_engine(LayoutEngine::Constrained(spacing.applied_to(base)))
            .map(|_| ())
            .map_err(|err| err.to_string())
    }

    /// Puts back the subplot parameters, then the engine.
    fn reset(&self, figure: &Figure) -> Result<(), String> {
        figure
            .subplots_adjust(overrides(self.params))
            .map_err(|err| err.to_string())?;
        figure
            .set_layout_engine(self.engine)
            .map(|_| ())
            .map_err(|err| err.to_string())
    }
}

/// Replaces a layout engine of `figure` that places its subplots at every
/// draw (constrained or tight layout) by a placeholder letting the subplot
/// parameters be, and puts the subplots back where the parameters place
/// them (a tight layout measures them there: where constrained layout had
/// them, its margins would come out wider).
fn let_params_be(figure: &Figure) -> Result<(), String> {
    let Some(engine) = figure.layout_engine().filter(|engine| {
        matches!(
            engine,
            LayoutEngine::Constrained(_) | LayoutEngine::Tight(_)
        )
    }) else {
        return Ok(());
    };
    let placeholder = LayoutEngine::PlaceHolder(PlaceHolderLayout {
        adjust_compatible: true,
        colorbar_gridspec: engine.colorbar_gridspec(),
    });
    figure
        .set_layout_engine(placeholder)
        .map_err(|err| err.to_string())?;
    figure
        .subplots_adjust(overrides(figure.subplot_params()))
        .map(|_| ())
        .map_err(|err| err.to_string())
}

/// The change setting every subplot parameter to its value in `params`.
fn overrides(params: SubplotParams) -> SubplotOverrides {
    SubplotOverrides {
        left: Some(params.left),
        bottom: Some(params.bottom),
        right: Some(params.right),
        top: Some(params.top),
        wspace: Some(params.wspace),
        hspace: Some(params.hspace),
    }
}
