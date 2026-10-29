//! Colours in the gaps of dashed edges (matplotlib's `gapcolor` of line
//! collections): before the edges, each dashed item is stroked once more
//! with the inverse of its dash pattern in its gap colour.

use pltrs_core::{Path, Point};
use pltrs_render::Dashes;

use super::Collection;
use crate::colors::Color;
use crate::lines::inverse_dashes;

impl Collection {
    /// Sets the colours drawn in the gaps of dashed edges, cycled over the
    /// items; `None` leaves the gaps empty (matplotlib's
    /// `LineCollection.set_gapcolor`). Solid edges have no gaps.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.vlines([1.0, 2.0], [0.0], [1.0])?
    ///     .set_linestyles(vec![LineStyle::Dashed])
    ///     .set_gapcolors(Some(vec!["orange".into()]));
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn set_gapcolors(&self, colors: Option<Vec<Color>>) -> &Self {
        self.update(|c| c.gapcolors = colors)
    }
}

/// The paths and dash patterns that fill the gaps: each path with the
/// inverse of its (cycled) dash pattern, and a path of one NaN vertex,
/// which draws nothing, for solid patterns (matplotlib's
/// `_get_inverse_paths_linestyles`).
pub(super) fn inverse_paths_and_dashes(
    paths: &[Path],
    dashes: &[Dashes],
) -> (Vec<Path>, Vec<Dashes>) {
    if dashes.is_empty() {
        return (Vec::new(), Vec::new());
    }
    paths
        .iter()
        .zip(dashes.iter().cycle())
        .map(|(path, dashes)| {
            if dashes.lengths().is_empty() {
                (
                    Path::polyline(vec![Point::new(f64::NAN, f64::NAN)]),
                    dashes.clone(),
                )
            } else {
                (path.clone(), inverse_dashes(dashes))
            }
        })
        .unzip()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solid_items_get_no_gaps_and_dashed_ones_the_inverse_pattern() {
        let line = Path::polyline(vec![Point::new(0.0, 0.0), Point::new(1.0, 0.0)]);
        let dashed = Dashes::new(0.0, vec![4.0, 1.0]);
        let (paths, dashes) = inverse_paths_and_dashes(
            &[line.clone(), line.clone(), line.clone()],
            &[dashed, Dashes::solid()],
        );
        assert_eq!(paths.len(), 3, "one gap path per item");
        assert_eq!(paths[0], line);
        assert!(
            paths[1].vertices()[0].x.is_nan(),
            "solid items draw nothing"
        );
        assert_eq!(dashes[0], Dashes::new(1.0, vec![1.0, 4.0]));
        assert_eq!(dashes[2], dashes[0], "the patterns are cycled");
    }

    #[test]
    fn gap_colours_add_a_pass_over_every_item() {
        use crate::figure::Figure;
        use crate::lines::LineStyle;
        use crate::testing::RecordingRenderer;

        let calls = |gaps: Option<Vec<Color>>| {
            let fig = Figure::new();
            fig.subplot()
                .vlines([0.25, 0.5], [0.0], [1.0])
                .unwrap()
                .set_linestyles(vec![LineStyle::Dashed, LineStyle::Solid])
                .set_gapcolors(gaps);
            let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
            fig.draw(&mut renderer).unwrap();
            renderer.calls.len()
        };
        let plain = calls(None);
        assert_eq!(calls(Some(vec!["orange".into()])), plain + 2);
        assert_eq!(
            calls(Some(vec!["none".into()])),
            plain + 2,
            "invisible, but drawn"
        );
    }

    #[test]
    fn no_patterns_no_gaps() {
        let line = Path::polyline(vec![Point::new(0.0, 0.0), Point::new(1.0, 0.0)]);
        assert_eq!(inverse_paths_and_dashes(&[line], &[]), (vec![], vec![]));
    }
}
