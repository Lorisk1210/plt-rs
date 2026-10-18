//! The labelled buttons of check and radio buttons: where they go
//! (matplotlib's `_Buttons._init_layout`), their markers and which one a
//! click selects.

use plt_core::Point;
use plt_text::{HAlign, VAlign};

use super::options::{ButtonLayout, ButtonsError};
use crate::artist::ArtistHandle;
use crate::axes::{Axes, ScatterColors, ScatterOptions};
use crate::collections::{Collection, EdgeColors};
use crate::markers::{FillStyle, MarkerStyle, MarkerSymbol};
use crate::numeric::linspace;
use crate::text::{Text, TextProps};

/// Extra distance in points within which a click hits a button marker
/// (matplotlib's default collection `pickradius`).
const PICK_RADIUS: f64 = 5.0;

/// The gap between the axes' left edge and the first column of buttons
/// in the measured layouts, in points.
const LEFT_MARGIN: f64 = 11.0;

/// The gap between a button and its label in the measured layouts, in
/// points.
const BUTTON_TEXT_OFFSET: f64 = 5.5;

/// The gap between a label and the next column of buttons in the
/// measured layouts, in points.
const COLUMN_SPACING: f64 = 11.0;

/// The look of the markers of one family of buttons.
pub(super) struct MarkerLook {
    /// Marker areas in points² (one, or one per button).
    pub(super) sizes: Vec<f64>,
    pub(super) colors: ScatterColors,
    pub(super) edgecolors: Option<EdgeColors>,
    pub(super) linewidths: Option<Vec<f64>>,
}

/// The labelled buttons laid out in an axes.
#[derive(Debug)]
pub(super) struct ButtonColumn {
    ax: Axes,
    pub(super) labels: Vec<Text>,
    /// The button centres in axes fractions.
    centres: Vec<Point>,
    /// The default marker areas in points²: the square of half each
    /// label's font size.
    default_sizes: Vec<f64>,
}

impl ButtonColumn {
    /// Sets up `ax` with one label per entry, styled by the cycled
    /// `label_props`, and the buttons arranged by `layout`.
    pub(super) fn new(
        ax: &Axes,
        labels: &[&str],
        layout: ButtonLayout,
        label_props: &[TextProps],
    ) -> Result<Self, ButtonsError> {
        ax.set_xticks(Vec::<f64>::new());
        ax.set_yticks(Vec::<f64>::new());
        ax.set_navigate(false);
        let shape = match layout {
            ButtonLayout::Legacy => None,
            ButtonLayout::Vertical => Some((labels.len(), 1)),
            ButtonLayout::Horizontal => Some((1, labels.len())),
            ButtonLayout::Grid { rows, cols } => {
                if rows * cols < labels.len() {
                    return Err(ButtonsError::TooFewPositions {
                        positions: rows * cols,
                        labels: labels.len(),
                    });
                }
                Some((rows, cols))
            }
        };
        let texts: Vec<Text> = labels
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let text = ax.text(0.0, 0.0, *label);
                text.set_transform(ax.trans_axes());
                text.set_halign(HAlign::Left).set_valign(VAlign::Center);
                if let Some(props) = cycled(label_props, i) {
                    text.update_props(props.clone());
                }
                text
            })
            .collect();
        let centres = match shape {
            None => legacy_centres(&texts),
            Some((rows, cols)) => measured_centres(ax, &texts, rows, cols)?,
        };
        let default_sizes = texts
            .iter()
            .map(|t| (t.font().size / 2.0).powi(2))
            .collect();
        Ok(Self {
            ax: ax.clone(),
            labels: texts,
            centres,
            default_sizes,
        })
    }

    /// `sizes`, or the default areas when unset.
    pub(super) fn sizes_or_default(&self, sizes: Option<&Vec<f64>>) -> Vec<f64> {
        sizes.cloned().unwrap_or_else(|| self.default_sizes.clone())
    }

    /// Markers of `symbol` at the button centres.
    pub(super) fn markers(&self, symbol: MarkerSymbol, look: MarkerLook) -> Collection {
        let (x, y): (Vec<f64>, Vec<f64>) = self.centres.iter().map(|p| (p.x, p.y)).unzip();
        let options = ScatterOptions {
            sizes: Some(look.sizes),
            colors: look.colors,
            edgecolors: look.edgecolors,
            linewidths: look.linewidths,
            marker: Some(MarkerStyle::new(symbol, FillStyle::Full).expect("a named marker")),
            offset_transform: Some(self.ax.trans_axes()),
            ..ScatterOptions::default()
        };
        self.ax
            .scatter_with(x, y, options)
            .expect("one marker, or one per label, of each kind")
    }

    /// The button a click at display point `at` selects: among the
    /// buttons whose marker (of area `sizes` in points²) or label contains
    /// it, the closest (matplotlib's `_clicked`).
    pub(super) fn hit(&self, at: Point, sizes: &[f64]) -> Option<usize> {
        if !self.ax.contains_point(at) {
            return None;
        }
        let figure = self.ax.figure();
        figure.layout();
        let bbox = self.ax.window_extent()?;
        let to_pixels = figure.dpi() / 72.0;
        self.centres
            .iter()
            .map(|c| {
                Point::new(
                    bbox.x0() + c.x * bbox.width(),
                    bbox.y0() + c.y * bbox.height(),
                )
            })
            .enumerate()
            .filter(|(i, c)| {
                let size = cycled(sizes, *i).copied().unwrap_or(0.0);
                let reach = (size.sqrt() / 2.0 + PICK_RADIUS) * to_pixels;
                (c.x - at.x).abs().max((c.y - at.y).abs()) <= reach
                    || self.labels[*i]
                        .window_extent()
                        .is_some_and(|b| b.contains(at))
            })
            .min_by(|(_, a), (_, b)| {
                let d = |p: &Point| (p.x - at.x).powi(2) + (p.y - at.y).powi(2);
                d(a).total_cmp(&d(b))
            })
            .map(|(i, _)| i)
    }
}

