//! Colormaps of the program's: a `Colormap` subclass whose mapping is its
//! own (it overrides `__call__`, or what matplotlib's `__call__` calls).
//! matplotlib calls the colormap object wherever it maps values
//! (`cmap(X, alpha=..., bytes=...)`); a [`Colormap`] given a
//! [`ProgramColormap`] calls it at the same places, with the same
//! arguments, and draws the colours it gives.

use std::fmt;
use std::rc::Rc;

use pltrs_core::Rgba;

use super::{Colormap, ColormapError, Normalized};
use crate::colors::Alpha;

/// What matplotlib hands a colormap it calls: normalised values as an
/// array of `shape` (masked where missing; a scalar for no dimension), the
/// alpha and whether it asks for 8-bit channels.
#[derive(Clone, Copy, Debug)]
pub struct ColormapCall<'a> {
    /// The normalised values, row-major.
    pub values: &'a [Normalized],
    /// The shape of the array they are handed as.
    pub shape: &'a [usize],
    /// The alpha given (`alpha=None` for [`Alpha::Keep`]).
    pub alpha: &'a Alpha,
    /// Whether the colours are asked as bytes (`bytes=True`).
    pub bytes: bool,
}

/// A colormap of the program's, which pltrs calls instead of looking its
/// table up.
pub trait ProgramColormap: fmt::Debug {
    /// The colours the program's colormap gives for `call`, one RGBA row
    /// per value: fractions, or with [`ColormapCall::bytes`] the 8-bit
    /// channels as numbers; `None` when it raised (the exception raises
    /// from the native call that drew).
    fn call(&self, call: ColormapCall<'_>) -> Option<Vec<[f64; 4]>>;
}

impl Colormap {
    /// The colormap, its values mapped by `program`, a colormap of the
    /// program's, wherever they are mapped as an array (see
    /// [`map_shaped`](Self::map_shaped)).
    #[must_use]
    pub fn with_program(self, program: Rc<dyn ProgramColormap>) -> Self {
        Self {
            program: Some(program),
            ..self
        }
    }

    /// Whether the program's colormap maps the values.
    pub fn calls_program(&self) -> bool {
        self.program.is_some()
    }

    /// The colours of `values`, an array of `shape`, with `alpha`
    /// (matplotlib's `cmap(X, alpha)`): the program's colormap's, else
    /// those of the table ([`Colormap::map`]). A call that raised gives
    /// transparent colours.
    ///
    /// # Errors
    ///
    /// When `alpha` gives a number per value for another number of values.
    pub fn map_shaped(
        &self,
        values: &[Normalized],
        shape: &[usize],
        alpha: &Alpha,
    ) -> Result<Vec<Rgba>, ColormapError> {
        let Some(program) = &self.program else {
            return self.map(values, alpha);
        };
        check_alpha(values, alpha)?;
        let call = ColormapCall {
            values,
            shape,
            alpha,
            bytes: false,
        };
        Ok(match program.call(call) {
            Some(rows) => rows
                .into_iter()
                .map(|[r, g, b, a]| Rgba::new(r, g, b, a))
                .collect(),
            None => vec![Rgba::TRANSPARENT; values.len()],
        })
    }

    /// [`map_shaped`](Self::map_shaped) as 8-bit channels (matplotlib's
    /// `cmap(X, alpha, bytes=True)`).
    ///
    /// # Errors
    ///
    /// As [`map_shaped`](Self::map_shaped).
    pub fn map_bytes_shaped(
        &self,
        values: &[Normalized],
        shape: &[usize],
        alpha: &Alpha,
    ) -> Result<Vec<[u8; 4]>, ColormapError> {
        let Some(program) = &self.program else {
            return self.map_bytes(values, alpha);
        };
        check_alpha(values, alpha)?;
        let call = ColormapCall {
            values,
            shape,
            alpha,
            bytes: true,
        };
        Ok(match program.call(call) {
            Some(rows) => rows.into_iter().map(|row| row.map(to_byte)).collect(),
            None => vec![[0; 4]; values.len()],
        })
    }
}

impl Colormap {
    /// The colour of one value handed over alone (matplotlib's
    /// `cmap(norm(value))` of a scalar): the program's colormap's, called
    /// with the scalar, else the table's.
    pub fn lookup_scalar(&self, value: Normalized) -> Rgba {
        if self.program.is_none() {
            return self.lookup(value, None);
        }
        (self.map_shaped(&[value], &[], &Alpha::Keep).ok())
            .and_then(|colors| colors.first().copied())
            .unwrap_or(Rgba::TRANSPARENT)
    }
}

/// A channel the program gave as a byte, as numpy casts it to `uint8`.
fn to_byte(value: f64) -> u8 {
    // The program's uint8 array, read back as numbers.
    value.clamp(0.0, 255.0) as u8
}

/// The error of an alpha per value given for another number of values.
fn check_alpha(values: &[Normalized], alpha: &Alpha) -> Result<(), ColormapError> {
    match alpha {
        Alpha::Each(alphas) if alphas.len() != values.len() => Err(ColormapError::AlphaCount {
            values: values.len(),
            alphas: alphas.len(),
        }),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::testing::rgb_colormap;

    /// A program's colormap recording its calls and painting everything red.
    #[derive(Debug, Default)]
    struct Red(RefCell<Vec<(Vec<usize>, bool)>>);

    impl ProgramColormap for Red {
        fn call(&self, call: ColormapCall<'_>) -> Option<Vec<[f64; 4]>> {
            self.0.borrow_mut().push((call.shape.to_vec(), call.bytes));
            let red = if call.bytes {
                [255.0, 0.0, 0.0, 255.0]
            } else {
                [1.0, 0.0, 0.0, 1.0]
            };
            Some(vec![red; call.values.len()])
        }
    }

    #[test]
    fn the_program_s_colormap_maps_arrays_with_their_shape() {
        let red = Rc::new(Red::default());
        let cmap = rgb_colormap().with_program(Rc::clone(&red) as Rc<dyn ProgramColormap>);
        assert!(cmap.calls_program());
        let values = [Normalized::Fraction(0.0), Normalized::Masked];
        let colors = cmap.map_shaped(&values, &[2, 1], &Alpha::Keep).unwrap();
        assert_eq!(colors, [Rgba::new(1.0, 0.0, 0.0, 1.0); 2]);
        let bytes = cmap.map_bytes_shaped(&values, &[2], &Alpha::Keep).unwrap();
        assert_eq!(bytes, [[255, 0, 0, 255]; 2]);
        assert_eq!(*red.0.borrow(), [(vec![2, 1], false), (vec![2], true)]);
        assert!(
            cmap.reversed().calls_program().eq(&false),
            "a derived map is pltrs'"
        );
        assert!(
            cmap.map_shaped(&values, &[2], &Alpha::Each(vec![1.0]))
                .is_err()
        );
    }
}
