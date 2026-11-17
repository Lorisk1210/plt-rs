//! Fitting the grid around the measured decorations (the second half of
//! matplotlib's `get_tight_layout_figure` and `_auto_adjust_subplotpars`):
//! the room decorations take beyond each cell edge, turned into margins and
//! spacings with padding.

use pltrs_core::Bbox;

use super::{Cell, Measured, TightLayout, TightLayoutError};
use crate::artist::{Artist, LayoutContext};
use crate::figure::{FigureFrame, FigureState, SuperText};
use crate::gridspec::SubplotOverrides;
use crate::text::TextArtist;

/// The room decorations take beyond the cell edges, in figure fractions
/// (matplotlib's `vspaces` and `hspaces`).
struct Spaces {
    /// Above and below each cell: `rows + 1` row edges × `cols`.
    vertical: Vec<Vec<f64>>,
    /// Left and right of each cell: `rows` × `cols + 1` column edges.
    horizontal: Vec<Vec<f64>>,
}

impl Spaces {
    /// Adds up what the decorations of each cell take beyond its edges.
    fn new((rows, cols): (usize, usize), cells: &[Cell], size: (f64, f64)) -> Self {
        let mut spaces = Self {
            vertical: vec![vec![0.0; cols]; rows + 1],
            horizontal: vec![vec![0.0; cols + 1]; rows],
        };
        for cell in cells {
            let Some(tight) = cell.tight else { continue };
            let tight = figure_fraction(tight, size);
            let ax = cell.position;
            for row in &mut spaces.horizontal[cell.rows.clone()] {
                row[cell.cols.start] += ax.xmin() - tight.xmin();
                row[cell.cols.end] += tight.xmax() - ax.xmax();
            }
            for col in cell.cols.clone() {
                spaces.vertical[cell.rows.start][col] += tight.ymax() - ax.ymax();
                spaces.vertical[cell.rows.end][col] += ax.ymin() - tight.ymin();
            }
        }
        spaces
    }

    /// The largest room along the column edge `edge`.
    fn column_edge(&self, edge: usize) -> f64 {
        largest(self.horizontal.iter().map(|row| row[edge]))
    }

    /// The largest room along the row edge `edge`.
    fn row_edge(&self, edge: usize) -> f64 {
        largest(self.vertical[edge].iter().copied())
    }

    /// The largest room at the interior column edges.
    fn between_columns(&self) -> f64 {
        largest(
            self.horizontal
                .iter()
                .flat_map(|row| row[1..row.len() - 1].iter().copied()),
        )
    }

    /// The largest room at the interior row edges.
    fn between_rows(&self) -> f64 {
        let rows = self.vertical.len() - 1;
        largest(self.vertical[1..rows].iter().flatten().copied())
    }
}

/// The largest value, `-inf` for none, NaN where one is NaN (numpy's
/// `max`, which matplotlib takes of the spaces: a NaN cell edge, of axes
/// placed at NaN, leaves the margins NaN).
fn largest(values: impl Iterator<Item = f64>) -> f64 {
    values.fold(f64::NEG_INFINITY, |largest, value| {
        if largest.is_nan() || value.is_nan() {
            f64::NAN
        } else {
            largest.max(value)
        }
    })
}

/// `room`, or no room when it is negative, as matplotlib's `max(room, 0)`
/// (Python's builtin, which keeps a NaN room).
fn at_least_none(room: f64) -> f64 {
    if 0.0 > room { 0.0 } else { room }
}

/// `bbox`, in pixels of a figure `width` × `height`, in figure fractions
/// as matplotlib's `fig.transFigure.inverted()` maps it: multiplied by the
/// reciprocal sizes, which rounds differently from dividing by the sizes.
fn figure_fraction(bbox: Bbox, (width, height): (f64, f64)) -> Bbox {
    let (sx, sy) = (1.0 / width, 1.0 / height);
    Bbox::from_extents(
        bbox.x0() * sx,
        bbox.y0() * sy,
        bbox.x1() * sx,
        bbox.y1() * sy,
    )
}