/// Item `i` of `items` cycled, if there are any.
pub(super) fn cycled<T>(items: &[T], i: usize) -> Option<&T> {
    (!items.is_empty()).then(|| &items[i % items.len()])
}

/// The legacy column: labels at x = 0.25 and buttons at 0.15, spread
/// evenly from top to bottom.
fn legacy_centres(texts: &[Text]) -> Vec<Point> {
    let ys = linspace(1.0, 0.0, texts.len() + 2);
    texts
        .iter()
        .zip(&ys[1..=texts.len()])
        .map(|(text, &y)| {
            text.set_position(Point::new(0.25, y));
            Point::new(0.15, y)
        })
        .collect()
}

/// The measured layouts: rows spread evenly from top to bottom, columns
/// as wide as their widest label plus fixed gaps in points; the labels
/// are clipped to the axes, as matplotlib adds them with `add_artist`.
fn measured_centres(
    ax: &Axes,
    texts: &[Text],
    rows: usize,
    cols: usize,
) -> Result<Vec<Point>, ButtonsError> {
    let figure = ax.figure();
    figure.layout();
    let points_per_pixel = 72.0 / figure.dpi();
    let axes_width = ax.window_extent().map_or(0.0, |b| b.width()) * points_per_pixel;
    let widths = texts
        .iter()
        .map(|text| Ok(text.layout()?.block.bbox.width() * points_per_pixel / axes_width))
        .collect::<Result<Vec<f64>, ButtonsError>>()?;
    let column_widths: Vec<f64> = (0..cols)
        .map(|col| {
            widths
                .iter()
                .skip(col)
                .step_by(cols)
                .copied()
                .fold(0.0, f64::max)
        })
        .collect();
    let (offset, spacing) = (BUTTON_TEXT_OFFSET / axes_width, COLUMN_SPACING / axes_width);
    let mut column_x = vec![LEFT_MARGIN / axes_width];
    for width in &column_widths[..cols.saturating_sub(1)] {
        let last = column_x[column_x.len() - 1];
        column_x.push(last + offset + width + spacing);
    }
    let ys = linspace(1.0, 0.0, rows + 2);
    Ok(texts
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let centre = Point::new(column_x[i % cols], ys[1 + i / cols]);
            text.set_position(Point::new(centre.x + offset, centre.y));
            text.set_clip_on(true);
            centre
        })
        .collect())
}
