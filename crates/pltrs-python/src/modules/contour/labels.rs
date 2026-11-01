//! Labelling line contours (matplotlib's `ContourLabeler`): `clabel`,
//! `add_label_near` and the cutting of lines under labels.
//!
//! matplotlib places every label through the overridable method
//! `_split_path_and_get_label_rotation`, so the Python side hands
//! `clabel` and `add_label_near` a callable for it: pltrs (Rust) places
//! the labels and calls it with each label to place, as matplotlib's
//! `labels` loop does, and `split_path` is what the method computes with.

use pltrs::colors::Color;
use pltrs::contour::{ClabelFormat, ClabelOptions, ContourSet, LabelCut};
use pltrs::fonts::FontSize;
use pltrs::text::Text;
use pltrs::ticker::LabelFormat;
use pltrs::transforms::Transform;
use pltrs_core::{Path, Point};

use super::super::path::{PathArrays, PyPath, path_arrays};
use crate::binding::prelude::*;

/// How labels are written: a template or callable, or `{level: text}`
/// pairs, or `None` for matplotlib's default formatter.
fn clabel_format(fmt: Option<Owned>) -> PyResult<ClabelFormat> {
    let Some(fmt) = fmt else {
        return Ok(ClabelFormat::Scalar);
    };
    Python::with_gil(|py| {
        let fmt = fmt.bind(py);
        if let Ok(template) = fmt.extract::<String>() {
            return Ok(ClabelFormat::Label(LabelFormat::Template(template)));
        }
        if let Ok(pairs) = fmt.extract::<Vec<(f64, String)>>() {
            return Ok(ClabelFormat::Map(pairs));
        }
        Ok(ClabelFormat::Label(super::super::archive::label_format(
            &fmt,
        )?))
    })
}

/// Runs `place` with the splitter calling `split`, a Python callable taking
/// `(vertices, codes, index, (x, y), width, spacing)` and returning
/// `(rotation, path)`, and raises what it raised. After a failure the
/// labels left are placed with their lines uncut, and the exception is
/// raised once pltrs (Rust) returns.
fn with_splitter<R>(
    split: &Owned,
    place: impl FnOnce(&mut dyn FnMut(LabelCut<'_>) -> (f64, Path)) -> PyResult<R>,
) -> PyResult<R> {
    let mut failure = None;
    let placed = Python::with_gil(|py| {
        let split = split.bind(py);
        let mut splitter = |cut: LabelCut<'_>| {
            let called = || {
                let (vertices, codes) = path_arrays(cut.path, false)?;
                let args = [
                    vertices.into_py(py)?,
                    codes.into_py(py)?,
                    cut.index.into_py(py)?,
                    (cut.position.x, cut.position.y).into_py(py)?,
                    cut.width.into_py(py)?,
                    cut.spacing.into_py(py)?,
                ];
                let (rotation, path): (f64, PyPath) = split.call(&args, &[])?.extract()?;
                Ok::<_, PyErr>((rotation, path.path))
            };
            match failure.is_none().then(called) {
                Some(Ok(split)) => split,
                Some(Err(err)) => {
                    failure = Some(err);
                    (0.0, cut.path.clone())
                }
                None => (0.0, cut.path.clone()),
            }
        };
        place(&mut splitter)
    });
    match failure {
        Some(err) => Err(err),
        None => placed,
    }
}

/// The label options: `(levels, fontsize, inline, inline_spacing,
/// colors, manual positions)`.
type ClabelLayout = (
    Option<Vec<f64>>,
    Option<f64>,
    bool,
    f64,
    Option<Vec<Color>>,
    Option<Vec<(f64, f64)>>,
);

/// Labels the contour lines (matplotlib's `clabel`): the label texts,
/// each line cut by `split` (see [`with_splitter`]).
fn clabel(
    set: ContourSet,
    (levels, fontsize, inline, inline_spacing, colors, manual): ClabelLayout,
    fmt: Option<Owned>,
    (rightside_up, zorder, use_clabeltext): (bool, Option<f64>, bool),
    split: Owned,
) -> PyResult<Vec<Text>> {
    let options = ClabelOptions {
        levels,
        fontsize: fontsize.map(FontSize::Points),
        inline,
        inline_spacing,
        fmt: clabel_format(fmt)?,
        colors,
        manual,
        rightside_up,
        zorder,
        use_clabeltext,
    };
    with_splitter(&split, |splitter| {
        Ok(set.clabel_split_by(options, Some(splitter))?)
    })
}

/// Labels the labelled line nearest to the display point `(x, y)` the way
/// the last `clabel` labelled the set, cutting the line under the label
/// when `inline` (matplotlib's `add_label_near`), the line cut by `split`
/// (see [`with_splitter`]).
fn add_label_near(
    set: ContourSet,
    (x, y): (f64, f64),
    inline: bool,
    inline_spacing: f64,
    split: Owned,
) -> PyResult<Text> {
    with_splitter(&split, |splitter| {
        Ok(set.add_label_near_split_by(
            Point::new(x, y),
            &Transform::Display,
            (inline, inline_spacing),
            Some(splitter),
        )?)
    })
}

/// The rotation of a label of `width` pixels at display position `(x, y)`
/// on `path`, after its vertex `index`, and the path cut around it leaving
/// `spacing` pixels on each side, as pltrs (Rust) places the labels of
/// the last `clabel` (matplotlib's `_split_path_and_get_label_rotation`).
fn split_path(
    set: ContourSet,
    path: PyPath,
    index: usize,
    (x, y): (f64, f64),
    (width, spacing): (f64, f64),
) -> PyResult<(f64, PathArrays)> {
    let (rotation, cut) = set.split_path_at_label(LabelCut {
        path: &path.path,
        index,
        position: Point::new(x, y),
        width,
        spacing,
    })?;
    Ok((rotation, path_arrays(&cut, false)?))
}

/// The texts of the set's labels.
fn label_texts(set: ContourSet) -> PyResult<Vec<Text>> {
    Ok(set.labels().into_iter().map(|label| label.artist).collect())
}

/// Removes the label at `index` among the set's labels, with its text
/// (matplotlib's `pop_label`; Python checks the index).
fn pop_label(set: ContourSet, index: usize) -> PyResult<()> {
    Ok(set.pop_label(Some(index))?)
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("clabel", clabel)?;
    module.add("add_label_near", add_label_near)?;
    module.add("split_path", split_path)?;
    module.add("label_texts", label_texts)?;
    module.add("pop_label", pop_label)
}
