//! Input routing: which widget receives each pointer and key event.

use plt_core::Point;
use plt_platform::{Modifiers, MouseButton};

use super::Ui;
use crate::event::UiEvent;
use crate::widget::{EventCx, Status, WidgetId, WidgetPod};

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
    /// The widget whose popup is open: it receives every event.
    pub(super) popup: Option<WidgetId>,
    /// The last pointer position in the window.
    pointer: Option<Point>,
}

impl Routing {
    /// How widget `id` is involved in the current interaction.
    pub(super) fn status(&self, id: WidgetId) -> Status {
        Status {
            hot: self.hot.contains(&id) && self.active.is_none_or(|active| active == id),
            active: self.active == Some(id),
            focused: self.focus == Some(id),
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
                .is_some_and(|pod| pod.widget().accepts_focus())
        });
        if focusable.is_some() && self.routing.popup.is_none() {
            self.routing.focus = focusable;
        }
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

    /// Recomputes the widgets under the pointer (`None`: outside the
    /// window) and tells those that the pointer left or entered. While a
    /// popup is open, only its widget (and its ancestors) count as under
    /// the pointer.
    pub(super) fn update_hot(&mut self, position: Option<Point>) {
        self.routing.pointer = position;
        let path = match self.routing.popup {
            Some(owner) => path_to(&self.root, owner),
            None => position.map_or_else(Vec::new, |p| hit_path(&self.root, p)),
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
            self.deliver(Some(id), &UiEvent::PointerLeave);
        }
        if let Some(position) = position {
            for id in entered {
                self.deliver(Some(id), &UiEvent::PointerEnter(position));
            }
        }
    }

    /// Hands `event` to the widget `target`, if it still exists.
    pub(super) fn deliver(&mut self, target: Option<WidgetId>, event: &UiEvent) {
        let Some(pod) = target.and_then(|id| self.root.find_mut(id)) else {
            return;
        };
        let id = pod.id();
        let rect = pod.rect();
        let mut cx = EventCx::new(&mut self.messages);
        pod.widget_mut().event(&mut cx, event, rect);
        if cx.layout_requested {
            self.needs_layout = true;
        }
        if cx.paint_requested || cx.layout_requested {
            self.needs_paint = true;
        }
        let popup = match cx.popup {
            Some(true) => Some(id),
            Some(false) if self.routing.popup == Some(id) => None,
            _ => return,
        };
        if popup != self.routing.popup {
            self.routing.popup = popup;
            self.update_hot(self.routing.pointer);
        }
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
