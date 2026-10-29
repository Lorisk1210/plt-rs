//! Key routing: focus traversal with Tab, keys for the focused widget,
//! and shortcuts for the keys it leaves unused.

use pltrs_platform::{ButtonState, Key, KeyEvent};

use super::Ui;
use crate::event::UiEvent;
use crate::widget::{WidgetId, WidgetPod};

impl<M: 'static> Ui<M> {
    /// Handles a key. Tab and Shift+Tab move the focus, unless the
    /// focused widget wants them; Ctrl+Tab and Ctrl+Shift+Tab move it
    /// even then (as in GTK), unless a tab bar takes them. Other key
    /// presses go to the key target, and those it leaves unused to the
    /// first widget shown that has them as a shortcut (see
    /// [`Widget::handles_shortcut`](crate::Widget::handles_shortcut));
    /// releases follow their presses.
    pub(super) fn key(&mut self, key: &KeyEvent) {
        self.keep_focus_shown();
        let pressed = key.state == ButtonState::Pressed;
        let popup = self.routing.popup.is_some();
        if key.key == Key::Tab && !popup && self.moves_focus(key) {
            if pressed && !(key.modifiers.control() && self.shortcut(key, None).is_some()) {
                self.move_focus(!key.modifiers.shift());
            }
            return;
        }
        let target = self.routing.key_target();
        if !pressed {
            let owner = self.routing.release(key).or(target);
            self.deliver(owner, &UiEvent::Key(key.clone()));
            return;
        }
        let owner = if self.deliver(target, &UiEvent::Key(key.clone())) {
            self.dismiss_tooltip();
            target
        } else if popup {
            target
        } else {
            self.shortcut(key, target).or(target)
        };
        self.routing.press(key, owner);
    }

    /// Whether the Tab key `key` moves the focus rather than going to the
    /// focused widget.
    fn moves_focus(&self, key: &KeyEvent) -> bool {
        let wanted = self
            .routing
            .key_target()
            .and_then(|id| self.root.find(id))
            .is_some_and(|pod| pod.widget().wants_tab());
        !wanted || key.modifiers.control()
    }

    /// Delivers `key` to the first widget shown that has it as a
    /// shortcut, other than `except` (which left it unused); returns that
    /// widget, if there was one.
    fn shortcut(&mut self, key: &KeyEvent, except: Option<WidgetId>) -> Option<WidgetId> {
        let id = shortcut_target(&self.root, key, except)?;
        self.deliver(Some(id), &UiEvent::Key(key.clone()));
        Some(id)
    }
}

/// The first widget shown, in tree order, other than `except`, that has
/// `key` as a shortcut.
fn shortcut_target<M: 'static>(
    pod: &WidgetPod<M>,
    key: &KeyEvent,
    except: Option<WidgetId>,
) -> Option<WidgetId> {
    if Some(pod.id()) != except && pod.widget().handles_shortcut(key) {
        return Some(pod.id());
    }
    pod.widget()
        .children()
        .iter()
        .find_map(|child| shortcut_target(child, key, except))
}
