//! Input routing: which widget receives each pointer and key event.

use pltrs_core::Point;
use pltrs_platform::{Key, KeyEvent, Modifiers, MouseButton};

use super::Ui;
use crate::event::UiEvent;
use crate::widget::{ClipboardRequest, EventCx, Focus, Status, WidgetId, WidgetPod};

/// Who receives pointer and key events.
#[derive(Debug, Default)]
pub(super) struct Routing {
    /// Widgets under the pointer, outermost first.
    hot: Vec<WidgetId>,
    /// The widget capturing the pointer while buttons are held.
    active: Option<WidgetId>,
    /// Number of mouse buttons held.
    buttons_held: u32,
    /// The widget receiving key events.
    pub(super) focus: Option<WidgetId>,
    /// Whether the keyboard moved the focus since the last click, so that
    /// the focused widget shows its focus ring.
    pub(super) focus_visible: bool,
    /// The widget whose popup is open: it receives every event.
    pub(super) popup: Option<WidgetId>,
    /// The keys held down, by scan code and key, with the widgets their
    /// presses went to, which receive their releases.
    held_keys: Vec<(u32, Key, WidgetId)>,
    /// The last pointer position in the window.
    pointer: Option<Point>,
}

impl Routing {
    /// How widget `id` is involved in the current interaction.
    pub(super) fn status(&self, id: WidgetId) -> Status {
        Status {
            hot: self.hot.contains(&id) && self.active.is_none_or(|active| active == id),
            active: self.active == Some(id),
            focus: match (self.focus == Some(id), self.focus_visible) {
                (false, _) => Focus::None,
                (true, false) => Focus::Focused,
                (true, true) => Focus::Visible,
            },
        }
    }

    /// The target of pointer events: the widget with a popup open, else
    /// the capturing widget, else the innermost widget under the pointer.
    pub(super) fn pointer_target(&self) -> Option<WidgetId> {
        self.popup
            .or(self.active)
            .or_else(|| self.hot.last().copied())
    }

    /// The target of key events: the widget with a popup open, else the
    /// focused widget, else the innermost widget under the pointer.
    pub(super) fn key_target(&self) -> Option<WidgetId> {
        self.popup
            .or(self.focus)
            .or_else(|| self.hot.last().copied())
    }

    /// The widgets under the pointer, outermost first.
    pub(super) fn hot(&self) -> &[WidgetId] {
        &self.hot
    }

    /// Remembers that the press of `key` went to `owner`.
    pub(super) fn press(&mut self, key: &KeyEvent, owner: Option<WidgetId>) {
        self.release(key);
        if let Some(owner) = owner {
            self.held_keys.push((key.scancode, key.key, owner));
        }
    }

    /// The widget the press of the released `key` went to, if known.
    pub(super) fn release(&mut self, key: &KeyEvent) -> Option<WidgetId> {
        let same = |&(scancode, held, _): &(u32, Key, WidgetId)| {
            if scancode != 0 && key.scancode != 0 {
                scancode == key.scancode
            } else {
                held == key.key
            }
        };
        let index = self.held_keys.iter().position(same)?;
        Some(self.held_keys.remove(index).2)
    }
}

