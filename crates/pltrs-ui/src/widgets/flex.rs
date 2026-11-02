//! Rows and columns of widgets.

use pltrs_core::Rgba;

use crate::error::UiError;
use crate::geometry::{Constraints, Rect, Size};
use crate::theme::Spacing;
use crate::widget::{LayoutCx, PaintCx, Widget, WidgetId, WidgetPod};

/// The direction children are laid out in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Left to right.
    Horizontal,
    /// Top to bottom.
    Vertical,
}

impl Axis {
    fn main(self, size: Size) -> f64 {
        match self {
            Self::Horizontal => size.width,
            Self::Vertical => size.height,
        }
    }

    fn cross(self, size: Size) -> f64 {
        match self {
            Self::Horizontal => size.height,
            Self::Vertical => size.width,
        }
    }

    fn size(self, main: f64, cross: f64) -> Size {
        match self {
            Self::Horizontal => Size::new(main, cross),
            Self::Vertical => Size::new(cross, main),
        }
    }

    fn rect(self, main_start: f64, cross_start: f64, main: f64, cross: f64) -> Rect {
        match self {
            Self::Horizontal => Rect::new(main_start, cross_start, main, cross),
            Self::Vertical => Rect::new(cross_start, main_start, cross, main),
        }
    }
}

/// Empty space sharing the free space of a [`Flex`].
#[derive(Debug)]
struct Gap;

impl<M> Widget<M> for Gap {
    fn measure(
        &mut self,
        _cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        Ok(constraints.constrain(Size::ZERO))
    }

    fn paint(&self, _cx: &mut PaintCx<'_>, _rect: Rect) -> Result<(), UiError> {
        Ok(())
    }
}

/// Lays children out in a row or a column.
///
/// Children with a flex factor of 0 get their natural size; the space
/// left is shared among the others in proportion to their factors.
/// Children stretch across the cross axis and are placed on whole
/// pixels. Padding and spacing are steps of the theme's spacing scale.
pub struct Flex<M> {
    axis: Axis,
    children: Vec<WidgetPod<M>>,
    /// Flex factor of each child.
    factors: Vec<f64>,
    /// Main-axis extents from the last measure.
    extents: Vec<f64>,
    /// Space left and right of the children.
    padding_x: Spacing,
    /// Space above and below the children.
    padding_y: Spacing,
    spacing: Spacing,
    background: Option<Rgba>,
}

impl<M: 'static> Flex<M> {
    /// An empty row.
    pub fn row() -> Self {
        Self::new(Axis::Horizontal)
    }

    /// An empty column.
    pub fn column() -> Self {
        Self::new(Axis::Vertical)
    }

    fn new(axis: Axis) -> Self {
        Self {
            axis,
            children: Vec::new(),
            factors: Vec::new(),
            extents: Vec::new(),
            padding_x: Spacing::None,
            padding_y: Spacing::None,
            spacing: Spacing::None,
            background: None,
        }
    }

    /// Adds a child with flex factor `flex` (0: natural size; negative and
    /// NaN factors count as 0) and returns its id.
    pub fn push(&mut self, child: impl Widget<M>, flex: f64) -> WidgetId {
        let pod = WidgetPod::new(child);
        let id = pod.id();
        self.children.push(pod);
        self.factors.push(if flex > 0.0 { flex } else { 0.0 });
        id
    }

    /// Inserts a child with flex factor `flex` before the child at
    /// `index` (at the end when `index` is past the last child) and
    /// returns its id; the owner of the tree lays it out again (see
    /// [`Ui::widget_mut`](crate::Ui::widget_mut)).
    pub fn insert(&mut self, index: usize, child: impl Widget<M>, flex: f64) -> WidgetId {
        let index = index.min(self.children.len());
        let pod = WidgetPod::new(child);
        let id = pod.id();
        self.children.insert(index, pod);
        self.factors
            .insert(index, if flex > 0.0 { flex } else { 0.0 });
        id
    }

    /// Removes the child `id`; returns whether it was a child.
    pub fn remove(&mut self, id: WidgetId) -> bool {
        let Some(index) = self.children.iter().position(|child| child.id() == id) else {
            return false;
        };
        self.children.remove(index);
        self.factors.remove(index);
        true
    }

    /// The number of children.
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Whether there are no children.
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Adds empty space taking a share `flex` of the free space, e.g. to
    /// push the children after it to the end.
    pub fn push_spacer(&mut self, flex: f64) {
        self.push(Gap, flex);
    }

    /// Adds a child of natural size.
    #[must_use]
    pub fn with_child(mut self, child: impl Widget<M>) -> Self {
        self.push(child, 0.0);
        self
    }

    /// Adds a child sharing the free space with flex factor `flex`.
    #[must_use]
    pub fn with_flex_child(mut self, child: impl Widget<M>, flex: f64) -> Self {
        self.push(child, flex);
        self
    }

    /// Adds empty space taking a share `flex` of the free space.
    #[must_use]
    pub fn with_spacer(mut self, flex: f64) -> Self {
        self.push_spacer(flex);
        self
    }

    /// Sets the space around the children.
    #[must_use]
    pub fn with_padding(self, padding: Spacing) -> Self {
        self.with_padding_xy(padding, padding)
    }

    /// Sets the space left and right (`horizontal`) and above and below
    /// (`vertical`) the children.
    #[must_use]
    pub fn with_padding_xy(mut self, horizontal: Spacing, vertical: Spacing) -> Self {
        self.padding_x = horizontal;
        self.padding_y = vertical;
        self
    }

    /// Sets the space between children.
    #[must_use]
    pub fn with_spacing(mut self, spacing: Spacing) -> Self {
        self.spacing = spacing;
        self
    }

    /// Fills the container's rectangle with `color` behind the children.
    #[must_use]
    pub fn with_background(mut self, color: Rgba) -> Self {
        self.background = Some(color);
        self
    }

    /// The padding along the main and the cross axis, in pixels.
    fn paddings(&self, cx: &LayoutCx<'_, M>) -> (f64, f64) {
        let theme = cx.theme();
        let x = theme.space(self.padding_x) * cx.scale_factor();
        let y = theme.space(self.padding_y) * cx.scale_factor();
        match self.axis {
            Axis::Horizontal => (x, y),
            Axis::Vertical => (y, x),
        }
    }
}

