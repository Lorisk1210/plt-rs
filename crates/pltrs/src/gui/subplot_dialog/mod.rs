//! The subplot configuration tool of figure windows (matplotlib's
//! `configure_subplots`, as its Qt tool `SubplotToolQt` has it): a slider
//! and a field for each subplot parameter, borders and spacings, and the
//! buttons "Export values", "Tight layout", "Reset" and "Close".
//!
//! As in the Qt tool, the values have three decimals, a border stays a
//! thousandth short of its opposite, the arrow keys step by 0.005, a
//! field applies its text when Enter is pressed or it loses the focus,
//! and the figure follows every change at once. "Export values" copies
//! the values to the clipboard, as `name=value` lines ready for
//! `subplots_adjust(...)`, rather than showing them in a window to copy
//! from.
//!
//! **Under constrained layout** (pltrs' modern looks), which places the
//! subplots itself and overrides the subplot parameters, a switch above
//! the values chooses what they adjust:
//!
//! * "Constrained layout": its own padding and spacing (`h_pad`, `w_pad`,
//!   `hspace`, `wspace` of `ConstrainedLayoutEngine.set`), keeping the
//!   layout it makes;
//! * "Subplot parameters": the six parameters, constrained layout giving
//!   way. The figure starts from a tight layout, which leaves the axes
//!   about where constrained layout had them, and a note says so.
//!
//! "Tight layout" switches over the same way, and "Reset" puts back
//! constrained layout. The switch shows when the figure uses constrained
//! layout, or `figure.constrained_layout.use` is on, as it is in the
//! modern looks; matplotlib's look without it shows the Qt tool's values
//! only.
//!
//! The tool only shows values and reports what the user asks
//! ([`SubplotChange`]); its owner carries the change out on the figure
//! and shows the figure's layout in it again: a figure window with
//! [`FigureLayout`], a program of [`WindowHost`](crate::gui::WindowHost)
//! with matplotlib's calls.

mod layout;
mod param;
#[cfg(test)]
mod tests;

use pltrs_platform::{Context, WindowEvent, WindowId};
use pltrs_ui::{
    Choice, Flex, Form, Label, Overflow, Slider, Spacing, Stack, TextInput, Tone, UiWindow,
    WidgetId,
};

pub(super) use layout::FigureLayout;
pub use layout::{ConstrainedSpacing, SubplotChange, SubplotLayout};
pub(super) use param::{LayoutParam, Param, SubplotParam};

use super::GuiError;
use super::dialog::{self, Dialog};
use crate::figure::Figure;
use crate::gridspec::SubplotParams;
use crate::layout::{ConstrainedLayout, LayoutEngine};

/// How a slider's share of a row compares with its field's.
const SLIDER_FLEX: f64 = 3.0;
/// The labels of the switch between the layouts, in page order.
const MODES: [&str; 2] = ["Constrained layout", "Subplot parameters"];
/// The page of the subplot parameters, when there is a switch.
const PARAMS_PAGE: usize = 1;
/// What the note says after "Export values".
const EXPORTED: &str = "Copied the values to the clipboard";
/// What the note says once constrained layout gave way.
const CONSTRAINED_OFF: &str = "Constrained layout is off: the subplot parameters apply";

/// What the tool's widgets report.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum SubplotMessage {
    /// The switch chose a page (see [`MODES`]).
    Mode(usize),
    /// A slider moved.
    Slider(Param, f64),
    /// A field's text was edited.
    Edited(Param),
    /// Enter was pressed in a field.
    Submitted(Param, String),
    /// Copy the values to the clipboard.
    Export,
    /// Lay the figure out tightly.
    TightLayout,
    /// Put back the layout the tool opened with.
    Reset,
    /// Close the tool.
    Close,
}

/// What the tool asks of its owner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Request {
    /// Carry `change` out on the figure, then show its layout.
    Change(SubplotChange),
    /// Close the tool.
    Close,
}

/// The widgets of one value.
#[derive(Debug, Clone, Copy)]
pub(super) struct Control {
    pub(super) param: Param,
    pub(super) slider: WidgetId,
    pub(super) field: WidgetId,
}

/// The switch between the layouts and the pages it shows.
#[derive(Debug, Clone, Copy)]
struct Switch {
    choice: WidgetId,
    stack: WidgetId,
}

