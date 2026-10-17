//! An area showing an image the application renders, such as a figure.

use std::fmt;

use plt_core::Rgba;
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
    selection: Option<Rect>,
    size: PhysicalSize,
    map: EventMap<M>,
}

impl<M> Surface<M> {
    /// A surface whose events become messages through `map` (`None`
    /// ignores an event).
    pub fn new(map: impl Fn(SurfaceEvent) -> Option<M> + 'static) -> Self {
        Self {
            image: None,
            selection: None,
            size: PhysicalSize::default(),
            map: Box::new(map),
        }
    }

    /// Shows (or with `None` hides) a selection rectangle over the image,
    /// in pixels from the surface's top-left corner, such as the box of a
    /// zoom being dragged. Corners may come in any order.
    pub fn set_selection(&mut self, selection: Option<Rect>) {
        self.selection = selection.map(normalized);
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
        let theme = cx.theme();
        cx.canvas().fill_rect(rect, theme.surface)?;
        if let Some(image) = &self.image {
            cx.canvas().draw_pixmap(image, rect.origin())?;
        }
        if let Some(selection) = self.selection {
            let placed = Rect::new(
                rect.x + selection.x,
                rect.y + selection.y,
                selection.width,
                selection.height,
            );
            let tint = Rgba::new(theme.accent.r, theme.accent.g, theme.accent.b, 0.12);
            cx.canvas().fill_rect(placed, tint)?;
            let width = cx.scale_factor();
            cx.canvas().stroke_rect(placed, width, theme.accent)?;
        }
        Ok(())
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

/// `rect` with a non-negative width and height.
fn normalized(rect: Rect) -> Rect {
    let (x0, x1) = (rect.x.min(rect.max_x()), rect.x.max(rect.max_x()));
    let (y0, y1) = (rect.y.min(rect.max_y()), rect.y.max(rect.max_y()));
    Rect::new(x0, y0, x1 - x0, y1 - y0)
}

impl<M> fmt::Debug for Surface<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Surface")
            .field("image", &self.image)
            .field("selection", &self.selection)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}
