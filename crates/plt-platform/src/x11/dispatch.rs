//! Turning batches of server events into crate events, and answering the
//! protocol messages (pings, keymap and resource changes) along the way.

use std::collections::VecDeque;

use super::X11Backend;
use super::atoms::predefined;
use super::connection::RawEvent;
use super::input;
use super::keymap::Keymap;
use super::request::{self, event_mask};
use super::resources;
use super::server_event::{PointerInfo, ServerEvent};
use crate::error::Error;
use crate::event::{ButtonState, Event, WindowEvent};
use crate::geometry::PhysicalSize;
use crate::window::WindowId;

impl X11Backend {
    /// Translates a batch of raw events, in arrival order, into `out`.
    pub(super) fn dispatch(
        &mut self,
        raw: &VecDeque<RawEvent>,
        out: &mut Vec<Event>,
    ) -> Result<(), Error> {
        let events: Vec<ServerEvent> = raw.iter().map(ServerEvent::parse).collect();
        let mut repeating = false;
        for (index, &event) in events.iter().enumerate() {
            match event {
                ServerEvent::Key {
                    state,
                    keycode,
                    pointer,
                } => {
                    if state == ButtonState::Released
                        && is_repeat_press(events.get(index + 1), keycode, pointer.time)
                    {
                        // Auto-repeat sends release/press pairs with equal
                        // timestamps: drop the release, flag the press.
                        repeating = true;
                        continue;
                    }
                    let repeat = std::mem::take(&mut repeating);
                    self.last_input_time = pointer.time;
                    let key = self.input.key(state, keycode, pointer.mask, repeat);
                    self.emit(out, pointer.window, WindowEvent::Key(key));
                }
                ServerEvent::Button {
                    state,
                    button,
                    pointer,
                } => {
                    self.last_input_time = pointer.time;
                    if let Some(event) = self.input.button(state, button, &pointer) {
                        self.emit(out, pointer.window, event);
                    }
                }
                ServerEvent::Motion(pointer) => self.motion(out, &pointer),
                ServerEvent::Crossing { entered, pointer } => self.crossing(out, entered, &pointer),
                ServerEvent::Focus { window, focused } => self.focus(out, window, focused),
                ServerEvent::Expose { window } => {
                    self.emit(out, window, WindowEvent::RedrawRequested);
                }
                ServerEvent::Configure {
                    window,
                    width,
                    height,
                } => {
                    let size = PhysicalSize::new(width.into(), height.into());
                    self.configure(out, window, size);
                }
                ServerEvent::Property { window, atom } => {
                    if window == self.root && atom == predefined::RESOURCE_MANAGER {
                        self.update_scale_factor(out)?;
                    }
                }
                ServerEvent::ClientMessage {
                    window,
                    message_type,
                    data,
                } => self.client_message(out, window, message_type, data)?,
                ServerEvent::SelectionRequest(asked) => self.answer_selection(asked)?,
                ServerEvent::SelectionNotify(answer) => self.receive_text(out, answer)?,
                ServerEvent::SelectionClear { selection } => self.clipboard_taken(selection),
                ServerEvent::KeyboardMappingChanged => {
                    self.input.set_keymap(Keymap::query(&mut self.conn)?);
                }
                ServerEvent::Ignored => {}
            }
        }
        Ok(())
    }

    /// Appends an event for `window` if it is one of ours.
    fn emit(&self, out: &mut Vec<Event>, window: u32, event: WindowEvent) {
        if self.windows.contains_key(&window) {
            let window = WindowId::from_raw(u64::from(window));
            out.push(Event::Window { window, event });
        }
    }

    /// Appends a motion event, replacing an immediately preceding motion
    /// of the same window.
    fn motion(&self, out: &mut Vec<Event>, pointer: &PointerInfo) {
        let id = WindowId::from_raw(u64::from(pointer.window));
        if let Some(Event::Window {
            window,
            event: WindowEvent::CursorMoved { .. },
        }) = out.last()
            && *window == id
        {
            out.pop();
        }
        self.emit(out, pointer.window, self.input.motion(pointer));
    }