/// A window adjusting how a figure places its subplots.
#[derive(Debug)]
pub(super) struct SubplotDialog {
    window: UiWindow<SubplotMessage>,
    pub(super) controls: Vec<Control>,
    switch: Option<Switch>,
    note: WidgetId,
    /// The layout shown.
    layout: SubplotLayout,
    /// The subplot parameters last shown, for the switch to start from.
    params: SubplotParams,
    /// The constrained layout spacing last shown, for the switch to go
    /// back to.
    spacing: ConstrainedSpacing,
    /// The field edited since its value was last applied or shown.
    edited: Option<Param>,
}

/// Whether the subplot tool of `figure` offers constrained layout's
/// values: when the figure uses constrained layout, or
/// `figure.constrained_layout.use` is on.
pub(super) fn offers_constrained(figure: &Figure) -> bool {
    matches!(figure.layout_engine(), Some(LayoutEngine::Constrained(_)))
        || crate::rc::params().figure_constrained_layout_use
}

impl SubplotDialog {
    /// Opens the tool showing `layout`, in the theme of `figure`'s
    /// windows, with the switch to constrained layout's values when
    /// `offer_constrained`.
    ///
    /// # Errors
    ///
    /// When the window cannot be created.
    pub(super) fn open(
        cx: &mut Context<'_>,
        figure: &Figure,
        layout: SubplotLayout,
        offer_constrained: bool,
    ) -> Result<Self, GuiError> {
        let params = match layout {
            SubplotLayout::Params(params) => params,
            SubplotLayout::Constrained(_) => figure.subplot_params(),
        };
        let spacing = match layout {
            SubplotLayout::Constrained(spacing) => spacing,
            SubplotLayout::Params(_) => ConstrainedSpacing::of(&ConstrainedLayout::default()),
        };
        let mut controls = Vec::new();
        let params_form = values_form(
            &mut controls,
            [
                (
                    "Borders",
                    SubplotParam::BORDERS.map(Param::Subplot).to_vec(),
                ),
                (
                    "Spacings",
                    SubplotParam::SPACINGS.map(Param::Subplot).to_vec(),
                ),
            ],
            &SubplotLayout::Params(params),
        );
        let (content, switch) = if offer_constrained {
            let constrained_form = values_form(
                &mut controls,
                [
                    (
                        "Padding (inches)",
                        LayoutParam::PADS.map(Param::Constrained).to_vec(),
                    ),
                    (
                        "Spacings",
                        LayoutParam::SPACINGS.map(Param::Constrained).to_vec(),
                    ),
                ],
                &SubplotLayout::Constrained(spacing),
            );
            let page = page_of(&layout);
            let mut stack = Stack::new();
            stack.push(constrained_form);
            stack.push(params_form);
            stack.show(page);
            let mut choice = Choice::new(MODES.map(str::to_owned).to_vec(), SubplotMessage::Mode);
            choice.set_chosen(page);
            let mut column = Flex::column().with_spacing(dialog::GAP);
            let choice = column.push(choice, 0.0);
            let stack = column.push(stack, 1.0);
            (column, Some(Switch { choice, stack }))
        } else {
            (Flex::column().with_flex_child(params_form, 1.0), None)
        };
        // The note says what a change did beyond the values: an engine
        // that gave way, a refusal, the values copied.
        let note = Label::new("")
            .with_tone(Tone::Secondary)
            .with_overflow(Overflow::ElideEnd);
        let mut content = content.with_spacing(dialog::GAP);
        let note = content.push(note, 0.0);
        let mut chrome = Dialog::without_status();
        chrome.button("Export values", SubplotMessage::Export);
        chrome.button("Tight layout", SubplotMessage::TightLayout);
        chrome.button("Reset", SubplotMessage::Reset);
        chrome.primary("Close", SubplotMessage::Close);
        let options = dialog::window_options("Subplot configuration tool");
        let window = dialog::open_fitted(cx, figure, &options, chrome.finish(content))?;
        Ok(Self {
            window,
            controls,
            switch,
            note,
            layout,
            params,
            spacing,
            edited: None,
        })
    }

    /// The window.
    pub(super) fn id(&self) -> WindowId {
        self.window.id()
    }

    /// Handles an event of the tool's window; returns what the user
    /// asked for.
    ///
    /// # Errors
    ///
    /// When the window cannot be updated.
    pub(super) fn handle(
        &mut self,
        cx: &mut Context<'_>,
        event: &WindowEvent,
    ) -> Result<Vec<Request>, GuiError> {
        if dialog::dismissed(event, self.window.ui()) {
            return Ok(vec![Request::Close]);
        }
        let messages = self.window.handle(cx, event)?;
        let mut requests = Vec::new();
        // A field applies its text once it loses the focus, as the Qt
        // tool's spin boxes do, before what took the focus acts.
        if let Some(param) = self.edited
            && self.window.ui().focus() != self.control(param).map(|c| c.field)
        {
            requests.extend(self.commit(cx, param, None)?);
        }
        for message in messages {
            requests.extend(self.message(cx, message)?);
        }
        Ok(requests)
    }

