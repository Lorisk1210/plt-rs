//! Statistical plots from precomputed statistics: `bxp` (which
//! `boxplot` draws with) and `violin` (which `violinplot` draws with),
//! and `pie`.

use pltrs::axes::{
    Axes, BoxArtist, BoxLabel, BoxPart, BoxProps, BxpOptions, PieDirection, PieOptions, PieShadow,
    ViolinDrawOptions, ViolinSide,
};
use pltrs::collections::Collection;
use pltrs::colors::Color;
use pltrs::fonts::FontSize;
use pltrs::lines::Line2D;
use pltrs::patches::{Patch, PathShape, Wedge};
use pltrs::stats::{BoxStats, ViolinStats};
use pltrs::text::{Text, TextProps};
use pltrs::ticker::LabelFormat;

use super::super::lines::LineKwargs;
use super::super::patches::PatchKwargs;
use super::fills::orientation;
use crate::binding::dict_value;
use crate::binding::prelude::*;

/// The entry `key` of a statistics dict, `None` when absent or `None`.
fn entry<'py>(dict: &Bound<'py>, key: &str) -> PyResult<Option<Bound<'py>>> {
    let value = dict.call_method("get", &[key.into_py(dict.py())?])?;
    Ok((!value.is_none()).then_some(value))
}

/// The number `key` of a statistics dict, `NaN` when absent.
fn optional_number(dict: &Bound<'_>, key: &str) -> PyResult<f64> {
    entry(dict, key)?.map_or(Ok(f64::NAN), |value| value.extract())
}

/// Reads the number `key` of a statistics dict.
fn number(dict: &Bound<'_>, key: &str) -> PyResult<f64> {
    dict_value(dict, key)
}

/// The statistics of one box, from the dict `cbook.boxplot_stats` makes.
pub(super) struct PyBoxStats(BoxStats);

impl FromPy<'_> for PyBoxStats {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let fliers: NdArray = entry(object, "fliers")?
            .map(|fliers| fliers.extract())
            .transpose()?
            .unwrap_or_else(|| NdArray::from_vec(Vec::new()));
        Ok(Self(BoxStats {
            label: entry(object, "label")?
                .map(|label| label.str())
                .transpose()?,
            mean: optional_number(object, "mean")?,
            iqr: optional_number(object, "iqr")?,
            cilo: optional_number(object, "cilo")?,
            cihi: optional_number(object, "cihi")?,
            whishi: number(object, "whishi")?,
            whislo: number(object, "whislo")?,
            fliers: fliers.into_vec()?,
            q1: number(object, "q1")?,
            med: number(object, "med")?,
            q3: number(object, "q3")?,
        }))
    }
}

/// `(positions, widths, capwidths, integer_positions)`: widths and cap
/// widths one per box, and whether the positions were given as integers
/// (whose tick labels read `2` rather than `2.0`).
pub(super) type BoxSizes = (Option<Vec<f64>>, Vec<f64>, Vec<f64>, bool);

/// `(shownotches, showmeans, showcaps, showbox, showfliers, meanline)`.
pub(super) type BoxFlags = (bool, bool, bool, bool, bool, bool);

/// `(whiskerprops, flierprops, medianprops, capprops, meanprops)`.
pub(super) type BoxLineProps = (LineKwargs, LineKwargs, LineKwargs, LineKwargs, LineKwargs);

/// `(manage_ticks, zorder, labels, cycled_colors)`: the legend labels one
/// per box, or one for the first, and the parts (named by the keys of
/// `boxplot`'s result) given the colour None, which take the colours of
/// the property cycle.
pub(super) type BoxMisc = (bool, Option<f64>, Option<Vec<String>>, Vec<String>);

/// The artists of a box plot: `(whiskers, caps, boxes as lines, boxes as
/// patches, medians, fliers, means)`.
pub(super) type BoxArtists = (
    Vec<Line2D>,
    Vec<Line2D>,
    Vec<Line2D>,
    Vec<Patch<PathShape>>,
    Vec<Line2D>,
    Vec<Line2D>,
    Vec<Line2D>,
);

