//! Translation of X keyboard and pointer events into window events,
//! compose sequences included.

use super::keymap::Keymap;
use super::server_event::PointerInfo;
use crate::click::ClickCounter;
use crate::compose::Composer;
use crate::event::{ButtonState, KeyEvent, MouseButton, ScrollDelta, WindowEvent};
use crate::geometry::PhysicalPosition;

/// Turns raw key and button codes into events, using the keyboard mapping
/// and remembering recent clicks and the compose sequence being typed.
#[derive(Debug)]
pub(crate) struct Input {
    keymap: Keymap,
    clicks: ClickCounter,
    composer: Composer,
}

impl Input {
    /// Translates input with `keymap`, composing with the process's
    /// Compose file.
    pub(crate) fn new(keymap: Keymap) -> Self {
        Self::with_composer(keymap, Composer::default())
    }

    /// Translates input with `keymap`, composing with `composer`.
    pub(crate) fn with_composer(keymap: Keymap, composer: Composer) -> Self {
        Self {
            keymap,
            clicks: ClickCounter::default(),
            composer,
        }
    }

    /// Replaces the keyboard mapping after the server reported a change.
    pub(crate) fn set_keymap(&mut self, keymap: Keymap) {
        self.keymap = keymap;
    }

    /// A key event for `keycode` under the modifier state `mask`.
    pub(crate) fn key(
        &mut self,
        state: ButtonState,
        keycode: u8,
        mask: u16,
        repeat: bool,
    ) -> KeyEvent {
        let keysym = self.keymap.keysym(keycode, mask);
        let modifiers = self.keymap.modifiers(mask);
        self.composer
            .key_event(keysym, state, modifiers, repeat, u32::from(keycode))
    }

    /// Ends the compose sequence being typed: the keyboard focus left.
    pub(crate) fn focus_lost(&mut self) {
        self.composer.reset();
    }

    /// The event for a button press or release: wheel buttons (4 to 7)
    /// scroll on press and are ignored on release.
    pub(crate) fn button(
        &mut self,
        state: ButtonState,
        button: u8,
        pointer: &PointerInfo,
    ) -> Option<WindowEvent> {
        let position = position(pointer);
        let modifiers = self.keymap.modifiers(pointer.mask);
        if let Some((x, y)) = wheel_direction(button) {
            return state.is_pressed().then_some(WindowEvent::MouseWheel {
                delta: ScrollDelta::Lines { x, y },
                position,
                modifiers,
            });
        }
        let button = mouse_button(button);
        let click_count = if state.is_pressed() {
            self.clicks.press(button, pointer.time, position)
        } else {
            1
        };
        Some(WindowEvent::MouseInput {
            button,
            state,
            position,
            modifiers,
            click_count,
        })
    }

    /// The event for pointer motion.
    pub(crate) fn motion(&self, pointer: &PointerInfo) -> WindowEvent {
        WindowEvent::CursorMoved {
            position: position(pointer),
            modifiers: self.keymap.modifiers(pointer.mask),
        }
    }
}

/// The pointer position of an event.
pub(crate) fn position(pointer: &PointerInfo) -> PhysicalPosition {
    PhysicalPosition::new(f64::from(pointer.x), f64::from(pointer.y))
}

/// Scroll notches of the wheel buttons: 4 up, 5 down, 6 left, 7 right.
const fn wheel_direction(button: u8) -> Option<(f64, f64)> {
    match button {
        4 => Some((0.0, 1.0)),
        5 => Some((0.0, -1.0)),
        6 => Some((-1.0, 0.0)),
        7 => Some((1.0, 0.0)),
        _ => None,
    }
}