/// matplotlib's `get_tight_layout_figure` from the margins on: the
/// subplot parameters (`None` without grid-placed axes), fitted into
/// `params.rect` by a second pass if given, over the axes as measured
/// `again` when they measure otherwise the second time (as `measured`
/// else).
pub(in crate::layout) fn adjusted_params(
    fig: &FigureState,
    measured: &Measured,
    again: Option<&Measured>,
    params: &TightLayout,
) -> Result<Option<SubplotOverrides>, TightLayoutError> {
    let Some(shape) = measured.shape else {
        return Ok(None);
    };
    let grid = Grid {
        fig,
        shape,
        spaces: Spaces::new(shape, &measured.cells, fig.size_pixels()),
    };
    let first = grid.subplot_params(params, None)?;
    let Some([left, bottom, right, top]) = params.rect else {
        return Ok(Some(first));
    };
    let set = |value: Option<f64>| value.unwrap_or_default();
    let grid = match again {
        Some(again) => Grid {
            spaces: Spaces::new(shape, &again.cells, fig.size_pixels()),
            ..grid
        },
        None => grid,
    };
    let rect = [
        left.map(|left| left + set(first.left)),
        bottom.map(|bottom| bottom + set(first.bottom)),
        right.map(|right| right - (1.0 - set(first.right))),
        top.map(|top| top - (1.0 - set(first.top))),
    ];
    grid.subplot_params(params, Some(rect)).map(Some)
}

/// The common grid being fitted.
struct Grid<'a> {
    fig: &'a FigureState,
    shape: (usize, usize),
    spaces: Spaces,
}

impl Grid<'_> {
    /// matplotlib's `_auto_adjust_subplotpars`: margins as large as the
    /// decorations along the grid's edges (plus the figure's title and
    /// labels) and spacings as large as those between cells, each with
    /// padding. The set, non-zero margins of `rect` (`[left, bottom,
    /// right, top]`) are kept as they are, as in matplotlib's truthiness
    /// tests.
    fn subplot_params(
        &self,
        params: &TightLayout,
        rect: Option<[Option<f64>; 4]>,
    ) -> Result<SubplotOverrides, TightLayoutError> {
        let (rows, cols) = self.shape;
        let (fig_width, fig_height) = self.fig.size_inches;
        let font_size_inch = crate::rc::params().font_size / 72.0;
        let pad = params.pad * font_size_inch;
        let w_pad = params.w_pad.map_or(pad, |p| p * font_size_inch) / fig_width;
        let h_pad = params.h_pad.map_or(pad, |p| p * font_size_inch) / fig_height;
        let (pad_w, pad_h) = (pad / fig_width, pad / fig_height);
        let given = |value: Option<f64>| value.filter(|&v| v != 0.0);
        let [left, bottom, right, top] = rect.map_or([None; 4], |[l, b, r, t]| {
            [
                given(l),
                given(b),
                given(given(r).map(|r| 1.0 - r)),
                given(given(t).map(|t| 1.0 - t)),
            ]
        });
        let texts = self.super_text_extents();
        let left = left.unwrap_or_else(|| {
            at_least_none(self.spaces.column_edge(0))
                + pad_w
                + texts.ylabel.map_or(0.0, |w| w.width() + pad_w)
        });
        let right = right.unwrap_or_else(|| at_least_none(self.spaces.column_edge(cols)) + pad_w);
        let top = top.unwrap_or_else(|| {
            at_least_none(self.spaces.row_edge(0))
                + pad_h
                + texts.title.map_or(0.0, |t| t.height() + pad_h)
        });
        let bottom = bottom.unwrap_or_else(|| {
            at_least_none(self.spaces.row_edge(rows))
                + pad_h
                + texts.xlabel.map_or(0.0, |x| x.height() + pad_h)
        });
        if left + right >= 1.0 {
            return Err(TightLayoutError::MarginsTooWide);
        }
        if bottom + top >= 1.0 {
            return Err(TightLayoutError::MarginsTooTall);
        }
        let mut overrides = SubplotOverrides {
            left: Some(left),
            right: Some(1.0 - right),
            bottom: Some(bottom),
            top: Some(1.0 - top),
            ..SubplotOverrides::default()
        };
        if cols > 1 {
            let hspace = self.spaces.between_columns() + w_pad;
            let axes_width = (1.0 - right - left - hspace * (cols - 1) as f64) / cols as f64;
            if axes_width < 0.0 {
                return Err(TightLayoutError::AxesTooWide);
            }
            overrides.wspace = Some(hspace / axes_width);
        }
        if rows > 1 {
            let vspace = self.spaces.between_rows() + h_pad;
            let axes_height = (1.0 - top - bottom - vspace * (rows - 1) as f64) / rows as f64;
            if axes_height < 0.0 {
                return Err(TightLayoutError::AxesTooTall);
            }
            overrides.hspace = Some(vspace / axes_height);
        }
        Ok(overrides)
    }

    /// The extents of the figure's title and labels taking part in the
    /// layout (placed by hand or not), in figure fractions.
    fn super_text_extents(&self) -> SuperExtents {
        let fig = self.fig;
        let layout = LayoutContext::new(fig);
        let extent = |text: Option<SuperText>| {
            let text = layout.store().get_as::<TextArtist>(text?.id)?;
            let base = text.base();
            if !base.in_layout {
                return None;
            }
            // matplotlib measures a hidden text as the unit box at the
            // origin (`Text.get_window_extent`): a pixel of room.
            let bbox = if base.visible {
                text.window_extent(&layout)?
            } else {
                Bbox::UNIT
            };
            Some(figure_fraction(bbox, fig.size_pixels()))
        };
        let texts = fig
            .store
            .get_as::<FigureFrame>(fig.root)
            .map(|frame| frame.super_texts)
            .unwrap_or_default();
        SuperExtents {
            title: extent(texts.title),
            xlabel: extent(texts.xlabel),
            ylabel: extent(texts.ylabel),
        }
    }
}

