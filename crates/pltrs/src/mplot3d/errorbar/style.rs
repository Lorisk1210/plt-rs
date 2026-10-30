//! The styles of the bars, caps and limit arrows of a 3-D error bar
//! plot, derived from the data line's (matplotlib's `eb_lines_style`,
//! `eb_cap_style` and `eb_quiver_style`).

use crate::collections::CollectionProps;
use crate::colors::Color;
use crate::lines::{LineProps, LineStyle};
use crate::markers::{Marker, MarkerSymbol};

/// The styles shared by the bars, caps and arrows of one call.
pub(super) struct CapStyle {
    /// The data line's style without its marker and line-style parts
    /// (matplotlib's `base_style`).
    pub(super) base: LineProps,
    pub(super) ecolor: Color,
    pub(super) capsize: f64,
    pub(super) capthick: Option<f64>,
    /// Width of the bars: the error width, or the line width given.
    pub(super) bar_width: Option<f64>,
    /// Length of the limit arrows in data units.
    pub(super) arrow_length: f64,
    /// Whether the bars and caps hide their points outside the view
    /// limits; the arrows never do, as in matplotlib.
    pub(super) axlim_clip: bool,
}

pltrs_core::archive_struct!(CapStyle {
    base,
    ecolor,
    capsize,
    capthick,
    bar_width,
    arrow_length,
    axlim_clip
});

impl CapStyle {
    /// The marker size of the caps, which also sizes the arrows.
    pub(super) fn markersize(&self) -> f64 {
        if self.capsize > 0.0 {
            2.0 * self.capsize
        } else {
            crate::rc::params().lines_markersize
        }
    }

    /// matplotlib's `eb_cap_style` with the cap `marker`.
    pub(super) fn cap_props(&self, marker: MarkerSymbol) -> LineProps {
        let mut props = self
            .base
            .clone()
            .linestyle(LineStyle::None)
            .marker(Marker::Symbol(marker))
            .color(self.ecolor.clone());
        if self.capsize > 0.0 {
            props.markersize = Some(2.0 * self.capsize);
        }
        props.markeredgewidth = self.capthick;
        props
    }

    /// matplotlib's `eb_lines_style`: the bars take the error colour, the
    /// error width (or the line width given), and the line's alpha and
    /// z-order.
    pub(super) fn bar_props(&self) -> CollectionProps {
        self.segment_props(self.bar_width)
    }

    /// The arrows' style: the cap style as a line collection (matplotlib's
    /// `eb_quiver_style`, whose `'None'` line style draws solid lines).
    pub(super) fn arrow_props(&self) -> CollectionProps {
        self.segment_props(self.base.linewidth)
    }

    /// Segments in the error colour, `width` wide, with the line's alpha,
    /// z-order and antialiasing.
    fn segment_props(&self, width: Option<f64>) -> CollectionProps {
        let mut props = CollectionProps::default().color(self.ecolor.clone());
        props.base = self.base.base.clone();
        props.linewidths = width.map(|w| vec![w]);
        props.antialiased = self.base.antialiased.map(|a| vec![a]);
        props
    }
}
