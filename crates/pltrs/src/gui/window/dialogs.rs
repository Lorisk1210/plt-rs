//! The windows a figure window opens from its toolbar: the subplot tool,
//! the save and figure options dialogs, and the keyboard shortcuts.

use pltrs_platform::Context;

use super::FigureWindow;
use crate::gui::GuiError;
use crate::gui::help_dialog::HelpDialog;
use crate::gui::options::OptionsDialog;
use crate::gui::save_dialog::SaveDialog;
use crate::gui::subplot_dialog::SubplotDialog;

impl FigureWindow {
    /// Opens a window with sliders for the figure's subplot parameters,
    /// or raises it when it is open (matplotlib's `configure_subplots`).
    pub(in crate::gui) fn configure_subplots(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Result<(), GuiError> {
        match &self.subplot_tool {
            Some(tool) => cx.raise_window(tool.id())?,
            None => self.subplot_tool = Some(SubplotDialog::open(cx, &self.figure)?),
        }
        Ok(())
    }

    /// Opens the save dialog, or raises it when it is open (matplotlib's
    /// `save_figure`).
    pub(super) fn save_figure(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        match &self.save_dialog {
            Some(dialog) => cx.raise_window(dialog.id())?,
            None => self.save_dialog = Some(SaveDialog::open(cx, &self.figure, &self.title)?),
        }
        Ok(())
    }

    /// Opens the figure options dialog, or raises it when it is open
    /// (matplotlib's `edit_parameters`).
    pub(super) fn edit_parameters(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        if let Some(dialog) = &self.options_dialog {
            cx.raise_window(dialog.id())?;
            return Ok(());
        }
        match OptionsDialog::open(cx, &self.figure)? {
            Some(dialog) => self.options_dialog = Some(dialog),
            None => self.set_status(cx, "There are no axes to edit".to_owned())?,
        }
        Ok(())
    }

    pub(super) fn close_options_dialog(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        match self.options_dialog.take() {
            Some(dialog) => dialog.close(cx),
            None => Ok(()),
        }
    }

    pub(super) fn close_save_dialog(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        match self.save_dialog.take() {
            Some(dialog) => dialog.close(cx),
            None => Ok(()),
        }
    }

    pub(super) fn close_subplot_tool(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        match self.subplot_tool.take() {
            Some(tool) => tool.close(cx),
            None => Ok(()),
        }
    }

    /// Shows the keyboard shortcuts, at most as tall as this window, or
    /// raises them when they are shown (matplotlib's `ToolHelpBase`).
    pub(super) fn show_help(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        if let Some(dialog) = &self.help_dialog {
            cx.raise_window(dialog.id())?;
        } else {
            let height = f64::from(cx.inner_size(self.id())?.height);
            let height = height / cx.scale_factor(self.id())?;
            self.help_dialog = Some(HelpDialog::open(cx, &self.figure, height)?);
        }
        Ok(())
    }

    pub(super) fn close_help_dialog(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        match self.help_dialog.take() {
            Some(dialog) => dialog.close(cx),
            None => Ok(()),
        }
    }
}
