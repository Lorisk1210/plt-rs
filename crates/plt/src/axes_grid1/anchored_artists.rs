//! Ready-made anchored boxes (matplotlib's
//! `mpl_toolkits.axes_grid1.anchored_artists`): a scale bar, a pair of
//! direction arrows, and anchored drawing areas, aux-transform boxes and
//! ellipses.
//!
//! Each is a description with matplotlib's defaults that becomes an
//! [`AnchoredBox`], added with
//! [`Axes::add_anchored_box`](crate::axes::Axes::add_anchored_box):
//!
//! ```
//! use plt::prelude::*;
//! use plt::axes_grid1::anchored_artists::AnchoredSizeBar;
//! use plt::offsetbox::AnchorLoc;
//! let ax = Figure::new().subplot();
//! ax.set_xlim(0.0, 10.0)?;
//! let bar = ax.add_anchored_box(AnchoredSizeBar::new(ax.trans_data(), 1.0, "1 unit", AnchorLoc::LowerLeft));
//! assert_eq!(bar.texts()[0].text(), "1 unit");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use plt_core::{Affine2D, Point};

use crate::colors::Color;
use crate::fonts::FontProperties;
use crate::offsetbox::{
    AnchorLoc, AnchoredBox, AnchoredOptions, AuxTransformBox, BoxItem, DrawingArea, OffsetBox,
    PackAlign, Packer, TextArea,
};
use crate::patches::{ArrowStyle, Ellipse, FancyArrowPatch, PatchProps, PathShape, Rectangle};
use crate::text::{TextError, TextProps, text_path};
use crate::transforms::Transform;

/// The options of an anchored box with the given paddings, frame and
/// padding font.
fn anchored(
    loc: AnchorLoc,
    pad: f64,
    borderpad: f64,
    prop: Option<FontProperties>,
    frameon: bool,
) -> AnchoredOptions {
    AnchoredOptions {
        pad,
        borderpad: (borderpad, borderpad),
        prop,
        frameon,
        ..AnchoredOptions::new(loc)
    }
}

/// A horizontal scale bar with a centred label below (or above) it
/// (matplotlib's `AnchoredSizeBar`).
#[derive(Clone, Debug, PartialEq)]
pub struct AnchoredSizeBar {
    /// The coordinates of `size` and `size_vertical` (usually the data
    /// coordinates of the axes).
    pub transform: Transform,
    /// Length of the bar.
    pub size: f64,
    /// The label.
    pub label: String,
    /// Where the bar goes.
    pub loc: AnchorLoc,
    /// Padding around the bar and label, as a fraction of the font size.
    pub pad: f64,
    /// Padding outside the frame, as a fraction of the font size.
    pub borderpad: f64,
    /// Space between the bar and the label, in points.
    pub sep: f64,
    /// Whether the frame is drawn.
    pub frameon: bool,
    /// Height of the bar.
    pub size_vertical: f64,
    /// Colour of the bar and the label.
    pub color: Color,
    /// Whether the label goes above the bar.
    pub label_top: bool,
    /// The label's font, also the reference of the paddings (`None` = the
    /// default font, paddings relative to `legend.fontsize`).
    pub fontproperties: Option<FontProperties>,
    /// Whether the bar is filled (`None` = when it has a height).
    pub fill_bar: Option<bool>,
}

impl AnchoredSizeBar {
    /// A black bar `size` long in `transform` coordinates, labelled
    /// `label`, with matplotlib's defaults.
    pub fn new(transform: Transform, size: f64, label: impl Into<String>, loc: AnchorLoc) -> Self {
        Self {
            transform,
            size,
            label: label.into(),
            loc,
            pad: 0.1,
            borderpad: 0.1,
            sep: 2.0,
            frameon: true,
            size_vertical: 0.0,
            color: Color::from("black"),
            label_top: false,
            fontproperties: None,
            fill_bar: None,
        }
    }
}

