//! The keyboard: its keymap, modifier state and focus, key events, and
//! key repeat, which Wayland leaves to clients.

use std::fs::File;
use std::os::fd::OwnedFd;
use std::os::unix::fs::FileExt;
use std::time::{Duration, Instant};

use super::protocol::Event;
use super::protocol::input::KEYMAP_XKB_V1;
use super::xkb::Keymap;
use super::{WaylandBackend, emit};
use crate::error::Error;
use crate::event::{ButtonState, Event as LoopEvent, WindowEvent};
use crate::keyboard::Modifiers;
use crate::keysym;

/// The largest keymap read (compositors send about 70 KiB).
const MAX_KEYMAP_LEN: u32 = 4 << 20;
/// Linux key codes are XKB key codes minus 8.
const KEYCODE_OFFSET: u32 = 8;

/// How keys repeat (`wl_keyboard.repeat_info`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RepeatInfo {
    /// Repeats per second; 0 disables repeating.
    rate: u32,
    /// Delay before the first repeat.
    delay: Duration,
}

impl Default for RepeatInfo {
    /// GNOME's defaults, until the compositor tells.
    fn default() -> Self {
        Self {
            rate: 33,
            delay: Duration::from_millis(500),
        }
    }
}

/// The key being repeated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Held {
    keycode: u32,
    window: u32,
    next: Instant,
}

/// The seat's keyboard.
#[derive(Debug)]
pub(crate) struct Keyboard {
    /// The `wl_keyboard` object.
    pub(crate) id: u32,
    keymap: Option<Keymap>,
    /// The effective modifiers (depressed, latched and locked).
    mods: u32,
    /// The active layout.
    group: u32,
    /// The focused window (surface id).
    focus: Option<u32>,
    repeat: RepeatInfo,
    held: Option<Held>,
}

impl Keyboard {
    /// A keyboard without a keymap yet.
    pub(crate) fn new(id: u32) -> Self {
        Self {
            id,
            keymap: None,
            mods: 0,
            group: 0,
            focus: None,
            repeat: RepeatInfo::default(),
            held: None,
        }
    }

    /// The modifiers held.
    pub(crate) fn modifiers(&self) -> Modifiers {
        self.keymap
            .as_ref()
            .map_or(Modifiers::NONE, |keymap| keymap.modifiers(self.mods))
    }

    /// The key event of `keycode` in the current state.
    fn event(&self, keycode: u32, state: ButtonState, repeat: bool) -> WindowEvent {
        let keysym = self.keymap.as_ref().map_or(keysym::NO_SYMBOL, |keymap| {
            keymap.keysym(keycode, self.mods, self.group)
        });
        WindowEvent::Key(keysym::key_event(
            keysym,
            state,
            self.modifiers(),
            repeat,
            keycode,
        ))
    }

    /// When the held key repeats next.
    pub(crate) fn next_repeat(&self) -> Option<Instant> {
        self.held.map(|held| held.next)
    }

    /// The repeats due by `now`, with their windows (missed repeats are
    /// skipped, not queued).
    fn repeats(&mut self, now: Instant) -> Option<(u32, WindowEvent)> {
        let held = self.held.as_mut()?;
        if now < held.next || self.repeat.rate == 0 {
            return None;
        }
        let interval = Duration::from_secs(1) / self.repeat.rate;
        while held.next <= now {
            held.next += interval;
        }
        let (keycode, window) = (held.keycode, held.window);
        Some((window, self.event(keycode, ButtonState::Pressed, true)))
    }
}

/// Reads a keymap from the file the compositor sent.
fn read_keymap(fd: OwnedFd, size: u32) -> Result<Keymap, Error> {
    if size > MAX_KEYMAP_LEN {
        return Err(Error::Malformed("keymap too large"));
    }
    let mut text = vec![0; size as usize];
    // `pread` leaves the file offset alone, which compositors may share
    // between clients.
    File::from(fd)
        .read_exact_at(&mut text, 0)
        .map_err(|error| Error::io("reading the keymap", &error))?;
    let text = String::from_utf8_lossy(&text);
    Keymap::parse(text.trim_end_matches('\0'))
}

impl WaylandBackend {
    /// Handles an event of the keyboard.
    pub(super) fn keyboard_event(&mut self, event: Event, out: &mut Vec<LoopEvent>) {
        let Some(keyboard) = self.seat.as_mut().and_then(|seat| seat.keyboard.as_mut()) else {
            return;
        };
        match event {
            Event::Keymap { format, fd, size } => {
                // A keymap that cannot be read leaves the previous one: keys
                // then report no symbol rather than ending the program.
                if format == KEYMAP_XKB_V1
                    && let Ok(keymap) = read_keymap(fd, size)
                {
                    keyboard.keymap = Some(keymap);
                }
            }
            Event::KeyboardEnter { serial, surface } => {
                keyboard.focus = Some(surface);
                self.note_serial(serial);
                self.focus(out, surface, true);
            }
            Event::KeyboardLeave { surface } => {
                keyboard.focus = None;
                keyboard.held = None;
                self.focus(out, surface, false);
            }
            Event::Key {
                serial,
                key,
                pressed,
            } => {
                let Some(window) = keyboard.focus else {
                    return;
                };
                let keycode = key.saturating_add(KEYCODE_OFFSET);
                let state = if pressed {
                    ButtonState::Pressed
                } else {
                    ButtonState::Released
                };
                let event = keyboard.event(keycode, state, false);
                if pressed {
                    let repeats = keyboard.keymap.as_ref().is_some_and(|k| k.repeats(keycode));
                    if repeats && keyboard.repeat.rate > 0 {
                        let next = Instant::now() + keyboard.repeat.delay;
                        keyboard.held = Some(Held {
                            keycode,
                            window,
                            next,
                        });
                    }
                } else if keyboard.held.is_some_and(|held| held.keycode == keycode) {
                    keyboard.held = None;
                }
                self.note_serial(serial);
                emit(out, window, event);
            }
            Event::Modifiers {
                depressed,
                latched,
                locked,
                group,
            } => {
                keyboard.mods = depressed | latched | locked;
                keyboard.group = group;
            }
            Event::RepeatInfo { rate, delay } => {
                keyboard.repeat = RepeatInfo {
                    rate: u32::try_from(rate).unwrap_or(0).min(1000),
                    delay: Duration::from_millis(u64::try_from(delay).unwrap_or(0)),
                };
            }
            _ => {}
        }
    }

