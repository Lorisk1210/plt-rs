//! Labelled segments side by side, the shared part of segmented controls
//! and tab bars: their layout, hit testing, hover tracking and arrow
//! keys.

use plt_platform::{ButtonState, Key, KeyEvent};

use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::Rect;
use crate::text::{TextLayout, TextStyle};

/// The laid-out labels of a row of segments and the segment under the
/// pointer.
#[derive(Debug, Default)]
pub(crate) struct Segments {
    /// The labels laid out by the last measure.
    pub(crate) layouts: Vec<TextLayout>,
    /// The width of each segment, in pixels.
    pub(crate) widths: Vec<f64>,
    /// The segment under the pointer.
    pub(crate) hot: Option<usize>,
}

impl Segments {
    /// Lays out `labels` in `style`, each segment `padding` pixels wider
    /// than its label on both sides and at least `min_width` wide.
    pub(crate) fn measure(
        &mut self,
        labels: &[String],
        style: &TextStyle,
        scale_factor: f64,
        padding: f64,
        min_width: f64,
    ) -> Result<(), UiError> {
        self.layouts = labels
            .iter()
            .map(|label| TextLayout::new(label, style, scale_factor))
            .collect::<Result<_, _>>()?;
        self.widths = self
            .layouts
            .iter()
            .map(|layout| (layout.size().width + 2.0 * padding).max(min_width))
            .collect();
        Ok(())
    }

    /// The width of all the segments.
    pub(crate) fn total_width(&self) -> f64 {
        self.widths.iter().sum()
    }

    /// The segment containing `x` pixels from the left edge of the row.
    pub(crate) fn at(&self, x: f64) -> Option<usize> {
        if x < 0.0 {
            return None;
        }
        let mut right = 0.0;
        self.widths.iter().position(|width| {
            right += width;
            x < right
        })
    }

    /// The segments with their left edges, from `x`.
    pub(crate) fn placed(&self, x: f64) -> impl Iterator<Item = (usize, f64, f64)> + '_ {
        self.widths
            .iter()
            .enumerate()
            .scan(x, |left, (index, &width)| {
                let start = *left;
                *left += width;
                Some((index, start, width))
            })
    }

    /// The segment a key press moves to from `current`: Left and Right
    /// (also Up and Down when `up_down`) step without wrapping, Home and
    /// End jump to the ends. `None` for other keys and releases.
    pub(crate) fn key_target(
        &self,
        key: &KeyEvent,
        current: usize,
        up_down: bool,
    ) -> Option<usize> {
        let last = self.widths.len().checked_sub(1)?;
        if key.state != ButtonState::Pressed || key.modifiers.control() || key.modifiers.alt() {
            return None;
        }
        match key.key {
            Key::Left => Some(current.saturating_sub(1)),
            Key::Up if up_down => Some(current.saturating_sub(1)),
            Key::Right => Some((current + 1).min(last)),
            Key::Down if up_down => Some((current + 1).min(last)),
            Key::Home => Some(0),
            Key::End => Some(last),
            _ => None,
        }
    }

    /// Follows the pointer over a row of segments at `rect`; returns
    /// whether the segment under it changed.
    pub(crate) fn track_hover(&mut self, event: &UiEvent, rect: Rect) -> bool {
        let hot = match event {
            UiEvent::PointerEnter(position) | UiEvent::PointerMove { position, .. } => {
                self.at(position.x - rect.x)
            }
            UiEvent::PointerLeave => None,
            _ => return false,
        };
        let changed = hot != self.hot;
        self.hot = hot;
        changed
    }
}

#[cfg(test)]
mod tests {
    use plt_core::Point;

    use super::*;

    #[test]
    fn segments_under_the_pointer() {
        let mut segments = Segments {
            widths: vec![30.0, 30.0, 40.0],
            ..Segments::default()
        };
        assert_eq!(segments.at(10.0), Some(0));
        assert_eq!(segments.at(65.0), Some(2));
        assert_eq!(segments.at(100.0), None);
        assert_eq!(segments.at(-1.0), None);
        assert_eq!(segments.total_width(), 100.0);
        let lefts: Vec<f64> = segments.placed(5.0).map(|(_, x, _)| x).collect();
        assert_eq!(lefts, [5.0, 35.0, 65.0]);
        let rect = Rect::new(10.0, 0.0, 100.0, 20.0);
        assert!(segments.track_hover(&UiEvent::PointerEnter(Point::new(15.0, 5.0)), rect));
        assert_eq!(segments.hot, Some(0));
        assert!(!segments.track_hover(&UiEvent::PointerEnter(Point::new(16.0, 5.0)), rect));
        assert!(segments.track_hover(&UiEvent::PointerLeave, rect));
        assert_eq!(segments.hot, None);
    }

    #[test]
    fn arrows_step_and_home_and_end_jump() {
        use plt_platform::Modifiers;

        let segments = Segments {
            widths: vec![30.0, 30.0, 40.0],
            ..Segments::default()
        };
        let press = |key, modifiers| KeyEvent {
            key,
            state: ButtonState::Pressed,
            repeat: false,
            text: None,
            modifiers,
            scancode: 0,
        };
        let to = |key, current, up_down| {
            segments.key_target(&press(key, Modifiers::NONE), current, up_down)
        };
        assert_eq!(to(Key::Right, 0, false), Some(1));
        assert_eq!(to(Key::Right, 2, false), Some(2), "no wrapping");
        assert_eq!(to(Key::Left, 0, false), Some(0));
        assert_eq!(to(Key::Down, 0, true), Some(1));
        assert_eq!(to(Key::Down, 0, false), None);
        assert_eq!(to(Key::End, 0, false), Some(2));
        assert_eq!(to(Key::Home, 2, false), Some(0));
        assert_eq!(to(Key::Enter, 0, true), None);
        let ctrl = press(Key::Right, Modifiers::CONTROL);
        assert_eq!(segments.key_target(&ctrl, 0, true), None);
        assert_eq!(
            Segments::default().key_target(&press(Key::Home, Modifiers::NONE), 0, true),
            None
        );
    }
}