    /// What a widget's message asks for.
    fn message(
        &mut self,
        cx: &mut Context<'_>,
        message: SubplotMessage,
    ) -> Result<Option<Request>, GuiError> {
        let change = match message {
            SubplotMessage::Mode(page) if page == page_of(&self.layout) => return Ok(None),
            // The subplot parameters start from a tight layout.
            SubplotMessage::Mode(PARAMS_PAGE) | SubplotMessage::TightLayout => {
                SubplotChange::TightLayout
            }
            SubplotMessage::Mode(_) => SubplotChange::Constrain(self.spacing),
            SubplotMessage::Slider(param, value) => match self.changed(param, value) {
                Some(change) => change,
                None => return Ok(None),
            },
            SubplotMessage::Edited(param) => {
                self.edited = Some(param);
                return Ok(None);
            }
            SubplotMessage::Submitted(param, text) => return self.commit(cx, param, Some(text)),
            SubplotMessage::Export => {
                let text = param::export_text(&self.layout);
                cx.set_clipboard_text(self.id(), text)?;
                self.set_note(cx, EXPORTED)?;
                return Ok(None);
            }
            SubplotMessage::Reset => SubplotChange::Reset,
            SubplotMessage::Close => return Ok(Some(Request::Close)),
        };
        self.anticipate(change);
        Ok(Some(Request::Change(change)))
    }

    /// Takes `change` as carried out until the owner shows the figure's
    /// layout, so that what is asked in the same breath (a field applied
    /// as a slider takes the focus) builds on it.
    fn anticipate(&mut self, change: SubplotChange) {
        match change {
            SubplotChange::Adjust(params) => {
                self.layout = SubplotLayout::Params(params);
                self.params = params;
            }
            SubplotChange::Constrain(spacing) => {
                self.layout = SubplotLayout::Constrained(spacing);
                self.spacing = spacing;
            }
            SubplotChange::TightLayout | SubplotChange::Reset => {}
        }
    }

    /// Applies the text of the field of `param` (`text`, else what the
    /// field holds): a number becomes the value, within its limits; other
    /// text gives way to the value, as in the Qt tool's spin boxes.
    fn commit(
        &mut self,
        cx: &mut Context<'_>,
        param: Param,
        text: Option<String>,
    ) -> Result<Option<Request>, GuiError> {
        self.edited = None;
        let Some(control) = self.control(param) else {
            return Ok(None);
        };
        let text = match text {
            Some(text) => text,
            None => self
                .window
                .ui_mut()
                .widget_mut::<TextInput<SubplotMessage>>(control.field)
                .map(|field| field.text().to_owned())
                .unwrap_or_default(),
        };
        let change = text
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .and_then(|value| self.changed(param, value));
        if let Some(change) = change {
            self.anticipate(change);
        }
        // The field shows the value taken (or kept) either way.
        self.show_value(cx, param)?;
        Ok(change.map(Request::Change))
    }

    /// The change setting `param` to `value` (within its limits), or
    /// `None` when the value stays as it is or `param` is not shown.
    fn changed(&self, param: Param, value: f64) -> Option<SubplotChange> {
        let change = match (param, self.layout) {
            (Param::Subplot(param), SubplotLayout::Params(params)) => {
                SubplotChange::Adjust(param.set(params, value))
            }
            (Param::Constrained(param), SubplotLayout::Constrained(spacing)) => {
                SubplotChange::Constrain(param.set(spacing, value))
            }
            _ => return None,
        };
        let unchanged = match change {
            SubplotChange::Adjust(params) => self.layout == SubplotLayout::Params(params),
            SubplotChange::Constrain(spacing) => self.layout == SubplotLayout::Constrained(spacing),
            SubplotChange::TightLayout | SubplotChange::Reset => false,
        };
        (!unchanged).then_some(change)
    }