/// Box-and-whisker plots of precomputed statistics (matplotlib's `bxp`,
/// whose argument checks the Python side did); the boxes are patches
/// styled by `boxprops` when `patch_artist`, else lines.
pub(super) fn bxp(
    axes: Axes,
    stats: Vec<PyBoxStats>,
    (positions, widths, capwidths, integer_positions): BoxSizes,
    orient: String,
    (shownotches, showmeans, showcaps, showbox, showfliers, meanline): BoxFlags,
    (patch_artist, boxprops): (bool, Owned),
    (whiskers, fliers, medians, caps, means): BoxLineProps,
    (manage_ticks, zorder, labels, cycled_colors): BoxMisc,
) -> PyResult<BoxArtists> {
    let boxes = Python::with_gil(|py| {
        let props = boxprops.bind(py);
        Ok::<_, PyErr>(if patch_artist {
            BoxProps::Patch(props.extract::<PatchKwargs>()?.0)
        } else {
            BoxProps::Line(props.extract::<LineKwargs>()?.0)
        })
    })?;
    let label = labels.map(|mut labels| {
        if labels.len() == 1 {
            BoxLabel::First(labels.remove(0))
        } else {
            BoxLabel::Each(labels)
        }
    });
    let options = BxpOptions {
        positions,
        integer_positions,
        widths,
        orientation: orientation(&orient),
        boxes: Some(boxes),
        shownotches: Some(shownotches),
        showmeans: Some(showmeans),
        showcaps: Some(showcaps),
        showbox: Some(showbox),
        showfliers: Some(showfliers),
        meanline: Some(meanline),
        whiskerprops: whiskers.0,
        flierprops: fliers.0,
        medianprops: medians.0,
        capprops: caps.0,
        meanprops: means.0,
        manage_ticks,
        zorder,
        capwidths,
        label,
        cycled_colors: cycled_colors
            .iter()
            .map(|part| box_part(part))
            .collect::<PyResult<_>>()?,
    };
    let stats: Vec<BoxStats> = stats.into_iter().map(|stats| stats.0).collect();
    let artists = axes.bxp(&stats, options)?;
    let (mut box_lines, mut box_patches) = (Vec::new(), Vec::new());
    for artist in artists.boxes {
        match artist {
            BoxArtist::Line(line) => box_lines.push(line),
            BoxArtist::Patch(patch) => box_patches.push(patch),
        }
    }
    Ok((
        artists.whiskers,
        artists.caps,
        box_lines,
        box_patches,
        artists.medians,
        artists.fliers,
        artists.means,
    ))
}

/// The part of a box plot a key of matplotlib's `boxplot` result names.
fn box_part(name: &str) -> PyResult<BoxPart> {
    Ok(match name {
        "boxes" => BoxPart::Boxes,
        "whiskers" => BoxPart::Whiskers,
        "caps" => BoxPart::Caps,
        "medians" => BoxPart::Medians,
        "means" => BoxPart::Means,
        "fliers" => BoxPart::Fliers,
        _ => return Err(PyErr::value_error(format!("no box plot part {name:?}"))),
    })
}

/// The statistics of one violin, from the dict `cbook.violin_stats`
/// makes.
pub(super) struct PyViolinStats(ViolinStats);

impl FromPy<'_> for PyViolinStats {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let array = |key: &str| -> PyResult<Vec<f64>> {
            entry(object, key)?
                .map(|values| values.extract::<NdArray>()?.into_vec())
                .transpose()
                .map(Option::unwrap_or_default)
        };
        Ok(Self(ViolinStats {
            coords: array("coords")?,
            vals: array("vals")?,
            mean: number(object, "mean")?,
            median: number(object, "median")?,
            min: number(object, "min")?,
            max: number(object, "max")?,
            quantiles: array("quantiles")?,
        }))
    }
}

/// `(showmeans, showextrema, showmedians)`.
pub(super) type ViolinFlags = (bool, bool, bool);

/// The parts of a violin plot: `(bodies, cmeans, cmaxes, cmins, cbars,
/// cmedians, cquantiles)`.
pub(super) type ViolinArtists = (
    Vec<Collection>,
    Option<Collection>,
    Option<Collection>,
    Option<Collection>,
    Option<Collection>,
    Option<Collection>,
    Option<Collection>,
);

