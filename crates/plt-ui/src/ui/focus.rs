//! Keyboard focus: the order of the widgets that accept it, moving it
//! with the keyboard, and keeping it on a widget shown.

use super::Ui;
use super::tooltip::Tooltip;
use crate::widget::{WidgetId, WidgetPod};

impl<M: 'static> Ui<M> {
    /// Gives keyboard focus to the first widget shown that accepts it, in
    /// tree order (the first field of a dialog), if any.
    pub fn focus_first(&mut self) {
        let first = focus_order(&self.root).first().copied();
        if first.is_some() && first != self.routing.focus {
            self.routing.focus = first;
            self.needs_paint = true;
        }
    }

    /// Moves the focus to the next (`forward`) or previous widget shown
    /// that accepts it, in tree order, wrapping around, as Tab and
    /// Shift+Tab do: the focus ring shows, and so does the widget's
    /// tooltip once the theme's delay has passed.
    pub(super) fn move_focus(&mut self, forward: bool) {
        let order = focus_order(&self.root);
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
        self.routing.focus_visible = true;
        let has_tooltip = self.root.find_mut(id).is_some_and(|pod| {
            pod.widget_mut().focused_by_keyboard();
            pod.widget().tooltip().is_some()
        });
        self.set_tooltip(if has_tooltip {
            Tooltip::Pending(id)
        } else {
            Tooltip::Hidden
        });
        self.needs_paint = true;
    }

    /// Moves the focus off a widget that is no longer shown (its tab page
    /// was hidden) or no longer accepts it (it was disabled): to the
    /// first widget accepting it from the deepest part of the tree still
    /// shown on (the page shown instead, as in Qt), else to the first
    /// widget accepting it; the focus is dropped when its widget is gone.
    pub(super) fn keep_focus_shown(&mut self) {
        let Some(focus) = self.routing.focus else {
            return;
        };
        let shown = shown_preorder(&self.root);
        if shown.iter().any(|&(id, accepts)| id == focus && accepts) {
            return;
        }
        let anchor = full_path(&self.root, focus)
            .into_iter()
            .rev()
            .find_map(|id| shown.iter().position(|&(shown, _)| shown == id));
        let next = anchor.and_then(|start| {
            let after = shown[start..].iter();
            let wrapped = shown[..start].iter();
            after
                .chain(wrapped)
                .find(|&&(_, accepts)| accepts)
                .map(|&(id, _)| id)
        });
        self.routing.focus = next;
        self.needs_paint = true;
    }
}

/// The widgets shown that accept the focus, in tree order.
fn focus_order<M: 'static>(root: &WidgetPod<M>) -> Vec<WidgetId> {
    shown_preorder(root)
        .into_iter()
        .filter_map(|(id, accepts)| accepts.then_some(id))
        .collect()
}

/// Every widget shown, in tree order, with whether it accepts the focus.
fn shown_preorder<M: 'static>(root: &WidgetPod<M>) -> Vec<(WidgetId, bool)> {
    fn visit<M: 'static>(pod: &WidgetPod<M>, out: &mut Vec<(WidgetId, bool)>) {
        out.push((pod.id(), pod.widget().accepts_focus()));
        for child in pod.widget().children() {
            visit(child, out);
        }
    }
    let mut out = Vec::new();
    visit(root, &mut out);
    out
}

/// The widgets from the root down to `target`, hidden ones included
/// (empty when there is no such widget).
fn full_path<M: 'static>(pod: &WidgetPod<M>, target: WidgetId) -> Vec<WidgetId> {
    if pod.id() == target {
        return vec![target];
    }
    pod.widget()
        .all_children()
        .iter()
        .find_map(|child| {
            let mut path = full_path(child, target);
            (!path.is_empty()).then(|| {
                path.insert(0, pod.id());
                path
            })
        })
        .unwrap_or_default()
}