    /// Reports a focus change of `window`.
    fn focus(&self, out: &mut Vec<LoopEvent>, window: u32, focused: bool) {
        if self.windows.contains_key(&window) {
            emit(out, window, WindowEvent::Focused(focused));
        }
    }

    /// The modifiers held, for pointer events.
    pub(super) fn modifiers(&self) -> Modifiers {
        self.seat
            .as_ref()
            .and_then(|seat| seat.keyboard.as_ref())
            .map_or(Modifiers::NONE, Keyboard::modifiers)
    }

    /// When the held key repeats next, if one is held.
    pub(super) fn next_repeat(&self) -> Option<Instant> {
        self.seat.as_ref()?.keyboard.as_ref()?.next_repeat()
    }

    /// Reports the key repeats due by `now`.
    pub(super) fn fire_repeats(&mut self, now: Instant, out: &mut Vec<LoopEvent>) {
        let repeat = self
            .seat
            .as_mut()
            .and_then(|seat| seat.keyboard.as_mut())
            .and_then(|keyboard| keyboard.repeats(now));
        if let Some((window, event)) = repeat
            && self.windows.contains_key(&window)
        {
            emit(out, window, event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyboard::Key;
    use crate::wayland::fixtures;

    fn keyboard() -> Keyboard {
        let mut keyboard = Keyboard::new(5);
        keyboard.keymap = Some(fixtures::keymap());
        keyboard
    }

    #[test]
    fn keys_translate_through_the_keymap() {
        let mut keyboard = keyboard();
        let WindowEvent::Key(a) = keyboard.event(38, ButtonState::Pressed, false) else {
            panic!("a key event");
        };
        assert_eq!(
            (a.key, a.text.as_deref(), a.scancode),
            (Key::Character('a'), Some("a"), 38)
        );
        keyboard.mods = 1 | 4;
        let WindowEvent::Key(ctrl) = keyboard.event(38, ButtonState::Pressed, false) else {
            panic!("a key event");
        };
        assert_eq!(ctrl.key, Key::Character('A'));
        assert_eq!(ctrl.text, None, "a shortcut");
        assert_eq!(ctrl.modifiers, Modifiers::SHIFT | Modifiers::CONTROL);
        keyboard.keymap = None;
        let WindowEvent::Key(unknown) = keyboard.event(38, ButtonState::Released, false) else {
            panic!("a key event");
        };
        assert_eq!(unknown.key, Key::Unidentified(0));
    }

    #[test]
    fn held_keys_repeat_at_the_rate() {
        let mut keyboard = keyboard();
        let start = Instant::now();
        keyboard.repeat = RepeatInfo {
            rate: 10,
            delay: Duration::from_millis(300),
        };
        keyboard.held = Some(Held {
            keycode: 38,
            window: 3,
            next: start + Duration::from_millis(300),
        });
        assert_eq!(keyboard.repeats(start), None);
        let (window, event) = keyboard
            .repeats(start + Duration::from_millis(350))
            .unwrap();
        assert_eq!(window, 3);
        assert!(matches!(event, WindowEvent::Key(key) if key.repeat));
        assert_eq!(
            keyboard.next_repeat(),
            Some(start + Duration::from_millis(400))
        );
        keyboard.repeats(start + Duration::from_millis(1000));
        assert_eq!(
            keyboard.next_repeat(),
            Some(start + Duration::from_millis(1100)),
            "missed repeats are skipped"
        );
        keyboard.repeat.rate = 0;
        assert_eq!(keyboard.repeats(start + Duration::from_secs(5)), None);
    }

    #[test]
    fn keymaps_are_read_from_their_file() {
        let file = crate::wayland::sys::memfd(c"keymap").unwrap();
        file.write_all_at(fixtures::DE_RU_KEYMAP.as_bytes(), 0)
            .unwrap();
        let size = fixtures::DE_RU_KEYMAP.len() as u32;
        let keymap = read_keymap(OwnedFd::from(file.try_clone().unwrap()), size).unwrap();
        assert_eq!(keymap.keysym(38, 0, 0), 0x61);
        assert!(read_keymap(OwnedFd::from(file.try_clone().unwrap()), size + 10).is_err());
        assert!(read_keymap(OwnedFd::from(file), MAX_KEYMAP_LEN + 1).is_err());
    }
}
