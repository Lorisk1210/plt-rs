//! Filled areas: `fill`, `fill_between`, `fill_betweenx`, `stairs` and
//! `stackplot`.

use pltrs::axes::{Axes, Baseline, FillBetweenOptions, StackplotOptions, StairsOptions};
use pltrs::collections::Collection;
use pltrs::colors::Color;
use pltrs::lines::StepWhere;
use pltrs::orientation::Orientation;
use pltrs::patches::{Patch, Polygon, StepBaseline, StepPatch};
use pltrs::transforms::Transform;

use super::super::patches::PatchKwargs;
use crate::binding::prelude::*;

/// The orientation `"vertical"` or `"horizontal"`.
pub(super) fn orientation(name: &str) -> Orientation {
    match name {
        "horizontal" => Orientation::Horizontal,
        _ => Orientation::Vertical,
    }
}

/// A filled polygon through `(x, y)` (one dataset of matplotlib's
/// `fill`), in the colour of its format string or the next colour of the
/// patch cycle, with the patch properties `given` (aliases resolved),
/// which the cycle does not set (nor advance for, when it cycles nothing
/// else).
pub(super) fn fill(
    axes: Axes,
    x: NdArray,
    y: NdArray,
    (format_color, closed): (Option<Color>, bool),
    given: PatchKwargs,
) -> PyResult<Patch<Polygon>> {
    let polygon =
        axes.fill_with_format_color(x.into_vec()?, y.into_vec()?, format_color, given.0)?;
    if !closed {
        let shape = polygon.shape();
        polygon.set_shape(Polygon::new(shape.vertices().to_vec(), false));
    }
    Ok(polygon)
}

/// The options of a fill between curves: where to fill (`None`
/// everywhere), whether to interpolate the crossings, and the step
/// placement (`None`, `"pre"`, `"mid"` or `"post"`).
pub(in crate::modules) fn fill_between_options(
    mask: Option<Vec<bool>>,
    interpolate: bool,
    step: Option<&str>,
) -> FillBetweenOptions {
    FillBetweenOptions {
        mask,
        interpolate,
        step: step.map(|name| match name {
            "mid" => StepWhere::Mid,
            "post" => StepWhere::Post,
            _ => StepWhere::Pre,
        }),
        ..FillBetweenOptions::default()
    }
}

/// The area between `y1` and `y2` along `t` (matplotlib's
/// `fill_between`, or `fill_betweenx` when `horizontal`): where to fill
/// (`None` everywhere), whether to interpolate the crossings, the step
/// placement (`None`, `"pre"`, `"mid"` or `"post"`), the `color` (face
/// and edges) and `facecolor` given (the next colour of the patch cycle
/// for the faces without either), and the coordinates of the curves
/// (`None`: the data coordinates; only the directions that are data
/// values extend the data limits).
pub(super) fn fill_between(
    axes: Axes,
    horizontal: bool,
    (t, y1, y2): (NdArray, NdArray, NdArray),
    (mask, interpolate, step): (Option<Vec<bool>>, bool, Option<String>),
    (color, facecolor): (Option<Color>, Option<Color>),
    transform: Option<Transform>,
) -> PyResult<Collection> {
    let mut options = fill_between_options(mask, interpolate, step.as_deref());
    options.props.color = color;
    options.props.facecolors = facecolor.map(|face| vec![face]);
    options.props.base.transform = transform;
    Ok(if horizontal {
        axes.fill_betweenx(t.into_vec()?, y1.into_vec()?, y2.into_vec()?, options)?
    } else {
        axes.fill_between(t.into_vec()?, y1.into_vec()?, y2.into_vec()?, options)?
    })
}

/// A stepwise constant function (matplotlib's `stairs`): the values and
/// edges (`None` for `0..=n`), the baseline (`None`, one value or one per
/// step), the orientation, whether it is filled, and its colour.
pub(super) fn stairs(
    axes: Axes,
    values: NdArray,
    edges: Option<NdArray>,
    baseline: Option<NdArray>,
    (orient, fill): (String, bool),
    color: Option<Color>,
) -> PyResult<Patch<StepPatch>> {
    let options = StairsOptions {
        edges: edges.map(NdArray::into_vec).transpose()?,
        baseline: match baseline {
            None => StepBaseline::None,
            Some(base) if base.shape.is_empty() => StepBaseline::Value(base.data[0]),
            Some(base) => StepBaseline::PerStep(base.into_vec()?),
        },
        orientation: orientation(&orient),
        fill,
        color,
    };
    Ok(axes.stairs(values.into_vec()?, options)?)
}

/// Stacked areas of `ys` along `x` (matplotlib's `stackplot`), from the
/// baseline `"zero"`, `"sym"`, `"wiggle"` or `"weighted_wiggle"`, in
/// `colors` (cycled; empty for the property cycle), each layer filled
/// as `fill_between` fills with where to fill (`None` everywhere), whether
/// to interpolate the crossings and the step placement.
pub(super) fn stackplot(
    axes: Axes,
    x: NdArray,
    ys: Vec<NdArray>,
    baseline: String,
    colors: Vec<Color>,
    (mask, interpolate, step): (Option<Vec<bool>>, bool, Option<String>),
) -> PyResult<Vec<Collection>> {
    let ys = ys
        .into_iter()
        .map(NdArray::into_vec)
        .collect::<PyResult<Vec<_>>>()?;
    let fill = fill_between_options(mask, interpolate, step.as_deref());
    let options = StackplotOptions {
        mask: fill.mask,
        interpolate: fill.interpolate,
        step: fill.step,
        baseline: match baseline.as_str() {
            "sym" => Baseline::Sym,
            "wiggle" => Baseline::Wiggle,
            "weighted_wiggle" => Baseline::WeightedWiggle,
            _ => Baseline::Zero,
        },
        colors,
        ..StackplotOptions::default()
    };
    Ok(axes.stackplot(x.into_vec()?, &ys, options)?)
}