impl From<AnchoredSizeBar> for AnchoredBox {
    fn from(bar: AnchoredSizeBar) -> Self {
        let fill = bar.fill_bar.unwrap_or(bar.size_vertical > 0.0);
        let rectangle = BoxItem::patch(
            Rectangle::new((0.0, 0.0), bar.size, bar.size_vertical),
            PatchProps::default()
                .fill(fill)
                .facecolor(bar.color.clone())
                .edgecolor(bar.color.clone()),
        );
        let size_bar = AuxTransformBox::new(bar.transform).with_artist(rectangle);
        let mut textprops = TextProps::default().color(bar.color);
        if let Some(font) = &bar.fontproperties {
            textprops = textprops.font(font.clone());
        }
        let label = TextArea::from_text(bar.label, textprops);
        let children = if bar.label_top {
            vec![label.into(), size_bar.into()]
        } else {
            vec![size_bar.into(), label.into()]
        };
        let child = OffsetBox::VPacker(Packer::new(0.0, bar.sep, PackAlign::Center, children));
        Self::new(
            child,
            anchored(
                bar.loc,
                bar.pad,
                bar.borderpad,
                bar.fontproperties,
                bar.frameon,
            ),
        )
    }
}

/// Two perpendicular arrows with labels showing directions (matplotlib's
/// `AnchoredDirectionArrows`).
#[derive(Clone, Debug, PartialEq)]
pub struct AnchoredDirectionArrows {
    /// The coordinates of the lengths and font size (usually the axes
    /// fractions of the axes).
    pub transform: Transform,
    /// Label of the x arrow.
    pub label_x: String,
    /// Label of the y arrow.
    pub label_y: String,
    /// Length of the x arrow.
    pub length: f64,
    /// Size of the labels.
    pub fontsize: f64,
    /// Where the arrows go.
    pub loc: AnchorLoc,
    /// Rotation of the arrows, in degrees.
    pub angle: f64,
    /// Length of the y arrow relative to the x arrow (negative points it
    /// the other way).
    pub aspect_ratio: f64,
    /// Padding around the arrows and labels, as a fraction of the font
    /// size.
    pub pad: f64,
    /// Padding outside the frame, as a fraction of the font size.
    pub borderpad: f64,
    /// Whether the frame is drawn.
    pub frameon: bool,
    /// Colour of the arrows and labels.
    pub color: Color,
    /// Opacity of the arrows and labels.
    pub alpha: f64,
    /// Horizontal shift of the labels from the arrow tips.
    pub sep_x: f64,
    /// Vertical shift of the labels from the arrow tips.
    pub sep_y: f64,
    /// The labels' font (`None` = the default font).
    pub fontproperties: Option<FontProperties>,
    /// The part of each arrow behind their crossing, as a fraction.
    pub back_length: f64,
    /// Width of the heads (of the `simple` arrow style).
    pub head_width: f64,
    /// Length of the heads.
    pub head_length: f64,
    /// Width of the tails.
    pub tail_width: f64,
    /// Further properties of the label patches (their colour and opacity
    /// come first, so these override them).
    pub text_props: PatchProps,
    /// Further properties of the arrows (likewise).
    pub arrow_props: PatchProps,
}

impl AnchoredDirectionArrows {
    /// White arrows labelled `label_x` and `label_y` in `transform`
    /// coordinates, with matplotlib's defaults.
    pub fn new(
        transform: Transform,
        label_x: impl Into<String>,
        label_y: impl Into<String>,
    ) -> Self {
        Self {
            transform,
            label_x: label_x.into(),
            label_y: label_y.into(),
            length: 0.15,
            fontsize: 0.08,
            loc: AnchorLoc::UpperLeft,
            angle: 0.0,
            aspect_ratio: 1.0,
            pad: 0.4,
            borderpad: 0.4,
            frameon: false,
            color: Color::from("w"),
            alpha: 1.0,
            sep_x: 0.01,
            sep_y: 0.0,
            fontproperties: None,
            back_length: 0.15,
            head_width: 10.0,
            head_length: 15.0,
            tail_width: 2.0,
            text_props: PatchProps::default(),
            arrow_props: PatchProps::default(),
        }
    }

