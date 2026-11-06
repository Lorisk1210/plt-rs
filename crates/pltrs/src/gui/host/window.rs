//! A hosted window: a figure's canvas above the program's toolbar, kept
//! as the program describes it until it is first shown.

use std::path::Path;

use pltrs_core::Bbox;
use pltrs_platform::{Context, CursorIcon, LogicalSize, TimerId, WindowEvent, WindowId};
use pltrs_ui::{Flex, Rect, Surface, SurfaceEvent, UiWindow, WidgetId};

use super::ShortcutGroup;
use super::input::InputState;
use super::toolbar::{ToolItem, Toolbar};
use super::{HostEvent, HostedWindowId};
use crate::figure::{Figure, Presenting};
use crate::gui::help_dialog::HelpDialog;
use crate::gui::save_dialog::{Outcome, SaveDialog};
use crate::gui::subplot_dialog::{Request, SubplotDialog, SubplotLayout};
use crate::gui::window::{present_figure, toolbar_height};
use crate::gui::{GuiError, dialog};

/// The smallest canvas area a window shrinks to, in logical pixels, as a
/// figure window's.
const MIN_SIZE: LogicalSize = LogicalSize::new(240.0, 160.0);

/// What the widgets of a hosted window report.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Message {
    /// The toolbar button of this name was clicked.
    Tool(String),
    /// Something happened on the canvas.
    Canvas(SurfaceEvent),
}

/// A shown window.
#[derive(Debug)]
struct Shown {
    window: UiWindow<Message>,
    canvas: WidgetId,
    /// Keeps the figure's draws and blits recorded for the window.
    _presenting: Presenting,
    input: InputState,
    /// The file chooser of `save_figure`, while it is open: the window
    /// takes no input meanwhile.
    save_dialog: Option<SaveDialog>,
    /// The keyboard shortcuts, while they are shown.
    help_dialog: Option<HelpDialog>,
    /// The subplot tool, while it is open.
    subplot_tool: Option<SubplotDialog>,
}

/// A window of a [`WindowHost`](super::WindowHost).
#[derive(Debug)]
pub(super) struct HostedWindow {
    id: HostedWindowId,
    figure: Figure,
    title: String,
    /// The canvas area the window opens with, in logical pixels.
    size: LogicalSize,
    toolbar: Option<Toolbar>,
    cursor: CursorIcon,
    /// The zoom rectangle over the canvas, in physical pixels from its
    /// top-left corner.
    rubberband: Option<Rect>,
    fullscreen: bool,
    shown: Option<Shown>,
}

impl HostedWindow {
    /// A window titled `title` for `figure`, `size` logical pixels for the
    /// canvas, with an empty toolbar unless `toolbar` is `None`; it opens
    /// when first shown.
    pub(super) fn new(
        id: HostedWindowId,
        figure: &Figure,
        title: &str,
        size: LogicalSize,
        toolbar: Option<Toolbar>,
    ) -> Self {
        Self {
            id,
            figure: figure.clone(),
            title: title.to_owned(),
            size,
            toolbar,
            cursor: CursorIcon::Default,
            rubberband: None,
            fullscreen: false,
            shown: None,
        }
    }

    pub(super) fn id(&self) -> HostedWindowId {
        self.id
    }

    pub(super) fn title(&self) -> &str {
        &self.title
    }

    pub(super) fn is_shown(&self) -> bool {
        self.shown.is_some()
    }

