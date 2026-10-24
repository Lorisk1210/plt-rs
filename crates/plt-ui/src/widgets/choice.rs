//! Segmented controls: a row of options, one of them chosen.

use std::fmt;
use std::rc::Rc;

use plt_core::Point;
use plt_platform::MouseButton;

use super::frame::paint_focus_ring;
use super::segments::Segments;
use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::theme::Spacing;
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};

/// Makes the message sent for an option.
type OptionMessage<M> = Rc<dyn Fn(usize) -> M>;

/// Space left and right of an option's label.
const PADDING: Spacing = Spacing::Md;
/// How far the chosen segment's raised face sits inside the track.
const THUMB_INSET: Spacing = Spacing::Xxs;
/// Space above and below the dividers between segments.
const DIVIDER_MARGIN: Spacing = Spacing::Sm;

/// A row of labelled segments of which exactly one is chosen; clicking a
/// segment chooses it and sends a message with its index, and so do the
/// arrow keys (Home and End choose the first and last) while it has the
/// focus.
///
/// The segments sit on a recessed track at their natural widths (at
/// least [`Theme::segment_min_width`](crate::Theme)), from the left edge;
/// the chosen one is a raised face.
pub struct Choice<M> {
    options: Vec<String>,
    chosen: usize,
    on_choose: OptionMessage<M>,
    segments: Segments,
}

impl<M> fmt::Debug for Choice<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Choice")
            .field("options", &self.options)
            .field("chosen", &self.chosen)
            .finish_non_exhaustive()
    }
}

impl<M> Choice<M> {
    /// Segments for `options` with the first chosen, sending
    /// `on_choose(index)` when the user picks one.
    pub fn new(options: Vec<String>, on_choose: impl Fn(usize) -> M + 'static) -> Self {
        Self {
            options,
            chosen: 0,
            on_choose: Rc::new(on_choose),
            segments: Segments::default(),
        }
    }

    /// The options.
    pub fn options(&self) -> &[String] {
        &self.options
    }

    /// The chosen option.
    pub fn chosen(&self) -> usize {
        self.chosen
    }

    /// Chooses option `index` (ignored when out of range); no message is
    /// sent.
    pub fn set_chosen(&mut self, index: usize) {
        if index < self.options.len() {
            self.chosen = index;
        }
    }

    /// Chooses option `index` for the user, sending a message if it is
    /// another.
    fn choose(&mut self, cx: &mut EventCx<'_, M>, index: usize) {
        if index != self.chosen {
            self.chosen = index;
            cx.emit((self.on_choose)(index));
            cx.request_paint();
        }
    }
}

impl<M: 'static> Widget<M> for Choice<M> {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        self.segments.measure(
            &self.options,
            &theme.text_style(),
            scale,
            theme.space(PADDING) * scale,
            theme.segment_min_width * scale,
        )?;
        let natural = Size::new(self.segments.total_width(), theme.control_height * scale);
        Ok(constraints.constrain(natural))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let radius = theme.corner_radius * scale;
        let height = (theme.control_height * scale).min(rect.height);
        let width = self.segments.total_width().min(rect.width);
        let track = Rect::new(rect.x, rect.center().y - height / 2.0, width, height).snapped();
        cx.canvas().fill_rounded_rect(track, radius, theme.track)?;
        if cx.status().focus_visible() {
            paint_focus_ring(cx, track, radius)?;
        }
        let inset = theme.space(THUMB_INSET) * scale;
        let margin = theme.space(DIVIDER_MARGIN) * scale;
        let hot = self.segments.hot;
        for (index, x, width) in self.segments.placed(track.x) {
            let segment = Rect::new(x, track.y, width, track.height);
            let face = segment.inset(inset).snapped();
            let chosen = index == self.chosen;
            if chosen {
                let shadow = Rect::new(face.x, face.y + scale, face.width, face.height);
                cx.canvas()
                    .fill_rounded_rect(shadow, radius - inset, theme.shadow)?;
                cx.canvas()
                    .fill_rounded_rect(face, radius - inset, theme.surface)?;
            } else if hot == Some(index) {
                cx.canvas()
                    .fill_rounded_rect(face, radius - inset, theme.hover)?;
            } else if index > 0 && index != self.chosen + 1 && hot != Some(index - 1) {
                let divider = Rect::new(
                    x.round(),
                    track.y + margin,
                    scale,
                    (track.height - 2.0 * margin).max(0.0),
                );
                cx.canvas().fill_rect(divider, theme.border)?;
            }
            let color = if chosen || hot == Some(index) {
                theme.text
            } else {
                theme.text_secondary
            };
            let layout = &self.segments.layouts[index];
            let size = layout.size();
            let origin = Point::new(
                (x + (width - size.width) / 2.0).round(),
                segment.center().y - size.height / 2.0,
            );
            cx.canvas().draw_text(layout, origin, color)?;
        }
        Ok(())
    }

    fn event(&mut self, cx: &mut EventCx<'_, M>, event: &UiEvent, rect: Rect) {
        match event {
            UiEvent::PointerDown {
                button: MouseButton::Left,
                position,
                ..
            } => {
                if let Some(index) = self.segments.at(position.x - rect.x) {
                    self.choose(cx, index);
                }
            }
            UiEvent::Key(key) => {
                if let Some(index) = self.segments.key_target(key, self.chosen, true) {
                    self.choose(cx, index);
                    cx.set_handled();
                }
            }
            _ => {}
        }
        if self.segments.track_hover(event, rect) {
            cx.request_paint();
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use plt_platform::Modifiers;

    use super::*;

    #[test]
    fn clicking_a_segment_chooses_it() {
        let mut choice = Choice::new(vec!["PNG".into(), "SVG".into(), "PDF".into()], |i| i);
        choice.segments.widths = vec![30.0, 30.0, 40.0];
        let press = |x| UiEvent::PointerDown {
            button: MouseButton::Left,
            position: Point::new(x, 5.0),
            modifiers: Modifiers::NONE,
            click_count: 1,
        };
        let rect = Rect::new(0.0, 0.0, 200.0, 28.0);
        let mut messages = Vec::new();
        let mut cx = EventCx::new(&mut messages);
        choice.event(&mut cx, &press(65.0), rect);
        choice.event(&mut cx, &press(150.0), rect);
        assert_eq!(messages, [2], "beyond the segments chooses nothing");
        assert_eq!(choice.chosen(), 2);
        choice.set_chosen(7);
        assert_eq!(choice.chosen(), 2);
    }

    #[test]
    fn arrow_keys_move_the_choice() {
        use plt_platform::{ButtonState, Key, KeyEvent};

        let mut choice = Choice::new(vec!["PNG".into(), "SVG".into(), "PDF".into()], |i| i);
        choice.segments.widths = vec![30.0, 30.0, 40.0];
        let key = |key| {
            UiEvent::Key(KeyEvent {
                key,
                state: ButtonState::Pressed,
                repeat: false,
                text: None,
                modifiers: Modifiers::NONE,
                scancode: 0,
            })
        };
        let rect = Rect::new(0.0, 0.0, 200.0, 28.0);
        let mut messages = Vec::new();
        let mut cx = EventCx::new(&mut messages);
        for pressed in [Key::Right, Key::Down, Key::Right, Key::Home, Key::Left] {
            choice.event(&mut cx, &key(pressed), rect);
        }
        assert!(cx.handled);
        assert_eq!(messages, [1, 2, 0], "only changes are reported");
    }
}
