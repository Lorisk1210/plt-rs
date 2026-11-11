//! Colormaps of the program's (a `Colormap` subclass mapping values its
//! own way): pltrs (Rust) calls the Python colormap where matplotlib calls
//! it, through the callable the package's `native_colormap` gives (in
//! `colors.py`), which hands it the values as matplotlib does.

use std::rc::Rc;

use pltrs::colors::{Alpha, ColormapCall, Normalized, ProgramColormap};

use crate::binding::lookup;
use crate::binding::prelude::*;

/// A Python colormap whose class maps values its own way.
#[derive(Debug)]
pub(crate) struct PythonColormap {
    /// `call(values, mask, integers, shape, alpha, alpha_values, bytes)`.
    call: Callback,
}

impl PythonColormap {
    /// The program's colormap `cmap`, if its class maps values its own way.
    pub(crate) fn of(cmap: &Bound<'_>) -> PyResult<Option<Rc<dyn ProgramColormap>>> {
        let call = lookup(cmap.py(), "native_colormap")?.call(std::slice::from_ref(cmap), &[])?;
        if call.is_none() {
            return Ok(None);
        }
        Ok(Some(Rc::new(Self {
            call: call.extract()?,
        })))
    }
}

impl ProgramColormap for PythonColormap {
    fn call(&self, call: ColormapCall<'_>) -> Option<Vec<[f64; 4]>> {
        let integers = call
            .values
            .iter()
            .any(|value| matches!(value, Normalized::Index(_)));
        let values: Vec<f64> = call
            .values
            .iter()
            .map(|value| match *value {
                Normalized::Fraction(x) => x,
                // Exactly representable: matplotlib's indices are small.
                #[expect(clippy::cast_precision_loss, reason = "colormap indices are small")]
                Normalized::Index(i) => i as f64,
                Normalized::Masked => 0.0,
            })
            .collect();
        let mask: Vec<bool> = call
            .values
            .iter()
            .map(|value| *value == Normalized::Masked)
            .collect();
        let (alpha, alpha_values) = match call.alpha {
            Alpha::Keep => (None, None),
            Alpha::Uniform(alpha) => (Some(*alpha), None),
            Alpha::Each(alphas) => (None, Some(NdArray::from_vec(alphas.clone()))),
        };
        let rows: NdArray = self.call.call((
            NdArray::from_vec(values),
            NdArray::from_vec(mask),
            integers,
            call.shape.to_vec(),
            alpha,
            alpha_values,
            call.bytes,
        ))?;
        Some(rows.data.as_chunks::<4>().0.to_vec())
    }
}
