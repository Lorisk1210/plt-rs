//! `_native.archive`: what pickling rests on (`pltrs/_pickling.py`).
//!
//! A figure, or the holder of an artist made outside any figure, is
//! archived whole by pltrs (Rust), artist ids included
//! ([`RawHandle::archive_figure`]); a handle unpickles as the handle of
//! its id in the figure read back. A native value (a transform, a grid,
//! a colorbar, a container, rcParams) is archived on its own.
//!
//! What an archive cannot hold, it refers to as *externals*, which cross
//! as `(role, object)` pairs the package pickles beside the bytes: the
//! Python objects pltrs (Rust) calls (formatters, norms and transforms of
//! Python classes, role `None`), the callables the binding made pltrs
//! (Rust) functions of (role: how it calls them, see [`Call`]) and the
//! figures that handles inside a value reach (role `"figure"`, the handle
//! of the figure's root). The binding's own types behind pltrs (Rust)'s
//! traits are archived by [`PythonProgram`]: a Python tick helper as the
//! helper (its adapter holds it weakly), which the package registers again
//! with its figure once the figure is read back (`_tick_helpers`).

use std::any::{Any, TypeId};
use std::rc::Rc;

use pltrs::artist::{Artist, ArtistBase, ArtistHandle, ArtistId, FigureRef, RawHandle};
use pltrs::axes_grid1::Divider;
use pltrs::colorbar::Colorbar;
use pltrs::colors::Norm;
use pltrs::container::Container;
use pltrs::figure::Figure;
use pltrs::gridspec::{GridSpec, SubplotSpec};
use pltrs::legend::LegendHandle;
use pltrs::rc::RcParams;
use pltrs::ticker::{FuncFormatter, LabelFormat};
use pltrs::transforms::{PointMap, Transform};
use pltrs_core::archive::{
    self, Archive, ArchiveError, Decoder, Encoder, External, KindOf, ProgramKinds,
};

use super::artist::PlainArtist;
use super::axis::{PyFormatter, PyLocator};
use super::colors::PythonNorm;
use super::transforms::PyPointMap;
use crate::binding::lookup;
use crate::binding::prelude::*;

impl From<ArchiveError> for PyErr {
    /// matplotlib's `TypeError` for what cannot be pickled, `ValueError`
    /// for a pickle that cannot be read.
    fn from(err: ArchiveError) -> Self {
        match err {
            ArchiveError::Unsupported(what) => PyErr::type_error(format!("cannot pickle {what}")),
            other => PyErr::value_error(format!("cannot unpickle: {other}")),
        }
    }
}

/// How the binding calls a Python callable it made a pltrs (Rust)
/// function of, which is how it makes the function again from the
/// callable when an archive is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Call {
    /// `f(value, position)`, the labels of a `FuncFormatter`.
    Ticks,
    /// `f(value)` as a formatter of coordinates (`fmt_xdata`).
    Coordinates,
    /// `f(value)` as a label format (`bar_label`, `clabel`, `pie`'s
    /// `autopct`, `Sankey`).
    Values,
}

impl Call {
    /// Every way of calling.
    const ALL: [Self; 3] = [Self::Ticks, Self::Coordinates, Self::Values];

    /// The name of the way of calling, the role of its externals.
    fn name(self) -> &'static str {
        match self {
            Self::Ticks => "tick-labels",
            Self::Coordinates => "data-labels",
            Self::Values => "value-labels",
        }
    }

    /// The function calling `callback` this way, as the `Rc` pltrs (Rust)
    /// keeps it in (boxed), and the `TypeId` of that `Rc`.
    fn function(self, callback: Callback) -> (TypeId, Box<dyn Any>) {
        type Labels = Rc<dyn Fn(f64, Option<usize>) -> String>;
        type Label = Rc<dyn Fn(f64) -> String>;
        match self {
            Self::Ticks => {
                let function: Labels = Rc::new(tick_labels(callback));
                (TypeId::of::<Labels>(), Box::new(function))
            }
            Self::Coordinates => {
                let function: Labels = Rc::new(data_labels(callback));
                (TypeId::of::<Labels>(), Box::new(function))
            }
            Self::Values => {
                let function: Label = Rc::new(value_label(callback));
                (TypeId::of::<Label>(), Box::new(function))
            }
        }
    }
}

/// `f(value, position)`, empty when it raised.
fn tick_labels(callback: Callback) -> impl Fn(f64, Option<usize>) -> String + Send + 'static {
    move |value, position| {
        callback
            .call::<String>((value, position))
            .unwrap_or_default()
    }
}

