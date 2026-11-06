//! `_native.sankey`: Sankey diagrams (matplotlib's `sankey`), built
//! subdiagram by subdiagram on an axes.
//!
//! The diagram under construction is a native object; the Python
//! `Sankey` checks the arguments as matplotlib does and keeps the
//! subdiagrams it is handed back.

use pltrs::axes::Axes;
use pltrs::patches::{Patch, PathShape};
use pltrs::sankey::{FlowOrientation, PathLengths, Sankey, SankeyError, SankeyOptions, Subdiagram};
use pltrs::text::Text;
use pltrs::ticker::LabelFormat;

use super::patches::PatchKwargs;
use crate::binding::prelude::*;
use crate::binding::wrap_native;

impl From<SankeyError> for PyErr {
    /// matplotlib's `ValueError` for diagrams it cannot lay out, and the
    /// axes' own for limits they refuse.
    fn from(err: SankeyError) -> Self {
        match err {
            SankeyError::Limits(err) => err.into(),
            other => PyErr::value_error(other.to_string()),
        }
    }
}

/// A diagram under construction, for Python.
struct NativeSankey(Sankey);

impl IntoPy for NativeSankey {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        wrap_native(py, self)
    }
}

/// How quantities are labelled: a `%`-style template or a function.
fn label_format(format: &Bound<'_>) -> PyResult<LabelFormat> {
    if let Ok(template) = format.extract::<String>() {
        return Ok(LabelFormat::Template(template));
    }
    super::archive::label_format(format)
}

/// The labels: `(scale, unit, format)`.
type Labels = (f64, Option<String>, Owned);

/// The geometry: `(gap, radius, shoulder, offset, head_angle, margin,
/// tolerance)`.
type Geometry = (f64, f64, f64, f64, f64, f64, f64);

/// An empty diagram on `axes` (matplotlib's `Sankey`).
fn new(
    axes: Axes,
    (scale, unit, format): Labels,
    (gap, radius, shoulder, offset, head_angle, margin, tolerance): Geometry,
) -> PyResult<NativeSankey> {
    let format = Python::with_gil(|py| label_format(&format.bind(py)))?;
    let options = SankeyOptions {
        scale,
        unit,
        format,
        gap,
        radius,
        shoulder,
        offset,
        head_angle,
        margin,
        tolerance,
    };
    Ok(NativeSankey(Sankey::new(&axes, options)?))
}

/// The flows of a subdiagram: `(flows, orientations (-1, 0 or 1),
/// labels)`.
type Flows = (Vec<f64>, Vec<i64>, Vec<Option<String>>);

/// The layout of a subdiagram: `(trunklength, pathlengths, prior,
/// connect, rotation)`; one path length is that of the inner flows.
type Layout = (f64, Vec<f64>, Option<usize>, (usize, usize), f64);

/// A subdiagram as Python keeps it: `(patch, flows, angles in quarter
/// turns, tips, text, texts)`.
type Diagram = (
    Patch<PathShape>,
    Vec<f64>,
    Vec<Option<f64>>,
    NdArray,
    Text,
    Vec<Text>,
);

/// Adds a subdiagram labelled `patchlabel` (matplotlib's `Sankey.add`),
/// its outline styled as `style` says (the face colour and line width
/// `add` takes; the next colour of the property cycle without a face
/// colour).
fn add(
    sankey: Native<NativeSankey>,
    patchlabel: String,
    (flows, orientations, labels): Flows,
    (trunklength, pathlengths, prior, connect, rotation): Layout,
    style: PatchKwargs,
) -> PyResult<Diagram> {
    let sub = Subdiagram {
        patchlabel,
        flows,
        orientations: orientations
            .into_iter()
            .map(|orientation| match orientation {
                1 => FlowOrientation::Top,
                -1 => FlowOrientation::Bottom,
                _ => FlowOrientation::Side,
            })
            .collect(),
        labels,
        trunklength,
        pathlengths: match pathlengths[..] {
            [inner] => PathLengths::Inner(inner),
            _ => PathLengths::PerFlow(pathlengths),
        },
        prior,
        connect,
        rotation,
        props: style.0,
    };
    let mut sankey = sankey.borrow_mut();
    let diagram = sankey
        .0
        .add(sub)?
        .diagrams()
        .last()
        .expect("a subdiagram was just added")
        .clone();
    let tips = diagram.tips.iter().flat_map(|p| [p.x, p.y]).collect();
    Ok((
        diagram.patch,
        diagram.flows,
        diagram.angles,
        NdArray::with_shape(tips, vec![diagram.tips.len(), 2])?,
        diagram.text,
        diagram.texts,
    ))
}

/// Fits the view around the diagram (matplotlib's `Sankey.finish`).
fn finish(sankey: Native<NativeSankey>) -> PyResult<()> {
    sankey.borrow().0.finish()?;
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("new", new)?;
    module.add("add", add)?;
    module.add("finish", finish)
}
