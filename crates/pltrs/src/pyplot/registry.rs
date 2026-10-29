//! The thread's figures: their numbers and labels, the current figure and
//! its current axes (matplotlib's `Gcf` with `figure`, `gcf`, `gca`,
//! `sca`, `close`, `get_fignums`, `get_figlabels` and `findobj`).

use std::cell::RefCell;

use super::interactive::{close_windows, draw_if_interactive};
use crate::artist::{ArtistHandle, RawHandle};
use crate::axes::Axes;
use crate::figure::Figure;

/// A figure managed by pyplot.
pub(super) struct Entry {
    /// The figure number.
    pub(super) num: u32,
    /// The figure label, empty unless the figure was made by
    /// [`figure_label`].
    pub(super) label: String,
    /// The figure.
    pub(super) figure: Figure,
    /// The title its manager gave its window, if any.
    pub(super) window_title: Option<String>,
    /// The canvas size in physical pixels its manager asked for, until
    /// the figure is next shown.
    pub(super) requested_size: Option<(u32, u32)>,
}

impl Entry {
    /// A figure numbered `num`, labelled `label`.
    fn new(num: u32, label: String, figure: Figure) -> Self {
        Self {
            num,
            label,
            figure,
            window_title: None,
            requested_size: None,
        }
    }

    /// The title of the figure's window: the one its manager set, else its
    /// label, else `Figure <num>`.
    pub(super) fn title(&self) -> String {
        match &self.window_title {
            Some(title) => title.clone(),
            None if self.label.is_empty() => format!("Figure {}", self.num),
            None => self.label.clone(),
        }
    }
}

/// The figures of one thread, by number, and the current one.
#[derive(Default)]
pub(super) struct Registry {
    pub(super) figures: Vec<Entry>,
    pub(super) current: Option<u32>,
    /// Whether figures show as they are made (matplotlib's interactive
    /// mode).
    pub(super) interactive: bool,
}

impl Registry {
    fn get(&self, num: u32) -> Option<&Figure> {
        self.figures
            .iter()
            .find(|entry| entry.num == num)
            .map(|entry| &entry.figure)
    }

    /// The number of the next new figure: one more than the highest
    /// number in use, so that numbers of closed figures are not reused
    /// while higher ones remain (matplotlib's `next_num`).
    fn next_number(&self) -> u32 {
        self.figures
            .iter()
            .map(|entry| entry.num)
            .max()
            .map_or(1, |highest| highest.saturating_add(1))
    }

    /// Registers `figure` under the next number and makes it current.
    fn add(&mut self, figure: Figure, label: String) -> u32 {
        let num = self.next_number();
        self.figures.push(Entry::new(num, label, figure));
        self.current = Some(num);
        num
    }

    /// Forgets the figures `doomed` selects and returns them; when the
    /// current figure goes, the latest remaining one becomes current.
    pub(super) fn remove_where(&mut self, doomed: impl Fn(&Entry) -> bool) -> Vec<Figure> {
        let (removed, kept) = std::mem::take(&mut self.figures)
            .into_iter()
            .partition::<Vec<_>, _>(|entry| doomed(entry));
        self.figures = kept;
        if self.current.is_some_and(|num| self.get(num).is_none()) {
            self.current = self.figures.last().map(|entry| entry.num);
        }
        removed.into_iter().map(|entry| entry.figure).collect()
    }
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default());
}

/// Runs `f` with the thread's figure registry.
pub(super) fn with_registry<R>(f: impl FnOnce(&mut Registry) -> R) -> R {
    REGISTRY.with(|registry| f(&mut registry.borrow_mut()))
}

/// Registers a new figure under the next number, makes it current
/// and shows it in interactive mode.
pub(super) fn new_figure(figure: Figure, label: String) -> Figure {
    with_registry(|registry| registry.add(figure.clone(), label));
    draw_if_interactive();
    figure
}

/// A new figure with the default size, made current (matplotlib's
/// `figure()`).
pub fn figure() -> Figure {
    new_figure(Figure::new(), String::new())
}

