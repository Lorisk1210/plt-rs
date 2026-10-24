//! Legend options resolved against the rc parameters: where the legend
//! goes and the sizes its entries are built with.

use super::{LegendError, LegendLoc, LegendOptions, OutsideEdge};
use crate::fonts::FontProperties;
use crate::offsetbox::AnchorLoc;
use crate::rc::RcParams;

/// A legend location as the layout uses it (matplotlib's `_loc`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Location {
    /// The least crowded of the nine anchored positions (`"best"`).
    Best,
    /// One of the anchored positions.
    Anchored(AnchorLoc),
    /// The lower-left corner, in fractions of the anchor box.
    Point(f64, f64),
}

/// A `loc` checked against the parent: where the legend goes and, for
/// the `"outside ..."` locations of figure legends, the edge it makes
/// room at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placement {
    /// Where the legend goes in its anchor box.
    pub(crate) location: Location,
    /// The edge an outside legend makes room at.
    pub(crate) outside: Option<OutsideEdge>,
}

impl Placement {
    /// The placement of `loc` for a figure (`figure`) or axes legend
    /// (matplotlib's `set_loc`).
    ///
    /// # Errors
    ///
    /// When `loc` is `"best"` for a figure legend or `"outside ..."` for
    /// an axes legend.
    pub(crate) fn of(loc: LegendLoc, figure: bool) -> Result<Self, LegendError> {
        let (location, outside) = match loc {
            LegendLoc::Outside { .. } if !figure => {
                return Err(LegendError::OutsideForAxes(loc.to_string()));
            }
            LegendLoc::Best if figure => return Err(LegendError::BestForFigure),
            LegendLoc::Best => (Location::Best, None),
            LegendLoc::Anchored(anchor) => (Location::Anchored(anchor), None),
            LegendLoc::Outside { edge, loc } => (Location::Anchored(loc), Some(edge)),
            LegendLoc::Point(x, y) => (Location::Point(x, y), None),
        };
        Ok(Self { location, outside })
    }

    /// The location this placement was made from.
    pub(crate) fn loc(self) -> LegendLoc {
        match (self.location, self.outside) {
            (Location::Best, _) => LegendLoc::Best,
            (Location::Anchored(loc), Some(edge)) => LegendLoc::Outside { edge, loc },
            (Location::Anchored(anchor), None) => LegendLoc::Anchored(anchor),
            (Location::Point(x, y), _) => LegendLoc::Point(x, y),
        }
    }
}

/// The sizes and counts a legend's entries are built with, from the
/// options or the `legend.*` rcParams.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Settings {
    /// Font of the labels.
    pub(crate) prop: FontProperties,
    /// Size of the labels in points; every spacing below is a multiple.
    pub(crate) fontsize: f64,
    /// Markers on a line entry.
    pub(crate) numpoints: usize,
    /// Marker size relative to the plotted markers.
    pub(crate) markerscale: f64,
    /// Markers on a scatter entry.
    pub(crate) scatterpoints: usize,
    /// One height fraction per scatter marker.
    pub(crate) scatteryoffsets: Vec<f64>,
    pub(crate) borderpad: f64,
    pub(crate) labelspacing: f64,
    pub(crate) handlelength: f64,
    pub(crate) handleheight: f64,
    pub(crate) handletextpad: f64,
    pub(crate) borderaxespad: f64,
    pub(crate) columnspacing: f64,
}

