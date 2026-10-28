//! The figure options dialog (matplotlib's Qt "Customize" editor,
//! `figureoptions.figure_edit`): titles, limits, labels and scales of the
//! axes, the style of their curves and the colormaps of their images.

mod artists;
mod axes_page;
mod fields;

use plt_platform::{Context, WindowEvent, WindowId};
use plt_ui::{Dropdown, Flex, Label, Spacing, Stack, Swatch, TextInput, UiWindow, WidgetId};

use self::axes_page::AxesPage;
use super::GuiError;
use super::dialog::{self, Dialog};
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
    /// A colour field changed: its swatch shows the colour typed.
    Color {
        /// The swatch beside the field.
        swatch: WidgetId,
        /// The text typed.
        text: String,
    },
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
        let mut content = Flex::column().with_spacing(dialog::GAP);
        if axes.len() > 1 {
            let names = axes
                .iter()
                .enumerate()
                .map(|(i, ax)| axes_name(&ax.shown_title().1, i))
                .collect();
            let chooser = Flex::row()
                .with_spacing(Spacing::Md)
                .with_child(Label::new("Axes"))
                .with_child(Dropdown::new(names, OptionsMessage::Axes));
            content.push(chooser, 0.0);
        }
        let mut stack = Stack::new();
        let pages: Vec<AxesPage> = axes
            .iter()
            .enumerate()
            .map(|(index, ax)| {
                let (page, widgets) = AxesPage::build(ax, index);
                stack.push(widgets);
                page
            })
            .collect();
        let axes_pages = content.push(stack, 1.0);
        let (mut chrome, status) = Dialog::with_status();
        chrome.button("Cancel", OptionsMessage::Cancel);
        chrome.button("Apply", OptionsMessage::Apply);
        chrome.primary("OK", OptionsMessage::Ok);
        let options = dialog::window_options("Figure options")?;
        let mut window = dialog::open_fitted(cx, figure, &options, chrome.finish(content))?;
        // The first field is the title of the first axes, past its tabs.
        let title = pages.first().map(AxesPage::title_field);
        window.ui_mut().set_focus(title);
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
        if dialog::dismissed(event, self.window.ui()) {
            return Ok(Outcome::Cancelled);
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
                OptionsMessage::Color { swatch, text } => {
                    if let Some(swatch) = self.window.ui_mut().widget_mut::<Swatch>(swatch) {
                        swatch.set_color(fields::swatch_color(&text));
                    }
                    Outcome::Open
                }
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

    /// Reads every page and applies them all, or shows the first problem,
    /// marks its field and changes nothing.
    fn apply(&mut self, cx: &mut Context<'_>, close: bool) -> Result<Outcome, GuiError> {
        let mut plans = Vec::with_capacity(self.pages.len());
        for page in &self.pages {
            match page.read(&mut self.window) {
                Ok(plan) => plans.push(plan),
                Err(problem) => {
                    self.window.update(
                        cx,
                        problem.field,
                        |field: &mut TextInput<OptionsMessage>| field.set_invalid(true),
                    )?;
                    self.window.update(cx, self.status, |label: &mut Label| {
                        label.set_text(problem.message);
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
    fn applying_unedited_fields_changes_no_limit() {
        use plt_platform::EventLoop;

        let figure = Figure::with_size(4.0, 3.0, 100.0);
        let axs = figure.subplots(1, 2).unwrap();
        axs[(0, 0)].plot([0.0, 9.8], [0.0, 0.7]).unwrap();
        axs[(0, 1)].set_xscale("log").unwrap();
        axs[(0, 1)].plot([0.03, 70.0], [-3.3, 0.1]).unwrap();
        let limits = || {
            figure
                .axes()
                .iter()
                .map(|ax| (ax.xlim(), ax.ylim()))
                .collect::<Vec<_>>()
        };
        let before = limits();
        let (mut event_loop, _handle) = EventLoop::headless();
        let mut cx = event_loop.context();
        let mut dialog = OptionsDialog::open(&mut cx, &figure).unwrap().unwrap();
        for close in [false, true] {
            let outcome = dialog.apply(&mut cx, close).unwrap();
            let limits_changed = false;
            assert_eq!(
                outcome,
                Outcome::Applied {
                    limits_changed,
                    close
                }
            );
            assert_eq!(limits(), before);
        }
    }

    #[test]
    fn axes_are_named_by_their_titles() {
        assert_eq!(axes_name("", 1), "Axes 2");
        assert_eq!(axes_name(" Signal ", 0), "Signal");
        assert_eq!(axes_name(&"x".repeat(40), 0).chars().count(), 24);
    }
}