/// A new figure of `width × height` inches at `dpi`, made current
/// (matplotlib's `figure(figsize=..., dpi=...)`).
pub fn figure_with_size(width: f64, height: f64, dpi: f64) -> Figure {
    new_figure(Figure::with_size(width, height, dpi), String::new())
}

/// The figure numbered `num`, made current; created if it does not exist
/// (matplotlib's `figure(num)`).
pub fn figure_num(num: u32) -> Figure {
    with_registry(|registry| {
        let fig = if let Some(fig) = registry.get(num) {
            fig.clone()
        } else {
            let fig = Figure::new();
            registry
                .figures
                .push(Entry::new(num, String::new(), fig.clone()));
            fig
        };
        registry.current = Some(num);
        fig
    })
}

/// The figure labelled `label`, made current; created under the next
/// number if no figure has that label (matplotlib's
/// `figure("label")`, which also titles the window).
pub fn figure_label(label: &str) -> Figure {
    let existing = with_registry(|registry| {
        let entry = registry.figures.iter().find(|entry| entry.label == label)?;
        let (num, figure) = (entry.num, entry.figure.clone());
        registry.current = Some(num);
        Some(figure)
    });
    existing.unwrap_or_else(|| new_figure(Figure::new(), label.to_owned()))
}

/// Makes `fig` the current figure, registering it under the next number
/// if pyplot does not manage it yet, and returns its number (matplotlib's
/// `figure(fig)`). For a subfigure, that is its root figure.
pub fn figure_activate(fig: &Figure) -> u32 {
    with_registry(|registry| {
        let known = registry
            .figures
            .iter()
            .find(|entry| entry.figure == *fig)
            .map(|entry| entry.num);
        match known {
            Some(num) => {
                registry.current = Some(num);
                num
            }
            None => registry.add(Figure::root_of(fig.raw().figure_state()), String::new()),
        }
    })
}

/// The numbers of the open figures, in increasing order (matplotlib's
/// `get_fignums`).
pub fn get_fignums() -> Vec<u32> {
    let mut nums: Vec<u32> =
        with_registry(|registry| registry.figures.iter().map(|entry| entry.num).collect());
    nums.sort_unstable();
    nums
}

/// The labels of the open figures by increasing number, empty for
/// unlabelled figures (matplotlib's `get_figlabels`).
pub fn get_figlabels() -> Vec<String> {
    let mut labelled: Vec<(u32, String)> = with_registry(|registry| {
        registry
            .figures
            .iter()
            .map(|entry| (entry.num, entry.label.clone()))
            .collect()
    });
    labelled.sort_unstable_by_key(|(num, _)| *num);
    labelled.into_iter().map(|(_, label)| label).collect()
}

/// Whether figure `num` is open (matplotlib's `fignum_exists`).
pub fn fignum_exists(num: u32) -> bool {
    with_registry(|registry| registry.get(num).is_some())
}

/// Whether a figure labelled `label` is open (matplotlib's
/// `fignum_exists` of a label).
pub fn figlabel_exists(label: &str) -> bool {
    with_registry(|registry| registry.figures.iter().any(|entry| entry.label == label))
}

/// The current figure, created if there is none (matplotlib's `gcf`).
pub fn gcf() -> Figure {
    let current = with_registry(|registry| registry.current.and_then(|n| registry.get(n).cloned()));
    current.unwrap_or_else(figure)
}

/// The current axes of the current figure, created if there is none
/// (matplotlib's `gca`).
pub fn gca() -> Axes {
    gcf().gca()
}

/// Makes `ax` the current axes and its figure the current figure, if that
/// figure is managed here (matplotlib's `sca`).
pub fn sca(ax: &Axes) {
    with_registry(|registry| {
        if let Some(entry) = registry
            .figures
            .iter()
            .find(|entry| entry.figure.contains_artist(ax))
        {
            entry.figure.sca(ax);
            registry.current = Some(entry.num);
        }
    });
}

/// Closes the current figure and its window (matplotlib's `close()`).
pub fn close() {
    close_where(|entry, current| Some(entry.num) == current);
}

/// Closes figure `num` and its window (matplotlib's `close(num)`).
pub fn close_num(num: u32) {
    close_where(|entry, _| entry.num == num);
}