/// `f(value)` for any position, empty when it raised.
fn data_labels(callback: Callback) -> impl Fn(f64, Option<usize>) -> String + Send + 'static {
    move |value, _| callback.call::<String>((value,)).unwrap_or_default()
}

/// `f(value)`, empty when it raised.
fn value_label(callback: Callback) -> impl Fn(f64) -> String + Send + 'static {
    move |value| callback.call::<String>((value,)).unwrap_or_default()
}

/// A Python object an archive refers to, and for a callable the binding
/// made a function of, how it calls it.
struct PythonExternal {
    object: Owned,
    call: Option<Call>,
}

/// The external of `object`.
fn object_external(object: &Owned) -> External {
    Python::with_gil(|py| {
        External::new(PythonExternal {
            object: object.clone_ref(py),
            call: None,
        })
    })
}

/// The external of `callable`, called as `call`.
fn function_external(callable: &Bound<'_>, call: Call) -> External {
    External::new(PythonExternal {
        object: callable.clone().unbind(),
        call: Some(call),
    })
}

/// The formatter labelling ticks with `callable(value, position)`
/// (matplotlib's `FuncFormatter`), archived as the callable.
pub(crate) fn tick_formatter(callable: &Bound<'_>) -> PyResult<FuncFormatter> {
    let callback: Callback = callable.extract()?;
    Ok(FuncFormatter::with_origin(
        tick_labels(callback),
        function_external(callable, Call::Ticks),
    ))
}

/// The formatter of coordinates `callable(value)` (matplotlib's
/// `fmt_xdata`), archived as the callable.
pub(crate) fn data_formatter(callable: &Bound<'_>) -> PyResult<FuncFormatter> {
    let callback: Callback = callable.extract()?;
    Ok(FuncFormatter::with_origin(
        data_labels(callback),
        function_external(callable, Call::Coordinates),
    ))
}

/// The labels `callable(value)` writes, archived as the callable.
pub(crate) fn label_format(callable: &Bound<'_>) -> PyResult<LabelFormat> {
    let callback: Callback = callable.extract()?;
    Ok(LabelFormat::func_with_origin(
        value_label(callback),
        function_external(callable, Call::Values),
    ))
}

/// An external as the `(role, object)` pair the package pickles.
type PythonPair = (Option<String>, Owned);

/// The externals of an archive as the package pickles them.
fn externals_to_python(externals: &[External]) -> PyResult<Vec<PythonPair>> {
    Python::with_gil(|py| {
        externals
            .iter()
            .map(|external| {
                if let Some(python) = external.downcast_ref::<PythonExternal>() {
                    let role = python.call.map(|call| call.name().to_owned());
                    return Ok((role, python.object.clone_ref(py)));
                }
                if let Some(figure) = external.downcast_ref::<FigureRef>() {
                    let root = figure.handle(ArtistId::from_index(0));
                    return Ok((Some("figure".to_owned()), root.into_py(py)?.unbind()));
                }
                Err(PyErr::type_error(
                    "cannot pickle an external of an unknown kind",
                ))
            })
            .collect()
    })
}

/// The externals of an archive from the pairs the package unpickled.
fn externals_from_python(pairs: Vec<PythonPair>) -> PyResult<Vec<External>> {
    Python::with_gil(|py| {
        pairs
            .into_iter()
            .map(|(role, object)| match role.as_deref() {
                None => Ok(External::new(PythonExternal { object, call: None })),
                Some("figure") => {
                    let root: RawHandle = object.bind(py).extract()?;
                    Ok(External::new(FigureRef::of(&root)))
                }
                Some(name) => Call::ALL
                    .into_iter()
                    .find(|call| call.name() == name)
                    .map(|call| function_external(&object.bind(py), call))
                    .ok_or_else(|| {
                        PyErr::value_error(format!("cannot unpickle: no role {name:?}"))
                    }),
            })
            .collect()
    })
}

/// How the binding archives its own types behind pltrs (Rust)'s traits:
/// a Python tick helper, norm or transform as the Python object, the bare
/// artist of `Artist()` as its common properties; and how it makes the
/// functions of [`Call`]s again.
struct PythonProgram;

/// The tags of the binding's kinds.
const PLAIN_ARTIST: &str = "plain-artist";
const LOCATOR: &str = "python-locator";
const FORMATTER: &str = "python-formatter";
const NORM: &str = "python-norm";
const POINT_MAP: &str = "python-transform";

