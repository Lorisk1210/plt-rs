//! Input on the figure canvas: figure events for connected callbacks,
//! pan/zoom navigation and keyboard shortcuts.

use plt_core::Point;
use plt_platform::{ButtonState, Context, Key, KeyEvent, Modifiers, MouseButton, ScrollDelta};
use plt_ui::{Rect, Surface, UiEvent};

use super::toolbar::Tool;
use super::{FigureWindow, Message};
use crate::gui::GuiError;
use crate::gui::actions;
use crate::gui::keys::{self, KeyAction};
use crate::navigation::{DragKey, PointerButton, Rubberband};

impl FigureWindow {
    /// Display coordinates (from the bottom-left corner of the figure) of
    /// a canvas position (from its top-left corner).
    fn to_display(&self, position: Point) -> Point {
        Point::new(position.x, f64::from(self.canvas_size.height) - position.y)
    }

    fn drag_key(&self, modifiers: Modifiers) -> DragKey {
        match self.held {
            Some('x') => DragKey::X,
            Some('y') => DragKey::Y,
            _ if modifiers.control() => DragKey::Control,
            _ if modifiers.shift() => DragKey::Shift,
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
        match event {
            UiEvent::PointerEnter(position) => {
                let at = self.to_display(*position);
                self.events.figure_enter(&self.figure, at.x, at.y);
            }
            UiEvent::PointerMove {
                position,
                modifiers,
            } => {
                let at = self.to_display(*position);
                self.pointer = Some(at);
                self.events.motion(&self.figure, at.x, at.y);
                if self.navigation.is_dragging() {
                    let rubberband = self.navigation.drag(at, self.drag_key(*modifiers));
                    self.show_rubberband(cx, rubberband)?;
                }
                let readout = self.navigation.readout(&self.figure, at);
                self.set_status(cx, readout)?;
            }
            UiEvent::PointerDown {
                button,
                position,
                click_count,
                ..
            } => {
                if let Some(button) = pointer_button(*button) {
                    let at = self.to_display(*position);
                    self.events
                        .button_press(&self.figure, at.x, at.y, button, *click_count == 2);
                    self.navigation.press(&self.figure, at, button);
                }
            }
            UiEvent::PointerUp {
                button,
                position,
                modifiers,
            } => {
                let at = self.to_display(*position);
                if let Some(button) = pointer_button(*button) {
                    self.events.button_release(&self.figure, at.x, at.y, button);
                }
                if self.navigation.is_dragging() {
                    self.navigation
                        .release(&self.figure, at, self.drag_key(*modifiers));
                    self.show_rubberband(cx, None)?;
                    self.refresh_tools(cx)?;
                }
            }
            UiEvent::Wheel {
                delta, position, ..
            } => {
                let steps = match *delta {
                    ScrollDelta::Lines { y, .. } => y,
                    ScrollDelta::Pixels { y, .. } => y / 40.0,
                };
                let at = self.to_display(*position);
                self.events.scroll(&self.figure, at.x, at.y, steps);
                self.navigation.wheel(&self.figure, at, steps);
                self.refresh_tools(cx)?;
            }
            UiEvent::PointerLeave => {
                self.pointer = None;
                self.events.figure_leave(&self.figure);
                self.set_status(cx, String::new())?;
            }
            UiEvent::Key(key) => return self.key(cx, key),
            _ => {}
        }
        Ok(true)
    }

    /// Delivers a key to the figure's callbacks, then runs its `keymap.*`
    /// shortcut unless the figure captures keys; the navigation and save
    /// shortcuts need the toolbar, as in matplotlib.
    fn key(&mut self, cx: &mut Context<'_>, event: &KeyEvent) -> Result<bool, GuiError> {
        if let Key::Character(c @ ('x' | 'y')) = event.key {
            self.held = (event.state == ButtonState::Pressed).then_some(c);
        }
        let Some(name) = keys::key_name(event) else {
            return Ok(true);
        };
        let at = self.pointer.unwrap_or(Point::new(f64::NAN, f64::NAN));
        if event.state != ButtonState::Pressed {
            self.events.key_release(&self.figure, &name, at.x, at.y);
            return Ok(true);
        }
        self.events.key_press(&self.figure, &name, at.x, at.y);
        if self.figure.captures_keys() {
            return Ok(true);
        }
        // The figure's own tools come first, as they were bound on purpose.
        if let Some(tool) = self.figure.tool_for_key(&name) {
            self.figure.trigger_tool(&tool);
            return self.refresh_tools(cx).map(|()| true);
        }
        let rc = crate::rc::params();
        let Some(action) = keys::action(&name, &rc) else {
            return Ok(true);
        };
        match action {
            KeyAction::Quit => return Ok(false),
            KeyAction::Fullscreen => {
                self.fullscreen = !self.fullscreen;
                cx.set_fullscreen(self.window.id(), self.fullscreen)?;
            }
            KeyAction::Grid | KeyAction::GridMinor | KeyAction::LogX | KeyAction::LogY => {
                let target = self
                    .pointer
                    .and_then(|at| crate::navigation::axes_at(&self.figure, at));
                if let Some(ax) = target {
                    actions::apply_to_axes(&ax, action);
                }
            }
            _ if self.toolbar.is_none() => {}
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

/// The navigation button of a mouse button.
fn pointer_button(button: MouseButton) -> Option<PointerButton> {
    match button {
        MouseButton::Left => Some(PointerButton::Left),
        MouseButton::Middle => Some(PointerButton::Middle),
        MouseButton::Right => Some(PointerButton::Right),
        _ => None,
    }
}
