//! What gridlines have of matplotlib's lines beyond their colour, width,
//! style and stroke: their draw style and gap colour, snapping, sketch,
//! clipping, URL and gid, rasterization, image filter and animation (the
//! `grid_drawstyle`, `grid_gapcolor`, ... tick parameters, which
//! matplotlib hands the gridlines' `Line2D`).

use pltrs_render::Sketch;

use crate::artist::AggFilter;
use crate::colors::Color;
use crate::lines::DrawStyle;

/// The line properties of the gridlines of a tick level beyond their
/// stroke.
#[derive(Clone, Debug, PartialEq)]
pub struct GridLineProps {
    /// How the two ends are joined (matplotlib's `drawstyle`; a step of a
    /// straight gridline draws the same line).
    pub drawstyle: DrawStyle,
    /// The colour of the gaps of dashed gridlines; `None` leaves them
    /// empty.
    pub gapcolor: Option<Color>,
    /// Pixel snapping (`None` lets the renderer decide).
    pub snap: Option<bool>,
    /// The hand-drawn wobble: `None` takes `path.sketch` when drawn, as
    /// gridlines made under it do; `Some(None)` draws them straight.
    pub sketch: Option<Option<Sketch>>,
    /// Whether the gridlines are clipped to the axes.
    pub clip_on: bool,
    /// The link of the gridlines in vector output.
    pub url: Option<String>,
    /// The id of the group of each gridline in vector output, unless the
    /// gridline of a tick has one of its own.
    pub gid: Option<String>,
    /// Whether the gridlines are rasterized in vector output.
    pub rasterized: bool,
    /// An image filter the gridlines are drawn through.
    pub agg_filter: Option<AggFilter>,
    /// Whether the gridlines are animated: matplotlib's ticks draw their
    /// gridline all the same, so this is only read back.
    pub animated: bool,
    /// The `Line2D` keyword (`marker`, `visible`) the tick parameters gave
    /// the gridlines, which matplotlib's ticks give theirs themselves: a
    /// tick of the level made from then on gets it twice, which fails
    /// (see [`TickerError::TickKeywordTwice`](crate::ticker::TickerError)).
    pub refused: Option<&'static str>,
}

pltrs_core::archive_struct!(GridLineProps {
    drawstyle,
    gapcolor,
    snap,
    sketch,
    clip_on,
    url,
    gid,
    rasterized,
    agg_filter,
    animated,
    refused
});

impl Default for GridLineProps {
    /// The properties of the gridlines matplotlib's ticks make: straight,
    /// clipped to the axes, with nothing else set.
    fn default() -> Self {
        Self {
            drawstyle: DrawStyle::Default,
            gapcolor: None,
            snap: None,
            sketch: None,
            clip_on: true,
            url: None,
            gid: None,
            rasterized: false,
            agg_filter: None,
            animated: false,
            refused: None,
        }
    }
}

/// Changes to a [`GridLineProps`]: each field set replaces its own.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GridLinePropsOptions {
    /// The draw style (`grid_drawstyle`).
    pub drawstyle: Option<DrawStyle>,
    /// The gap colour (`grid_gapcolor`; `Some(None)` removes it).
    pub gapcolor: Option<Option<Color>>,
    /// Snapping (`grid_snap`; `Some(None)` lets the renderer decide).
    pub snap: Option<Option<bool>>,
    /// The sketch (`grid_sketch_params`; `Some(None)` draws straight).
    pub sketch: Option<Option<Sketch>>,
    /// Clipping to the axes (`grid_clip_on`, `grid_clip_box=None`).
    pub clip_on: Option<bool>,
    /// The link (`grid_url`).
    pub url: Option<Option<String>>,
    /// The group id (`grid_gid`).
    pub gid: Option<Option<String>>,
    /// Rasterization (`grid_rasterized`).
    pub rasterized: Option<bool>,
    /// The image filter (`grid_agg_filter`).
    pub agg_filter: Option<Option<AggFilter>>,
    /// Animation (`grid_animated`).
    pub animated: Option<bool>,
    /// A keyword the gridlines of new ticks would get twice (`grid_marker`,
    /// `grid_visible`).
    pub refused: Option<&'static str>,
}

impl GridLinePropsOptions {
    /// Applies the fields set to `line`.
    ///
    /// ```
    /// use pltrs::axes::{GridLineProps, GridLinePropsOptions};
    /// use pltrs::lines::DrawStyle;
    /// let mut line = GridLineProps::default();
    /// let steps = GridLinePropsOptions { drawstyle: Some(DrawStyle::StepsPre), ..Default::default() };
    /// steps.apply(&mut line);
    /// assert_eq!(line.drawstyle, DrawStyle::StepsPre);
    /// assert!(line.clip_on);
    /// ```
    pub fn apply(&self, line: &mut GridLineProps) {
        fn set<T: Clone>(slot: &mut T, value: Option<&T>) {
            if let Some(value) = value {
                slot.clone_from(value);
            }
        }
        set(&mut line.drawstyle, self.drawstyle.as_ref());
        set(&mut line.gapcolor, self.gapcolor.as_ref());
        set(&mut line.snap, self.snap.as_ref());
        if let Some(sketch) = self.sketch {
            line.sketch = Some(sketch);
        }
        set(&mut line.clip_on, self.clip_on.as_ref());
        set(&mut line.url, self.url.as_ref());
        set(&mut line.gid, self.gid.as_ref());
        set(&mut line.rasterized, self.rasterized.as_ref());
        set(&mut line.agg_filter, self.agg_filter.as_ref());
        set(&mut line.animated, self.animated.as_ref());
        if self.refused.is_some() {
            line.refused = self.refused;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_replace_only_what_they_set() {
        let mut line = GridLineProps::default();
        GridLinePropsOptions {
            gapcolor: Some(Some(Color::from("r"))),
            clip_on: Some(false),
            sketch: Some(None),
            ..GridLinePropsOptions::default()
        }
        .apply(&mut line);
        assert_eq!(line.gapcolor, Some(Color::from("r")));
        assert!(!line.clip_on);
        assert_eq!(line.sketch, Some(None));
        assert_eq!(line.drawstyle, DrawStyle::Default);
        GridLinePropsOptions {
            gapcolor: Some(None),
            ..GridLinePropsOptions::default()
        }
        .apply(&mut line);
        assert_eq!(line.gapcolor, None);
        assert!(!line.clip_on, "untouched fields stay");
    }
}
