//! Where the tick labels of an axis artist land and how large they are
//! (matplotlib's `TickLabels.get_window_extents` and
//! `get_texts_widths_heights_descents`), and what its helper says of the
//! line, ticks and label (the `get_line`, `get_tick_iterators` and
//! `get_axislabel_pos_angle` of matplotlib's axis artist helpers).

use pltrs_core::{Bbox, Path, Point};
use pltrs_text::mathtext::is_math_text;

use super::{AxisArtistState, LabelGap, Level};
use crate::artist::LayoutContext;
use crate::axes::{AxesState, PlacedText};
use crate::axisartist::helper::{GONE, HelperContext, TickLevels};
use crate::axisartist::ticks::tick_info;
use crate::text::measure_text;
use crate::transforms::Transform;

impl AxisArtistState {
    /// What the helper consults in `layout`.
    fn helper_context<'a>(&self, layout: LayoutContext<'a>) -> Result<HelperContext<'a>, String> {
        let axes = layout.store().get_as::<AxesState>(self.axes).ok_or(GONE)?;
        Ok(HelperContext {
            layout,
            axes,
            id: self.axes,
        })
    }

    /// The major and minor ticks the helper places in `layout`.
    pub(crate) fn helper_ticks(&self, layout: LayoutContext<'_>) -> Result<TickLevels, String> {
        self.helper.ticks(self.helper_context(layout)?)
    }

    /// The line the helper runs along in `layout`, and its coordinates.
    pub(crate) fn helper_line(
        &self,
        layout: LayoutContext<'_>,
    ) -> Result<Option<(Path, Transform)>, String> {
        self.helper.line(self.helper_context(layout)?).map(Some)
    }

    /// Where the helper puts the label in `layout`, in display pixels, and
    /// the direction of the line there.
    pub(crate) fn helper_label_position(
        &self,
        layout: LayoutContext<'_>,
    ) -> Result<Option<(Point, f64)>, String> {
        self.helper.label_position(self.helper_context(layout)?)
    }

    /// The display boxes of the tick labels of `level` as drawing places
    /// them now, none when they are hidden; remembers the gap they leave
    /// for the label, as matplotlib's `get_window_extents` does.
    pub(crate) fn ticklabel_extents(
        &self,
        layout: LayoutContext<'_>,
        level: Level,
    ) -> Result<Vec<Bbox>, String> {
        let drawing = self.drawing(layout, layout.points_to_pixels(1.0), LabelGap::Current)?;
        let index = match level {
            Level::Major => 0,
            Level::Minor => 1,
        };
        Ok(drawing.levels[index]
            .texts
            .iter()
            .map(PlacedText::bbox)
            .collect())
    }

    /// The width, height and descent in pixels of each non-empty tick
    /// label of `level`, in the font of the tick labels.
    pub(crate) fn ticklabel_metrics(
        &self,
        layout: LayoutContext<'_>,
        level: Level,
    ) -> Result<Vec<(f64, f64, f64)>, String> {
        let (major, minor) = self.helper_ticks(layout)?;
        let (ticks, state) = match level {
            Level::Major => (major, &self.major_ticklabels),
            Level::Minor => (minor, &self.minor_ticklabels),
        };
        let (_, labels) = tick_info(&ticks, self.ticklabel_direction.add_angle());
        let style = state.text_style(&self.rc, &self.rc.text_color);
        labels
            .iter()
            .filter(|label| !label.text.trim().is_empty())
            .map(|label| {
                let math = self.rc.text_parse_math && is_math_text(&label.text);
                measure_text(&label.text, &style.font, math, layout.dpi(), &self.rc)
                    .map(|metrics| (metrics.width, metrics.height, metrics.descent))
                    .map_err(|err| err.to_string())
            })
            .collect()
    }
}
