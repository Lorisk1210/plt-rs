//! Sliders: a value picked along a track.

use std::fmt;
use std::rc::Rc;

use plt_core::{Path, Point};
use plt_platform::{ButtonState, Key, KeyEvent, MouseButton};

use super::frame::paint_focus_ring;
use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::theme::Spacing;
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};

/// Thickness of the track.
const TRACK: Spacing = Spacing::Xs;

/// Makes the message sent with the value.
type ValueMessage<M> = Rc<dyn Fn(f64) -> M>;

/// How many small steps of the arrow keys span a slider's range, roughly.
const STEPS_PER_RANGE: f64 = 100.0;
/// Values closer than this many steps to a multiple of the step count as
/// on it.
const STEP_TOLERANCE: f64 = 1e-9;

/// A step of the keys moving a slider: `units` (1, 2 or 5) times ten to
/// the `exponent`, a round number.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Step {
    units: f64,
    exponent: i32,
}

impl Step {
    /// The round step closest to `length`, if it is positive and finite.
    fn near(length: f64) -> Option<Self> {
        if !(length.is_finite() && length > 0.0) {
            return None;
        }
        // Finite positive lengths have decimal exponents within ±400.
        let exponent = length.log10().floor() as i32;
        let mantissa = length / 10f64.powi(exponent);
        let (units, exponent) = match mantissa {
            m if m < 1.5 => (1.0, exponent),
            m if m < 3.5 => (2.0, exponent),
            m if m < 7.5 => (5.0, exponent),
            _ => (1.0, exponent + 1),
        };
        Some(Self { units, exponent })
    }

    /// Ten times the step.
    fn tenfold(self) -> Self {
        Self {
            exponent: self.exponent + 1,
            ..self
        }
    }

    /// `count` steps, computed so that round steps add up to round
    /// numbers (13 steps of 0.01 are 0.13).
    fn times(self, count: f64) -> f64 {
        let units = count * self.units;
        if self.exponent < 0 {
            units / 10f64.powi(-self.exponent)
        } else {
            units * 10f64.powi(self.exponent)
        }
    }
}

/// A horizontal track with a round thumb marking a value between a
/// minimum and a maximum; the track is filled with the accent colour up
/// to the thumb. Pressing the track moves the thumb there and dragging
/// moves it along, sending a message with each new value.
///
/// While it has the focus, the arrow keys move the value to the next
/// multiple of a round step of about a hundredth of the range (0.01 for
/// a range of 1), Page Up and Page Down by ten steps, and Home and End to
/// the ends.
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
        self.change(cx, value);
    }

    /// Shows `value` for the user, sending it if it changed.
    fn change(&mut self, cx: &mut EventCx<'_, M>, value: f64) {
        if value.total_cmp(&self.value).is_ne() {
            self.value = value;
            cx.emit((self.on_change)(value));
            cx.request_paint();
        }
    }

    /// The value a key press moves to, or `None` for keys that do not
    /// move the slider.
    fn key_target(&self, key: &KeyEvent) -> Option<f64> {
        if key.state != ButtonState::Pressed || key.modifiers.control() || key.modifiers.alt() {
            return None;
        }
        let step = Step::near((self.max - self.min) / STEPS_PER_RANGE);
        let stepped = |step: Option<Step>, forward: bool| {
            Some(step.map_or(self.value, |step| self.stepped(step, forward)))
        };
        match key.key {
            Key::Right | Key::Up => stepped(step, true),
            Key::Left | Key::Down => stepped(step, false),
            Key::PageUp => stepped(step.map(Step::tenfold), true),
            Key::PageDown => stepped(step.map(Step::tenfold), false),
            Key::Home => Some(self.min),
            Key::End => Some(self.max),
            _ => None,
        }
    }

    /// The next multiple of `step` from the minimum after (`forward`) or
    /// before the value, within the range.
    fn stepped(&self, step: Step, forward: bool) -> f64 {
        let steps = (self.value - self.min) / step.times(1.0);
        let count = if forward {
            (steps + STEP_TOLERANCE).floor() + 1.0
        } else {
            (steps - STEP_TOLERANCE).ceil() - 1.0
        };
        (self.min + step.times(count)).clamp(self.min, self.max)
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
        cx.canvas().fill_path(&thumb, theme.raised)?;
        let outline = if status.active || status.hot {
            theme.accent
        } else {
            theme.border
        };
        let ring = Path::circle(Point::new(x, y), radius - scale / 2.0);
        cx.canvas().stroke_path(&ring, scale, outline)?;
        if status.focus_visible() {
            let thumb = Rect::new(x - radius, y - radius, self.thumb, self.thumb);
            paint_focus_ring(cx, thumb, radius)?;
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
            UiEvent::Key(key) => {
                if let Some(value) = self.key_target(key) {
                    self.change(cx, value);
                    cx.set_handled();
                }
            }
            _ => {}
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

    #[test]
    fn steps_are_round_numbers_near_a_hundredth_of_the_range() {
        let step = |length| Step::near(length).map(|step| step.times(1.0));
        assert_eq!(step(0.01), Some(0.01));
        assert_eq!(step(2.55), Some(2.0));
        assert_eq!(step(0.4), Some(0.5));
        assert_eq!(step(80.0), Some(100.0));
        assert_eq!(step(0.0), None);
        assert_eq!(step(f64::NAN), None);
        assert_eq!(Step::near(0.01).map(|s| s.times(13.0)), Some(0.13));
        assert_eq!(Step::near(0.01).map(|s| s.tenfold().times(1.0)), Some(0.1));
    }

    #[test]
    fn keys_move_the_value_by_round_steps() {
        let key = |key| KeyEvent {
            key,
            state: ButtonState::Pressed,
            repeat: false,
            text: None,
            modifiers: Modifiers::NONE,
            scancode: 0,
        };
        let mut slider = Slider::new(0.0, 1.0, 0.125, |value| value);
        assert_eq!(slider.key_target(&key(Key::Right)), Some(0.13));
        assert_eq!(slider.key_target(&key(Key::Left)), Some(0.12));
        assert_eq!(slider.key_target(&key(Key::PageUp)), Some(0.2));
        assert_eq!(slider.key_target(&key(Key::PageDown)), Some(0.1));
        assert_eq!(slider.key_target(&key(Key::End)), Some(1.0));
        assert_eq!(slider.key_target(&key(Key::Enter)), None);
        slider.set_value(0.13);
        assert_eq!(
            slider.key_target(&key(Key::Up)),
            Some(0.14),
            "from a multiple"
        );
        slider.set_value(0.995);
        assert_eq!(
            slider.key_target(&key(Key::PageUp)),
            Some(1.0),
            "within range"
        );
        let mut messages = Vec::new();
        let mut cx = EventCx::new(&mut messages);
        let rect = Rect::new(0.0, 0.0, 110.0, 28.0);
        slider.event(&mut cx, &UiEvent::Key(key(Key::End)), rect);
        slider.event(&mut cx, &UiEvent::Key(key(Key::Right)), rect);
        slider.event(&mut cx, &UiEvent::Key(key(Key::Home)), rect);
        assert!(cx.handled, "keys at an end are used too");
        assert_eq!(messages, [1.0, 0.0]);
        let flat = Slider::new(2.0, 2.0, 2.0, |value| value);
        assert_eq!(flat.key_target(&key(Key::Right)), Some(2.0));
    }
}