/// Closes the figure labelled `label` and its window (matplotlib's
/// `close("label")`).
pub fn close_label(label: &str) {
    close_where(|entry, _| entry.label == label);
}

/// Closes `fig` and its window, if pyplot manages it (matplotlib's
/// `close(fig)`).
pub fn close_figure(fig: &Figure) {
    close_where(|entry, _| entry.figure == *fig);
}

/// Closes every figure and window (matplotlib's `close("all")`); the
/// interactive mode stays.
pub fn close_all() {
    close_where(|_, _| true);
}

/// Forgets the figures `doomed` selects (given the current number) and
/// closes their windows.
fn close_where(doomed: impl Fn(&Entry, Option<u32>) -> bool) {
    let closed = with_registry(|registry| {
        let current = registry.current;
        registry.remove_where(|entry| doomed(entry, current))
    });
    close_windows(&closed);
}

/// The artists of the current figure, the figure included, for which
/// `matches` holds (matplotlib's `findobj(match=...)`; see
/// [`ArtistHandle::findobj`](crate::artist::ArtistHandle::findobj)).
pub fn findobj(matches: impl Fn(&RawHandle) -> bool) -> Vec<RawHandle> {
    gcf().findobj(matches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn figures_are_numbered_and_current() {
        close_all();
        let first = figure();
        let second = figure();
        assert_eq!(get_fignums(), [1, 2]);
        assert_eq!(gcf(), second);
        assert_eq!(figure_num(1), first);
        assert_eq!(gcf(), first);
        close();
        assert_eq!(get_fignums(), [2]);
        let third = figure();
        assert_eq!(
            get_fignums(),
            [2, 3],
            "matplotlib numbers after the highest"
        );
        assert_eq!(figure_num(3), third);
        close_all();
        assert_eq!(get_fignums(), [] as [u32; 0]);
        figure_num(5);
        assert_eq!(figure_label("next"), figure_num(6));
        close_all();
    }

    #[test]
    fn activating_a_subfigure_registers_its_root() {
        close_all();
        let fig = Figure::new();
        let parts = fig
            .subfigures(1, 2, crate::figure::SubfigureOptions::default())
            .expect("a grid");
        assert_eq!(figure_activate(&parts[(0, 1)]), 1);
        assert_eq!(gcf().id(), fig.id(), "the root figure is current");
        figure();
        assert_eq!(figure_activate(&parts[(0, 0)]), 1, "already managed");
        assert_eq!(gcf().id(), fig.id());
        close_all();
    }

    #[test]
    fn labels_name_and_find_figures() {
        close_all();
        let results = figure_label("results");
        let other = figure_num(7);
        assert_eq!(get_figlabels(), ["results", ""]);
        assert_eq!(figure_label("results"), results);
        assert_eq!(gcf(), results);
        close_label("results");
        assert_eq!(gcf(), other);
        close_num(7);
        assert_eq!(get_fignums(), [] as [u32; 0]);
    }

    #[test]
    fn outside_figures_can_be_made_current() {
        close_all();
        let mine = Figure::with_size(3.0, 2.0, 50.0);
        assert_eq!(figure_activate(&mine), 1);
        assert_eq!(gcf(), mine);
        figure();
        assert_eq!(figure_activate(&mine), 1, "already managed");
        assert_eq!(gcf(), mine);
        close_figure(&mine);
        assert_eq!(get_fignums(), [2]);
        let sized = figure_with_size(2.0, 1.0, 10.0);
        assert_eq!(sized.size_pixels(), (20.0, 10.0));
        close_all();
    }

    #[test]
    fn sca_switches_figure_and_findobj_searches_it() {
        close_all();
        let first = figure();
        let ax = first.subplot();
        figure();
        sca(&ax);
        assert_eq!((gcf(), gca()), (first.clone(), ax.clone()));
        let found = findobj(|artist| artist == ax.raw());
        assert_eq!(found, [ax.raw().clone()]);
        assert!(fignum_exists(2) && !fignum_exists(3));
        close_all();
    }
}