impl<M: 'static> Widget<M> for Flex<M> {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let axis = self.axis;
        let (pad_main, pad_cross) = self.paddings(cx);
        let gaps = self.children.len().saturating_sub(1) as f64;
        let spacing = cx.theme().space(self.spacing) * cx.scale_factor() * gaps;
        let max_main = (axis.main(constraints.max) - 2.0 * pad_main - spacing).max(0.0);
        let max_cross = (axis.cross(constraints.max) - 2.0 * pad_cross).max(0.0);
        self.extents = vec![0.0; self.children.len()];
        let mut used = 0.0;
        let mut cross: f64 = 0.0;
        for (index, child) in self.children.iter_mut().enumerate() {
            if self.factors[index] == 0.0 {
                let available = axis.size((max_main - used).max(0.0), max_cross);
                let size = child.measure(cx, Constraints::loose(available))?;
                self.extents[index] = axis.main(size);
                used += axis.main(size);
                cross = cross.max(axis.cross(size));
            }
        }
        let total_flex: f64 = self.factors.iter().sum();
        let free = (max_main - used).max(0.0);
        for (index, child) in self.children.iter_mut().enumerate() {
            let flex = self.factors[index];
            if flex > 0.0 {
                let constraints = if free.is_finite() {
                    let main = free * flex / total_flex;
                    Constraints {
                        min: axis.size(main, 0.0),
                        max: axis.size(main, max_cross),
                    }
                } else {
                    Constraints::loose(axis.size(f64::INFINITY, max_cross))
                };
                let size = child.measure(cx, constraints)?;
                self.extents[index] = axis.main(size);
                cross = cross.max(axis.cross(size));
            }
        }
        let content: f64 = self.extents.iter().sum::<f64>() + spacing;
        let main = if total_flex > 0.0 && max_main.is_finite() {
            axis.main(constraints.max)
        } else {
            content + 2.0 * pad_main
        };
        Ok(constraints.constrain(axis.size(main, cross + 2.0 * pad_cross)))
    }

    fn arrange(&mut self, cx: &mut LayoutCx<'_, M>, rect: Rect) -> Result<(), UiError> {
        let axis = self.axis;
        let (pad_main, pad_cross) = self.paddings(cx);
        let spacing = cx.theme().space(self.spacing) * cx.scale_factor();
        let (main_start, cross_start) = match axis {
            Axis::Horizontal => (rect.x, rect.y),
            Axis::Vertical => (rect.y, rect.x),
        };
        let cross = (axis.cross(rect.size()) - 2.0 * pad_cross).max(0.0);
        let mut position = main_start + pad_main;
        for (child, &extent) in self.children.iter_mut().zip(&self.extents) {
            let placed = axis.rect(position, cross_start + pad_cross, extent, cross);
            child.arrange(cx, placed.snapped())?;
            position += extent + spacing;
        }
        Ok(())
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        match self.background {
            Some(color) => cx.canvas().fill_rect(rect, color),
            None => Ok(()),
        }
    }

    fn children(&self) -> &[WidgetPod<M>] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [WidgetPod<M>] {
        &mut self.children
    }
}

