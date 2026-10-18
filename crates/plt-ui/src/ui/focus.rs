//! Keyboard focus: Tab and Shift+Tab move it between the widgets that
//! accept it.

use plt_platform::{ButtonState, Key, KeyEvent};

use super::Ui;
use crate::event::UiEvent;
use crate::widget::{WidgetId, WidgetPod};

impl<M: 'static> Ui<M> {
    /// Handles a key: Tab and Shift+Tab move the focus, unless the focused
    /// widget wants them; other keys go to the key target.
    pub(super) fn key(&mut self, key: &KeyEvent) {
        let target = self.routing.key_target();
        if key.key == Key::Tab && self.routing.popup.is_none() {
            let wanted = target
                .and_then(|id| self.root.find(id))
                .is_some_and(|pod| pod.widget().wants_tab());
            if !wanted {
                if key.state == ButtonState::Pressed {
                    self.move_focus(!key.modifiers.shift());
                }
                return;
            }
        }
        self.deliver(target, &UiEvent::Key(key.clone()));
    }

    /// Moves the focus to the next (`forward`) or previous widget shown
    /// that accepts it, in tree order, wrapping around.
    fn move_focus(&mut self, forward: bool) {
        let mut order = Vec::new();
        focusable(&self.root, &mut order);
        let count = order.len();
        if count == 0 {
            return;
        }
        let current = self
            .routing
            .focus
            .and_then(|focus| order.iter().position(|&id| id == focus));
        let next = match current {
            Some(index) if forward => (index + 1) % count,
            Some(index) => (index + count - 1) % count,
            None if forward => 0,
            None => count - 1,
        };
        let id = order[next];
        self.routing.focus = Some(id);
        if let Some(pod) = self.root.find_mut(id) {
            pod.widget_mut().focused_by_keyboard();
        }
        self.needs_paint = true;
    }
}

/// Appends the widgets of `pod`'s subtree that are shown and accept the
/// focus, in tree order.
fn focusable<M: 'static>(pod: &WidgetPod<M>, order: &mut Vec<WidgetId>) {
    if pod.widget().accepts_focus() {
        order.push(pod.id());
    }
    for child in pod.widget().children() {
        focusable(child, order);
    }
}
