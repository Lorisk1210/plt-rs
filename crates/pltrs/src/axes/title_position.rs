//! Lifting the titles of an axes above what rises over its top edge:
//! tick labels or an axis label on top, a y offset text in the way
//! (matplotlib's `_update_title_position`).

use std::any::Any;

use pltrs_core::Bbox;

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
    /// Whether the decorations of this x axis are on top, for titles to
    /// clear: its ticks on the top or in an unknown arrangement, or its
    /// label on top.
    pub(super) fn decorations_on_top(&self) -> bool {
        matches!(
            self.ticks_position(),
            TicksPosition::Second | TicksPosition::Unknown
        ) || self.label.side == SpineSide::Top
    }

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
    /// [`lift_titles`](Self::lift_titles) as the figure is laid out:
    /// titles the program places stay put.
    pub(crate) fn update_title_position(&mut self, axes: ArtistId) {
        let placement = self
            .store
            .get_as::<AxesState>(axes)
            .map(|a| a.title_placement);
        if placement != Some(TitlePlacement::Program) {
            self.lift_titles(axes);
        }
    }

    /// Moves the titles of `axes` up so that their bottom clears the top
    /// of the axes and of its twins' and insets' x axis decorations, the
    /// insets placed first, then aligns them (matplotlib's
    /// `_update_title_position`). Titles placed by `axes.titley` stay put.
    pub(crate) fn lift_titles(&mut self, axes: ArtistId) {
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return;
        };
        // The order matters: a title the y offset text pushes up lowers
        // the obstacle for the titles after it.
        let titles: Vec<ArtistId> = state.titles_centre_first().collect();
        if state.title_placement == TitlePlacement::Fixed
            || !titles.iter().any(|&title| self.has_text(title))
        {
            return;
        }
        // The insets are placed first, where their locators put them.
        for child in self.child_axes(axes) {
            self.apply_aspect(child);
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
                for &ax in &group {
                    // matplotlib measures the title only against an offset
                    // text there is.
                    if let Some(offset) = self.y_offset_extent(ax)
                        && self
                            .title_extent(title)
                            .is_some_and(|bbox| offset.intersection(&bbox).is_some())
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
                // matplotlib measures the title once more for the place it
                // moves it to, after the test (the font lookups count).
                if self.title_extent(title).is_some_and(|b| b.ymin() < top)
                    && let Some(bottom) = self.title_extent(title).map(|b| b.ymin())
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
    /// ([`crate::figure::Figure::align_titles`]), each once (matplotlib
    /// gathers them in a set: `axes` is among its twins and its aligned
    /// axes alike).
    pub(super) fn title_group(&self, axes: ArtistId) -> Vec<ArtistId> {
        let mut group = self.shared.twins(axes);
        if let Some(frame) = self.store.get(axes).and_then(|a| a.base().parent) {
            group.extend(
                self.layout
                    .label_groups
                    .siblings(frame, Aligned::Titles, axes),
            );
        }
        group.extend(self.child_axes(axes));
        let mut seen = std::collections::HashSet::new();
        group.retain(|&ax| seen.insert(ax));
        group
    }

    /// The child axes of `axes` (matplotlib's `child_axes`: insets and
    /// secondary axes, which parasites are not).
    pub(super) fn child_axes(&self, axes: ArtistId) -> Vec<ArtistId> {
        self.store
            .get_as::<AxesState>(axes)
            .map(|state| {
                (state.children.iter().copied())
                    .filter(|&child| {
                        self.store.get_as::<AxesState>(child).is_some()
                            && !is_parasite(&self.store, child)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The top of `ax`'s x axis decorations when they are on top (ticks on
    /// the top or in an unknown arrangement, or the label on top), else
    /// the top of the axes, or of its outline for a colorbar.
    fn top_obstacle(&self, ax: ArtistId) -> Option<f64> {
        let state = self.store.get_as::<AxesState>(ax)?;
        let xaxis = state.axis(AxisName::X);
        let decorations = (xaxis.decorations_on_top() && xaxis.visible)
            .then(|| {
                let layout = LayoutContext::new(self);
                axis_tight_bbox(layout, &AxisPainter::new(ax, state, AxisName::X), false)
            })
            .flatten();
        decorations
            .or_else(|| crate::colorbar::outline_extent(&LayoutContext::new(self), ax))
            .or_else(|| self.axes_bbox(ax))
            .map(|bbox| bbox.ymax())
    }

    /// The extent of the y offset text of `ax`, if it is shown.
    fn y_offset_extent(&self, ax: ArtistId) -> Option<Bbox> {
        let state = self.store.get_as::<AxesState>(ax)?;
        let layout = LayoutContext::new(self);
        let texts = AxisPainter::new(ax, state, AxisName::Y).texts(&mut { layout }, &state.rc);
        texts.offset.map(|offset| offset.bbox())
    }

    /// Whether `title` has text.
    pub(super) fn has_text(&self, title: ArtistId) -> bool {
        self.store
            .get_as::<TextArtist>(title)
            .is_some_and(|title| !title.text.is_empty())
    }

    /// The display box of `title` as matplotlib's `Text.get_window_extent`
    /// gives it: the unit box for a hidden title, which the lift compares
    /// with the decorations all the same (moving it far up).
    fn title_extent(&self, title: ArtistId) -> Option<Bbox> {
        let text = self.store.get_as::<TextArtist>(title)?;
        if !text.base().visible {
            return Some(Bbox::from_extents(0.0, 0.0, 1.0, 1.0));
        }
        text.window_extent(&LayoutContext::new(self))
    }

    fn set_title_y(&mut self, title: ArtistId, y: f64) {
        if let Some(title) = self.store.get_as_mut::<TextArtist>(title) {
            title.position.y = y;
        }
    }
}

#[cfg(test)]
mod tests {
    use pltrs_text::{HAlign, VAlign};

    use super::TitlePlacement;
    use crate::artist::ArtistHandle;
    use crate::axes::{AxesState, TitleLocation};
    use crate::figure::Figure;
    use crate::fonts::FontWeight;
    use crate::spines::SpineSide;

    #[test]
    fn the_shown_title_prefers_the_usual_location() {
        let ax = Figure::new().subplot();
        assert_eq!(ax.shown_title(), (TitleLocation::Center, String::new()));
        ax.set_title_at(TitleLocation::Right, "right");
        assert_eq!(ax.shown_title(), (TitleLocation::Right, "right".to_owned()));
        crate::rc::context(|| {
            crate::rc::set("axes.titlelocation", "left").unwrap();
            ax.set_title("left");
            assert_eq!(ax.shown_title(), (TitleLocation::Left, "left".to_owned()));
        });
    }

    #[test]
    fn a_hidden_title_is_lifted_as_the_unit_box() {
        // matplotlib measures a hidden title as the unit box at the
        // canvas' corner, below the top of the axes: it lifts it twice.
        let fig = Figure::with_size(6.4, 4.8, 100.0);
        let ax = fig.subplot();
        let title = ax.set_title("title");
        title.set_visible(false);
        fig.layout();
        let expected = (2.0 * 422.4 - 52.8) / 369.6;
        assert!(
            (title.position().y - expected).abs() < 1e-12,
            "{}",
            title.position().y
        );
    }

    #[test]
    fn setting_a_title_restores_its_defaults() {
        // matplotlib's `set_title` resets the size, weight and alignment
        // and the height, to `axes.titley` when set, which also stops
        // lifting the titles above the decorations on top.
        let fig = Figure::new();
        let ax = fig.subplot();
        let rc = crate::rc::params();
        let title = ax.set_title_at(TitleLocation::Left, "first");
        title
            .set_fontsize(30.0)
            .set_weight(FontWeight::BOLD)
            .set_halign(HAlign::Center)
            .set_y(0.5);
        ax.set_title_at(TitleLocation::Left, "second");
        assert_eq!(title.font().size, rc.axes_titlesize.to_points(rc.font_size));
        assert_eq!(title.font().weight, rc.axes_titleweight);
        assert_eq!(
            (title.halign(), title.valign()),
            (HAlign::Left, VAlign::Baseline)
        );
        assert!((title.position().y - 1.0).abs() < f64::EPSILON);
        let placement = || ax.raw().with::<AxesState, _>(|axes| axes.title_placement);
        assert_eq!(placement(), TitlePlacement::Auto);
        crate::rc::context(|| {
            crate::rc::set("axes.titley", "0.5").expect("a valid height");
            let centre = ax.set_title("fixed");
            assert!((centre.position().y - 0.5).abs() < f64::EPSILON);
        });
        assert_eq!(placement(), TitlePlacement::Fixed);
    }

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

    #[test]
    fn colorbar_titles_clear_the_extensions() {
        // matplotlib's test_title_text_loc: above the triangle at the top.
        use crate::colorbar::ColorbarOptions;
        use crate::colors::{Extend, Normalize, ScalarMappable, colormaps};
        let fig = Figure::new();
        let ax = fig.subplot();
        let mappable = ScalarMappable::with_norm(
            colormaps::get("viridis").expect("a colormap"),
            Box::new(Normalize::new(0.0, 1.0)),
        );
        let options = ColorbarOptions {
            extend: Some(Extend::Max),
            ..ColorbarOptions::beside(&ax)
        };
        let colorbar = fig.colorbar(&mappable, options).expect("a colorbar");
        let title = colorbar.ax().set_title("Aardvark");
        fig.layout();
        let title = title.window_extent().expect("laid out");
        let outline = colorbar.outline().window_extent().expect("laid out");
        assert!(
            title.ymin() >= outline.ymax() - 1e-9,
            "{title:?} over {outline:?}"
        );
    }

    #[test]
    fn an_offset_text_under_the_left_title_does_not_lower_aligned_titles() {
        // matplotlib's test_align_titles: the left title meets the y
        // offset text, but the centre title, placed first, already cleared
        // the tick labels on top of the aligned axes, and all three follow.
        let fig = Figure::new();
        let axs = fig.subplots(1, 2).expect("a 1 x 2 grid");
        let (left, right) = (&axs[(0, 0)], &axs[(0, 1)]);
        left.plot([0.0, 1.0], [0.0, 1e6]).expect("equal lengths");
        let titles = [
            crate::axes::TitleLocation::Left,
            crate::axes::TitleLocation::Center,
            crate::axes::TitleLocation::Right,
        ]
        .map(|location| left.set_title_at(location, "title"));
        let aligned = right.set_title("aligned");
        right
            .xaxis()
            .tick_to(SpineSide::Top)
            .expect("top is a side of the x axis");
        fig.align_titles(None);
        fig.layout();
        let ymin = |title: &crate::text::Text| title.window_extent().expect("laid out").ymin();
        for title in &titles {
            assert!((ymin(title) - ymin(&aligned)).abs() < 1e-9);
        }
    }
}
