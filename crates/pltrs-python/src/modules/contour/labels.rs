//! Labelling line contours (matplotlib's `ContourLabeler`): `clabel`,
//! `labels`, `add_label_near`, `add_label`, the placement rules
//! (`print_label`, `too_close`, `locate_label`) and the cutting of lines
//! under labels.
//!
//! matplotlib places every label through the overridable method
//! `_split_path_and_get_label_rotation`, so the Python side hands
//! `clabel` and `add_label_near` a callable for it: pltrs (Rust) places
//! the labels and calls it with each label to place, as matplotlib's
//! `labels` loop does, and `split_path` is what the method computes with.

use pltrs::colors::Color;
use pltrs::contour::{
    ClabelFormat, ClabelOptions, ContourSet, LabelCut, locate_label, print_label, too_close,
};
use pltrs::fonts::FontSize;
use pltrs::text::Text;
use pltrs::ticker::LabelFormat;
use pltrs::transforms::Transform;
use pltrs_core::{Path, Point};

use super::super::path::{PathArrays, PyPath, path_arrays};
use super::super::transforms::points;
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

/// Places labels along every labelled line again, the way the last
/// `clabel` labelled the set (matplotlib's `labels`), each line cut by
/// `split` (see [`with_splitter`]).
fn place_labels(
    set: ContourSet,
    inline: bool,
    inline_spacing: f64,
    split: Owned,
) -> PyResult<Vec<Text>> {
    with_splitter(&split, |splitter| {
        Ok(set.place_labels_split_by(inline, inline_spacing, Some(splitter))?)
    })
}

/// Adds a label of the level at index `level` centred at the display
/// point `(x, y)`, turned by `rotation` degrees (matplotlib's
/// `add_label`).
fn add_label(set: ContourSet, (x, y): (f64, f64), rotation: f64, level: usize) -> PyResult<Text> {
    Ok(set.add_label(Point::new(x, y), &Transform::Display, rotation, level)?)
}

/// The display positions of the set's labels (matplotlib's `labelXYs`).
fn label_positions(set: &ContourSet) -> Vec<Point> {
    set.labels().iter().map(|label| label.position).collect()
}

/// The display positions `(x, y)` of the set's labels.
fn get_label_positions(set: ContourSet) -> PyResult<Vec<(f64, f64)>> {
    Ok(label_positions(&set).iter().map(|p| (p.x, p.y)).collect())
}

/// Whether the `(N, 2)` display line is long enough to hold a label
/// `width` pixels wide (`print_label`).
fn label_fits(line: NdArray, width: f64) -> PyResult<bool> {
    Ok(print_label(&points(&line)?, width))
}

/// Whether the display point `(x, y)` is within `1.2 * width` of a label
/// of the set (`too_close`).
fn label_too_close(set: ContourSet, (x, y): (f64, f64), width: f64) -> PyResult<bool> {
    Ok(too_close(Point::new(x, y), width, &label_positions(&set)))
}

/// Where a label `width` pixels wide goes on the `(N, 2)` display line,
/// away from the set's labels: `(x, y, index)` (`locate_label`).
fn locate(set: ContourSet, line: NdArray, width: f64) -> PyResult<(f64, f64, usize)> {
    let line = points(&line)?;
    if line.is_empty() {
        return Err(PyErr::value_error(
            "a label needs a line of at least one vertex",
        ));
    }
    let (at, index) = locate_label(&line, width, &label_positions(&set));
    Ok((at.x, at.y, index))
}

/// The levels last labelled (`labelLevelList`).
fn label_levels(set: ContourSet) -> PyResult<Vec<f64>> {
    Ok(set.labelled_levels())
}

/// The level of each of the set's labels, in order.
fn label_text_levels(set: ContourSet) -> PyResult<Vec<f64>> {
    Ok(set.labels().iter().map(|label| label.level).collect())
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
    module.add("place_labels", place_labels)?;
    module.add("label_levels", label_levels)?;
    module.add("label_text_levels", label_text_levels)?;
    module.add("add_label", add_label)?;
    module.add("label_positions", get_label_positions)?;
    module.add("print_label", label_fits)?;
    module.add("too_close", label_too_close)?;
    module.add("locate_label", locate)?;
    module.add("pop_label", pop_label)
}