impl<M: 'static> Ui<M> {
    pub(super) fn pointer_moved(&mut self, position: Point, modifiers: Modifiers) {
        self.update_hot(Some(position));
        let event = UiEvent::PointerMove {
            position,
            modifiers,
        };
        self.deliver(self.routing.pointer_target(), &event);
    }

    pub(super) fn pointer_down(
        &mut self,
        button: MouseButton,
        position: Point,
        modifiers: Modifiers,
        click_count: u32,
    ) {
        self.update_hot(Some(position));
        self.dismiss_tooltip();
        let target = self.routing.pointer_target();
        if self.routing.active.is_none() {
            self.routing.active = target;
        }
        self.routing.buttons_held += 1;
        let focusable = self.routing.hot.iter().rev().copied().find(|&id| {
            self.root
                .find(id)
                .is_some_and(|pod| pod.widget().focus_on_click())
        });
        if focusable.is_some() && self.routing.popup.is_none() {
            self.routing.focus = focusable;
        }
        self.routing.focus_visible = false;
        self.needs_paint = true;
        let event = UiEvent::PointerDown {
            button,
            position,
            modifiers,
            click_count,
        };
        self.deliver(target, &event);
    }

    pub(super) fn pointer_up(
        &mut self,
        button: MouseButton,
        position: Point,
        modifiers: Modifiers,
    ) {
        let event = UiEvent::PointerUp {
            button,
            position,
            modifiers,
        };
        self.deliver(self.routing.pointer_target(), &event);
        self.routing.buttons_held = self.routing.buttons_held.saturating_sub(1);
        if self.routing.buttons_held == 0 {
            self.routing.active = None;
            self.needs_paint = true;
        }
        self.update_hot(Some(position));
    }

    /// Delivers a wheel event to the widget with a popup open or the
    /// capturing widget, else to the innermost widget under the pointer
    /// and then to its ancestors in turn, until one acts on it: a list
    /// inside a scrolled view scrolls itself, the text beside it scrolls
    /// the view.
    pub(super) fn wheel(&mut self, event: &UiEvent) {
        if let Some(target) = self.routing.popup.or(self.routing.active) {
            self.deliver(Some(target), event);
            return;
        }
        let under = self.routing.hot.clone();
        for id in under.into_iter().rev() {
            if self.deliver(Some(id), event) {
                return;
            }
        }
    }

    /// Recomputes the widgets under the pointer (`None`: outside the
    /// window) and tells those that the pointer left or entered. While a
    /// popup is open, only its widget (and its ancestors) count as under
    /// the pointer.
    pub(super) fn update_hot(&mut self, position: Option<Point>) {
        self.retarget(position, position);
    }

    /// The pointer left the window for `position` (window pixels, if the
    /// platform tells): no widget is under it any more.
    pub(super) fn pointer_left(&mut self, position: Option<Point>) {
        self.retarget(None, position);
    }

    /// [`Self::update_hot`] for the pointer at `position`, which the
    /// widgets' leave events report, hitting the widgets under `hit`.
    fn retarget(&mut self, hit: Option<Point>, position: Option<Point>) {
        self.routing.pointer = hit;
        let path = match self.routing.popup {
            Some(owner) => path_to(&self.root, owner),
            None => hit.map_or_else(Vec::new, |p| hit_path(&self.root, p)),
        };
        let left: Vec<WidgetId> = self
            .routing
            .hot
            .iter()
            .filter(|id| !path.contains(id))
            .copied()
            .collect();
        let entered: Vec<WidgetId> = path
            .iter()
            .filter(|id| !self.routing.hot.contains(id))
            .copied()
            .collect();
        if left.is_empty() && entered.is_empty() {
            return;
        }
        self.routing.hot = path;
        self.needs_paint = true;
        self.update_tooltip();
        for id in left.into_iter().rev() {
            self.deliver(Some(id), &UiEvent::PointerLeave(position));
        }
        if let Some(position) = hit {
            for id in entered {
                self.deliver(Some(id), &UiEvent::PointerEnter(position));
            }
        }
    }

    /// Hands `event` to the widget `target`, if it still exists; returns
    /// whether the widget acted on it (see
    /// [`EventCx::set_handled`](crate::EventCx::set_handled)).
    pub(super) fn deliver(&mut self, target: Option<WidgetId>, event: &UiEvent) -> bool {
        let Some(pod) = target.and_then(|id| self.root.find_mut(id)) else {
            return false;
        };
        let id = pod.id();
        let rect = pod.rect();
        let mut cx = EventCx::new(&mut self.messages);
        pod.widget_mut().event(&mut cx, event, rect);
        let EventCx {
            paint_requested,
            layout_requested,
            popup,
            handled,
            clipboard,
            ..
        } = cx;
        if layout_requested {
            self.needs_layout = true;
        }
        if paint_requested || layout_requested {
            self.needs_paint = true;
        }
        if let Some(request) = clipboard {
            if request == ClipboardRequest::Paste {
                self.paste_target = Some(id);
            }
            self.clipboard_requests.push(request);
        }
        let popup = match popup {
            Some(true) => Some(id),
            Some(false) if self.routing.popup == Some(id) => None,
            _ => return handled,
        };
        if popup != self.routing.popup {
            self.routing.popup = popup;
            self.update_hot(self.routing.pointer);
        }
        handled
    }
}

/// The widgets from the root down to `target` (empty when `target` is not
/// shown).
fn path_to<M: 'static>(pod: &WidgetPod<M>, target: WidgetId) -> Vec<WidgetId> {
    if pod.id() == target {
        return vec![pod.id()];
    }
    pod.widget()
        .children()
        .iter()
        .find_map(|child| {
            let mut path = path_to(child, target);
            (!path.is_empty()).then(|| {
                path.insert(0, pod.id());
                path
            })
        })
        .unwrap_or_default()
}

/// The widgets containing `point`, outermost first, following the topmost
/// (last) child at each level.
fn hit_path<M: 'static>(root: &WidgetPod<M>, point: Point) -> Vec<WidgetId> {
    let mut path = Vec::new();
    let mut pod = root;
    if !pod.rect().contains(point) {
        return path;
    }
    loop {
        path.push(pod.id());
        let children = pod.widget().children();
        match children
            .iter()
            .rev()
            .find(|child| child.rect().contains(point))
        {
            Some(child) => pod = child,
            None => return path,
        }
    }
}
