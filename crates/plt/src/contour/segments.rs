//! The contours of a set split into their connected pieces (matplotlib's
//! `ContourSet.allsegs` and `allkinds`).

use plt_core::{Path, PathCode, Point};

use super::ContourSet;

impl ContourSet {
    /// The vertices of every connected piece of every level (lines) or
    /// band (fills), as currently drawn (matplotlib's `allsegs`): the
    /// paths split before each move. A path without codes, such as the
    /// empty path of a level without contours, is a single piece.
    ///
    /// ```
    /// use plt::contour::{ContourOptions, Levels, QuadField};
    /// use plt::numeric::Grid;
    /// use plt::prelude::*;
    ///
    /// let z = Grid::from_fn(3, 3, |i, j| if (i, j) == (1, 1) { 1.0 } else { 0.0 });
    /// let options = ContourOptions { levels: Levels::Values(vec![0.5, 2.0]), ..ContourOptions::default() };
    /// let cs = Figure::new().subplot().contour(&QuadField::new(z), options)?;
    /// let segments = cs.allsegs();
    /// assert_eq!((segments[0].len(), segments[1].len()), (1, 1));
    /// assert!(segments[1][0].is_empty(), "no contour at 2");
    /// # Ok::<(), plt::contour::ContourError>(())
    /// ```
    pub fn allsegs(&self) -> Vec<Vec<Vec<Point>>> {
        self.pieces(|piece| piece.vertices().to_vec())
    }

    /// The path codes of every piece of [`ContourSet::allsegs`] (matplotlib's
    /// `allkinds`); `None` for a piece without codes.
    pub fn allkinds(&self) -> Vec<Vec<Option<Vec<PathCode>>>> {
        self.pieces(|piece| piece.codes().map(<[PathCode]>::to_vec))
    }

    /// `f` of every connected piece of every path.
    fn pieces<T>(&self, f: impl Fn(&Path) -> T) -> Vec<Vec<T>> {
        self.paths()
            .iter()
            .map(|path| {
                if path.codes().is_some() {
                    path.subpaths().iter().map(&f).collect()
                } else {
                    vec![f(path)]
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::contour::{ContourOptions, Levels, QuadCoords, QuadField};
    use crate::figure::Figure;
    use crate::numeric::Grid;
    use plt_core::PathCode;

    #[test]
    fn pieces_split_at_moves() {
        // matplotlib's `test_allsegs_allkinds`: sin(x) cos(y) on 0, 2, ..., 8.
        let x: Vec<f64> = (0..5).map(|i| f64::from(2 * i)).collect();
        let z = Grid::from_fn(5, 5, |i, j| x[j].sin() * x[i].cos());
        let field = QuadField::new(z).with_coords(QuadCoords::Axes { x: x.clone(), y: x });
        let options = ContourOptions {
            levels: Levels::Values(vec![0.0, 0.5]),
            ..ContourOptions::default()
        };
        let cs = Figure::new().subplot().contour(&field, options).unwrap();
        let (segments, kinds) = (cs.allsegs(), cs.allkinds());
        assert_eq!(segments.iter().map(Vec::len).collect::<Vec<_>>(), [5, 4]);
        assert_eq!(kinds.iter().map(Vec::len).collect::<Vec<_>>(), [5, 4]);
        let first = kinds[0][0].as_ref().expect("codes");
        assert_eq!(first[0], PathCode::MoveTo);
        assert_eq!(first.len(), segments[0][0].len());
    }
}