impl Settings {
    /// The settings of a legend made with `options` under `rc`.
    pub(crate) fn resolve(options: &LegendOptions, rc: &RcParams) -> Result<Self, LegendError> {
        let prop = options.prop.clone().unwrap_or_else(|| {
            FontProperties::from_rc(rc)
                .with_size(options.fontsize.unwrap_or(rc.legend_fontsize), rc.font_size)
        });
        let count = |value: Option<usize>, rc_value: i64| {
            value.unwrap_or_else(|| usize::try_from(rc_value).unwrap_or(0))
        };
        let numpoints = count(options.numpoints, rc.legend_numpoints);
        if numpoints == 0 {
            return Err(LegendError::NoPoints);
        }
        let scatterpoints = count(options.scatterpoints, rc.legend_scatterpoints);
        let offsets = options
            .scatteryoffsets
            .clone()
            .unwrap_or_else(|| vec![3.0 / 8.0, 4.0 / 8.0, 2.5 / 8.0]);
        if offsets.is_empty() {
            return Err(LegendError::NoScatterOffsets);
        }
        Ok(Self {
            fontsize: prop.size,
            prop,
            numpoints,
            markerscale: options.markerscale.unwrap_or(rc.legend_markerscale),
            scatterpoints,
            scatteryoffsets: offsets
                .iter()
                .copied()
                .cycle()
                .take(scatterpoints)
                .collect(),
            borderpad: options.borderpad.unwrap_or(rc.legend_borderpad),
            labelspacing: options.labelspacing.unwrap_or(rc.legend_labelspacing),
            handlelength: options.handlelength.unwrap_or(rc.legend_handlelength),
            handleheight: options.handleheight.unwrap_or(rc.legend_handleheight),
            handletextpad: options.handletextpad.unwrap_or(rc.legend_handletextpad),
            borderaxespad: options.borderaxespad.unwrap_or(rc.legend_borderaxespad),
            columnspacing: options.columnspacing.unwrap_or(rc.legend_columnspacing),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placements_check_the_parent() {
        let anchored = |anchor| Placement {
            location: Location::Anchored(anchor),
            outside: None,
        };
        let upper_left = LegendLoc::Anchored(AnchorLoc::UpperLeft);
        assert_eq!(
            Placement::of(upper_left, true),
            Ok(anchored(AnchorLoc::UpperLeft))
        );
        let outside: LegendLoc = "outside right upper".parse().unwrap();
        let placed = Placement::of(outside, true).unwrap();
        assert_eq!(
            placed,
            Placement {
                location: Location::Anchored(AnchorLoc::UpperRight),
                outside: Some(OutsideEdge::Right)
            }
        );
        assert_eq!(placed.loc(), outside);
        assert_eq!(
            Placement::of(outside, false),
            Err(LegendError::OutsideForAxes("outside right upper".into()))
        );
        assert_eq!(
            Placement::of(LegendLoc::Best, true),
            Err(LegendError::BestForFigure)
        );
        let best = Placement::of(LegendLoc::Best, false).unwrap();
        assert_eq!(best.location, Location::Best);
        assert_eq!(best.loc(), LegendLoc::Best);
        let point = Placement::of(LegendLoc::Point(0.2, 0.3), false).unwrap();
        assert_eq!(point.location, Location::Point(0.2, 0.3));
    }

    #[test]
    fn rc_locations_ignore_case_and_accept_pairs() {
        let location = |text: &str| {
            let mut rc = RcParams::matplotlib();
            rc.set("legend.loc", text).ok()?;
            Some(rc.legend_loc.loc)
        };
        assert_eq!(
            location("Upper Left"),
            Some(LegendLoc::Anchored(AnchorLoc::UpperLeft))
        );
        assert_eq!(location("0"), Some(LegendLoc::Best));
        assert_eq!(location("(0.5, 1)"), Some(LegendLoc::Point(0.5, 1.0)));
        assert_eq!(location("middle"), None);
        assert_eq!(location("outside upper right"), None);
    }

    #[test]
    fn scatter_offsets_cycle_to_the_point_count() {
        let rc = RcParams::matplotlib();
        let options = LegendOptions::default().scatterpoints(5_usize);
        let settings = Settings::resolve(&options, &rc).unwrap();
        assert_eq!(settings.scatteryoffsets, [0.375, 0.5, 0.3125, 0.375, 0.5]);
        assert_eq!(settings.fontsize, 10.0);
        assert_eq!(
            Settings::resolve(&LegendOptions::default().numpoints(0_usize), &rc),
            Err(LegendError::NoPoints)
        );
    }
}
