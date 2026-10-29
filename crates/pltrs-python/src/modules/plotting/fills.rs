//! Filled areas: `fill`, `fill_between`, `fill_betweenx`, `stairs` and
//! `stackplot`.

use pltrs::axes::{Axes, Baseline, FillBetweenOptions, StackplotOptions, StairsOptions};
use pltrs::collections::Collection;
use pltrs::colors::Color;
use pltrs::lines::StepWhere;
use pltrs::orientation::Orientation;
use pltrs::patches::{Patch, PatchProps, Polygon, StepBaseline, StepPatch};

use super::values::PyAxisValues;
use crate::binding::prelude::*;

/// The orientation `"vertical"` or `"horizontal"`.
pub(super) fn orientation(name: &str) -> Orientation {
    match name {
        "horizontal" => Orientation::Horizontal,
        _ => Orientation::Vertical,
    }
}

/// A filled polygon through `(x, y)` (one dataset of matplotlib's
/// `fill`), in `color` or the next colour of the patch cycle.
pub(super) fn fill(
    axes: Axes,
    x: PyAxisValues,
    y: PyAxisValues,
    color: Option<Color>,
    closed: bool,
) -> PyResult<Patch<Polygon>> {
    let props = PatchProps {
        facecolor: color,
        ..PatchProps::default()
    };
    let polygon = axes.fill(x.0, y.0, props)?;
    if !closed {
        let shape = polygon.shape();
        polygon.set_shape(Polygon::new(shape.vertices().to_vec(), false));
    }
    Ok(polygon)
}

/// The area between `y1` and `y2` along `t` (matplotlib's
/// `fill_between`, or `fill_betweenx` when `horizontal`): where to fill
/// (`None` everywhere), whether to interpolate the crossings, the step
/// placement (`None`, `"pre"`, `"mid"` or `"post"`), and the face colour
/// (`None` for the next of the patch cycle).
pub(super) fn fill_between(
    axes: Axes,
    horizontal: bool,
    (t, y1, y2): (PyAxisValues, PyAxisValues, PyAxisValues),
    mask: Option<Vec<bool>>,
    interpolate: bool,
    step: Option<String>,
    color: Option<Color>,
) -> PyResult<Collection> {
    let mut options = FillBetweenOptions {
        mask,
        interpolate,
        step: step.map(|name| match name.as_str() {
            "mid" => StepWhere::Mid,
            "post" => StepWhere::Post,
            _ => StepWhere::Pre,
        }),
        ..FillBetweenOptions::default()
    };
    options.props.color = color;
    Ok(if horizontal {
        axes.fill_betweenx(t.0, y1.0, y2.0, options)?
    } else {
        axes.fill_between(t.0, y1.0, y2.0, options)?
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
/// `colors` (cycled; empty for the property cycle).
pub(super) fn stackplot(
    axes: Axes,
    x: PyAxisValues,
    ys: Vec<Vec<f64>>,
    baseline: String,
    colors: Vec<Color>,
) -> PyResult<Vec<Collection>> {
    let options = StackplotOptions {
        baseline: match baseline.as_str() {
            "sym" => Baseline::Sym,
            "wiggle" => Baseline::Wiggle,
            "weighted_wiggle" => Baseline::WeightedWiggle,
            _ => Baseline::Zero,
        },
        colors,
        ..StackplotOptions::default()
    };
    Ok(axes.stackplot(x.0, &ys, options)?)
}
