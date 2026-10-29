//! The subplot configuration tool of figure windows (matplotlib's
//! `configure_subplots`): a slider and a field for each margin and
//! spacing of the figure's subplot grid, grouped into borders and
//! spacings as in matplotlib's Qt tool, and a button putting back the
//! parameters the tool opened with. Under constrained layout (pltrs'
//! modern look), which sets the margins itself, a note says that it
//! overrides them.

use pltrs_platform::{Context, WindowEvent, WindowId};
use pltrs_ui::{Flex, Form, Slider, Spacing, TextInput, UiWindow, WidgetId};

use super::GuiError;
use super::dialog::{self, Dialog};
use crate::figure::Figure;
use crate::gridspec::{SubplotOverrides, SubplotParams};
use crate::layout::LayoutEngine;

/// How a slider's share of a row compares with its field's.
const SLIDER_FLEX: f64 = 3.0;

/// A subplot parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Param {
    /// Left edge of the grid.
    Left,
    /// Bottom edge of the grid.
    Bottom,
    /// Right edge of the grid.
    Right,
    /// Top edge of the grid.
    Top,
    /// Gap between columns.
    Wspace,
    /// Gap between rows.
    Hspace,
}

impl Param {
    /// The borders, in the order shown.
    const BORDERS: [Self; 4] = [Self::Top, Self::Bottom, Self::Left, Self::Right];
    /// The spacings, in the order shown.
    const SPACINGS: [Self; 2] = [Self::Hspace, Self::Wspace];

    /// matplotlib's name of the parameter.
    fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Bottom => "bottom",
            Self::Right => "right",
            Self::Top => "top",
            Self::Wspace => "wspace",
            Self::Hspace => "hspace",
        }
    }

    /// The parameter's value in `params`.
    fn get(self, params: &SubplotParams) -> f64 {
        match self {
            Self::Left => params.left,
            Self::Bottom => params.bottom,
            Self::Right => params.right,
            Self::Top => params.top,
            Self::Wspace => params.wspace,
            Self::Hspace => params.hspace,
        }
    }

    /// `value`, limited to what the parameter may take given the others
    /// in `params`: from 0 to 1, and a border never past its opposite (the
    /// linked sliders of matplotlib's tool).
    fn limit(self, value: f64, params: &SubplotParams) -> f64 {
        let value = value.clamp(0.0, 1.0);
        match self {
            Self::Left => value.min(params.right),
            Self::Right => value.max(params.left),
            Self::Bottom => value.min(params.top),
            Self::Top => value.max(params.bottom),
            Self::Wspace | Self::Hspace => value,
        }
    }

    /// The change setting the parameter to `value`.
    fn overrides(self, value: f64) -> SubplotOverrides {
        let mut overrides = SubplotOverrides::default();
        let slot = match self {
            Self::Left => &mut overrides.left,
            Self::Bottom => &mut overrides.bottom,
            Self::Right => &mut overrides.right,
            Self::Top => &mut overrides.top,
            Self::Wspace => &mut overrides.wspace,
            Self::Hspace => &mut overrides.hspace,
        };
        *slot = Some(value);
        overrides
    }
}

/// The text a field shows for `value`: three decimals, as matplotlib's Qt
/// tool shows.
fn value_text(value: f64) -> String {
    format!("{value:.3}")
}

/// What the tool's widgets report.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum SubplotMessage {
    /// A slider moved.
    Slider(Param, f64),
    /// A field was edited.
    Field(Param, String),
    /// Put back the parameters the tool opened with.
    Reset,
    /// Close the tool.
    Close,
}

/// The widgets of one parameter.
#[derive(Debug, Clone, Copy)]
pub(super) struct Control {
    pub(super) param: Param,
    pub(super) slider: WidgetId,
    pub(super) field: WidgetId,
}

/// A window adjusting the subplot parameters of a figure.
#[derive(Debug)]
pub(super) struct SubplotDialog {
    figure: Figure,
    window: UiWindow<SubplotMessage>,
    pub(super) controls: Vec<Control>,
    /// The parameters when the tool opened.
    initial: SubplotParams,
}

