//! Legend options resolved against the rc parameters: where the legend
//! goes and the sizes its entries are built with.

use super::{LegendError, LegendLoc, LegendOptions};
use crate::fonts::FontProperties;
use crate::offsetbox::AnchorLoc;
use crate::rc::{Expr, RcParams};

/// A validated legend location (matplotlib's `_loc`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Location {
    /// The least crowded of the nine anchored positions (`"best"`).
    Best,
    /// One of the anchored positions.
    Anchored(AnchorLoc),
    /// The lower-left corner, in fractions of the anchor box.
    Point(f64, f64),
}

impl Location {
    /// Validates a `loc` option (matplotlib's `set_loc`).
    pub(crate) fn parse(loc: &LegendLoc) -> Result<Self, LegendError> {
        match loc {
            LegendLoc::Name(name) if name == "best" => Ok(Self::Best),
            LegendLoc::Name(name) => name
                .parse()
                .map(Self::Anchored)
                .map_err(|_| LegendError::InvalidLocation(format!("{name:?}"))),
            LegendLoc::Code(0) => Ok(Self::Best),
            LegendLoc::Code(code) => AnchorLoc::from_code(*code)
                .map(Self::Anchored)
                .ok_or_else(|| LegendError::InvalidLocation(code.to_string())),
            LegendLoc::Point(x, y) => Ok(Self::Point(*x, *y)),
        }
    }

    /// The location `legend.loc` names: a name in any case, a code or an
    /// `(x, y)` pair (matplotlib's `_validate_legend_loc`).
    pub(crate) fn from_rc(text: &str) -> Result<Self, LegendError> {
        let invalid = || LegendError::InvalidLocation(format!("{text:?}"));
        let lowered = text.to_ascii_lowercase();
        if let Ok(location) = Self::parse(&LegendLoc::Name(lowered)) {
            return Ok(location);
        }
        if let Ok(code) = text.trim().parse::<i64>() {
            return Self::parse(&LegendLoc::Code(code));
        }
        let pair = Expr::parse(text).ok().and_then(|expr| {
            let items = expr.as_sequence()?;
            match items {
                [x, y] => Some((x.as_number()?, y.as_number()?)),
                _ => None,
            }
        });
        pair.map(|(x, y)| Self::Point(x, y)).ok_or_else(invalid)
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
    fn locations_parse_like_matplotlib() {
        assert_eq!(Location::parse(&"best".into()), Ok(Location::Best));
        assert_eq!(
            Location::parse(&LegendLoc::Code(6)),
            Ok(Location::Anchored(AnchorLoc::CenterLeft))
        );
        assert_eq!(
            Location::parse(&(0.2, 0.3).into()),
            Ok(Location::Point(0.2, 0.3))
        );
        assert!(Location::parse(&"Upper Left".into()).is_err());
        assert!(Location::parse(&LegendLoc::Code(11)).is_err());
    }

    #[test]
    fn rc_locations_ignore_case_and_accept_pairs() {
        assert_eq!(
            Location::from_rc("Upper Left"),
            Ok(Location::Anchored(AnchorLoc::UpperLeft))
        );
        assert_eq!(Location::from_rc("0"), Ok(Location::Best));
        assert_eq!(Location::from_rc("(0.5, 1)"), Ok(Location::Point(0.5, 1.0)));
        assert!(Location::from_rc("middle").is_err());
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
