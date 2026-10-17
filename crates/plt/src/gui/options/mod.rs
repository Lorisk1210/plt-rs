//! The figure options dialog (matplotlib's Qt "Customize" editor,
//! `figureoptions.figure_edit`): titles, limits, labels and scales of the
//! axes, the style of their curves and the colormaps of their images.

mod axes_page;
mod fields;

use plt_platform::{ButtonState, Context, Key, LogicalSize, WindowEvent, WindowId, WindowOptions};
use plt_ui::{Button, Choice, Flex, Label, Stack, UiWindow, WidgetId};

use self::axes_page::AxesPage;
use super::{GuiError, icons};
use crate::axes::TitleLocation;
use crate::figure::Figure;

/// What the dialog's widgets report.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum OptionsMessage {
    /// Another axes was chosen.
    Axes(usize),
    /// Another tab of axes page `page` was chosen.
    Tab {
        /// The axes page.
        page: usize,
        /// The tab.
        tab: usize,
    },
    /// Another curve of axes page `page` was selected.
    Curve {
        /// The axes page.
        page: usize,
        /// The curve.
        index: usize,
    },
    /// Another image of axes page `page` was selected.
    Image {
        /// The axes page.
        page: usize,
        /// The image.
        index: usize,
    },
    /// A field changed (read when applying).
    Edited,
    /// Apply the fields and keep the dialog open.
    Apply,
    /// Apply the fields and close.
    Ok,
    /// Close without applying.
    Cancel,
}

/// How the dialog's last event ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    /// Still open, nothing applied.
    Open,
    /// The settings were applied; `limits_changed` says whether a view
    /// limit moved (matplotlib then records the view for navigation).
    Applied {
        /// Whether a view limit changed.
        limits_changed: bool,
        /// Whether the dialog should close.
        close: bool,
    },
    /// Closed without applying.
    Cancelled,
}

/// The name of axes `index` in the axes choice.
fn axes_name(title: &str, index: usize) -> String {
    let title = title.trim();
    if title.is_empty() {
        format!("Axes {}", index + 1)
    } else if title.chars().count() > 24 {
        format!("{}…", title.chars().take(23).collect::<String>())
    } else {
        title.to_owned()
    }
}

/// A window editing the axes of a figure.
#[derive(Debug)]
pub(super) struct OptionsDialog {
    window: UiWindow<OptionsMessage>,
    pages: Vec<AxesPage>,
    axes_pages: WidgetId,
    status: WidgetId,
}

impl OptionsDialog {
    /// Opens the dialog for the axes of `figure`, or `None` when it has
    /// none.
    ///
    /// # Errors
    ///
    /// When the window cannot be created.
    pub(super) fn open(cx: &mut Context<'_>, figure: &Figure) -> Result<Option<Self>, GuiError> {
        let axes = figure.axes();
        if axes.is_empty() {
            return Ok(None);
        }
        let mut root = Flex::column().with_padding(10.0).with_spacing(8.0);
        if axes.len() > 1 {
            let names = axes
                .iter()
                .enumerate()
                .map(|(i, ax)| axes_name(&ax.title(TitleLocation::Center), i))
                .collect();
            root.push(Choice::new(names, OptionsMessage::Axes), 0.0);
        }
        let mut stack = Stack::new();
        let pages = axes
            .iter()
            .enumerate()
            .map(|(index, ax)| {
                let (page, widgets) = AxesPage::build(ax, index);
                stack.push(widgets);
                page
            })
            .collect();
        let axes_pages = root.push(stack, 1.0);
        let mut buttons = Flex::row().with_spacing(8.0);
        let status = buttons.push(Label::new(""), 1.0);
        buttons.push(Button::new("Cancel", OptionsMessage::Cancel), 0.0);
        buttons.push(Button::new("Apply", OptionsMessage::Apply), 0.0);
        buttons.push(Button::new("OK", OptionsMessage::Ok).primary(), 0.0);
        root.push(buttons, 0.0);
        let options = WindowOptions::new("Figure options")
            .with_app_id("plt-rs")
            .with_icon(icons::window_icon()?)
            .with_inner_size(LogicalSize::new(700.0, 560.0))
            .with_min_inner_size(LogicalSize::new(460.0, 420.0));
        let window = UiWindow::open(cx, &options, root)?;
        Ok(Some(Self {
            window,
            pages,
            axes_pages,
            status,
        }))
    }

