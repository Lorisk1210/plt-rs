//! The values the subplot tool adjusts: each one's name, range and
//! limits, as matplotlib's Qt tool (`SubplotToolQt`) has them, and the
//! text its fields show and its export copies.

use pltrs_core::pyfmt::FormatSpec;

use super::layout::{ConstrainedSpacing, SubplotLayout};
use crate::gridspec::SubplotParams;

/// The gap the Qt tool keeps between a border and its opposite: left
/// stays below right and bottom below top by this much.
const BORDER_GAP: f64 = 0.001;
/// The step of the arrow keys on a slider, as the Qt tool's spin boxes.
pub(super) const STEP: f64 = 0.005;
/// The largest padding and spacing offered under constrained layout,
/// in inches and figure fractions: beyond, the axes have no room left.
const CONSTRAINED_MAX: f64 = 0.5;

/// One of the figure's subplot parameters (`figure.subplotpars`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui) enum SubplotParam {
    /// Left edge of the grid.
    Left,
    /// Bottom edge of the grid.
    Bottom,
    /// Right edge of the grid.
    Right,
    /// Top edge of the grid.
    Top,
    /// Gap between columns.
    Wspace,
    /// Gap between rows.
    Hspace,
}

/// One of constrained layout's parameters (`ConstrainedLayoutEngine.set`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui) enum LayoutParam {
    /// Padding above and below the decorations, in inches.
    HPad,
    /// Padding left and right of the decorations, in inches.
    WPad,
    /// Space between rows, as a fraction of the figure.
    Hspace,
    /// Space between columns, as a fraction of the figure.
    Wspace,
}

/// A value the tool adjusts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui) enum Param {
    /// A subplot parameter.
    Subplot(SubplotParam),
    /// A parameter of constrained layout.
    Constrained(LayoutParam),
}

impl SubplotParam {
    /// The borders, in the Qt tool's order.
    pub(super) const BORDERS: [Self; 4] = [Self::Top, Self::Bottom, Self::Left, Self::Right];
    /// The spacings, in the Qt tool's order.
    pub(super) const SPACINGS: [Self; 2] = [Self::Hspace, Self::Wspace];

    /// matplotlib's name of the parameter.
    fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Bottom => "bottom",
            Self::Right => "right",
            Self::Top => "top",
            Self::Wspace => "wspace",
            Self::Hspace => "hspace",
        }
    }

    /// The parameter's value in `params`.
    pub(super) fn get(self, params: &SubplotParams) -> f64 {
        match self {
            Self::Left => params.left,
            Self::Bottom => params.bottom,
            Self::Right => params.right,
            Self::Top => params.top,
            Self::Wspace => params.wspace,
            Self::Hspace => params.hspace,
        }
    }

    /// `params` with the parameter set to `value`, limited as the Qt tool
    /// limits it: to three decimals from 0 to 1, a border staying a
    /// thousandth short of its opposite.
    pub(super) fn set(self, params: SubplotParams, value: f64) -> SubplotParams {
        let value = value.clamp(0.0, 1.0);
        let value = round(match self {
            Self::Left => value.min(params.right - BORDER_GAP),
            Self::Right => value.max(params.left + BORDER_GAP),
            Self::Bottom => value.min(params.top - BORDER_GAP),
            Self::Top => value.max(params.bottom + BORDER_GAP),
            Self::Wspace | Self::Hspace => value,
        });
        let mut params = params;
        *match self {
            Self::Left => &mut params.left,
            Self::Bottom => &mut params.bottom,
            Self::Right => &mut params.right,
            Self::Top => &mut params.top,
            Self::Wspace => &mut params.wspace,
            Self::Hspace => &mut params.hspace,
        } = value;
        params
    }
}

impl LayoutParam {
    /// The paddings, in the order of `ConstrainedLayoutEngine.set`.
    pub(super) const PADS: [Self; 2] = [Self::HPad, Self::WPad];
    /// The spacings, in the same order.
    pub(super) const SPACINGS: [Self; 2] = [Self::Hspace, Self::Wspace];

    /// matplotlib's name of the parameter.
    fn name(self) -> &'static str {
        match self {
            Self::HPad => "h_pad",
            Self::WPad => "w_pad",
            Self::Hspace => "hspace",
            Self::Wspace => "wspace",
        }
    }

    /// The parameter's value in `spacing`.
    pub(super) fn get(self, spacing: &ConstrainedSpacing) -> f64 {
        match self {
            Self::HPad => spacing.h_pad,
            Self::WPad => spacing.w_pad,
            Self::Hspace => spacing.hspace,
            Self::Wspace => spacing.wspace,
        }
    }

    /// `spacing` with the parameter set to `value`, to three decimals
    /// within the range offered.
    pub(super) fn set(self, spacing: ConstrainedSpacing, value: f64) -> ConstrainedSpacing {
        let value = round(value.clamp(0.0, CONSTRAINED_MAX));
        let mut spacing = spacing;
        *match self {
            Self::HPad => &mut spacing.h_pad,
            Self::WPad => &mut spacing.w_pad,
            Self::Hspace => &mut spacing.hspace,
            Self::Wspace => &mut spacing.wspace,
        } = value;
        spacing
    }
}

impl Param {
    /// matplotlib's name of the value.
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Subplot(param) => param.name(),
            Self::Constrained(param) => param.name(),
        }
    }

    /// The range its slider spans.
    pub(super) fn range(self) -> (f64, f64) {
        match self {
            Self::Subplot(_) => (0.0, 1.0),
            Self::Constrained(_) => (0.0, CONSTRAINED_MAX),
        }
    }

    /// Its value in `layout`, if `layout` has it.
    pub(super) fn get(self, layout: &SubplotLayout) -> Option<f64> {
        match (self, layout) {
            (Self::Subplot(param), SubplotLayout::Params(params)) => Some(param.get(params)),
            (Self::Constrained(param), SubplotLayout::Constrained(spacing)) => {
                Some(param.get(spacing))
            }
            _ => None,
        }
    }
}

