//! A colormap of several variates, bivariate or multivariate (what
//! matplotlib's colorizers accept as `cmap` for multivariate data).

use pltrs_core::Rgba;

use super::{BivarColormap, MultivarColormap, MultivarError, bivar_colormap, multivar_colormap};
use crate::colors::{Colormap, Normalized};

/// A colormap taking one value per variate.
///
/// ```
/// use pltrs::colors::Normalized::Fraction;
/// use pltrs::colors::multivar::MultiColormap;
/// let cmap = MultiColormap::get("BiOrangeBlue")?;
/// assert_eq!(cmap.n_variates(), 2);
/// let white = cmap.lookup(&[Fraction(1.0), Fraction(1.0)], None)?;
/// assert_eq!(white.to_array(), [1.0; 4]);
/// # Ok::<(), pltrs::colors::multivar::MultivarError>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum MultiColormap {
    /// Two variates looked up in a 2-D table.
    Bivariate(BivarColormap),
    /// One colormap per variate, mixed.
    Multivariate(MultivarColormap),
}

impl MultiColormap {
    /// The registered multivariate or bivariate colormap `name` (looked
    /// up in that order, as matplotlib's `_ensure_cmap` does).
    ///
    /// # Errors
    ///
    /// When no colormap of several variates has that name.
    pub fn get(name: &str) -> Result<Self, MultivarError> {
        match multivar_colormap(name) {
            Ok(cmap) => Ok(cmap.into()),
            Err(MultivarError::Unknown(_)) => bivar_colormap(name).map(Self::from),
            Err(err) => Err(err),
        }
    }

    /// The name.
    pub fn name(&self) -> &str {
        match self {
            Self::Bivariate(cmap) => cmap.name(),
            Self::Multivariate(cmap) => cmap.name(),
        }
    }

    /// The number of variates.
    pub fn n_variates(&self) -> usize {
        match self {
            Self::Bivariate(_) => 2,
            Self::Multivariate(cmap) => cmap.n_variates(),
        }
    }

    /// The number of colours along each variate (matplotlib's
    /// `(cmap.N, cmap.M)` or `[part.N for part in cmap]`).
    pub fn sizes(&self) -> Vec<usize> {
        match self {
            Self::Bivariate(cmap) => {
                let (n, m) = cmap.size();
                vec![n, m]
            }
            Self::Multivariate(cmap) => cmap.colormaps().iter().map(Colormap::n).collect(),
        }
    }

    /// The colour of one point, `values[i]` for variate `i` (matplotlib's
    /// `cmap(X, alpha)`): a bivariate colormap replaces the opacity with
    /// `alpha`, a multivariate one multiplies it.
    ///
    /// # Errors
    ///
    /// When `values` does not hold one value per variate, or a circular
    /// bivariate colormap gets table indices.
    pub fn lookup(&self, values: &[Normalized], alpha: Option<f64>) -> Result<Rgba, MultivarError> {
        match (self, values) {
            (Self::Bivariate(cmap), &[x0, x1]) => cmap.lookup(x0, x1, alpha),
            (Self::Bivariate(_), _) => Err(MultivarError::VariateCount {
                expected: 2,
                given: values.len(),
            }),
            (Self::Multivariate(cmap), _) => cmap.lookup(values, alpha),
        }
    }
}

impl From<BivarColormap> for MultiColormap {
    fn from(cmap: BivarColormap) -> Self {
        Self::Bivariate(cmap)
    }
}

impl From<MultivarColormap> for MultiColormap {
    fn from(cmap: MultivarColormap) -> Self {
        Self::Multivariate(cmap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::Normalized::{Fraction, Masked};

    #[test]
    fn registries_are_searched_in_turn() {
        let multi = MultiColormap::get("3VarAddA").unwrap();
        assert_eq!((multi.name(), multi.n_variates()), ("3VarAddA", 3));
        assert_eq!(multi.sizes(), [256, 256, 256]);
        let bivar = MultiColormap::get("BiPeak").unwrap();
        assert_eq!(bivar.sizes(), [256, 256]);
        assert_eq!(
            MultiColormap::get("viridis"),
            Err(MultivarError::Unknown("viridis".into()))
        );
    }

    #[test]
    fn lookups_check_the_variates() {
        let bivar = MultiColormap::get("BiOrangeBlue").unwrap();
        assert_eq!(
            bivar.lookup(&[Fraction(0.5)], None),
            Err(MultivarError::VariateCount {
                expected: 2,
                given: 1
            })
        );
        assert_eq!(
            bivar.lookup(&[Masked, Fraction(0.5)], None),
            Ok(Rgba::TRANSPARENT)
        );
        let multi = MultiColormap::get("2VarAddA").unwrap();
        assert!(multi.lookup(&[Fraction(0.5); 3], None).is_err());
    }
}