impl SubplotDialog {
    /// Opens the tool for `figure`.
    ///
    /// # Errors
    ///
    /// When the window cannot be created.
    pub(super) fn open(cx: &mut Context<'_>, figure: &Figure) -> Result<Self, GuiError> {
        let initial = figure.subplot_params();
        let mut form = Form::new();
        let mut controls = Vec::new();
        for (title, params) in [
            ("Borders", &Param::BORDERS[..]),
            ("Spacings", &Param::SPACINGS[..]),
        ] {
            form.section(title);
            for &param in params {
                let value = param.get(&initial);
                let mut row = Flex::row().with_spacing(Spacing::Sm);
                let slider = row.push(
                    Slider::new(0.0, 1.0, value, move |value| {
                        SubplotMessage::Slider(param, value)
                    }),
                    SLIDER_FLEX,
                );
                let field = row.push(
                    TextInput::new(value_text(value), move |text| {
                        SubplotMessage::Field(param, text)
                    }),
                    1.0,
                );
                form.row(param.name(), row);
                controls.push(Control {
                    param,
                    slider,
                    field,
                });
            }
        }
        let constrained = matches!(figure.layout_engine(), Some(LayoutEngine::Constrained(_)));
        let mut chrome = if constrained {
            Dialog::with_note("Constrained layout overrides these settings")
        } else {
            Dialog::without_status()
        };
        chrome.button("Reset", SubplotMessage::Reset);
        chrome.primary("Close", SubplotMessage::Close);
        let options = dialog::window_options("Subplot configuration tool")?;
        let window = dialog::open_fitted(cx, figure, &options, chrome.finish(form))?;
        Ok(Self {
            figure: figure.clone(),
            window,
            controls,
            initial,
        })
    }

    /// The window.
    pub(super) fn id(&self) -> WindowId {
        self.window.id()
    }

    /// Handles an event of the tool's window; returns whether it stays
    /// open.
    ///
    /// # Errors
    ///
    /// When the window cannot be updated.
    pub(super) fn handle(
        &mut self,
        cx: &mut Context<'_>,
        event: &WindowEvent,
    ) -> Result<bool, GuiError> {
        if dialog::dismissed(event, self.window.ui()) {
            return Ok(false);
        }
        for message in self.window.handle(cx, event)? {
            match message {
                SubplotMessage::Slider(param, value) => self.adjust(cx, param, value, true)?,
                SubplotMessage::Field(param, text) => {
                    if let Ok(value) = text.trim().parse::<f64>() {
                        self.adjust(cx, param, value, false)?;
                    }
                }
                SubplotMessage::Reset => self.reset(cx)?,
                SubplotMessage::Close => return Ok(false),
            }
        }
        Ok(true)
    }

    /// Sets `param` to `value` (within its limits) and shows it on the
    /// slider, and in the field too when `from_slider`.
    fn adjust(
        &mut self,
        cx: &mut Context<'_>,
        param: Param,
        value: f64,
        from_slider: bool,
    ) -> Result<(), GuiError> {
        let value = param.limit(value, &self.figure.subplot_params());
        // A border meeting its opposite is refused and keeps the previous
        // parameters, as in matplotlib's tool.
        let _ = self.figure.subplots_adjust(param.overrides(value));
        self.show(cx, param, value, from_slider)
    }

    /// Puts back the parameters the tool opened with.
    fn reset(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        let initial = self.initial;
        let overrides = SubplotOverrides {
            left: Some(initial.left),
            bottom: Some(initial.bottom),
            right: Some(initial.right),
            top: Some(initial.top),
            wspace: Some(initial.wspace),
            hspace: Some(initial.hspace),
        };
        // The parameters were valid when the tool opened.
        let _ = self.figure.subplots_adjust(overrides);
        for param in Param::BORDERS.into_iter().chain(Param::SPACINGS) {
            self.show(cx, param, param.get(&initial), true)?;
        }
        Ok(())
    }

