//! The shapes of matplotlib's built-in colormap definitions and how each
//! becomes a [`Colormap`].

use std::f64::consts::PI;

use pltrs_core::Rgba;

use crate::colors::colormap::cubehelix_channel;
use crate::colors::{
    Color, Colormap, ColormapError, Segment, SegmentData, SegmentFn, color_sequence,
};

/// How `_cm.py` / `_cm_listed.py` define a base colormap.
#[derive(Clone, Copy, Debug)]
pub(super) enum Builtin {
    /// `ListedColormap(rows)`.
    Listed(&'static [[f64; 3]]),
    /// `twilight_shifted`: the rows rotated by half their length, reversed.
    Shifted(&'static [[f64; 3]]),
    /// `LinearSegmentedColormap(name, {"red": .., "green": .., "blue": ..})`.
    Segmented([ChannelDef; 3]),
    /// `LinearSegmentedColormap.from_list(name, colours)`.
    FromList(&'static [[f64; 3]]),
    /// `LinearSegmentedColormap.from_list(name, [(value, colour), ...])`.
    FromStops(&'static [(f64, [f64; 3])]),
    /// `ListedColormap` of the colour sequence with the same name.
    Sequence(&'static str),
}

/// One channel of a built-in segmented colormap.
#[derive(Clone, Copy, Debug)]
pub(super) enum ChannelDef {
    /// Anchor rows `[x, y_below, y_above]`.
    Anchors(&'static [[f64; 3]]),
    /// A segment function from `_cm.py`.
    Func(BuiltinFn),
}

/// The segment functions defined in `_cm.py`.
#[derive(Clone, Copy, Debug)]
pub(super) enum BuiltinFn {
    /// gnuplot's palette formula `_g<n>` (n in 0..=36).
    Gnuplot(u8),
    /// `_flag_red`.
    FlagRed,
    /// `_flag_green`.
    FlagGreen,
    /// `_flag_blue`.
    FlagBlue,
    /// `_prism_red`.
    PrismRed,
    /// `_prism_green`.
    PrismGreen,
    /// `_prism_blue`.
    PrismBlue,
    /// `_gist_heat_red`.
    GistHeatRed,
    /// `_gist_heat_green`.
    GistHeatGreen,
    /// `_gist_heat_blue`.
    GistHeatBlue,
    /// `_gist_yarg`.
    GistYarg,
    /// `partial(_ch_helper, gamma, s, r, h, p0, p1)`.
    Cubehelix([f64; 6]),
}

impl BuiltinFn {
    /// The function, named like its Python counterpart.
    fn segment_fn(self) -> SegmentFn {
        let named = |name: &'static str, f: fn(f64) -> f64| SegmentFn::new(name, f);
        match self {
            Self::Gnuplot(n) => SegmentFn::new(format!("_g{n}"), gnuplot(n)),
            Self::FlagRed => named("_flag_red", |x| 0.75 * ((x * 31.5 + 0.25) * PI).sin() + 0.5),
            Self::FlagGreen => named("_flag_green", |x| (x * 31.5 * PI).sin()),
            Self::FlagBlue => named("_flag_blue", |x| {
                0.75 * ((x * 31.5 - 0.25) * PI).sin() + 0.5
            }),
            Self::PrismRed => named("_prism_red", |x| {
                0.75 * ((x * 20.9 + 0.25) * PI).sin() + 0.67
            }),
            Self::PrismGreen => named("_prism_green", |x| {
                0.75 * ((x * 20.9 - 0.25) * PI).sin() + 0.33
            }),
            Self::PrismBlue => named("_prism_blue", |x| -1.1 * ((x * 20.9) * PI).sin()),
            Self::GistHeatRed => named("_gist_heat_red", |x| 1.5 * x),
            Self::GistHeatGreen => named("_gist_heat_green", |x| 2.0 * x - 1.0),
            Self::GistHeatBlue => named("_gist_heat_blue", |x| 4.0 * x - 3.0),
            Self::GistYarg => named("_gist_yarg", |x| 1.0 - x),
            Self::Cubehelix(params) => cubehelix_channel(params),
        }
    }
}

/// gnuplot's `rgbformulae` palette functions (matplotlib's `_cm.gfunc`),
/// with numpy's order of operations.
fn gnuplot(n: u8) -> fn(f64) -> f64 {
    const FORMULAE: [fn(f64) -> f64; 37] = [
        |_| 0.0,
        |_| 0.5,
        |_| 1.0,
        |x| x,
        |x| x * x,
        |x| x.powf(3.0),
        |x| x.powf(4.0),
        f64::sqrt,
        |x| x.sqrt().sqrt(),
        |x| (x * PI / 2.0).sin(),
        |x| (x * PI / 2.0).cos(),
        |x| (x - 0.5).abs(),
        |x| (2.0 * x - 1.0) * (2.0 * x - 1.0),
        |x| (x * PI).sin(),
        |x| (x * PI).cos().abs(),
        |x| (x * 2.0 * PI).sin(),
        |x| (x * 2.0 * PI).cos(),
        |x| (x * 2.0 * PI).sin().abs(),
        |x| (x * 2.0 * PI).cos().abs(),
        |x| (x * 4.0 * PI).sin().abs(),
        |x| (x * 4.0 * PI).cos().abs(),
        |x| 3.0 * x,
        |x| 3.0 * x - 1.0,
        |x| 3.0 * x - 2.0,
        |x| (3.0 * x - 1.0).abs(),
        |x| (3.0 * x - 2.0).abs(),
        |x| (3.0 * x - 1.0) / 2.0,
        |x| (3.0 * x - 2.0) / 2.0,
        |x| ((3.0 * x - 1.0) / 2.0).abs(),
        |x| ((3.0 * x - 2.0) / 2.0).abs(),
        |x| x / 0.32 - 0.78125,
        |x| 2.0 * x - 0.84,
        |x| {
            if x < 0.25 {
                4.0 * x
            } else if x < 0.92 {
                -2.0 * x + 1.84
            } else if x >= 0.92 {
                x / 0.08 - 11.5
            } else {
                0.0
            }
        },
        |x| (2.0 * x - 0.5).abs(),
        |x| 2.0 * x,
        |x| 2.0 * x - 0.5,
        |x| 2.0 * x - 1.0,
    ];
    FORMULAE[usize::from(n)]
}

impl ChannelDef {
    fn segment(self) -> Segment {
        match self {
            Self::Anchors(rows) => Segment::Anchors(rows.to_vec()),
            Self::Func(func) => Segment::Func(func.segment_fn()),
        }
    }
}

impl Builtin {
    /// Builds the colormap; segmented maps get `lutsize` entries.
    pub(super) fn build(self, name: &str, lutsize: usize) -> Result<Colormap, ColormapError> {
        let rgb = |rows: &[[f64; 3]]| -> Vec<Color> { rows.iter().map(|&c| c.into()).collect() };
        match self {
            Self::Listed(rows) => Colormap::listed(name, rgb(rows)),
            Self::Shifted(rows) => {
                let half = rows.len() / 2;
                let mut shifted = [&rows[half..], &rows[..half]].concat();
                shifted.reverse();
                Colormap::listed(name, rgb(&shifted))
            }
            Self::Segmented([red, green, blue]) => {
                let data = SegmentData::new(red.segment(), green.segment(), blue.segment());
                Colormap::segmented(name, data, lutsize, 1.0)
            }
            Self::FromList(rows) => Colormap::from_list(name, rgb(rows), lutsize, 1.0),
            Self::FromStops(stops) => Colormap::from_stops(
                name,
                stops.iter().map(|&(x, c)| (x, Color::from(c))),
                lutsize,
                1.0,
            ),
            Self::Sequence(sequence) => {
                let colors: Vec<Rgba> = color_sequence(sequence)
                    .ok_or_else(|| ColormapError::Unknown(sequence.to_owned()))?;
                Colormap::listed(name, colors)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gnuplot_formulae() {
        assert_eq!(gnuplot(7)(0.25), 0.5);
        assert_eq!(gnuplot(32)(0.1), 0.4);
        assert_eq!(gnuplot(32)(0.5), -2.0 * 0.5 + 1.84);
        assert_eq!(gnuplot(36)(0.75), 0.5);
        assert_eq!(BuiltinFn::Gnuplot(15).segment_fn().name(), "_g15");
    }

    #[test]
    fn cubehelix_names_like_partial() {
        let f = BuiltinFn::Cubehelix([1.0, 0.5, -1.5, 1.0, 1.97294, 0.0]).segment_fn();
        assert_eq!(f.name(), "_ch_helper(1.0, 0.5, -1.5, 1.0, 1.97294, 0.0)");
        assert_eq!(f.eval(0.0), 0.0);
        assert_eq!(f.eval(1.0), 1.0);
    }
}