/// Violins of precomputed statistics (matplotlib's `violin`): at
/// `positions` with `widths`, in `orient`, on `side` (`"both"`, `"low"`
/// or `"high"`), filled with `facecolors` and lined with `linecolors`
/// (empty for the defaults).
pub(super) fn violin(
    axes: Axes,
    stats: Vec<PyViolinStats>,
    (positions, widths): (Option<Vec<f64>>, Vec<f64>),
    orient: String,
    (showmeans, showextrema, showmedians): ViolinFlags,
    side: String,
    (facecolors, linecolors): (Vec<Color>, Vec<Color>),
) -> PyResult<ViolinArtists> {
    let options = ViolinDrawOptions {
        positions,
        orientation: orientation(&orient),
        widths,
        showmeans,
        showextrema,
        showmedians,
        side: match side.as_str() {
            "low" => ViolinSide::Low,
            "high" => ViolinSide::High,
            _ => ViolinSide::Both,
        },
        facecolors,
        linecolors,
    };
    let stats: Vec<ViolinStats> = stats.into_iter().map(|stats| stats.0).collect();
    let parts = axes.violin(&stats, options)?;
    Ok((
        parts.bodies,
        parts.cmeans,
        parts.cmaxes,
        parts.cmins,
        parts.cbars,
        parts.cmedians,
        parts.cquantiles,
    ))
}

/// The wedge layout of a pie: `(explode, radius, startangle,
/// counterclock, wedge width, center)`.
pub(super) type PieLayout = (Vec<f64>, f64, f64, bool, Option<f64>, (f64, f64));

/// The labels of a pie: `(labels, autopct, pctdistance, labeldistance,
/// rotatelabels, (text size, text colour, usetex))`; `autopct` is a
/// template or a callable, whose percent signs are escaped when TeX
/// typesets the labels.
pub(super) type PieLabels = (
    Vec<String>,
    Option<Owned>,
    f64,
    Option<f64>,
    bool,
    (Option<f64>, Option<Color>, Option<bool>),
);

/// The look of a pie: `(colors, hatches, wedgeprops, shadow, frame,
/// normalize)`; the shadow is `None` or `(ox, oy, shade, props)`, its
/// `props` overriding those it copies from its wedge.
pub(super) type PieLook = (
    Vec<Color>,
    Vec<String>,
    PatchKwargs,
    Option<(f64, f64, f64, PatchKwargs)>,
    bool,
    bool,
);

/// The artists of a pie: `(wedges, labels, percentage labels)`.
pub(super) type PieArtists = (Vec<Patch<Wedge>>, Vec<Text>, Vec<Text>);

/// A pie chart of `values` (matplotlib's `pie`, whose argument checks
/// the Python side did).
pub(super) fn pie(
    axes: Axes,
    values: Vec<f64>,
    (explode, radius, startangle, counterclock, wedge_width, center): PieLayout,
    (labels, autopct, pctdistance, labeldistance, rotatelabels, (size, color, usetex)): PieLabels,
    (colors, hatches, wedgeprops, shadow, frame, normalize): PieLook,
) -> PyResult<PieArtists> {
    let autopct = autopct
        .map(|format| {
            Python::with_gil(|py| {
                let format = format.bind(py);
                if let Ok(template) = format.extract::<String>() {
                    return Ok(LabelFormat::Template(template));
                }
                crate::modules::archive::label_format(&format)
            })
        })
        .transpose()?;
    let options = PieOptions {
        explode,
        labels,
        colors,
        autopct,
        pctdistance,
        labeldistance,
        startangle,
        radius,
        direction: if counterclock {
            PieDirection::CounterClockwise
        } else {
            PieDirection::Clockwise
        },
        wedge_width,
        wedgeprops: wedgeprops.0,
        text_size: size.map(FontSize::Points),
        text_color: color,
        textprops: TextProps {
            usetex,
            ..TextProps::default()
        },
        center,
        frame,
        rotatelabels,
        normalize,
        hatches,
        shadow: shadow.map(|(ox, oy, shade, props)| PieShadow {
            ox,
            oy,
            shade,
            props: props.0,
        }),
    };
    let pie = axes.pie(values, options)?;
    Ok((pie.wedges, pie.labels, pie.pct_labels))
}
