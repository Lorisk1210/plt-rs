//! Four image axes showing an RGB image beside its red, green and blue
//! channels (matplotlib's `axes_grid1.axes_rgb`).

use super::divider::{DividerAspect, DividerId, DividerRect, DividerState};
use super::size::Size;
use crate::artist::{Artist, ArtistHandle};
use crate::axes::{Axes, AxesState, AxisName, ImshowOptions, TickParams};
use crate::colors::Color;
use crate::figure::{Figure, insert_axes};
use crate::image::{AxesImage, ImageData, ImageError};
use crate::numeric::Grid;

/// The axes of an RGB image and of its channels, laid out by a divider of
/// the image axes' position (matplotlib's `RGBAxes`):
///
/// ```text
/// ┌───────────────┬─────┐
/// │               │  R  │
/// │               ├─────┤
/// │      RGB      │  G  │
/// │               ├─────┤
/// │               │  B  │
/// └───────────────┴─────┘
/// ```
///
/// The channel axes share both axes with the image axes and show no tick
/// labels; every axes has white spines and ticks.
#[derive(Clone, Debug, PartialEq)]
pub struct RgbAxes {
    /// The axes of the three-channel image.
    pub rgb: Axes,
    /// The axes of the red channel.
    pub r: Axes,
    /// The axes of the green channel.
    pub g: Axes,
    /// The axes of the blue channel.
    pub b: Axes,
}

impl RgbAxes {
    /// The four axes in `rect` (`[left, bottom, width, height]` in figure
    /// fractions), the channel axes `pad` image heights apart (matplotlib's
    /// `RGBAxes(fig, rect, pad=pad)`, which lays them out with
    /// [`make_rgb_axes`]).
    ///
    /// ```
    /// use plt::axes::ImshowOptions;
    /// use plt::axes_grid1::RgbAxes;
    /// use plt::numeric::Grid;
    /// use plt::prelude::*;
    /// let fig = Figure::new();
    /// let axes = RgbAxes::new(&fig, [0.1, 0.1, 0.8, 0.8], 0.0);
    /// let ramp = Grid::from_fn(4, 4, |r, c| (r + c) as f64 / 6.0);
    /// axes.imshow_rgb(&ramp, &ramp, &ramp, ImshowOptions::default)?;
    /// assert_eq!(fig.axes().len(), 4);
    /// # Ok::<(), plt::image::ImageError>(())
    /// ```
    pub fn new(fig: &Figure, rect: [f64; 4], pad: f64) -> Self {
        let rgb = fig.add_axes(rect);
        let [r, g, b] = make_rgb_axes(&rgb, pad);
        let all = Self { rgb, r, g, b };
        for axes in all.axes() {
            for spine in axes.spines().iter() {
                spine.set_color("w");
            }
            // Only the edges of the major tick marks turn white
            // (matplotlib's `major_ticks.set_markeredgecolor("w")`).
            axes.raw().with_mut::<AxesState, _>(|state| {
                for name in [AxisName::X, AxisName::Y] {
                    state.axis_mut(name).major_style.edgecolor = Some(Color::from("w"));
                }
            });
        }
        all
    }

    /// The image, red, green and blue axes.
    pub fn axes(&self) -> [&Axes; 4] {
        [&self.rgb, &self.r, &self.g, &self.b]
    }

    /// Shows the image of channels `r`, `g` and `b` (floats in `[0, 1]`)
    /// and each channel alone, each image with the options `options`
    /// makes (matplotlib's `imshow_rgb`, which passes its keyword
    /// arguments to all four `imshow` calls).
    ///
    /// # Errors
    ///
    /// When the channels differ in shape, or an image cannot be made.
    pub fn imshow_rgb(
        &self,
        r: &Grid<f64>,
        g: &Grid<f64>,
        b: &Grid<f64>,
        options: impl Fn() -> ImshowOptions,
    ) -> Result<[AxesImage; 4], ImageError> {
        let (rows, cols) = r.shape();
        for (what, channel) in [("green channel", g), ("blue channel", b)] {
            if channel.shape() != (rows, cols) {
                return Err(ImageError::ShapeMismatch {
                    what,
                    expected: (rows, cols),
                    actual: channel.shape(),
                });
            }
        }
        let pixels = |pick: [bool; 3]| {
            Grid::from_fn(rows, cols, |i, j| {
                let value = |on: bool, grid: &Grid<f64>| if on { grid[(i, j)] } else { 0.0 };
                [value(pick[0], r), value(pick[1], g), value(pick[2], b)]
            })
        };
        let show = |axes: &Axes, pick| axes.imshow(ImageData::Rgb(pixels(pick)), options());
        Ok([
            show(&self.rgb, [true, true, true])?,
            show(&self.r, [true, false, false])?,
            show(&self.g, [false, true, false])?,
            show(&self.b, [false, false, true])?,
        ])
    }
}