impl<M> std::fmt::Debug for Flex<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Flex")
            .field("axis", &self.axis)
            .field("children", &self.children)
            .field("factors", &self.factors)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use crate::widgets::testing::Fixed;

    fn layout(flex: &mut Flex<()>, size: Size, scale_factor: f64) -> Size {
        let theme = Theme::default();
        let mut messages = Vec::new();
        let mut cx = LayoutCx::for_tests(&theme, &mut messages, scale_factor);
        let measured = flex.measure(&mut cx, Constraints::tight(size)).unwrap();
        flex.arrange(&mut cx, Rect::new(0.0, 0.0, size.width, size.height))
            .unwrap();
        measured
    }

    fn rects(flex: &Flex<()>) -> Vec<Rect> {
        flex.children.iter().map(WidgetPod::rect).collect()
    }

    #[test]
    fn row_shares_free_space_by_flex_factor() {
        let mut row = Flex::row()
            .with_padding(Spacing::Xs)
            .with_spacing(Spacing::Sm)
            .with_child(Fixed(Size::new(50.0, 20.0)))
            .with_flex_child(Fixed(Size::ZERO), 1.0)
            .with_flex_child(Fixed(Size::ZERO), 3.0);
        assert_eq!(
            layout(&mut row, Size::new(300.0, 40.0), 1.0),
            Size::new(300.0, 40.0)
        );
        // 300 - 2·4 padding - 2·8 spacing - 50 fixed = 226 free: 56.5 +
        // 169.5, on whole pixels.
        assert_eq!(
            rects(&row),
            [
                Rect::new(4.0, 4.0, 50.0, 32.0),
                Rect::new(62.0, 4.0, 57.0, 32.0),
                Rect::new(127.0, 4.0, 169.0, 32.0),
            ]
        );
    }

    #[test]
    fn column_scales_padding_and_spacing() {
        let mut column = Flex::column()
            .with_padding_xy(Spacing::Xs, Spacing::Xxs)
            .with_spacing(Spacing::Xxs)
            .with_child(Fixed(Size::new(10.0, 10.0)))
            .with_child(Fixed(Size::new(10.0, 10.0)));
        layout(&mut column, Size::new(100.0, 100.0), 2.0);
        assert_eq!(
            rects(&column),
            [
                Rect::new(8.0, 4.0, 84.0, 10.0),
                Rect::new(8.0, 18.0, 84.0, 10.0)
            ]
        );
    }

    #[test]
    fn natural_size_without_flex_children() {
        let mut row: Flex<()> = Flex::row()
            .with_padding(Spacing::Xxs)
            .with_child(Fixed(Size::new(10.0, 5.0)))
            .with_child(Fixed(Size::new(20.0, 8.0)));
        let theme = Theme::default();
        let mut messages = Vec::new();
        let mut cx = LayoutCx::for_tests(&theme, &mut messages, 1.0);
        let size = row
            .measure(&mut cx, Constraints::loose(Size::INFINITE))
            .unwrap();
        assert_eq!(size, Size::new(34.0, 12.0));
    }

    #[test]
    fn spacers_push_children_to_the_end() {
        let mut row = Flex::row()
            .with_spacer(1.0)
            .with_child(Fixed(Size::new(30.0, 10.0)));
        layout(&mut row, Size::new(100.0, 10.0), 1.0);
        assert_eq!(rects(&row)[1], Rect::new(70.0, 0.0, 30.0, 10.0));
    }

    #[test]
    fn fixed_children_never_get_negative_space() {
        let mut row = Flex::row()
            .with_child(Fixed(Size::new(80.0, 10.0)))
            .with_child(Fixed(Size::new(80.0, 10.0)))
            .with_flex_child(Fixed(Size::ZERO), -2.0);
        layout(&mut row, Size::new(100.0, 10.0), 1.0);
        let widths: Vec<f64> = rects(&row).iter().map(|r| r.width).collect();
        assert_eq!(widths, [80.0, 20.0, 0.0]);
    }

    #[test]
    fn children_are_inserted_and_removed_in_place() {
        let mut row: Flex<()> = Flex::row();
        assert!(row.is_empty());
        let last = row.push(Fixed(Size::new(30.0, 10.0)), 0.0);
        let first = row.insert(0, Fixed(Size::new(10.0, 10.0)), 0.0);
        let beyond = row.insert(99, Fixed(Size::new(20.0, 10.0)), 0.0);
        layout(&mut row, Size::new(100.0, 10.0), 1.0);
        let order: Vec<WidgetId> = row.children.iter().map(WidgetPod::id).collect();
        assert_eq!(order, [first, last, beyond]);
        assert_eq!(rects(&row)[1], Rect::new(10.0, 0.0, 30.0, 10.0));
        assert!(row.remove(last));
        assert!(!row.remove(last), "removed already");
        layout(&mut row, Size::new(100.0, 10.0), 1.0);
        assert_eq!(row.len(), 2);
        assert_eq!(rects(&row)[1], Rect::new(10.0, 0.0, 20.0, 10.0));
    }
}
