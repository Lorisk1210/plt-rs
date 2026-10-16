//! A [`Ui`] shown in a `plt-platform` window.

use plt_platform::{Context, WindowEvent, WindowId, WindowOptions};

use crate::error::UiError;
use crate::ui::Ui;
use crate::widget::{Widget, WidgetId};

/// A window displaying a widget tree: forwards the window's events to the
/// [`Ui`], presents frames when asked to redraw, and requests redraws
/// when the UI changes.
#[derive(Debug)]
pub struct UiWindow<M> {
    id: WindowId,
    ui: Ui<M>,
}

impl<M: 'static> UiWindow<M> {
    /// Opens a window showing `root`.
    pub fn open(
        cx: &mut Context<'_>,
        options: &WindowOptions,
        root: impl Widget<M>,
    ) -> Result<Self, UiError> {
        let id = cx.create_window(options)?;
        let ui = Ui::new(root, cx.inner_size(id)?, cx.scale_factor(id)?)?;
        Ok(Self { id, ui })
    }

    /// The window.
    pub fn id(&self) -> WindowId {
        self.id
    }

    /// The UI.
    pub fn ui(&self) -> &Ui<M> {
        &self.ui
    }

    /// The UI, mutably. After changing widgets through it, request a
    /// redraw (or use [`UiWindow::update`], which does).
    pub fn ui_mut(&mut self) -> &mut Ui<M> {
        &mut self.ui
    }

    /// Removes the messages widgets emitted outside of
    /// [`UiWindow::handle`], such as those of the initial layout.
    pub fn take_messages(&mut self) -> Vec<M> {
        self.ui.take_messages()
    }

    /// Changes the widget `widget` of type `W` with `change` and schedules
    /// a redraw. Returns `None` if there is no such widget.
    pub fn update<W: Widget<M>, R>(
        &mut self,
        cx: &mut Context<'_>,
        widget: WidgetId,
        change: impl FnOnce(&mut W) -> R,
    ) -> Result<Option<R>, UiError> {
        let result = self.ui.widget_mut(widget).map(change);
        cx.request_redraw(self.id)?;
        Ok(result)
    }

    /// Handles an event of this window and returns the messages widgets
    /// emitted.
    pub fn handle(&mut self, cx: &mut Context<'_>, event: &WindowEvent) -> Result<Vec<M>, UiError> {
        let mut messages = self.ui.handle(event)?;
        if *event == WindowEvent::RedrawRequested {
            let frame = self.ui.paint()?;
            cx.present(self.id, &frame)?;
            messages.extend(self.ui.take_messages());
        } else if self.ui.needs_paint() {
            cx.request_redraw(self.id)?;
        }
        Ok(messages)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use plt_platform::{
        ButtonState, Event, EventLoop, Modifiers, MouseButton, PhysicalPosition, PhysicalSize,
    };

    use super::*;
    use crate::widgets::{Button, Flex, Label};

    /// Runs one loop iteration, feeding the window's events to `window`.
    fn pump(event_loop: &mut EventLoop, window: &mut UiWindow<&'static str>) -> Vec<&'static str> {
        let mut messages = Vec::new();
        event_loop
            .pump_events(Some(Duration::ZERO), |cx, event| {
                if let Event::Window { window: id, event } = event
                    && id == window.id()
                {
                    messages.extend(window.handle(cx, &event).unwrap());
                }
            })
            .unwrap();
        messages
    }

    #[test]
    fn presents_frames_and_turns_input_into_messages() {
        let (mut event_loop, handle) = EventLoop::headless();
        let mut toolbar = Flex::row();
        let home = toolbar.push(Button::new("Home", "home"), 0.0);
        let status = toolbar.push(Label::new("ready"), 1.0);
        let options = WindowOptions::new("ui").with_inner_size(PhysicalSize::new(200, 40));
        let mut window = UiWindow::open(&mut event_loop.context(), &options, toolbar).unwrap();
        assert_eq!(pump(&mut event_loop, &mut window), [] as [&str; 0]);
        let frames = handle.take_frames();
        assert_eq!(frames.len(), 1, "the first redraw presents a frame");
        assert_eq!(frames[0].image.pixel(199, 39), Some([240, 240, 240, 255]));

        let center = window.ui().widget_rect(home).unwrap().center();
        for state in [ButtonState::Pressed, ButtonState::Released] {
            let click = WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                position: PhysicalPosition::new(center.x, center.y),
                modifiers: Modifiers::NONE,
                click_count: 1,
            };
            handle.inject(window.id(), click).unwrap();
        }
        assert_eq!(pump(&mut event_loop, &mut window), ["home"]);
        assert_eq!(
            handle.take_frames().len(),
            1,
            "the pressed look is repainted once"
        );

        let mut cx = event_loop.context();
        let updated = window.update(&mut cx, status, |label: &mut Label| {
            label.set_text("clicked");
        });
        assert_eq!(updated.unwrap(), Some(()));
        handle
            .inject(
                window.id(),
                WindowEvent::Resized(PhysicalSize::new(300, 50)),
            )
            .unwrap();
        pump(&mut event_loop, &mut window);
        let frames = handle.take_frames();
        assert_eq!(
            frames.len(),
            1,
            "updates and resizes are coalesced into one frame"
        );
        assert_eq!(frames[0].image.size(), PhysicalSize::new(300, 50));
    }
}
