//! The options of sliders: their direction, snapping, look of the track,
//! the selection and the handles (matplotlib's `Slider` and `RangeSlider`
//! keywords).

use crate::colors::Color;
use crate::orientation::Orientation;
use crate::patches::PatchProps;

/// Values a slider snaps to (matplotlib's `valstep`).
#[derive(Clone, Debug, PartialEq)]
pub enum ValStep {
    /// Multiples of this step from the minimum.
    Step(f64),
    /// The closest of these values.
    Values(Vec<f64>),
}

/// Options of a [`Slider`](super::Slider) (and, apart from `valinit` and
/// `initcolor`, of a [`RangeSlider`](crate::widgets::RangeSlider)).
#[derive(Clone, Debug, PartialEq)]
pub struct SliderOptions {
    /// The initial value.
    pub valinit: f64,
    /// A printf-style format of the value (`%1.2f`); by default the
    /// axis' scalar formatter.
    pub valfmt: Option<String>,
    /// Whether the minimum can be reached.
    pub closedmin: bool,
    /// Whether the maximum can be reached.
    pub closedmax: bool,
    /// Whether the value follows the pointer while dragging (otherwise it
    /// only jumps on clicks).
    pub dragging: bool,
    /// Values to snap to.
    pub valstep: Option<ValStep>,
    /// The direction.
    pub orientation: Orientation,
    /// Colour of the line marking the initial value; `"none"` makes it
    /// invisible (the line is still there, as in matplotlib).
    pub initcolor: Color,
    /// Colour of the track.
    pub track_color: Color,
    /// The look of the handles.
    pub handle_style: HandleStyle,
    /// Properties of the patch showing the selected part of the track
    /// (matplotlib's extra keyword arguments: face and edge colours,
    /// alpha, hatch, ...); the patch defaults otherwise.
    pub poly: PatchProps,
}

/// The look of the round handles of a slider (matplotlib's
/// `handle_style`).
#[derive(Clone, Debug, PartialEq)]
pub struct HandleStyle {
    /// Fill colour.
    pub facecolor: Color,
    /// Edge colour.
    pub edgecolor: Color,
    /// Diameter in points.
    pub size: f64,
}

impl Default for HandleStyle {
    /// matplotlib's white handles with light grey edges, 10 points wide.
    fn default() -> Self {
        Self {
            facecolor: Color::from("white"),
            edgecolor: Color::from(".75"),
            size: 10.0,
        }
    }
}

impl Default for SliderOptions {
    /// matplotlib's defaults: from 0.5, an initial-value line in
    /// `pltrs.widgets.slider.initcolor` (red) on a track in
    /// `pltrs.widgets.slider.track_color` (light grey).
    fn default() -> Self {
        let rc = crate::rc::params();
        Self {
            valinit: 0.5,
            valfmt: None,
            closedmin: true,
            closedmax: true,
            dragging: true,
            valstep: None,
            orientation: Orientation::Horizontal,
            initcolor: rc.pltrs_widgets_slider_initcolor.clone(),
            track_color: rc.pltrs_widgets_slider_track_color.clone(),
            handle_style: HandleStyle::default(),
            poly: PatchProps::default(),
        }
    }
}
