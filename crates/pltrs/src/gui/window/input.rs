//! Input on the figure canvas: figure events for connected callbacks,
//! pan/zoom navigation and keyboard shortcuts, from the input as
//! matplotlib's Qt backend reports it ([`crate::gui::input`]).

use pltrs_core::Point;
use pltrs_platform::{ButtonState, Context, Key};
use pltrs_ui::{Rect, Surface, UiEvent};

use super::toolbar::Tool;
use super::{FigureWindow, Message};
use crate::gui::GuiError;
use crate::gui::actions;
use crate::gui::input::{CanvasInput, InputKind};
use crate::gui::keys::{self, HistoryStep, KeyAction};
use crate::navigation::{DragKey, PointerButton, Rubberband};

impl FigureWindow {
    /// Display coordinates (from the bottom-left corner of the figure) of
    /// a canvas position (from its top-left corner).
    fn to_display(&self, position: Point) -> Point {
        Point::new(position.x, f64::from(self.canvas_size.height) - position.y)
    }

    /// How a drag or a wheel turn is constrained: by the `x` or `y` key
    /// held, else by Control or Shift among `modifiers` (matplotlib's
    /// names).
    fn drag_key(&self, modifiers: &[&str]) -> DragKey {
        match self.held {
            Some('x') => DragKey::X,
            Some('y') => DragKey::Y,
            _ if modifiers.contains(&"ctrl") => DragKey::Control,
            _ if modifiers.contains(&"shift") => DragKey::Shift,
            _ => DragKey::None,
        }
    }

    /// Handles input on the canvas; returns `false` once the window
    /// should close.
    pub(super) fn canvas_input(
        &mut self,
        cx: &mut Context<'_>,
        event: &UiEvent,
    ) -> Result<bool, GuiError> {
        if let UiEvent::Key(key) = event
            && let Key::Character(c @ ('x' | 'y')) = key.key
        {
            self.held = (key.state == ButtonState::Pressed).then_some(c);
        }
        for input in self.input.convert(event) {
            if !self.canvas_event(cx, input)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Handles one event of the canvas; returns `false` once the window
    /// should close.
    fn canvas_event(&mut self, cx: &mut Context<'_>, input: CanvasInput) -> Result<bool, GuiError> {
        let at = input.position.map(|position| self.to_display(position));
        let (x, y) = at.map_or((f64::NAN, f64::NAN), |at| (at.x, at.y));
        match input.kind {
            InputKind::Enter => {
                self.pointer = at;
                self.events.figure_enter(&self.figure, x, y);
            }
            InputKind::Leave => {
                self.pointer = at;
                self.events.figure_leave(&self.figure, x, y);
                self.set_status(cx, String::new())?;
            }
            InputKind::Motion => {
                self.pointer = at;
                self.events.motion(&self.figure, x, y);
                if let Some(at) = at {
                    if self.navigation.is_dragging() {
                        let key = self.drag_key(&input.modifiers);
                        let rubberband = self.navigation.drag(at, key);
                        self.show_rubberband(cx, rubberband)?;
                    }
                    let readout = self.navigation.readout(&self.figure, at);
                    self.set_status(cx, readout)?;
                }
            }
            InputKind::Press { button, double } => {
                let button = pointer_button(button);
                if let (Some(button), Some(at)) = (button, at) {
                    self.events.button_press(&self.figure, x, y, button, double);
                    self.navigation.press(&self.figure, at, button);
                    match keys::history_step(button, &crate::rc::params()) {
                        Some(HistoryStep::Back) => self.use_tool(cx, Tool::Back)?,
                        Some(HistoryStep::Forward) => self.use_tool(cx, Tool::Forward)?,
                        None => {}
                    }
                }
            }
            InputKind::Release { button } => {
                if let Some(button) = pointer_button(button) {
                    self.events.button_release(&self.figure, x, y, button);
                }
                if let Some(at) = at
                    && self.navigation.is_dragging()
                {
                    let key = self.drag_key(&input.modifiers);
                    self.navigation.release(&self.figure, at, key);
                    self.show_rubberband(cx, None)?;
                    self.refresh_tools(cx)?;
                }
            }
            InputKind::Scroll { step } => {
                self.events.scroll(&self.figure, x, y, step);
                if let Some(at) = at {
                    let key = self.drag_key(&input.modifiers);
                    self.navigation.wheel(&self.figure, at, step, key);
                    self.refresh_tools(cx)?;
                }
            }
            InputKind::KeyPress(name) => return self.key_press(cx, &name, x, y),
            InputKind::KeyRelease(name) => self.events.key_release(&self.figure, &name, x, y),
        }
        Ok(true)
    }

    /// Delivers the press of key `name` (matplotlib's) with the pointer
    /// at display point `(x, y)` to the figure's callbacks, then runs its
    /// `keymap.*` shortcut unless the figure captures keys; returns
    /// `false` once the window should close.
    fn key_press(
        &mut self,
        cx: &mut Context<'_>,
        name: &str,
        x: f64,
        y: f64,
    ) -> Result<bool, GuiError> {
        self.events.key_press(&self.figure, name, x, y);
        if self.figure.captures_keys() {
            return Ok(true);
        }
        // The figure's own tools come first, as they were bound on purpose.
        if let Some(tool) = self.figure.tool_for_key(name) {
            self.figure.trigger_tool(&tool);
            return self.refresh_tools(cx).map(|()| true);
        }
        let rc = crate::rc::params();
        let Some(action) = keys::action(name, &rc) else {
            return Ok(true);
        };
        match action {
            KeyAction::Quit => return Ok(false),
            KeyAction::QuitAll => {
                self.quit_all = true;
                return Ok(false);
            }
            KeyAction::Fullscreen => {
                self.fullscreen = !self.fullscreen;
                cx.set_fullscreen(self.window.id(), self.fullscreen)?;
            }
            KeyAction::Copy => self.copy_to_clipboard(cx)?,
            KeyAction::Grid | KeyAction::GridMinor | KeyAction::LogX | KeyAction::LogY => {
                let target = self
                    .pointer
                    .and_then(|at| crate::navigation::axes_at(&self.figure, at));
                if let Some(ax) = target {
                    actions::apply_to_axes(&ax, action);
                }
            }
            KeyAction::Home => self.use_tool(cx, Tool::Home)?,
            KeyAction::Back => self.use_tool(cx, Tool::Back)?,
            KeyAction::Forward => self.use_tool(cx, Tool::Forward)?,
            KeyAction::Pan => self.use_tool(cx, Tool::Pan)?,
            KeyAction::Zoom => self.use_tool(cx, Tool::Zoom)?,
            KeyAction::Save => self.use_tool(cx, Tool::Save)?,
            KeyAction::Help => self.use_tool(cx, Tool::Help)?,
        }
        Ok(true)
    }

    /// Shows the zoom rectangle (display coordinates) on the canvas.
    fn show_rubberband(
        &mut self,
        cx: &mut Context<'_>,
        rubberband: Option<Rubberband>,
    ) -> Result<(), GuiError> {
        let height = f64::from(self.canvas_size.height);
        let selection = rubberband.map(|band| {
            let b = band.bbox;
            Rect::new(b.x0(), height - b.y0(), b.x1() - b.x0(), b.y0() - b.y1())
        });
        self.window
            .update(cx, self.canvas, |canvas: &mut Surface<Message>| {
                canvas.set_selection(selection);
            })?;
        Ok(())
    }
}

/// The navigation button of matplotlib's button number `number`.
fn pointer_button(number: u16) -> Option<PointerButton> {
    u8::try_from(number)
        .ok()
        .and_then(PointerButton::from_number)
}
