//! An area showing an image the application renders, such as a figure.

use std::fmt;

use plt_platform::PhysicalSize;
use plt_raster::Pixmap;

use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};

/// What happened to a [`Surface`].
#[derive(Debug, Clone, PartialEq)]
pub enum SurfaceEvent {
    /// The surface was laid out at a new size, in physical pixels; the
    /// application should render a new image of that size.
    Resized(PhysicalSize),
    /// Input over the surface, with positions relative to its top-left
    /// corner.
    Input(UiEvent),
}

/// Converts surface events into application messages.
type EventMap<M> = Box<dyn Fn(SurfaceEvent) -> Option<M>>;

/// Shows an image supplied by the application, filling all the space it
/// is given, and reports its size and the input over it. It takes
/// keyboard focus when clicked, so key presses reach it.
pub struct Surface<M> {
    image: Option<Pixmap>,
    size: PhysicalSize,
    map: EventMap<M>,
}

impl<M> Surface<M> {
    /// A surface whose events become messages through `map` (`None`
    /// ignores an event).
    pub fn new(map: impl Fn(SurfaceEvent) -> Option<M> + 'static) -> Self {
        Self {
            image: None,
            size: PhysicalSize::default(),
            map: Box::new(map),
        }
    }

    /// Shows `image`, drawn at the top-left corner and clipped to the
    /// surface.
    pub fn set_image(&mut self, image: Pixmap) {
        self.image = Some(image);
    }

    /// The size from the last layout.
    pub fn size(&self) -> PhysicalSize {
        self.size
    }
}

impl<M: 'static> Widget<M> for Surface<M> {
    fn measure(
        &mut self,
        _cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let natural = self.image.as_ref().map_or(Size::ZERO, |image| {
            Size::new(f64::from(image.width()), f64::from(image.height()))
        });
        let fill = |max: f64, natural: f64| if max.is_finite() { max } else { natural };
        let wanted = Size::new(
            fill(constraints.max.width, natural.width),
            fill(constraints.max.height, natural.height),
        );
        Ok(constraints.constrain(wanted))
    }

    fn arrange(&mut self, cx: &mut LayoutCx<'_, M>, rect: Rect) -> Result<(), UiError> {
        // Float-to-int `as` casts saturate; sizes are never negative.
        let size = PhysicalSize::new(rect.width.round() as u32, rect.height.round() as u32);
        if size != self.size {
            self.size = size;
            if let Some(message) = (self.map)(SurfaceEvent::Resized(size)) {
                cx.emit(message);
            }
        }
        Ok(())
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let background = cx.theme().surface;
        cx.canvas().fill_rect(rect, background)?;
        match &self.image {
            Some(image) => cx.canvas().draw_pixmap(image, rect.origin()),
            None => Ok(()),
        }
    }

    fn event(&mut self, cx: &mut EventCx<'_, M>, event: &UiEvent, rect: Rect) {
        let local = event.relative_to(rect.origin());
        if let Some(message) = (self.map)(SurfaceEvent::Input(local)) {
            cx.emit(message);
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }
}

impl<M> fmt::Debug for Surface<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Surface")
            .field("image", &self.image)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}