    /// The window.
    pub(super) fn id(&self) -> WindowId {
        self.window.id()
    }

    /// Handles an event of the dialog's window.
    ///
    /// # Errors
    ///
    /// When the window cannot be updated.
    pub(super) fn handle(
        &mut self,
        cx: &mut Context<'_>,
        event: &WindowEvent,
    ) -> Result<Outcome, GuiError> {
        match event {
            WindowEvent::CloseRequested => return Ok(Outcome::Cancelled),
            WindowEvent::Key(key)
                if key.key == Key::Escape && key.state == ButtonState::Pressed =>
            {
                return Ok(Outcome::Cancelled);
            }
            _ => {}
        }
        let mut outcome = Outcome::Open;
        for message in self.window.handle(cx, event)? {
            outcome = match message {
                OptionsMessage::Axes(index) => {
                    if let Some(stack) = self
                        .window
                        .ui_mut()
                        .widget_mut::<Stack<OptionsMessage>>(self.axes_pages)
                    {
                        stack.show(index);
                    }
                    Outcome::Open
                }
                OptionsMessage::Tab { page, tab } => {
                    if let Some(axes) = self.pages.get(page) {
                        axes.show_tab(&mut self.window, tab);
                    }
                    Outcome::Open
                }
                OptionsMessage::Curve { page, index } => {
                    if let Some(axes) = self.pages.get(page) {
                        axes.show_curve(&mut self.window, index);
                    }
                    Outcome::Open
                }
                OptionsMessage::Image { page, index } => {
                    if let Some(axes) = self.pages.get(page) {
                        axes.show_image(&mut self.window, index);
                    }
                    Outcome::Open
                }
                OptionsMessage::Edited => Outcome::Open,
                OptionsMessage::Apply => self.apply(cx, false)?,
                OptionsMessage::Ok => self.apply(cx, true)?,
                OptionsMessage::Cancel => Outcome::Cancelled,
            };
            if outcome != Outcome::Open {
                break;
            }
        }
        cx.request_redraw(self.window.id())?;
        Ok(outcome)
    }

    /// Reads every page and applies them all, or shows the first problem
    /// and changes nothing.
    fn apply(&mut self, cx: &mut Context<'_>, close: bool) -> Result<Outcome, GuiError> {
        let mut plans = Vec::with_capacity(self.pages.len());
        for page in &self.pages {
            match page.read(&mut self.window) {
                Ok(plan) => plans.push(plan),
                Err(problem) => {
                    self.window.update(cx, self.status, |label: &mut Label| {
                        label.set_text(problem.0);
                    })?;
                    return Ok(Outcome::Open);
                }
            }
        }
        let limits_changed = self
            .pages
            .iter()
            .zip(plans)
            .fold(false, |changed, (page, plan)| page.apply(plan) | changed);
        self.window
            .update(cx, self.status, |label: &mut Label| label.set_text(""))?;
        Ok(Outcome::Applied {
            limits_changed,
            close,
        })
    }

    /// Closes the dialog's window.
    ///
    /// # Errors
    ///
    /// When the window system fails to close it.
    pub(super) fn close(self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        cx.close_window(self.id())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axes_are_named_by_their_titles() {
        assert_eq!(axes_name("", 1), "Axes 2");
        assert_eq!(axes_name(" Signal ", 0), "Signal");
        assert_eq!(axes_name(&"x".repeat(40), 0).chars().count(), 24);
    }
}
