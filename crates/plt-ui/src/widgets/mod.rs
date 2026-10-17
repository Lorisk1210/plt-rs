//! The built-in widgets.

mod button;
mod flex;
mod label;
mod separator;
mod surface;
mod tool_button;

pub use button::Button;
pub use flex::{Axis, Flex};
pub use label::Label;
pub use separator::Separator;
pub use surface::{Surface, SurfaceEvent};
pub use tool_button::ToolButton;

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