    /// Shows `layout`, the figure's after a change (or when the tool is
    /// raised), sending nothing (the Qt tool's `blockSignals`), and a
    /// note below it: `problem`, why the figure refused the change, else
    /// that constrained layout gave way to the subplot parameters, if it
    /// did. A field being edited keeps its text.
    ///
    /// # Errors
    ///
    /// When the window cannot be updated.
    pub(super) fn show(
        &mut self,
        cx: &mut Context<'_>,
        layout: SubplotLayout,
        problem: &str,
    ) -> Result<(), GuiError> {
        let gave_way = matches!(
            (self.layout, layout),
            (SubplotLayout::Constrained(_), SubplotLayout::Params(_))
        );
        let note = match problem {
            "" if gave_way => CONSTRAINED_OFF,
            problem => problem,
        };
        self.layout = layout;
        match layout {
            SubplotLayout::Params(params) => self.params = params,
            SubplotLayout::Constrained(spacing) => self.spacing = spacing,
        }
        if let Some(switch) = self.switch {
            let page = page_of(&layout);
            self.window
                .update(cx, switch.choice, |choice: &mut Choice<SubplotMessage>| {
                    choice.set_chosen(page);
                })?;
            self.window
                .update(cx, switch.stack, |stack: &mut Stack<SubplotMessage>| {
                    stack.show(page);
                })?;
        }
        let params: Vec<Param> = self.controls.iter().map(|c| c.param).collect();
        for param in params {
            if self.edited != Some(param) {
                self.show_value(cx, param)?;
            }
        }
        self.set_note(cx, note)
    }

    /// Shows the value of `param` on its slider and in its field.
    fn show_value(&mut self, cx: &mut Context<'_>, param: Param) -> Result<(), GuiError> {
        let value = match param {
            Param::Subplot(param) => param.get(&self.params),
            Param::Constrained(param) => param.get(&self.spacing),
        };
        let Some(control) = self.control(param) else {
            return Ok(());
        };
        self.window
            .update(cx, control.slider, |slider: &mut Slider<SubplotMessage>| {
                slider.set_value(value);
            })?;
        self.window.update(
            cx,
            control.field,
            |field: &mut TextInput<SubplotMessage>| field.set_text(param::field_text(value)),
        )?;
        Ok(())
    }

    /// Shows `note` below the values.
    fn set_note(&mut self, cx: &mut Context<'_>, note: &str) -> Result<(), GuiError> {
        self.window
            .update(cx, self.note, |label: &mut Label| label.set_text(note))?;
        Ok(())
    }

    /// The widgets of `param`.
    fn control(&self, param: Param) -> Option<Control> {
        self.controls.iter().find(|c| c.param == param).copied()
    }

    /// Raises the tool.
    ///
    /// # Errors
    ///
    /// When the window system fails.
    pub(super) fn raise(&self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        cx.raise_window(self.id())?;
        Ok(())
    }

    /// The tool's window, for tests.
    #[cfg(test)]
    pub(super) fn window(&self) -> &UiWindow<SubplotMessage> {
        &self.window
    }

    /// The tool's window, mutably, for tests.
    #[cfg(test)]
    pub(super) fn window_mut(&mut self) -> &mut UiWindow<SubplotMessage> {
        &mut self.window
    }

    /// Closes the tool's window.
    ///
    /// # Errors
    ///
    /// When the window system fails to close it.
    pub(super) fn close(self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        cx.close_window(self.id())?;
        Ok(())
    }
}

/// The page showing `layout`'s values, when there is a switch.
fn page_of(layout: &SubplotLayout) -> usize {
    match layout {
        SubplotLayout::Constrained(_) => 0,
        SubplotLayout::Params(_) => PARAMS_PAGE,
    }
}

/// A form of sections of rows, each row a slider and a field for one of
/// the values `layout` holds; their widgets go to `controls`.
fn values_form(
    controls: &mut Vec<Control>,
    sections: [(&str, Vec<Param>); 2],
    layout: &SubplotLayout,
) -> Form<SubplotMessage> {
    let mut form = Form::new();
    for (title, params) in sections {
        form.section(title);
        for param in params {
            let value = param.get(layout).unwrap_or_default();
            let (min, max) = param.range();
            let mut row = Flex::row().with_spacing(Spacing::Sm);
            let slider = Slider::new(min, max, value, move |value| {
                SubplotMessage::Slider(param, value)
            })
            .with_step(param::STEP);
            let slider = row.push(slider, SLIDER_FLEX);
            let field = TextInput::new(param::field_text(value), move |_| {
                SubplotMessage::Edited(param)
            })
            .on_submit(move |text| SubplotMessage::Submitted(param, text));
            let field = row.push(field, 1.0);
            form.row(param.name(), row);
            controls.push(Control {
                param,
                slider,
                field,
            });
        }
    }
    form
}
