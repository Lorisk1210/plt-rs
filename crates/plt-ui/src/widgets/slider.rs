//! Sliders: a value picked along a track.

use std::fmt;
use std::rc::Rc;

use plt_core::{Path, Point};
use plt_platform::MouseButton;

use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::theme::Spacing;
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};

/// Thickness of the track.
const TRACK: Spacing = Spacing::Xs;

/// Makes the message sent with the value.
type ValueMessage<M> = Rc<dyn Fn(f64) -> M>;

/// A horizontal track with a round thumb marking a value between a
/// minimum and a maximum; the track is filled with the accent colour up
/// to the thumb. Pressing the track moves the thumb there and dragging
/// moves it along, sending a message with each new value.
pub struct Slider<M> {
    min: f64,
    max: f64,
    value: f64,
    on_change: ValueMessage<M>,
    /// Whether the thumb is being dragged.
    dragging: bool,
    /// The thumb's diameter in pixels, from the last measure.
    thumb: f64,
}

impl<M> fmt::Debug for Slider<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Slider")
            .field("min", &self.min)
            .field("max", &self.max)
            .field("value", &self.value)
            .finish_non_exhaustive()
    }
}

impl<M> Slider<M> {
    /// A slider from `min` to `max` (swapped when given in the wrong
    /// order) showing `value`, sending `on_change(value)` when the user
    /// moves it.
    pub fn new(min: f64, max: f64, value: f64, on_change: impl Fn(f64) -> M + 'static) -> Self {
        let (min, max) = if min <= max { (min, max) } else { (max, min) };
        let mut slider = Self {
            min,
            max,
            value: min,
            on_change: Rc::new(on_change),
            dragging: false,
            thumb: 0.0,
        };
        slider.set_value(value);
        slider
    }

    /// The value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Shows `value`, limited to the slider's range (NaN is ignored); no
    /// message is sent.
    pub fn set_value(&mut self, value: f64) {
        if !value.is_nan() {
            self.value = value.clamp(self.min, self.max);
        }
    }

    /// Where the thumb's centre travels within `rect`: its left and right
    /// ends.
    fn travel(&self, rect: Rect) -> (f64, f64) {
        let radius = self.thumb / 2.0;
        (
            rect.x + radius,
            (rect.max_x() - radius).max(rect.x + radius),
        )
    }

    /// The value whose thumb is centred at `x` in a slider at `rect`.
    fn value_at(&self, rect: Rect, x: f64) -> f64 {
        let (start, end) = self.travel(rect);
        let fraction = if end > start {
            ((x - start) / (end - start)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.min + fraction * (self.max - self.min)
    }

    /// The thumb's centre for the value, in a slider at `rect`.
    fn thumb_x(&self, rect: Rect) -> f64 {
        let (start, end) = self.travel(rect);
        let span = self.max - self.min;
        let fraction = if span > 0.0 {
            (self.value - self.min) / span
        } else {
            0.0
        };
        start + fraction * (end - start)
    }

    fn drag_to(&mut self, cx: &mut EventCx<'_, M>, rect: Rect, x: f64) {
        let value = self.value_at(rect, x);
        if value.total_cmp(&self.value).is_ne() {
            self.value = value;
            cx.emit((self.on_change)(value));
            cx.request_paint();
        }
    }
}

impl<M: 'static> Widget<M> for Slider<M> {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        self.thumb = theme.checkbox_size * scale;
        let natural = Size::new(theme.field_width * scale, theme.control_height * scale);
        Ok(constraints.constrain(natural))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let status = cx.status();
        let thickness = theme.space(TRACK) * scale;
        let (start, end) = self.travel(rect);
        let y = rect.center().y;
        let track = Rect::new(start, y - thickness / 2.0, end - start, thickness);
        cx.canvas()
            .fill_rounded_rect(track, thickness / 2.0, theme.track)?;
        let x = self.thumb_x(rect);
        let filled = Rect::new(start, track.y, x - start, thickness);
        cx.canvas()
            .fill_rounded_rect(filled, thickness / 2.0, theme.accent)?;
        let radius = self.thumb / 2.0;
        let shadow = Path::circle(Point::new(x, y + scale), radius);
        cx.canvas().fill_path(&shadow, theme.shadow)?;
        let thumb = Path::circle(Point::new(x, y), radius);
        cx.canvas().fill_path(&thumb, theme.surface)?;
        let outline = if status.active || status.hot {
            theme.accent
        } else {
            theme.border
        };
        let ring = Path::circle(Point::new(x, y), radius - scale / 2.0);
        cx.canvas().stroke_path(&ring, scale, outline)
    }

    fn event(&mut self, cx: &mut EventCx<'_, M>, event: &UiEvent, rect: Rect) {
        match event {
            UiEvent::PointerDown {
                button: MouseButton::Left,
                position,
                ..
            } => {
                self.dragging = true;
                self.drag_to(cx, rect, position.x);
            }
            UiEvent::PointerMove { position, .. } if self.dragging => {
                self.drag_to(cx, rect, position.x);
            }
            UiEvent::PointerUp {
                button: MouseButton::Left,
                ..
            } => self.dragging = false,
            UiEvent::PointerEnter(_) | UiEvent::PointerLeave => cx.request_paint(),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use plt_platform::Modifiers;

    use super::*;

    #[test]
    fn values_follow_the_pointer_within_the_range() {
        let mut slider = Slider::new(1.0, 0.0, 0.25, |value| value);
        assert_eq!(slider.value(), 0.25);
        slider.set_value(f64::NAN);
        slider.set_value(7.0);
        assert_eq!(slider.value(), 1.0, "limited to the range");
        slider.thumb = 10.0;
        let rect = Rect::new(0.0, 0.0, 110.0, 28.0);
        assert_eq!(slider.travel(rect), (5.0, 105.0));
        assert_eq!(slider.thumb_x(rect), 105.0);
        let mut messages = Vec::new();
        let mut cx = EventCx::new(&mut messages);
        let press = UiEvent::PointerDown {
            button: MouseButton::Left,
            position: Point::new(35.0, 14.0),
            modifiers: Modifiers::NONE,
            click_count: 1,
        };
        slider.event(&mut cx, &press, rect);
        let drag = UiEvent::PointerMove {
            position: Point::new(-50.0, 14.0),
            modifiers: Modifiers::NONE,
        };
        slider.event(&mut cx, &drag, rect);
        assert_eq!(
            *cx.messages,
            [0.3, 0.0],
            "pressed at 30%, dragged off the end"
        );
    }
}