/// An exception raised while reading an archive, as an archive error.
fn read_failure(py: Python<'_>, err: &PyErr) -> ArchiveError {
    ArchiveError::invalid(err.message(py))
}

/// The Python tick helper behind the adapter `adapter` (`helper()`).
fn tick_helper(adapter: &Owned) -> Result<Owned, ArchiveError> {
    Python::with_gil(|py| {
        let helper = adapter
            .bind(py)
            .call_method("helper", &[])
            .map_err(|err| read_failure(py, &err))?;
        if helper.is_none() {
            return Err(ArchiveError::unsupported("a tick helper no longer alive"));
        }
        Ok(helper.unbind())
    })
}

/// The adapter of the package calling the tick helper `helper`, a
/// `"locator"` or a `"formatter"` (`_tick_helpers._adapter`).
fn tick_adapter<'py>(helper: &Bound<'py>, kind: &str) -> PyResult<Bound<'py>> {
    let py = helper.py();
    lookup(py, "tick_helper_adapter")?.call(&[kind.into_py(py)?, helper.clone()], &[])
}

impl ProgramKinds for PythonProgram {
    fn encode_kind(
        &self,
        of: KindOf,
        value: &dyn Any,
        encoder: &mut Encoder<'_>,
    ) -> Result<(), ArchiveError> {
        if let Some(artist) = value.downcast_ref::<PlainArtist>() {
            encoder.write_str(PLAIN_ARTIST);
            return artist.base.encode(encoder);
        }
        let (tag, object) = if let Some(locator) = value.downcast_ref::<PyLocator>() {
            (LOCATOR, tick_helper(&locator.adapter)?)
        } else if let Some(formatter) = value.downcast_ref::<PyFormatter>() {
            (FORMATTER, tick_helper(&formatter.adapter)?)
        } else if let Some(norm) = value.downcast_ref::<PythonNorm>() {
            (NORM, Python::with_gil(|py| norm.object().clone_ref(py)))
        } else if let Some(map) = value.downcast_ref::<PyPointMap>() {
            (POINT_MAP, Python::with_gil(|py| map.object().clone_ref(py)))
        } else {
            return Err(ArchiveError::unsupported(format!(
                "a {} of a type of the program's",
                of.name
            )));
        };
        encoder.write_str(tag);
        encoder.write_external(&object_external(&object));
        Ok(())
    }

    fn decode_kind(
        &self,
        of: KindOf,
        decoder: &mut Decoder<'_>,
    ) -> Result<Box<dyn Any>, ArchiveError> {
        let tag = decoder.read_str()?;
        if tag == PLAIN_ARTIST {
            let artist: Box<dyn Artist> = Box::new(PlainArtist {
                base: ArtistBase::decode(decoder)?,
            });
            return Ok(Box::new(artist));
        }
        let external = decoder.read_external()?;
        let python = external
            .downcast_ref::<PythonExternal>()
            .ok_or_else(|| ArchiveError::invalid("a Python kind without its object"))?;
        Python::with_gil(|py| {
            let object = python.object.bind(py);
            let made: PyResult<Box<dyn Any>> = match tag.as_str() {
                LOCATOR => tick_adapter(&object, "locator")
                    .and_then(|adapter| PyLocator::of(&adapter))
                    .map(|locator| {
                        let locator: Box<dyn pltrs::ticker::Locator> = Box::new(locator);
                        Box::new(locator) as Box<dyn Any>
                    }),
                FORMATTER => tick_adapter(&object, "formatter")
                    .and_then(|adapter| PyFormatter::of(&adapter))
                    .map(|formatter| {
                        let formatter: Box<dyn pltrs::ticker::Formatter> = Box::new(formatter);
                        Box::new(formatter) as Box<dyn Any>
                    }),
                NORM => PythonNorm::of(&object).map(|norm| {
                    let norm: Box<dyn Norm> = Box::new(norm);
                    Box::new(norm) as Box<dyn Any>
                }),
                POINT_MAP => PyPointMap::new(&object).map(|map| {
                    let map: Box<dyn PointMap> = Box::new(map);
                    Box::new(map) as Box<dyn Any>
                }),
                _ => {
                    return Err(ArchiveError::invalid(format!(
                        "no {} of the binding's called {tag:?}",
                        of.name
                    )));
                }
            };
            made.map_err(|err| read_failure(py, &err))
        })
    }

