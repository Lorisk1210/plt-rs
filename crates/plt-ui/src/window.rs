//! A [`Ui`] shown in a `plt-platform` window.

use plt_platform::{
    Context, LogicalSize, PhysicalSize, TimerId, WindowEvent, WindowId, WindowOptions,
};

use crate::error::UiError;
use crate::ui::Ui;
use crate::widget::{Widget, WidgetId};

/// A window displaying a widget tree: forwards the window's events to the
/// [`Ui`], presents frames when asked to redraw, requests redraws when
/// the UI changes, and times tooltips.
///
/// Tooltips need the window's timer events: route
/// [`Event::Timer`](plt_platform::Event::Timer) to
/// [`UiWindow::handle_timer`] when [`UiWindow::owns_timer`] says so.
#[derive(Debug)]
pub struct UiWindow<M> {
    id: WindowId,
    ui: Ui<M>,
    /// The timer running until the pending tooltip of a widget shows.
    tooltip_timer: Option<(TimerId, WidgetId)>,
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
        Ok(Self {
            id,
            ui,
            tooltip_timer: None,
        })
    }

    /// Opens a window showing `root` at the content's natural size (see
    /// [`Ui::natural_size`]), ignoring the size in `options`; dialogs use
    /// this to fit their fields.
    pub fn open_fitted(
        cx: &mut Context<'_>,
        options: &WindowOptions,
        root: impl Widget<M>,
    ) -> Result<Self, UiError> {
        // Measure at scale factor 1 to get logical pixels, before the
        // window (and so its scale factor) exists.
        let mut ui = Ui::new(root, PhysicalSize::new(1, 1), 1.0)?;
        // That layout's messages describe a size never shown.
        ui.take_messages();
        let natural = ui.natural_size()?;
        let logical = LogicalSize::new(natural.width.ceil(), natural.height.ceil());
        let id = cx.create_window(&options.clone().with_inner_size(logical))?;
        let scale_factor = cx.scale_factor(id)?;
        ui.resize(cx.inner_size(id)?, scale_factor)?;
        // Text does not scale exactly with the scale factor: grow the
        // window if the content needs a few more pixels.
        let needed = ui.natural_size()?;
        let size = ui.size();
        let wanted = PhysicalSize::new(
            size.width.max(needed.width.ceil() as u32),
            size.height.max(needed.height.ceil() as u32),
        );
        if wanted != size {
            cx.set_inner_size(id, wanted)?;
            ui.resize(wanted, scale_factor)?;
        }
        Ok(Self {
            id,
            ui,
            tooltip_timer: None,
        })
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
        self.sync_tooltip_timer(cx);
        Ok(messages)
    }

    /// Whether `timer` is one this window started (for a tooltip).
    pub fn owns_timer(&self, timer: TimerId) -> bool {
        self.tooltip_timer.is_some_and(|(id, _)| id == timer)
    }

    /// Handles the firing of a timer of this window (see
    /// [`UiWindow::owns_timer`]): shows the pending tooltip.
    pub fn handle_timer(&mut self, cx: &mut Context<'_>, timer: TimerId) -> Result<(), UiError> {
        if self.owns_timer(timer) {
            self.tooltip_timer = None;
            self.ui.show_tooltip();
            cx.request_redraw(self.id)?;
        }
        Ok(())
    }

    /// Starts the tooltip delay when a widget's tooltip becomes pending,
    /// and stops it when the pointer moves on.
    fn sync_tooltip_timer(&mut self, cx: &mut Context<'_>) {
        let pending = self.ui.pending_tooltip();
        if self.tooltip_timer.map(|(_, widget)| widget) == pending {
            return;
        }
        if let Some((timer, _)) = self.tooltip_timer.take() {
            cx.cancel_timer(timer);
        }
        if let Some(widget) = pending {
            let timer = cx.set_timeout(self.ui.theme().tooltip_delay);
            self.tooltip_timer = Some((timer, widget));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use plt_platform::{
        ButtonState, Event, EventLoop, Modifiers, MouseButton, PhysicalPosition, PhysicalSize,
    };

    use super::*;
    use crate::icon::Icon;
    use crate::theme::Spacing;
    use crate::widgets::{Button, Flex, Label, ToolButton};

    /// Runs one loop iteration, feeding the window's events to `window`.
    fn pump(event_loop: &mut EventLoop, window: &mut UiWindow<&'static str>) -> Vec<&'static str> {
        pump_for(event_loop, window, Duration::ZERO)
    }

    /// Runs one loop iteration waiting up to `timeout` for events, feeding
    /// the window's events and timers to `window`.
    fn pump_for(
        event_loop: &mut EventLoop,
        window: &mut UiWindow<&'static str>,
        timeout: Duration,
    ) -> Vec<&'static str> {
        let mut messages = Vec::new();
        event_loop
            .pump_events(Some(timeout), |cx, event| match event {
                Event::Window { window: id, event } if id == window.id() => {
                    messages.extend(window.handle(cx, &event).unwrap());
                }
                Event::Timer(timer) if window.owns_timer(timer) => {
                    window.handle_timer(cx, timer).unwrap();
                }
                _ => {}
            })
            .unwrap();
        messages
    }

    #[test]
    fn fitted_windows_take_the_size_of_their_contents() {
        let (mut event_loop, handle) = EventLoop::headless();
        handle.set_scale_factor(2.0).unwrap();
        let row = Flex::row()
            .with_padding(Spacing::Lg)
            .with_child(Button::new("OK", "ok"));
        let options = WindowOptions::new("dialog");
        let window = UiWindow::open_fitted(&mut event_loop.context(), &options, row).unwrap();
        let size = handle.window(window.id()).unwrap().size;
        // The narrowest button, 80 × 28 logical pixels, in 16-pixel margins.
        assert_eq!(size, PhysicalSize::new(2 * (80 + 32), 2 * (28 + 32)));
        assert_eq!(window.ui().size(), size);
    }

    #[test]
    fn tooltips_show_once_the_pointer_rests() {
        let (mut event_loop, handle) = EventLoop::headless();
        let button = ToolButton::new(Icon::new(24.0), "go").with_tooltip("Go");
        let options = WindowOptions::new("tools").with_inner_size(PhysicalSize::new(200, 100));
        let mut window = UiWindow::open(
            &mut event_loop.context(),
            &options,
            Flex::row().with_child(button),
        )
        .unwrap();
        pump(&mut event_loop, &mut window);
        let moved = WindowEvent::CursorMoved {
            position: PhysicalPosition::new(16.0, 16.0),
            modifiers: Modifiers::NONE,
        };
        handle.inject(window.id(), moved).unwrap();
        pump(&mut event_loop, &mut window);
        let pending = window.ui().pending_tooltip();
        assert!(pending.is_some() && window.tooltip_timer.is_some());
        handle.take_frames();
        let delay = window.ui().theme().tooltip_delay;
        let deadline = std::time::Instant::now() + 4 * delay;
        while window.ui().shown_tooltip().is_none() && std::time::Instant::now() < deadline {
            pump_for(&mut event_loop, &mut window, delay);
        }
        assert_eq!(window.ui().shown_tooltip(), pending);
        assert!(window.tooltip_timer.is_none(), "the timer is spent");
        pump(&mut event_loop, &mut window);
        assert!(!handle.take_frames().is_empty(), "the tooltip is painted");
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
        assert_eq!(frames[0].image.pixel(199, 39), Some([246, 247, 249, 255]));

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