/// `value` to three decimals, the precision of the Qt tool's spin boxes.
fn round(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

/// The text a field shows for `value`: three decimals, as the Qt tool's
/// spin boxes show.
pub(super) fn field_text(value: f64) -> String {
    format!("{value:.3}")
}

/// The values of `layout` as the Qt tool exports them, one `name=value`
/// per line, ready for `subplots_adjust(...)` (or, under constrained
/// layout, `get_layout_engine().set(...)`): the value to three decimals,
/// written with Python's `f"{value:.3}"`.
pub(super) fn export_text(layout: &SubplotLayout) -> String {
    let values: Vec<(&str, f64)> = match layout {
        SubplotLayout::Params(params) => SubplotParam::BORDERS
            .into_iter()
            .chain(SubplotParam::SPACINGS)
            .map(|param| (param.name(), param.get(params)))
            .collect(),
        SubplotLayout::Constrained(spacing) => LayoutParam::PADS
            .into_iter()
            .chain(LayoutParam::SPACINGS)
            .map(|param| (param.name(), param.get(spacing)))
            .collect(),
    };
    let spec = FormatSpec::parse(".3").expect("a valid format spec");
    values
        .into_iter()
        .map(|(name, value)| {
            let text = spec
                .format_float(round(value))
                .expect("floats take a precision");
            format!("{name}={text}")
        })
        .collect::<Vec<_>>()
        .join(",\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> SubplotParams {
        SubplotParams {
            left: 0.125,
            bottom: 0.11,
            right: 0.9,
            top: 0.88,
            wspace: 0.2,
            hspace: 0.2,
        }
    }

    #[test]
    fn borders_stay_a_thousandth_short_of_their_opposites() {
        // The values matplotlib's Qt tool takes for the same inputs.
        let params = params();
        assert_eq!(SubplotParam::Left.set(params, 0.95).left, 0.899);
        assert_eq!(SubplotParam::Bottom.set(params, 0.95).bottom, 0.879);
        assert_eq!(SubplotParam::Right.set(params, 0.0).right, 0.126);
        assert_eq!(SubplotParam::Top.set(params, -1.0).top, 0.111);
        assert_eq!(SubplotParam::Left.set(params, 0.3).left, 0.3);
        assert_eq!(SubplotParam::Wspace.set(params, 3.0).wspace, 1.0);
        assert_eq!(SubplotParam::Hspace.set(params, 0.12345).hspace, 0.123);
        let unchanged = SubplotParam::Left.set(params, 0.3);
        assert_eq!((unchanged.right, unchanged.top), (0.9, 0.88));
        // Past a border not on the thousandths, the limit stays below it.
        let tight = SubplotParams {
            right: 0.969_531,
            ..params
        };
        let left = SubplotParam::Left.set(tight, 1.0).left;
        assert!(left < tight.right, "{left}");
        assert_eq!(left, 0.969);
    }

    #[test]
    fn constrained_values_stay_within_their_range() {
        let spacing = ConstrainedSpacing {
            h_pad: 0.0833,
            w_pad: 0.0833,
            hspace: 0.02,
            wspace: 0.02,
        };
        assert_eq!(LayoutParam::WPad.set(spacing, 0.2).w_pad, 0.2);
        assert_eq!(LayoutParam::HPad.set(spacing, 2.0).h_pad, 0.5);
        assert_eq!(LayoutParam::Wspace.set(spacing, -1.0).wspace, 0.0);
        assert_eq!(LayoutParam::Hspace.set(spacing, 0.0512).hspace, 0.051);
        let layout = SubplotLayout::Constrained(spacing);
        assert_eq!(
            Param::Constrained(LayoutParam::WPad).get(&layout),
            Some(0.0833)
        );
        assert_eq!(Param::Subplot(SubplotParam::Left).get(&layout), None);
        assert_eq!(Param::Constrained(LayoutParam::HPad).range(), (0.0, 0.5));
        assert_eq!(Param::Subplot(SubplotParam::Top).range(), (0.0, 1.0));
    }

    #[test]
    fn values_are_shown_and_exported_as_the_qt_tool_does() {
        assert_eq!(field_text(0.88), "0.880");
        assert_eq!(field_text(0.052_690_972), "0.053");
        // matplotlib's export of the default parameters, and of a tight
        // layout's (Python's f"{value:.3}" of the spin boxes' values).
        assert_eq!(
            export_text(&SubplotLayout::Params(params())),
            "top=0.88,\nbottom=0.11,\nleft=0.125,\nright=0.9,\nhspace=0.2,\nwspace=0.2"
        );
        let tight = SubplotParams {
            left: 0.052_690_972,
            bottom: 0.080_439_815,
            right: 0.969_531_25,
            top: 0.957_758_02,
            wspace: 0.139_355_884,
            hspace: 0.232_684_503,
        };
        assert_eq!(
            export_text(&SubplotLayout::Params(tight)),
            "top=0.958,\nbottom=0.08,\nleft=0.053,\nright=0.97,\nhspace=0.233,\nwspace=0.139"
        );
        let spacing = ConstrainedSpacing {
            h_pad: 0.04167,
            w_pad: 0.0833,
            hspace: 0.02,
            wspace: 1.0,
        };
        assert_eq!(
            export_text(&SubplotLayout::Constrained(spacing)),
            "h_pad=0.042,\nw_pad=0.083,\nhspace=0.02,\nwspace=1.0"
        );
    }
}
