//! Figure managers (matplotlib's `FigureManagerBase` as pyplot hands it
//! out, `fig.canvas.manager`): a figure's number and the title of its
//! window.

use super::interactive::draw_if_interactive;
use super::registry::{gcf, with_registry};
use crate::figure::Figure;

/// The manager of a pyplot figure: its number and window title.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FigureManager {
    num: u32,
}

impl FigureManager {
    /// The figure number.
    pub fn num(&self) -> u32 {
        self.num
    }

    /// The figure, unless it was closed.
    pub fn figure(&self) -> Option<Figure> {
        with_registry(|registry| {
            registry
                .figures
                .iter()
                .find(|entry| entry.num == self.num)
                .map(|entry| entry.figure.clone())
        })
    }

    /// The title of the figure's window (`Figure <num>`, the figure's
    /// label, or the one set), unless the figure was closed (matplotlib's
    /// `get_window_title`).
    pub fn window_title(&self) -> Option<String> {
        with_registry(|registry| {
            registry
                .figures
                .iter()
                .find(|entry| entry.num == self.num)
                .map(super::registry::Entry::title)
        })
    }

    /// Sets the title of the figure's window (matplotlib's
    /// `set_window_title`): an open window is renamed in interactive mode,
    /// else when the figure is next shown. The figure's label is left
    /// alone.
    pub fn set_window_title(&self, title: &str) {
        with_registry(|registry| {
            if let Some(entry) = registry
                .figures
                .iter_mut()
                .find(|entry| entry.num == self.num)
            {
                entry.window_title = Some(title.to_owned());
            }
        });
        draw_if_interactive();
    }
}

/// The manager of the current figure, which is made if there is none
/// (matplotlib's `get_current_fig_manager`).
pub fn get_current_fig_manager() -> FigureManager {
    let figure = gcf();
    let num = with_registry(|registry| {
        registry
            .figures
            .iter()
            .find(|entry| entry.figure == figure)
            .map(|entry| entry.num)
    })
    .expect("gcf registers the current figure");
    FigureManager { num }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pyplot::{close_all, figure_label, get_figlabels};

    #[test]
    fn managers_title_windows_without_relabelling() {
        close_all();
        let fig = figure_label("data");
        let manager = get_current_fig_manager();
        assert_eq!(manager.figure(), Some(fig));
        assert_eq!(manager.window_title().as_deref(), Some("data"));
        manager.set_window_title("My plot");
        assert_eq!(manager.window_title().as_deref(), Some("My plot"));
        assert_eq!(get_figlabels(), ["data"], "the label stays");
        close_all();
        assert_eq!(manager.window_title(), None, "closed");
        let fresh = get_current_fig_manager();
        assert_eq!(
            fresh.window_title(),
            Some(format!("Figure {}", fresh.num()))
        );
    }
}