    /// The outline of `label` at `xy`, as a patch styled like the labels.
    fn label(&self, xy: Point, label: &str, props: &PatchProps) -> Result<BoxItem, TextError> {
        let font = self
            .fontproperties
            .clone()
            .unwrap_or_else(|| FontProperties::from_rc(&crate::rc::params()));
        let outline = text_path(xy, label, Some(self.fontsize), &font)?;
        Ok(BoxItem::patch(PathShape(outline), props.clone()))
    }
}

impl TryFrom<AnchoredDirectionArrows> for AnchoredBox {
    type Error = TextError;

    /// The arrows and labels in an aux-transform box rotated by `angle`.
    ///
    /// # Errors
    ///
    /// When a label holds malformed math.
    fn try_from(arrows: AnchoredDirectionArrows) -> Result<Self, TextError> {
        let colored = |props: &PatchProps| {
            PatchProps::default()
                .color(arrows.color.clone())
                .alpha(arrows.alpha)
                .overlay(props.clone())
        };
        let (arrow_props, text_props) = (colored(&arrows.arrow_props), colored(&arrows.text_props));
        let style = ArrowStyle::Simple {
            head_length: arrows.head_length,
            head_width: arrows.head_width,
            tail_width: arrows.tail_width,
        };
        let length_x = arrows.length;
        let length_y = arrows.length * arrows.aspect_ratio;
        let back = arrows.back_length;
        let arrow = |from: (f64, f64), to: (f64, f64)| {
            BoxItem::patch(
                FancyArrowPatch::between(from, to)
                    .with_arrowstyle(style)
                    .with_shrink(0.0, 0.0),
                arrow_props.clone(),
            )
        };
        let label_x = arrows.label(
            Point::new(length_x + arrows.sep_x, back * length_y + arrows.sep_y),
            &arrows.label_x,
            &text_props,
        )?;
        let label_y = arrows.label(
            Point::new(
                length_x * back + arrows.sep_x,
                length_y * (1.0 - back) + arrows.sep_y,
            ),
            &arrows.label_y,
            &text_props,
        )?;
        let rotated = arrows.transform.clone().then(Transform::Affine(
            Affine2D::identity().rotate_deg(arrows.angle),
        ));
        let child = AuxTransformBox::new(rotated)
            .with_artist(arrow((0.0, back * length_y), (length_x, back * length_y)))
            .with_artist(arrow((back * length_x, 0.0), (back * length_x, length_y)))
            .with_artist(label_x)
            .with_artist(label_y);
        Ok(Self::new(
            child,
            anchored(
                arrows.loc,
                arrows.pad,
                arrows.borderpad,
                None,
                arrows.frameon,
            ),
        ))
    }
}

/// An anchored drawing area of fixed size whose artists are placed in
/// points (matplotlib's `AnchoredDrawingArea`; matplotlib's docstring
/// says pixels, but drawing areas scale with the resolution).
#[derive(Debug)]
pub struct AnchoredDrawingArea {
    /// The drawing area, with its artists.
    pub drawing_area: DrawingArea<BoxItem>,
    /// Where the area goes.
    pub loc: AnchorLoc,
    /// Padding around the area, as a fraction of `legend.fontsize`.
    pub pad: f64,
    /// Padding outside the frame, as a fraction of `legend.fontsize`.
    pub borderpad: f64,
    /// Whether the frame is drawn.
    pub frameon: bool,
}

impl AnchoredDrawingArea {
    /// An empty, framed area of the given size and descents with
    /// matplotlib's paddings.
    pub fn new(width: f64, height: f64, xdescent: f64, ydescent: f64, loc: AnchorLoc) -> Self {
        Self {
            drawing_area: DrawingArea::new(width, height, xdescent, ydescent),
            loc,
            pad: 0.4,
            borderpad: 0.5,
            frameon: true,
        }
    }

    /// The same area with `artist` drawn last.
    #[must_use]
    pub fn with_artist(mut self, artist: BoxItem) -> Self {
        self.drawing_area.artists.push(artist);
        self
    }
}

impl From<AnchoredDrawingArea> for AnchoredBox {
    /// matplotlib ignores any padding font here: the paddings are
    /// fractions of `legend.fontsize`.
    fn from(area: AnchoredDrawingArea) -> Self {
        Self::new(
            area.drawing_area,
            anchored(area.loc, area.pad, area.borderpad, None, area.frameon),
        )
    }
}

