//! Attributes of the package that a program may replace, which pltrs
//! (Rust) reads where it needs to know whether they are replaced.
//!
//! matplotlib's tests replace module functions and methods of its classes
//! with `monkeypatch.setattr` (`text._get_text_metrics_with_cache`,
//! `Text.draw`, `PsfontsMap.__getitem__`) or take them away
//! (`RendererAgg.draw_text`), and matplotlib calls whatever is there when
//! it measures or draws. pltrs (Rust) measures and draws itself, so the
//! package hands it, with the replacement's stand-in, a [`Watched`]
//! attribute, which pltrs (Rust) reads each time it would use the
//! stand-in: an attribute lookup, and the module and the class stay plain
//! Python objects (no metaclass or module class watching assignments).

use super::convert::FromPy;
use super::error::PyResult;
use super::gil::Python;
use super::object::{Bound, Owned};

/// The attribute `name` of `owner`, and the object it is while the
/// program has not replaced it.
#[derive(Debug)]
pub(crate) struct Watched {
    owner: Owned,
    name: String,
    value: Owned,
}

impl Watched {
    /// Whether `owner.name` is now the object it was given with (Python's
    /// `is`); an attribute that cannot be read is not.
    pub(crate) fn holds(&self) -> bool {
        Python::with_gil(|py| {
            self.owner
                .bind(py)
                .getattr(&self.name)
                .is_ok_and(|current| current.as_ptr() == self.value.as_ptr())
        })
    }
}

impl FromPy<'_> for Watched {
    /// From `(owner, name, value)`.
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let (owner, name, value) = object.extract::<(Owned, String, Owned)>()?;
        Ok(Self { owner, name, value })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, import, with_python};

    #[test]
    fn an_attribute_holds_its_object_until_it_is_replaced_or_removed() {
        with_python(|py| {
            let namespace = eval(py, "", "type('Namespace', (), {'f': len})");
            let builtins = import(py, "builtins");
            let (setattr, delattr) = (
                builtins.getattr("setattr").unwrap(),
                builtins.getattr("delattr").unwrap(),
            );
            let name = eval(py, "", "'f'");
            let watched = Watched {
                owner: namespace.clone().unbind(),
                name: "f".to_owned(),
                value: eval(py, "", "len").unbind(),
            };
            assert!(watched.holds());
            let set = |value: &str| {
                setattr
                    .call(&[namespace.clone(), name.clone(), eval(py, "", value)], &[])
                    .unwrap();
            };
            set("abs");
            assert!(!watched.holds(), "replaced");
            set("len");
            assert!(watched.holds(), "restored");
            delattr
                .call(&[namespace.clone(), name.clone()], &[])
                .unwrap();
            assert!(!watched.holds(), "removed");
            let given: Watched = eval(py, "", "(len, '__name__', None)").extract().unwrap();
            assert!(!given.holds(), "len.__name__ is a str");
        });
    }
}
