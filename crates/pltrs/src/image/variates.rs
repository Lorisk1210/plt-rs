//! Data of several variates, mapped to colours together through a
//! [`MultiNorm`](crate::colors::MultiNorm) and a bivariate or multivariate
//! colormap (matplotlib's multivariate arrays, made by
//! `_ensure_multivariate_data` from a tuple of equally shaped arrays).

use super::{ImageError, ScalarField};

/// Two or more scalar fields of one shape, each with its own mask.
///
/// A cell is drawn in the colormap's *bad* colour when any of its variates
/// is masked.
///
/// ```
/// use pltrs::image::MultiField;
/// use pltrs::numeric::Grid;
/// let a = Grid::from_fn(2, 3, |r, c| (r + c) as f64);
/// let b = Grid::from_fn(2, 3, |r, c| (r * c) as f64);
/// let field = MultiField::new([a, b])?;
/// assert_eq!((field.n_variates(), field.shape()), (2, (2, 3)));
/// assert_eq!(field.at((1, 2)), Some(vec![Some(3.0), Some(2.0)]));
/// # Ok::<(), pltrs::image::ImageError>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct MultiField {
    variates: Vec<ScalarField>,
}

impl MultiField {
    /// The variates, in order.
    ///
    /// # Errors
    ///
    /// When fewer than two are given or their shapes differ.
    pub fn new(
        variates: impl IntoIterator<Item = impl Into<ScalarField>>,
    ) -> Result<Self, ImageError> {
        let variates: Vec<ScalarField> = variates.into_iter().map(Into::into).collect();
        let Some(first) = variates.first().map(|v| v.values().shape()) else {
            return Err(ImageError::TooFewVariates(0));
        };
        if variates.len() < 2 {
            return Err(ImageError::TooFewVariates(variates.len()));
        }
        if let Some(other) = variates.iter().find(|v| v.values().shape() != first) {
            return Err(ImageError::ShapeMismatch {
                what: "variate",
                expected: first,
                actual: other.values().shape(),
            });
        }
        Ok(Self { variates })
    }

    /// The variates.
    pub fn variates(&self) -> &[ScalarField] {
        &self.variates
    }

    /// The number of variates.
    pub fn n_variates(&self) -> usize {
        self.variates.len()
    }

    /// `(rows, columns)` of every variate.
    pub fn shape(&self) -> (usize, usize) {
        self.variates[0].values().shape()
    }

    /// Each variate's values with masked cells as `None`, row-major.
    pub fn masked_values(&self) -> Vec<Vec<Option<f64>>> {
        self.variates
            .iter()
            .map(ScalarField::masked_values)
            .collect()
    }

    /// The value of each variate at `(row, column)` (`None` = masked), or
    /// `None` outside the field.
    pub fn at(&self, (row, col): (usize, usize)) -> Option<Vec<Option<f64>>> {
        self.variates
            .iter()
            .map(|variate| {
                let value = *variate.values().get(row, col)?;
                let masked = *variate.mask().get(row, col)?;
                Some((!masked).then_some(value))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numeric::Grid;

    #[test]
    fn variates_need_company_and_one_shape() {
        let one = Grid::filled(2, 2, 1.0);
        assert_eq!(
            MultiField::new([one.clone()]).err(),
            Some(ImageError::TooFewVariates(1))
        );
        assert_eq!(
            MultiField::new(Vec::<Grid<f64>>::new()).err(),
            Some(ImageError::TooFewVariates(0))
        );
        let other = Grid::filled(2, 3, 1.0);
        assert!(matches!(
            MultiField::new([one, other]),
            Err(ImageError::ShapeMismatch {
                what: "variate",
                ..
            })
        ));
    }

    #[test]
    fn masks_stay_per_variate() {
        let a = Grid::from_vec(1, 2, vec![1.0, f64::NAN]).unwrap();
        let b = Grid::from_vec(1, 2, vec![2.0, 3.0]).unwrap();
        let field = MultiField::new([a, b]).unwrap();
        assert_eq!(
            field.masked_values(),
            [vec![Some(1.0), None], vec![Some(2.0), Some(3.0)]]
        );
        assert_eq!(field.at((0, 1)), Some(vec![None, Some(3.0)]));
        assert_eq!(field.at((1, 0)), None);
    }
}