/// The extents of the figure's title and labels, in figure fractions.
struct SuperExtents {
    title: Option<Bbox>,
    xlabel: Option<Bbox>,
    ylabel: Option<Bbox>,
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use super::*;

    #[test]
    fn figure_fractions_scale_by_reciprocal_sizes() {
        // 3 / 640 and 100 / 1200 round apart from 3 * (1 / 640) and
        // 100 * (1 / 1200), which is what matplotlib computes.
        let bbox = figure_fraction(
            Bbox::from_extents(3.0, 100.0, 640.0, 1200.0),
            (640.0, 1200.0),
        );
        assert_eq!(bbox.x0(), 3.0 * (1.0 / 640.0));
        assert_ne!(bbox.x0(), 3.0 / 640.0);
        assert_eq!(bbox.y0(), 100.0 * (1.0 / 1200.0));
        assert_ne!(bbox.y0(), 100.0 / 1200.0);
        assert_eq!((bbox.x1(), bbox.y1()), (1.0, 1.0));
    }

    #[test]
    fn a_hidden_suptitle_takes_a_pixel_of_room() {
        use crate::artist::ArtistHandle;
        use crate::figure::Figure;
        use crate::layout::TightLayout;
        let fig = Figure::new();
        let ax = fig.subplot();
        fig.suptitle("hidden").set_visible(false);
        fig.tight_layout(TightLayout::default()).unwrap();
        // matplotlib: (0.073, 0.0804, 0.8864, 0.844), the hidden title
        // measured as the unit box.
        let (x, y, w, h) = ax.position().bounds();
        let rounded = |v: f64| (v * 1e4).round() / 1e4;
        assert_eq!([x, y, w, h].map(rounded), [0.073, 0.0804, 0.8864, 0.844]);
    }

    #[test]
    fn nan_rooms_stay_nan_as_numpy_and_python_take_them() {
        assert!(largest([1.0, f64::NAN, 3.0].into_iter()).is_nan());
        assert_eq!(largest([1.0, 3.0].into_iter()), 3.0);
        assert_eq!(largest(std::iter::empty()), f64::NEG_INFINITY);
        assert!(at_least_none(f64::NAN).is_nan());
        assert_eq!(at_least_none(-0.5), 0.0);
        assert_eq!(at_least_none(0.25), 0.25);
    }

    fn cell(rows: Range<usize>, cols: Range<usize>, position: Bbox, tight: Bbox) -> Cell {
        Cell {
            rows,
            cols,
            position,
            tight: Some(tight),
        }
    }

    #[test]
    fn spaces_add_up_per_edge() {
        // Two cells side by side on a 100 × 100 pixel canvas.
        let left = cell(
            0..1,
            0..1,
            Bbox::from_extents(0.1, 0.1, 0.4, 0.9),
            Bbox::from_extents(5.0, 8.0, 42.0, 93.0),
        );
        let right = cell(
            0..1,
            1..2,
            Bbox::from_extents(0.6, 0.1, 0.9, 0.9),
            Bbox::from_extents(55.0, 10.0, 90.0, 90.0),
        );
        let spaces = Spaces::new((1, 2), &[left, right], (100.0, 100.0));
        let close = |a: f64, b: f64| (a - b).abs() < 1e-12;
        assert!(close(spaces.column_edge(0), 0.05));
        // The interior edge holds the left cell's right overhang plus the
        // right cell's left overhang.
        assert!(close(spaces.between_columns(), 0.02 + 0.05));
        assert!(close(spaces.row_edge(0), 0.03));
        assert!(close(spaces.row_edge(1), 0.02));
    }
}