    fn crossing(&mut self, out: &mut Vec<Event>, entered: bool, pointer: &PointerInfo) {
        let Some(state) = self.windows.get_mut(&pointer.window) else {
            return;
        };
        if state.pointer_inside == entered {
            return;
        }
        state.pointer_inside = entered;
        let event = if entered {
            WindowEvent::CursorEntered {
                position: input::position(pointer),
            }
        } else {
            WindowEvent::CursorLeft
        };
        self.emit(out, pointer.window, event);
    }

    fn focus(&mut self, out: &mut Vec<Event>, window: u32, focused: bool) {
        let Some(state) = self.windows.get_mut(&window) else {
            return;
        };
        if state.focused != focused {
            state.focused = focused;
            self.emit(out, window, WindowEvent::Focused(focused));
        }
    }

    fn configure(&mut self, out: &mut Vec<Event>, window: u32, size: PhysicalSize) {
        let Some(state) = self.windows.get_mut(&window) else {
            return;
        };
        if state.size != size {
            state.size = size;
            self.emit(out, window, WindowEvent::Resized(size));
        }
    }

    /// Re-reads `Xft.dpi` and reports a changed scale factor to every
    /// window.
    fn update_scale_factor(&mut self, out: &mut Vec<Event>) -> Result<(), Error> {
        let scale_factor = resources::query_scale_factor(&mut self.conn, self.root)?;
        if (scale_factor - self.scale_factor).abs() <= f64::EPSILON {
            return Ok(());
        }
        self.scale_factor = scale_factor;
        for &window in self.windows.keys() {
            let window = WindowId::from_raw(u64::from(window));
            out.push(Event::Window {
                window,
                event: WindowEvent::ScaleFactorChanged(scale_factor),
            });
        }
        Ok(())
    }

    /// Handles wake-ups, close requests and window-manager pings.
    fn client_message(
        &mut self,
        out: &mut Vec<Event>,
        window: u32,
        message_type: u32,
        data: [u32; 5],
    ) -> Result<(), Error> {
        if self.waker.accept(window, message_type) {
            if !out.contains(&Event::WakeUp) {
                out.push(Event::WakeUp);
            }
            return Ok(());
        }
        if message_type != self.atoms.wm_protocols || !self.windows.contains_key(&window) {
            return Ok(());
        }
        let [protocol, ..] = data;
        if protocol == self.atoms.wm_delete_window {
            self.emit(out, window, WindowEvent::CloseRequested);
        } else if protocol == self.atoms.net_wm_ping {
            // EWMH: answer by sending the message back to the root window.
            let pong = request::client_message(self.root, message_type, data);
            let mask = event_mask::SUBSTRUCTURE_NOTIFY | event_mask::SUBSTRUCTURE_REDIRECT;
            self.conn
                .send(&request::send_event(self.root, mask, &pong))?;
        }
        Ok(())
    }
}

/// Whether `next` is the press half of an auto-repeat pair.
fn is_repeat_press(next: Option<&ServerEvent>, keycode: u8, time: u32) -> bool {
    matches!(
        next,
        Some(ServerEvent::Key { state: ButtonState::Pressed, keycode: next_code, pointer })
            if *next_code == keycode && pointer.time == time
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(state: ButtonState, keycode: u8, time: u32) -> ServerEvent {
        ServerEvent::Key {
            state,
            keycode,
            pointer: PointerInfo {
                window: 1,
                time,
                x: 0,
                y: 0,
                mask: 0,
            },
        }
    }

    #[test]
    fn auto_repeat_pairs_are_recognised() {
        let press = key(ButtonState::Pressed, 38, 100);
        assert!(is_repeat_press(Some(&press), 38, 100));
        assert!(!is_repeat_press(Some(&press), 38, 99), "a real release");
        assert!(!is_repeat_press(Some(&press), 39, 100), "another key");
        assert!(!is_repeat_press(
            Some(&key(ButtonState::Released, 38, 100)),
            38,
            100
        ));
        assert!(!is_repeat_press(None, 38, 100));
    }
}