    pub(super) fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }

    /// The window system's window, once shown.
    pub(super) fn window_id(&self) -> Option<WindowId> {
        self.shown.as_ref().map(|shown| shown.window.id())
    }

    pub(super) fn toolbar(&self) -> Option<&Toolbar> {
        self.toolbar.as_ref()
    }

    /// The toolbar, with the window to update once shown.
    fn toolbar_parts(&mut self) -> Option<(&mut Toolbar, Option<&mut UiWindow<Message>>)> {
        let window = self.shown.as_mut().map(|shown| &mut shown.window);
        self.toolbar.as_mut().map(|toolbar| (toolbar, window))
    }

    /// Opens the window, or raises it when `figure.raise_window` is on
    /// (as matplotlib's backends do on every show); what opening reports
    /// (the canvas' size) goes to `events`.
    pub(super) fn show(
        &mut self,
        cx: &mut Context<'_>,
        events: &mut Vec<HostEvent>,
    ) -> Result<(), GuiError> {
        match &self.shown {
            Some(shown) => {
                if crate::rc::params().figure_raise_window {
                    cx.raise_window(shown.window.id())?;
                }
                Ok(())
            }
            None => self.open(cx, events),
        }
    }

    fn open(&mut self, cx: &mut Context<'_>, events: &mut Vec<HostEvent>) -> Result<(), GuiError> {
        let mut root = Flex::column();
        // The canvas keeps its keys while a toolbar button has the focus.
        let canvas = Surface::new(|event| Some(Message::Canvas(event))).with_unused_keys();
        let canvas = root.push(canvas, 1.0);
        let mut height = self.size.height;
        if let Some(toolbar) = &mut self.toolbar {
            toolbar.build(&mut root);
            height += toolbar_height();
        }
        let options = dialog::window_options(&self.title)
            .with_inner_size(LogicalSize::new(self.size.width, height))
            .with_min_inner_size(MIN_SIZE);
        let mut window = UiWindow::open(cx, &options, root)?;
        window
            .ui_mut()
            .set_theme(dialog::chrome_theme(&self.figure));
        // Keys reach the canvas from the start, as in matplotlib's windows.
        window.ui_mut().set_focus(Some(canvas));
        if let Some(toolbar) = &self.toolbar {
            toolbar.show_message(&mut window, cx)?;
        }
        if self.cursor != CursorIcon::Default {
            cx.set_cursor(window.id(), self.cursor)?;
        }
        if self.fullscreen {
            cx.set_fullscreen(window.id(), true)?;
        }
        let rubberband = self.rubberband;
        window.update(cx, canvas, |surface: &mut Surface<Message>| {
            surface.set_selection(rubberband);
        })?;
        let messages = window.take_messages();
        self.shown = Some(Shown {
            window,
            canvas,
            _presenting: Presenting::new(&self.figure),
            input: InputState::default(),
            save_dialog: None,
            help_dialog: None,
            subplot_tool: None,
        });
        // The first layout reports the canvas' size.
        for message in messages {
            self.apply(message, events);
        }
        Ok(())
    }

    /// Closes the window and the windows it opened, if shown.
    pub(super) fn close(self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        let Some(shown) = self.shown else {
            return Ok(());
        };
        if let Some(dialog) = shown.save_dialog {
            dialog.close(cx)?;
        }
        if let Some(dialog) = shown.help_dialog {
            dialog.close(cx)?;
        }
        if let Some(tool) = shown.subplot_tool {
            tool.close(cx)?;
        }
        cx.close_window(shown.window.id())?;
        Ok(())
    }

    /// Whether `window` is this window or one it opened.
    pub(super) fn owns_window(&self, window: WindowId) -> bool {
        self.shown.as_ref().is_some_and(|shown| {
            shown.window.id() == window
                || shown.save_dialog.as_ref().is_some_and(|d| d.id() == window)
                || shown.help_dialog.as_ref().is_some_and(|d| d.id() == window)
                || shown
                    .subplot_tool
                    .as_ref()
                    .is_some_and(|t| t.id() == window)
        })
    }

    /// Whether `timer` is one of the window's (its tooltips).
    pub(super) fn owns_timer(&self, timer: TimerId) -> bool {
        self.shown
            .as_ref()
            .is_some_and(|shown| shown.window.owns_timer(timer))
    }

    /// Runs the window's timer `timer` (shows a tooltip).
    pub(super) fn tick(&mut self, cx: &mut Context<'_>, timer: TimerId) -> Result<(), GuiError> {
        if let Some(shown) = &mut self.shown {
            shown.window.handle_timer(cx, timer)?;
        }
        Ok(())
    }

    /// Handles an event of this window or of a window it opened, putting
    /// what the program is to know in `events`.
    pub(super) fn handle(
        &mut self,
        cx: &mut Context<'_>,
        window: WindowId,
        event: &WindowEvent,
        events: &mut Vec<HostEvent>,
    ) -> Result<(), GuiError> {
        let id = self.id;
        let Some(shown) = &mut self.shown else {
            return Ok(());
        };
        if let Some(dialog) = shown.save_dialog.as_mut().filter(|d| d.id() == window) {
            let path = match dialog.handle(cx, event)? {
                Outcome::Open => return Ok(()),
                Outcome::Done(path) => Some(path),
                Outcome::Cancelled => None,
            };
            if let Some(dialog) = shown.save_dialog.take() {
                dialog.close(cx)?;
            }
            events.push(HostEvent::FileChosen { window: id, path });
            return Ok(());
        }
        if let Some(dialog) = shown.help_dialog.as_mut().filter(|d| d.id() == window) {
            if !dialog.handle(cx, event)?
                && let Some(dialog) = shown.help_dialog.take()
            {
                dialog.close(cx)?;
            }
            return Ok(());
        }
        if let Some(tool) = shown.subplot_tool.as_mut().filter(|t| t.id() == window) {
            let mut open = true;
            for request in tool.handle(cx, event)? {
                match request {
                    Request::Change(change) => {
                        events.push(HostEvent::SubplotChange { window: id, change });
                    }
                    Request::Close => open = false,
                }
            }
            if !open && let Some(tool) = shown.subplot_tool.take() {
                tool.close(cx)?;
            }
            return Ok(());
        }
        if window != shown.window.id() {
            return Ok(());
        }
        if *event == WindowEvent::CloseRequested {
            events.push(HostEvent::CloseRequested { window: id });
            return Ok(());
        }
        for message in shown.window.handle(cx, event)? {
            self.apply(message, events);
        }
        Ok(())
    }

    /// Reports what a widget said, unless a modal dialog is open and it
    /// is input.
    fn apply(&mut self, message: Message, events: &mut Vec<HostEvent>) {
        let window = self.id;
        let Some(shown) = &mut self.shown else {
            return;
        };
        let modal = shown.save_dialog.is_some();
        match message {
            Message::Canvas(SurfaceEvent::Resized(size)) => events.push(HostEvent::Resized {
                window,
                width: size.width,
                height: size.height,
                scale_factor: shown.window.ui().scale_factor(),
            }),
            Message::Canvas(SurfaceEvent::Input(input)) if !modal => {
                if let Some(input) = shown.input.convert(&input) {
                    events.push(HostEvent::Input { window, input });
                }
            }
            Message::Tool(name) if !modal => events.push(HostEvent::ToolClicked { window, name }),
            Message::Canvas(_) | Message::Tool(_) => {}
        }
    }

    /// Shows what changed on the figure's canvas since it was last shown.
    pub(super) fn present(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        match &mut self.shown {
            Some(shown) => present_figure(&self.figure, &mut shown.window, shown.canvas, cx),
            None => Ok(()),
        }
    }

    /// Renames the window (matplotlib's `set_window_title`).
    pub(super) fn set_title(&mut self, cx: &mut Context<'_>, title: &str) -> Result<(), GuiError> {
        if let Some(window) = self.window_id() {
            cx.set_title(window, title)?;
        }
        title.clone_into(&mut self.title);
        Ok(())
    }

    /// Asks for a canvas of `width × height` physical pixels (matplotlib's
    /// `manager.resize`); a window not shown yet opens at that size. The
    /// program learns of the new size once the window system resized the
    /// window ([`HostEvent::Resized`]).
    pub(super) fn resize(
        &mut self,
        cx: &mut Context<'_>,
        width: u32,
        height: u32,
    ) -> Result<(), GuiError> {
        let Some(window) = self.window_id() else {
            self.size = LogicalSize::new(f64::from(width), f64::from(height));
            return Ok(());
        };
        let scale = cx.scale_factor(window)?;
        let toolbar = if self.toolbar.is_some() {
            toolbar_height()
        } else {
            0.0
        };
        let size = LogicalSize::new(
            f64::from(width) / scale,
            f64::from(height) / scale + toolbar,
        );
        cx.set_inner_size(window, size)?;
        Ok(())
    }

    /// Makes the window full screen, or not.
    pub(super) fn set_fullscreen(
        &mut self,
        cx: &mut Context<'_>,
        fullscreen: bool,
    ) -> Result<(), GuiError> {
        if let Some(window) = self.window_id() {
            cx.set_fullscreen(window, fullscreen)?;
        }
        self.fullscreen = fullscreen;
        Ok(())
    }

    /// Shows `cursor` over the window (matplotlib's `set_cursor`).
    pub(super) fn set_cursor(
        &mut self,
        cx: &mut Context<'_>,
        cursor: CursorIcon,
    ) -> Result<(), GuiError> {
        if let Some(window) = self.window_id()
            && cursor != self.cursor
        {
            cx.set_cursor(window, cursor)?;
        }
        self.cursor = cursor;
        Ok(())
    }

    /// Shows the zoom rectangle `rubberband` over the canvas (in physical
    /// pixels from its top-left corner, corners in any order), or hides it.
    pub(super) fn set_rubberband(
        &mut self,
        cx: &mut Context<'_>,
        rubberband: Option<Bbox>,
    ) -> Result<(), GuiError> {
        let rubberband =
            rubberband.map(|b| Rect::new(b.x0(), b.y0(), b.x1() - b.x0(), b.y1() - b.y0()));
        self.rubberband = rubberband;
        if let Some(shown) = &mut self.shown {
            shown
                .window
                .update(cx, shown.canvas, |surface: &mut Surface<Message>| {
                    surface.set_selection(rubberband);
                })?;
        }
        Ok(())
    }

    /// Adds a toolbar button (see [`Toolbar::add`]); a window without a
    /// toolbar refuses.
    pub(super) fn add_tool(
        &mut self,
        cx: &mut Context<'_>,
        item: ToolItem,
        position: Option<usize>,
    ) -> Result<(), GuiError> {
        let (toolbar, window) = self.toolbar_parts().ok_or(GuiError::NotShown)?;
        toolbar.add(item, position, window, cx)
    }

    /// Removes the toolbar buttons called `name`.
    pub(super) fn remove_tool(
        &mut self,
        cx: &mut Context<'_>,
        name: &str,
    ) -> Result<bool, GuiError> {
        match self.toolbar_parts() {
            Some((toolbar, window)) => toolbar.remove(name, window, cx),
            None => Ok(false),
        }
    }

    /// Checks or unchecks the toolbar buttons called `name`.
    pub(super) fn set_tool_checked(
        &mut self,
        cx: &mut Context<'_>,
        name: &str,
        checked: bool,
    ) -> Result<(), GuiError> {
        match self.toolbar_parts() {
            Some((toolbar, window)) => toolbar.set_checked(name, checked, window, cx),
            None => Ok(()),
        }
    }

    /// Enables or disables the toolbar buttons called `name`.
    pub(super) fn set_tool_enabled(
        &mut self,
        cx: &mut Context<'_>,
        name: &str,
        enabled: bool,
    ) -> Result<(), GuiError> {
        match self.toolbar_parts() {
            Some((toolbar, window)) => toolbar.set_enabled(name, enabled, window, cx),
            None => Ok(()),
        }
    }

    /// Shows `text` in the toolbar's status line.
    pub(super) fn set_message(&mut self, cx: &mut Context<'_>, text: &str) -> Result<(), GuiError> {
        match self.toolbar_parts() {
            Some((toolbar, window)) => toolbar.set_message(text, window, cx),
            None => Ok(()),
        }
    }

    /// Opens the file chooser of `save_figure` in `directory` (else where
    /// the figure windows' starts) proposing the file name `name`, or
    /// raises it when it is open; the choice arrives as
    /// [`HostEvent::FileChosen`].
    pub(super) fn choose_file(
        &mut self,
        cx: &mut Context<'_>,
        directory: Option<&Path>,
        name: &str,
    ) -> Result<(), GuiError> {
        let shown = self.shown.as_mut().ok_or(GuiError::NotShown)?;
        if let Some(dialog) = &shown.save_dialog {
            cx.raise_window(dialog.id())?;
        } else {
            let dialog = SaveDialog::choose(cx, &self.figure, directory, name)?;
            shown.save_dialog = Some(dialog);
        }
        Ok(())
    }

    /// Shows the keyboard shortcuts `groups` lists, or raises them when
    /// they are shown.
    pub(super) fn show_help(
        &mut self,
        cx: &mut Context<'_>,
        groups: Vec<ShortcutGroup>,
    ) -> Result<(), GuiError> {
        let shown = self.shown.as_mut().ok_or(GuiError::NotShown)?;
        if let Some(dialog) = &shown.help_dialog {
            cx.raise_window(dialog.id())?;
            return Ok(());
        }
        let window = shown.window.id();
        let height = f64::from(cx.inner_size(window)?.height) / cx.scale_factor(window)?;
        shown.help_dialog = Some(HelpDialog::open_listing(cx, &self.figure, height, groups)?);
        Ok(())
    }

    /// Opens the subplot tool showing `layout`, with the switch to
    /// constrained layout's values when `offer_constrained`, or shows
    /// `layout` in it and raises it when it is open.
    pub(super) fn open_subplot_tool(
        &mut self,
        cx: &mut Context<'_>,
        layout: SubplotLayout,
        offer_constrained: bool,
    ) -> Result<(), GuiError> {
        let shown = self.shown.as_mut().ok_or(GuiError::NotShown)?;
        if let Some(tool) = &mut shown.subplot_tool {
            tool.show(cx, layout, "")?;
            tool.raise(cx)
        } else {
            let tool = SubplotDialog::open(cx, &self.figure, layout, offer_constrained)?;
            shown.subplot_tool = Some(tool);
            Ok(())
        }
    }

    /// Shows `layout` in the subplot tool, if it is open, with `problem`
    /// (see [`SubplotDialog::show`]).
    pub(super) fn show_subplot_layout(
        &mut self,
        cx: &mut Context<'_>,
        layout: SubplotLayout,
        problem: &str,
    ) -> Result<(), GuiError> {
        match self.shown.as_mut().and_then(|s| s.subplot_tool.as_mut()) {
            Some(tool) => tool.show(cx, layout, problem),
            None => Ok(()),
        }
    }

    /// Puts what the canvas shows on the clipboard as a PNG image
    /// (matplotlib's `ToolCopyToClipboard`).
    pub(super) fn copy_to_clipboard(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        let window = self.window_id().ok_or(GuiError::NotShown)?;
        let png = self.figure.canvas().to_png()?;
        cx.set_clipboard_image(window, png)?;
        Ok(())
    }
}