/// An anchored box of artists in the coordinates of a transform, as large
/// as they are (matplotlib's `AnchoredAuxTransformBox`).
#[derive(Debug)]
pub struct AnchoredAuxTransformBox {
    /// The box, with its artists.
    pub drawing_area: AuxTransformBox<BoxItem>,
    /// Where the box goes.
    pub loc: AnchorLoc,
    /// Padding around the artists, as a fraction of the font size.
    pub pad: f64,
    /// Padding outside the frame, as a fraction of the font size.
    pub borderpad: f64,
    /// The font the paddings refer to (`None` = `legend.fontsize`).
    pub prop: Option<FontProperties>,
    /// Whether the frame is drawn.
    pub frameon: bool,
}

impl AnchoredAuxTransformBox {
    /// An empty, framed box for artists in `transform` coordinates with
    /// matplotlib's paddings.
    pub fn new(transform: Transform, loc: AnchorLoc) -> Self {
        Self {
            drawing_area: AuxTransformBox::new(transform),
            loc,
            pad: 0.4,
            borderpad: 0.5,
            prop: None,
            frameon: true,
        }
    }

    /// The same box with `artist` drawn last.
    #[must_use]
    pub fn with_artist(mut self, artist: BoxItem) -> Self {
        self.drawing_area.artists.push(artist);
        self
    }
}

impl From<AnchoredAuxTransformBox> for AnchoredBox {
    fn from(aux: AnchoredAuxTransformBox) -> Self {
        Self::new(
            aux.drawing_area,
            anchored(aux.loc, aux.pad, aux.borderpad, aux.prop, aux.frameon),
        )
    }
}

/// An anchored ellipse whose size is given in the coordinates of a
/// transform (matplotlib's `AnchoredEllipse`, removed in matplotlib 3.8
/// in favour of an [`AnchoredAuxTransformBox`] holding an ellipse, which
/// is what this builds).
#[derive(Clone, Debug, PartialEq)]
pub struct AnchoredEllipse {
    /// The coordinates of the ellipse.
    pub transform: Transform,
    /// Width of the ellipse.
    pub width: f64,
    /// Height of the ellipse.
    pub height: f64,
    /// Rotation of the ellipse, in degrees.
    pub angle: f64,
    /// Where the ellipse goes.
    pub loc: AnchorLoc,
    /// Padding around the ellipse, as a fraction of the font size.
    pub pad: f64,
    /// Padding outside the frame, as a fraction of the font size.
    pub borderpad: f64,
    /// The font the paddings refer to (`None` = `legend.fontsize`).
    pub prop: Option<FontProperties>,
    /// Whether the frame is drawn.
    pub frameon: bool,
    /// Colours, widths... of the ellipse.
    pub props: PatchProps,
}

impl AnchoredEllipse {
    /// A framed ellipse with matplotlib's paddings of 0.1.
    pub fn new(transform: Transform, width: f64, height: f64, angle: f64, loc: AnchorLoc) -> Self {
        Self {
            transform,
            width,
            height,
            angle,
            loc,
            pad: 0.1,
            borderpad: 0.1,
            prop: None,
            frameon: true,
            props: PatchProps::default(),
        }
    }
}