    fn function(&self, function: TypeId, origin: &External) -> Result<Box<dyn Any>, ArchiveError> {
        let python = origin
            .downcast_ref::<PythonExternal>()
            .ok_or_else(|| ArchiveError::invalid("a function without its callable"))?;
        let call = python
            .call
            .ok_or_else(|| ArchiveError::invalid("a callable without its way of calling"))?;
        let callback: Callback = Python::with_gil(|py| {
            python
                .object
                .bind(py)
                .extract()
                .map_err(|err| read_failure(py, &err))
        })?;
        let (made, value) = call.function(callback);
        if made == function {
            Ok(value)
        } else {
            Err(ArchiveError::invalid(format!(
                "a callable called as {} where pltrs takes another function",
                call.name()
            )))
        }
    }
}

/// A number telling the figure `artist` is in apart from every other
/// figure alive (the package keeps one pickling holder per figure).
fn figure_key(artist: RawHandle) -> PyResult<usize> {
    Ok(artist.figure_key())
}

/// Archives the figure `artist` is in, or the holder of the loose artist:
/// `(bytes, externals)`.
fn archive_figure(artist: RawHandle) -> PyResult<(Bytes, Vec<PythonPair>)> {
    let (bytes, externals) = artist.archive_figure(&PythonProgram)?;
    Ok((Bytes(bytes), externals_to_python(&externals)?))
}

/// Replaces the whole state of the figure `artist` is in with the figure
/// archived in `bytes` (with its `externals`), as unpickling a figure
/// does: the handles of the figure, made before, reach the artists read
/// back.
fn restore_figure(artist: RawHandle, bytes: Bytes, externals: Vec<PythonPair>) -> PyResult<()> {
    let externals = externals_from_python(externals)?;
    Ok(artist.restore_figure(&bytes.0, &externals, &PythonProgram)?)
}

/// A new figure to restore an archived one into: the handle of its root.
fn new_figure() -> PyResult<RawHandle> {
    Ok(Figure::with_size(1.0, 1.0, 72.0).raw().clone())
}

/// The handle of artist `id` in the figure `artist` is in.
fn handle(artist: RawHandle, id: u32) -> PyResult<RawHandle> {
    Ok(artist.sibling(ArtistId::from_index(id)))
}

/// Whether `object` is the native object of an artist's handle.
fn is_handle(object: Owned) -> PyResult<bool> {
    Ok(Python::with_gil(|py| {
        object.bind(py).extract::<RawHandle>().is_ok()
    }))
}

/// The native values archived on their own, each under the kind
/// [`archive_value`] gives and [`restore_value`] takes.
macro_rules! native_values {
    ($($kind:literal => $ty:ty),* $(,)?) => {
        /// Archives the native value `object`: `(kind, bytes, externals)`.
        fn archive_value(object: Owned) -> PyResult<(&'static str, Bytes, Vec<PythonPair>)> {
            let object = Python::with_gil(|py| object.bind(py).clone().unbind());
            $(if let Ok(value) = Python::with_gil(|py| object.bind(py).extract::<$ty>()) {
                let (bytes, externals) = archive::to_bytes(&value, &PythonProgram)?;
                return Ok(($kind, Bytes(bytes), externals_to_python(&externals)?));
            })*
            let repr = Python::with_gil(|py| object.bind(py).repr())?;
            Err(PyErr::type_error(format!("cannot pickle {repr}")))
        }

        /// The native value archived as `kind` in `bytes`, with its
        /// `externals`.
        fn restore_value(kind: String, bytes: Bytes, externals: Vec<PythonPair>) -> PyResult<Owned> {
            let externals = externals_from_python(externals)?;
            $(if kind == $kind {
                let value: $ty = archive::from_bytes(&bytes.0, &externals, &PythonProgram)?;
                return Python::with_gil(|py| Ok(value.into_py(py)?.unbind()));
            })*
            Err(PyErr::value_error(format!("cannot unpickle a native {kind:?}")))
        }
    };
}

native_values! {
    "transform" => Transform,
    "gridspec" => GridSpec,
    "subplotspec" => SubplotSpec,
    "colorbar" => Colorbar,
    "divider" => Divider,
    "container" => Container,
    "legend-handle" => LegendHandle,
    "rcparams" => RcParams,
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("figure_key", figure_key)?;
    module.add("archive_figure", archive_figure)?;
    module.add("restore_figure", restore_figure)?;
    module.add("new_figure", new_figure)?;
    module.add("handle", handle)?;
    module.add("is_handle", is_handle)?;
    module.add("archive_value", archive_value)?;
    module.add("restore_value", restore_value)
}