    /// Shows `value` on the slider of `param`, and in its field when
    /// `field`.
    fn show(
        &mut self,
        cx: &mut Context<'_>,
        param: Param,
        value: f64,
        field: bool,
    ) -> Result<(), GuiError> {
        let Some(control) = self.controls.iter().find(|c| c.param == param).copied() else {
            return Ok(());
        };
        self.window
            .update(cx, control.slider, |slider: &mut Slider<SubplotMessage>| {
                slider.set_value(value);
            })?;
        if field {
            self.window.update(
                cx,
                control.field,
                |input: &mut TextInput<SubplotMessage>| {
                    input.set_text(value_text(value));
                },
            )?;
        }
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

#[cfg(test)]
mod tests {
    use pltrs_platform::{ButtonState, EventLoop, Key, KeyEvent, Modifiers};

    use super::*;

    #[test]
    fn fields_and_reset_adjust_the_figure() {
        let figure = Figure::with_size(4.0, 3.0, 100.0);
        figure.subplots(2, 2).expect("a 2×2 grid has cells");
        let initial = figure.subplot_params();
        let (mut event_loop, handle) = EventLoop::headless();
        let mut cx = event_loop.context();
        let mut tool = SubplotDialog::open(&mut cx, &figure).expect("the tool opens");
        let left = tool.controls[2];
        assert_eq!(left.param, Param::Left);
        tool.window.ui_mut().set_focus(Some(left.field));
        let key = |key: Key, text: Option<&str>, modifiers| {
            WindowEvent::Key(KeyEvent {
                key,
                state: ButtonState::Pressed,
                repeat: false,
                text: text.map(str::to_owned),
                modifiers,
                scancode: 0,
            })
        };
        let typed = [
            key(Key::Character('a'), Some("a"), Modifiers::CONTROL),
            key(Key::Character('0'), Some("0"), Modifiers::NONE),
            key(Key::Character('.'), Some("."), Modifiers::NONE),
            key(Key::Character('3'), Some("3"), Modifiers::NONE),
        ];
        for event in &typed {
            assert!(tool.handle(&mut cx, event).expect("the tool handles keys"));
        }
        assert_eq!(figure.subplot_params().left, 0.3);
        let slider = tool
            .window
            .ui_mut()
            .widget_mut::<Slider<SubplotMessage>>(left.slider)
            .map(|slider| slider.value());
        assert_eq!(slider, Some(0.3), "the slider follows the field");
        tool.reset(&mut cx).expect("the tool resets");
        assert_eq!(figure.subplot_params(), initial);
        assert!(
            !tool
                .handle(&mut cx, &WindowEvent::CloseRequested)
                .expect("the tool handles closing")
        );
        tool.close(&mut cx).expect("the tool closes");
        assert_eq!(handle.windows(), Vec::<WindowId>::new());
    }

    #[test]
    fn borders_never_pass_their_opposites() {
        let params = SubplotParams {
            left: 0.2,
            bottom: 0.1,
            right: 0.8,
            top: 0.9,
            wspace: 0.2,
            hspace: 0.3,
        };
        assert_eq!(Param::Left.limit(0.95, &params), 0.8);
        assert_eq!(Param::Right.limit(0.1, &params), 0.2);
        assert_eq!(Param::Top.limit(-1.0, &params), 0.1);
        assert_eq!(Param::Bottom.limit(0.5, &params), 0.5);
        assert_eq!(Param::Wspace.limit(3.0, &params), 1.0);
        assert_eq!(Param::Hspace.get(&params), 0.3);
        assert_eq!(Param::Top.overrides(0.7).top, Some(0.7));
        assert_eq!(Param::Top.overrides(0.7).left, None);
        assert_eq!(value_text(0.125), "0.125");
        let names: Vec<&str> = Param::BORDERS
            .iter()
            .chain(&Param::SPACINGS)
            .map(|p| p.name())
            .collect();
        assert_eq!(
            names,
            ["top", "bottom", "left", "right", "hspace", "wspace"]
        );
    }
}