impl From<AnchoredEllipse> for AnchoredBox {
    fn from(ellipse: AnchoredEllipse) -> Self {
        let shape = Ellipse {
            angle: ellipse.angle,
            ..Ellipse::new((0.0, 0.0), ellipse.width, ellipse.height)
        };
        AnchoredAuxTransformBox {
            pad: ellipse.pad,
            borderpad: ellipse.borderpad,
            prop: ellipse.prop,
            frameon: ellipse.frameon,
            ..AnchoredAuxTransformBox::new(ellipse.transform, ellipse.loc)
        }
        .with_artist(BoxItem::patch(shape, ellipse.props))
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::offsetbox::AnchoredOffsetbox;
    use crate::testing::{RecordingRenderer, data_axes};

    /// The frame of `anchored` once the figure is drawn.
    fn framed(fig: &Figure, anchored: &AnchoredOffsetbox) -> plt_core::Bbox {
        let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
        fig.draw(&mut renderer).expect("drawable");
        anchored.window_extent().expect("an extent")
    }

    #[test]
    fn size_bars_put_the_label_below_or_above_the_bar() {
        let (fig, ax) = data_axes();
        let below = ax.add_anchored_box(AnchoredSizeBar {
            size_vertical: 1.0,
            ..AnchoredSizeBar::new(ax.trans_data(), 2.0, "2 units", AnchorLoc::LowerLeft)
        });
        let above = ax.add_anchored_box(AnchoredSizeBar {
            label_top: true,
            ..AnchoredSizeBar::new(ax.trans_data(), 2.0, "2 units", AnchorLoc::UpperLeft)
        });
        let frame = framed(&fig, &below);
        assert!(
            frame.width() > 2.0 * 49.6 && frame.height() > 36.96,
            "{frame:?}"
        );
        let [bar, label] = parts(&fig, &below);
        assert!((bar.height() - 36.96).abs() < 1e-9 && label.y1() <= bar.y0());
        let [label, bar] = parts(&fig, &above);
        assert!(label.y0() >= bar.y1(), "the label is on top");
    }

    /// The extents of the two parts of a size bar, in box order.
    fn parts(fig: &Figure, anchored: &AnchoredOffsetbox) -> [plt_core::Bbox; 2] {
        let ids: Vec<_> = anchored
            .raw()
            .with::<crate::offsetbox::AnchoredBoxArtist, _>(|a| {
                a.child.artists().into_iter().copied().collect()
            });
        let state = fig.raw().figure_state().borrow();
        let ctx = crate::artist::LayoutContext::new(&state);
        [ids[0], ids[1]].map(|id| {
            state
                .store
                .get(id)
                .and_then(|artist| artist.window_extent(&ctx))
                .expect("both parts have an extent")
        })
    }

    #[test]
    fn direction_arrows_hold_two_arrows_and_two_labels() {
        let (fig, ax) = data_axes();
        let arrows = AnchoredDirectionArrows::new(ax.trans_axes(), "x", "y");
        let anchored = ax.add_anchored_box(AnchoredBox::try_from(arrows).expect("valid labels"));
        let extent = framed(&fig, &anchored);
        // The arrows are 15 % of the axes long, plus the labels.
        assert!(extent.width() > 0.15 * 496.0, "{extent:?}");
        assert!(!anchored.patch().visible(), "unframed by default");
        let count = anchored
            .raw()
            .with::<crate::offsetbox::AnchoredBoxArtist, _>(|a| a.child.artists().len());
        assert_eq!(count, 4);
        let broken = AnchoredDirectionArrows::new(ax.trans_axes(), r"$\frac$", "y");
        assert!(AnchoredBox::try_from(broken).is_err());
    }

    #[test]
    fn drawing_areas_and_ellipses_take_their_own_size() {
        let (fig, ax) = data_axes();
        let area = AnchoredDrawingArea {
            pad: 0.0,
            borderpad: 0.0,
            ..AnchoredDrawingArea::new(36.0, 18.0, 0.0, 0.0, AnchorLoc::UpperRight)
        };
        let area = ax.add_anchored_box(area);
        let frame = framed(&fig, &area);
        assert!((frame.width() - 50.0).abs() < 1e-9 && (frame.height() - 25.0).abs() < 1e-9);
        assert!((frame.x1() - 576.0).abs() < 1e-9, "{frame:?}");
        let ellipse = AnchoredEllipse {
            pad: 0.0,
            ..AnchoredEllipse::new(ax.trans_data(), 2.0, 1.0, 0.0, AnchorLoc::LowerLeft)
        };
        let ellipse = ax.add_anchored_box(ellipse);
        let frame = framed(&fig, &ellipse);
        assert!((frame.width() - 2.0 * 49.6).abs() < 1e-6, "{frame:?}");
        assert!((frame.height() - 36.96).abs() < 1e-6);
    }
}