const fn mouse_button(button: u8) -> MouseButton {
    match button {
        1 => MouseButton::Left,
        2 => MouseButton::Middle,
        3 => MouseButton::Right,
        8 => MouseButton::Back,
        9 => MouseButton::Forward,
        other => MouseButton::Other(other as u16),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyboard::{Key, Modifiers};
    use crate::x11::fixtures;

    const SHIFT: u16 = 1;
    const CONTROL: u16 = 4;
    const KEY_A: u8 = 38;
    const KEY_ESCAPE: u8 = 9;
    const KEY_SHIFT_L: u8 = 50;

    fn input() -> Input {
        Input::with_composer(fixtures::keymap(), Composer::without_sequences())
    }

    fn pointer(time: u32, mask: u16) -> PointerInfo {
        PointerInfo {
            window: 1,
            time,
            x: 3,
            y: 4,
            mask,
        }
    }

    #[test]
    fn printable_keys_carry_text() {
        let mut input = input();
        let event = input.key(ButtonState::Pressed, KEY_A, SHIFT, false);
        assert_eq!(event.key, Key::Character('A'));
        assert_eq!(event.text.as_deref(), Some("A"));
        assert_eq!(event.modifiers, Modifiers::SHIFT);
        assert_eq!(event.scancode, 38);
        let release = input.key(ButtonState::Released, KEY_A, 0, false);
        assert_eq!((release.key, release.text), (Key::Character('a'), None));
    }

    #[test]
    fn shortcuts_and_named_keys_carry_no_text() {
        let mut input = input();
        let ctrl_a = input.key(ButtonState::Pressed, KEY_A, CONTROL, false);
        assert_eq!(ctrl_a.key, Key::Character('a'));
        assert_eq!(ctrl_a.text, None);
        let escape = input.key(ButtonState::Pressed, KEY_ESCAPE, 0, true);
        assert_eq!(
            (escape.key, escape.text, escape.repeat),
            (Key::Escape, None, true)
        );
        let shift = input.key(ButtonState::Pressed, KEY_SHIFT_L, 0, false);
        assert_eq!((shift.key, shift.modifiers), (Key::Shift, Modifiers::NONE));
    }

    #[test]
    fn buttons_count_clicks_and_wheel_scrolls() {
        let mut input = input();
        let press =
            |input: &mut Input, time| input.button(ButtonState::Pressed, 1, &pointer(time, 0));
        assert!(matches!(
            press(&mut input, 100),
            Some(WindowEvent::MouseInput { click_count: 1, .. })
        ));
        assert!(matches!(
            press(&mut input, 200),
            Some(WindowEvent::MouseInput { click_count: 2, .. })
        ));
        let release = input.button(ButtonState::Released, 3, &pointer(300, 0));
        assert!(matches!(
            release,
            Some(WindowEvent::MouseInput {
                button: MouseButton::Right,
                state: ButtonState::Released,
                click_count: 1,
                ..
            })
        ));
        let scroll = input.button(ButtonState::Pressed, 5, &pointer(400, CONTROL));
        assert_eq!(
            scroll,
            Some(WindowEvent::MouseWheel {
                delta: ScrollDelta::Lines { x: 0.0, y: -1.0 },
                position: PhysicalPosition::new(3.0, 4.0),
                modifiers: Modifiers::CONTROL,
            })
        );
        assert_eq!(
            input.button(ButtonState::Released, 5, &pointer(410, 0)),
            None
        );
        let side = input.button(ButtonState::Pressed, 8, &pointer(500, 0));
        assert!(matches!(
            side,
            Some(WindowEvent::MouseInput {
                button: MouseButton::Back,
                ..
            })
        ));
        let other = input.button(ButtonState::Pressed, 12, &pointer(600, 0));
        assert!(matches!(
            other,
            Some(WindowEvent::MouseInput {
                button: MouseButton::Other(12),
                ..
            })
        ));
    }

    #[test]
    fn motion_reports_position_and_modifiers() {
        let input = input();
        assert_eq!(
            input.motion(&pointer(0, SHIFT | 0x100)),
            WindowEvent::CursorMoved {
                position: PhysicalPosition::new(3.0, 4.0),
                modifiers: Modifiers::SHIFT,
            }
        );
    }
}
