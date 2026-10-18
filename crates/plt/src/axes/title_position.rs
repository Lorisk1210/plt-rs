//! Lifting the titles of an axes above what rises over its top edge:
//! tick labels or an axis label on top, a y offset text in the way
//! (matplotlib's `_update_title_position`).

use std::any::Any;

use plt_core::Bbox;

use super::AxesState;
use super::axis::AxisState;
use super::axis_layout::AxisPainter;
use super::state::TitlePlacement;
use super::tight_bbox::axis_tight_bbox;
use crate::artist::{Artist, ArtistId, LayoutContext};
use crate::axes_grid1::is_parasite;
use crate::figure::FigureState;
use crate::layout::Aligned;
use crate::spines::SpineSide;
use crate::text::TextArtist;
use crate::ticker::{AxisName, NullLocator};
use crate::transforms::TransformContext;

/// Where the ticks of an axis are (matplotlib's `_get_ticks_position`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TicksPosition {
    /// Marks and labels on the first side only.
    First,
    /// Marks and labels on the second side only.
    Second,
    /// Marks on both sides, labels on the first.
    Default,
    /// Anything else.
    Unknown,
}

impl AxisState {
    /// The tick position judged from the major and minor tick styles,
    /// skipping levels without ticks.
    fn ticks_position(&self) -> TicksPosition {
        let used = |locator: &dyn Any| !locator.is::<NullLocator>();
        let styles: Vec<_> = [
            (used(self.major_locator.as_ref()), &self.major_style),
            (used(self.minor_locator.as_ref()), &self.minor_style),
        ]
        .into_iter()
        .filter_map(|(used, style)| used.then_some(style))
        .collect();
        let all = |marks: (bool, bool), labels: (bool, bool)| {
            styles.iter().all(|style| {
                (style.ticks.first, style.ticks.second) == marks
                    && (style.labels.first, style.labels.second) == labels
            })
        };
        if all((true, false), (true, false)) {
            TicksPosition::First
        } else if all((false, true), (false, true)) {
            TicksPosition::Second
        } else if all((true, true), (true, false)) {
            TicksPosition::Default
        } else {
            TicksPosition::Unknown
        }
    }
}

impl FigureState {
    /// Moves the titles of `axes` up so that their bottom clears the top
    /// of the axes and of its twins' and insets' x axis decorations, then
    /// aligns them (matplotlib's `_update_title_position`). Titles placed
    /// by `axes.titley` stay put.
    pub(crate) fn update_title_position(&mut self, axes: ArtistId) {
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return;
        };
        let titles = state.titles.clone();
        if state.title_placement == TitlePlacement::Fixed
            || !titles.iter().any(|&title| self.has_text(title))
        {
            return;
        }
        let group = self.title_group(axes);
        let Some(extent) = self.axes_bbox(axes) else {
            return;
        };
        let mut top = group
            .iter()
            .filter_map(|&ax| self.top_obstacle(ax))
            .fold(f64::NEG_INFINITY, f64::max);
        let to_axes = |y: f64| (y - extent.y0()) / extent.height();
        for &title in &titles {
            self.set_title_y(title, 1.0);
            if self.has_text(title) {
                let bbox = self.title_extent(title);
                for &ax in &group {
                    if let Some(offset) = self.y_offset_extent(ax)
                        && bbox.is_some_and(|bbox| offset.intersection(&bbox).is_some())
                    {
                        top = offset.ymax();
                    }
                }
            }
            if top < 0.0 {
                return;
            }
            if self.title_extent(title).is_some_and(|b| b.ymin() < top) {
                self.set_title_y(title, to_axes(top));
                if let Some(bottom) = self.title_extent(title).map(|b| b.ymin())
                    && bottom < top
                {
                    self.set_title_y(title, to_axes(2.0 * top - bottom));
                }
            }
        }
        let highest = titles
            .iter()
            .filter_map(|&title| self.store.get_as::<TextArtist>(title))
            .map(|title| title.position.y)
            .fold(f64::NEG_INFINITY, f64::max);
        for &title in &titles {
            self.set_title_y(title, highest);
        }
    }

    /// The axes whose decorations titles must clear: the twins and the
    /// insets of `axes`, and the axes its titles are aligned with
    /// ([`crate::figure::Figure::align_titles`]).
    fn title_group(&self, axes: ArtistId) -> Vec<ArtistId> {
        let mut group = self.shared.twins(axes);
        if let Some(frame) = self.store.get(axes).and_then(|a| a.base().parent) {
            group.extend(
                self.layout
                    .label_groups
                    .siblings(frame, Aligned::Titles, axes),
            );
        }
        if let Some(state) = self.store.get_as::<AxesState>(axes) {
            // matplotlib's child axes, which parasites are not.
            group.extend(state.children.iter().copied().filter(|&child| {
                self.store.get_as::<AxesState>(child).is_some() && !is_parasite(&self.store, child)
            }));
        }
        group
    }

    /// The top of `ax`'s x axis decorations when they are on top (ticks on
    /// the top or in an unknown arrangement, or the label on top), else
    /// the top of the axes.
    fn top_obstacle(&self, ax: ArtistId) -> Option<f64> {
        let state = self.store.get_as::<AxesState>(ax)?;
        let xaxis = state.axis(AxisName::X);
        let on_top = matches!(
            xaxis.ticks_position(),
            TicksPosition::Second | TicksPosition::Unknown
        ) || xaxis.label.side == SpineSide::Top;
        let decorations = (on_top && xaxis.visible)
            .then(|| {
                let layout = LayoutContext::new(self);
                axis_tight_bbox(layout, &AxisPainter::new(ax, state, AxisName::X), false)
            })
            .flatten();
        decorations
            .or_else(|| self.axes_bbox(ax))
            .map(|bbox| bbox.ymax())
    }

    /// The extent of the y offset text of `ax`, if it is shown.
    fn y_offset_extent(&self, ax: ArtistId) -> Option<Bbox> {
        let state = self.store.get_as::<AxesState>(ax)?;
        let layout = LayoutContext::new(self);
        let texts = AxisPainter::new(ax, state, AxisName::Y).texts(layout, &state.rc);
        texts.offset.map(|offset| offset.bbox())
    }

    /// Whether `title` has text.
    fn has_text(&self, title: ArtistId) -> bool {
        self.store
            .get_as::<TextArtist>(title)
            .is_some_and(|title| !title.text.is_empty())
    }

    fn title_extent(&self, title: ArtistId) -> Option<Bbox> {
        let layout = LayoutContext::new(self);
        self.store
            .get_as::<TextArtist>(title)?
            .window_extent(&layout)
    }

    fn set_title_y(&mut self, title: ArtistId, y: f64) {
        if let Some(title) = self.store.get_as_mut::<TextArtist>(title) {
            title.position.y = y;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::spines::SpineSide;

    #[test]
    fn titles_clear_tick_labels_on_top() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let title = ax.set_title("title");
        let bottom = |title: &crate::text::Text| {
            fig.layout();
            title.window_extent().expect("laid out").ymin()
        };
        let plain = bottom(&title);
        ax.xaxis()
            .tick_to(SpineSide::Top)
            .expect("top is a side of the x axis");
        let lifted = bottom(&title);
        assert!(lifted > plain + 5.0, "{lifted} vs {plain}");
    }
}
