//! The built-in widgets.

mod button;
mod checkbox;
mod choice;
mod dropdown;
mod flex;
mod form;
mod frame;
mod keycaps;
mod label;
mod list;
mod scroll;
mod scroll_bar;
mod segments;
mod separator;
mod slider;
mod stack;
mod surface;
mod swatch;
mod tabs;
mod text_input;
mod tool_button;
mod type_ahead;

pub use button::Button;
pub use checkbox::Checkbox;
pub use choice::Choice;
pub use dropdown::Dropdown;
pub use flex::{Axis, Flex};
pub use form::Form;
pub use keycaps::Keycaps;
pub use label::{Align, Label, Overflow, Tone};
pub use list::{List, ListItem};
pub use scroll::Scroll;
pub use separator::Separator;
pub use slider::Slider;
pub use stack::Stack;
pub use surface::{Surface, SurfaceEvent};
pub use swatch::Swatch;
pub use tabs::Tabs;
pub use text_input::TextInput;
pub use tool_button::{ToolButton, ToolIcon};

/// Widgets for the tests.
#[cfg(test)]
pub(crate) mod testing {
    use crate::error::UiError;
    use crate::geometry::{Constraints, Rect, Size};
    use crate::widget::{LayoutCx, PaintCx, Widget};

    /// A widget that prefers a fixed size and paints nothing.
    #[derive(Debug)]
    pub(crate) struct Fixed(pub(crate) Size);

    impl<M> Widget<M> for Fixed {
        fn measure(
            &mut self,
            _cx: &mut LayoutCx<'_, M>,
            constraints: Constraints,
        ) -> Result<Size, UiError> {
            Ok(constraints.constrain(self.0))
        }

        fn paint(&self, _cx: &mut PaintCx<'_>, _rect: Rect) -> Result<(), UiError> {
            Ok(())
        }
    }
}
