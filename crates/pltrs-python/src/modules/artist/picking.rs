//! Picking (matplotlib's `Artist.set_picker`, `Artist.contains` and
//! `Artist.pick`).
//!
//! pltrs (Rust) keeps the pickers it can decide with: `True` (the
//! artist's own hit test) and a tolerance. A Python callable stays with
//! the Python artist, which names the artists it makes pickable that way
//! when it asks for the order of a pick ([`pick_order`]).

use pltrs::artist::{ArtistHandle, Picker, RawHandle};
use pltrs::axes::Axes;
use pltrs::figure::Figure;
use pltrs_core::Point;

use crate::binding::prelude::*;

/// A picker pltrs (Rust) decides with: `True` or a tolerance (`False`
/// and `None` make the artist unpickable).
pub(crate) struct PyPicker(pub(crate) Option<Picker>);

impl FromPy<'_> for PyPicker {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if object.is_none() {
            return Ok(Self(None));
        }
        if object.type_name() == "bool" {
            return Ok(Self(object.is_truthy()?.then_some(Picker::Default)));
        }
        Ok(Self(Some(Picker::Radius(object.extract()?))))
    }
}

/// Sets the picker of `artist` (`None`, `True` or a tolerance).
fn set_picker(artist: RawHandle, picker: PyPicker) -> PyResult<()> {
    artist.set_picker(picker.0);
    Ok(())
}

/// The picker of `artist` as matplotlib's `get_picker` gives it: `None`,
/// `True` or the tolerance.
fn get_picker(artist: RawHandle) -> PyResult<Owned> {
    Python::with_gil(|py| {
        let picker = match artist.picker() {
            None => Bound::none(py),
            Some(Picker::Radius(radius)) => radius.into_py(py)?,
            Some(Picker::Default | Picker::Custom(_)) => true.into_py(py)?,
        };
        Ok(picker.unbind())
    })
}

/// The items of `artist` display point `(x, y)` hits within `radius`
/// (the artist's own tolerance with `None`), empty for an artist hit as
/// a whole; `None` when the point misses (matplotlib's `contains`).
fn contains(
    artist: RawHandle,
    x: f64,
    y: f64,
    radius: Option<f64>,
) -> PyResult<Option<Vec<usize>>> {
    Ok(artist.contains_within(Point::new(x, y), radius))
}

/// The artists under `root` that a press over `inaxes` asks, in the order
/// matplotlib's `Artist.pick` asks them: those with a picker pltrs (Rust)
/// holds and those of `also` (with a Python callable).
fn pick_order(
    root: RawHandle,
    inaxes: Option<Axes>,
    also: Vec<RawHandle>,
) -> PyResult<Vec<RawHandle>> {
    let figure: Figure = root.figure();
    Ok(figure.pick_order(&root, inaxes.as_ref(), &also))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_picker", set_picker)?;
    module.add("get_picker", get_picker)?;
    module.add("contains", contains)?;
    module.add("pick_order", pick_order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{extension, with_python};

    #[test]
    fn pickers_read_back_and_order_the_pick() {
        with_python(|py| {
            extension(py);
            let fig = Figure::with_size(6.4, 4.8, 100.0);
            let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]).unwrap();
            let line = ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
            let raw = line.raw().clone();
            assert!(get_picker(raw.clone()).unwrap().bind(py).is_none());
            set_picker(raw.clone(), PyPicker(Some(Picker::Radius(3.0)))).unwrap();
            let radius: f64 = get_picker(raw.clone()).unwrap().bind(py).extract().unwrap();
            assert_eq!(radius, 3.0);
            let order = pick_order(fig.raw().clone(), Some(ax.clone()), Vec::new()).unwrap();
            assert_eq!(order, std::slice::from_ref(&raw));
            let (x, y) = (320.0, 240.0);
            assert_eq!(contains(raw.clone(), x, y, None).unwrap(), Some(vec![0]));
            assert_eq!(contains(raw, x, y + 100.0, Some(1.0)).unwrap(), None);
        });
    }
}
