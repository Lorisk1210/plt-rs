//! Pages of which one is shown.

use crate::error::UiError;
use crate::geometry::{Constraints, Rect, Size};
use crate::widget::{LayoutCx, PaintCx, Widget, WidgetId, WidgetPod};

/// Children stacked on top of each other with only one shown — the pages
/// of a tabbed panel. The hidden pages are neither laid out, painted nor
/// given events, but their widgets can still be found by id.
pub struct Stack<M> {
    pages: Vec<WidgetPod<M>>,
    shown: usize,
}

impl<M> std::fmt::Debug for Stack<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stack")
            .field("pages", &self.pages.len())
            .field("shown", &self.shown)
            .finish()
    }
}

impl<M: 'static> Default for Stack<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: 'static> Stack<M> {
    /// A stack without pages.
    pub fn new() -> Self {
        Self {
            pages: Vec::new(),
            shown: 0,
        }
    }

    /// Adds a page and returns its id; the first page is shown.
    pub fn push(&mut self, page: impl Widget<M>) -> WidgetId {
        let pod = WidgetPod::new(page);
        let id = pod.id();
        self.pages.push(pod);
        id
    }

    /// The page shown.
    pub fn shown(&self) -> usize {
        self.shown
    }

    /// Shows page `index` (ignored when out of range).
    pub fn show(&mut self, index: usize) {
        if index < self.pages.len() {
            self.shown = index;
        }
    }
}

impl<M: 'static> Widget<M> for Stack<M> {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        match self.pages.get_mut(self.shown) {
            Some(page) => page.measure(cx, constraints),
            None => Ok(constraints.constrain(Size::new(0.0, 0.0))),
        }
    }

    fn arrange(&mut self, cx: &mut LayoutCx<'_, M>, rect: Rect) -> Result<(), UiError> {
        match self.pages.get_mut(self.shown) {
            Some(page) => page.arrange(cx, rect),
            None => Ok(()),
        }
    }

    fn paint(&self, _cx: &mut PaintCx<'_>, _rect: Rect) -> Result<(), UiError> {
        Ok(())
    }

    fn children(&self) -> &[WidgetPod<M>] {
        let end = (self.shown + 1).min(self.pages.len());
        &self.pages[self.shown.min(end)..end]
    }

    fn children_mut(&mut self) -> &mut [WidgetPod<M>] {
        let end = (self.shown + 1).min(self.pages.len());
        &mut self.pages[self.shown.min(end)..end]
    }

    fn all_children(&self) -> &[WidgetPod<M>] {
        &self.pages
    }

    fn all_children_mut(&mut self) -> &mut [WidgetPod<M>] {
        &mut self.pages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::testing::Fixed;

    #[test]
    fn only_the_shown_page_is_a_child() {
        let mut stack: Stack<()> = Stack::new();
        assert!(stack.children().is_empty());
        let first = stack.push(Fixed(Size::new(10.0, 10.0)));
        let second = stack.push(Fixed(Size::new(20.0, 20.0)));
        assert_eq!(stack.children()[0].id(), first);
        stack.show(1);
        assert_eq!(stack.children()[0].id(), second);
        stack.show(5);
        assert_eq!(stack.shown(), 1);
    }
}