/// Divides the area of `ax` into the image axes and, on its right, three
/// axes for the red, green and blue channels stacked top to bottom, `pad`
/// image heights apart; the channel axes share both axes with `ax` and
/// show no tick labels (matplotlib's `make_rgb_axes`). Returns the red,
/// green and blue axes.
///
/// ```
/// use plt::axes_grid1::make_rgb_axes;
/// use plt::prelude::*;
/// let fig = Figure::new();
/// let ax = fig.subplot();
/// let [r, _, b] = make_rgb_axes(&ax, 0.02);
/// let (x, _, w, _) = ax.position().bounds();
/// assert!(r.position().bounds().0 >= x + w, "beside the image");
/// assert!(b.position().bounds().1 < r.position().bounds().1, "blue under red");
/// assert_eq!(fig.axes().len(), 4);
/// ```
pub fn make_rgb_axes(ax: &Axes, pad: f64) -> [Axes; 3] {
    let figure = ax.raw().figure_state();
    let (frame, position) = {
        let state = figure.borrow();
        let axes = state.store.get_as::<AxesState>(ax.id());
        (
            axes.and_then(|a| a.base().parent).unwrap_or(state.root),
            axes.map_or(plt_core::Bbox::UNIT, |a| a.position),
        )
    };
    let channel = 1.0 - 2.0 * pad;
    let mut divider = DividerState::new(
        frame,
        DividerRect::Axes(ax.id()),
        DividerAspect::OfAxes(ax.id()),
    );
    let pad_size = Size::axes_y(ax.id()).times(pad);
    let height = Size::axes_y(ax.id()).times(channel / 3.0);
    divider.horizontal = vec![
        Size::axes_x(ax.id()),
        pad_size.clone(),
        Size::axes_x(ax.id()).times(channel / 3.0),
    ];
    divider.vertical = vec![
        height.clone(),
        pad_size.clone(),
        height.clone(),
        pad_size,
        height,
    ];
    let id = DividerId(figure.borrow().layout.dividers.len());
    let main = divider.cell(id, (0, 1), (0, -1));
    let channels = [4, 2, 0].map(|ny| divider.cell(id, (2, 3), (ny, ny + 1)));
    {
        let mut state = figure.borrow_mut();
        state.add_divider(divider);
        state.set_divider_cell(ax.id(), main);
    }
    channels.map(|cell| {
        let axes = insert_axes(figure, frame, position, None);
        figure.borrow_mut().set_divider_cell(axes.id(), cell);
        // Sharing with a fresh axes cannot form a cycle.
        axes.share_x(ax).expect("a fresh axes shares freely");
        axes.share_y(ax).expect("a fresh axes shares freely");
        axes.tick_params(&TickParams {
            labelbottom: Some(false),
            labelleft: Some(false),
            ..TickParams::default()
        });
        axes
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_stack_beside_the_image() {
        let fig = Figure::with_size(5.0, 4.0, 100.0);
        let axes = RgbAxes::new(&fig, [0.1, 0.1, 0.8, 0.8], 0.0);
        axes.rgb.set_xlim(0.0, 3.0).set_ylim(0.0, 3.0);
        let bounds = |ax: &Axes| {
            let (x, y, w, h) = ax.position().bounds();
            [x, y, w, h]
        };
        let [x, y, w, h] = bounds(&axes.rgb);
        let [rx, ry, rw, rh] = bounds(&axes.r);
        let [bx, by, ..] = bounds(&axes.b);
        assert!((rx - (x + w)).abs() < 1e-12, "right of the image");
        assert!((rw - w / 3.0).abs() < 1e-12 && (rh - h / 3.0).abs() < 1e-12);
        assert!((ry + rh - (y + h)).abs() < 1e-12, "red on top");
        assert!(
            (bx - rx).abs() < 1e-12 && (by - y).abs() < 1e-12,
            "blue at the bottom"
        );
        let edge = axes
            .g
            .raw()
            .with::<AxesState, _>(|state| state.axis(AxisName::Y).major_style.edgecolor.clone());
        assert_eq!(
            edge,
            Some(Color::from("w")),
            "white tick edges, black faces"
        );
        let small = Grid::from_fn(2, 2, |_, _| 0.5);
        let other = Grid::from_fn(3, 2, |_, _| 0.5);
        assert!(
            axes.imshow_rgb(&small, &other, &small, ImshowOptions::default)
                .is_err()
        );
    }
}
