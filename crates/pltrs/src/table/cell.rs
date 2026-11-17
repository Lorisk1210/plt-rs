//! Table cells (matplotlib's `table.Cell`): a rectangle with some of its
//! edges drawn and a text inside.

use std::fmt;
use std::str::FromStr;

use pltrs_core::{Affine2D, Bbox, Path, PathCode, Point};
use pltrs_render::GraphicsContext;
use pltrs_text::{BlockStyle, HAlign, VAlign};

use super::TableError;
use super::layout::AxesMaps;
use crate::artist::{ArtistBase, DrawContext, LayoutContext};
use crate::colors::Color;
use crate::fonts::{FontProperties, FontSize};
use crate::patches::{PatchParts, PatchShape, PatchStyle, RotationPoint};
use crate::rc::RcParams;
use crate::text::{LaidOutText, TextDrawError, TextOwner, draw_laid_out, lay_out};
use crate::transforms::Transform;

/// Padding between a cell's text and its edges, as a fraction of the cell
/// width.
const PAD: f64 = 0.1;

/// Which edges of a cell are drawn (matplotlib's `visible_edges`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one independent flag per side of the cell"
)]
pub struct CellEdges {
    /// The bottom edge.
    pub bottom: bool,
    /// The right edge.
    pub right: bool,
    /// The top edge.
    pub top: bool,
    /// The left edge.
    pub left: bool,
}

pltrs_core::archive_struct!(CellEdges {
    bottom,
    right,
    top,
    left
});

impl CellEdges {
    /// Every edge (`"closed"`).
    pub const CLOSED: Self = Self {
        bottom: true,
        right: true,
        top: true,
        left: true,
    };

    /// No edge (`"open"`).
    pub const OPEN: Self = Self {
        bottom: false,
        right: false,
        top: false,
        left: false,
    };

    /// In matplotlib's order: bottom, right, top, left.
    fn in_order(self) -> [bool; 4] {
        [self.bottom, self.right, self.top, self.left]
    }

    /// The outline of a cell: the unit square from its lower left corner,
    /// counter-clockwise, drawing only these edges and closed when it
    /// draws all four (matplotlib's `Cell.get_path`).
    ///
    /// ```
    /// use pltrs::table::CellEdges;
    /// use pltrs_core::PathCode::{ClosePoly, LineTo, MoveTo};
    /// assert_eq!(
    ///     CellEdges::CLOSED.outline().resolved_codes(),
    ///     [MoveTo, LineTo, LineTo, LineTo, ClosePoly]
    /// );
    /// let top_only: CellEdges = "T".parse()?;
    /// assert_eq!(
    ///     top_only.outline().resolved_codes(),
    ///     [MoveTo, MoveTo, MoveTo, LineTo, MoveTo]
    /// );
    /// # Ok::<(), pltrs::table::TableError>(())
    /// ```
    pub fn outline(self) -> Path {
        let mut codes = vec![PathCode::MoveTo];
        codes.extend(self.in_order().map(|visible| {
            if visible {
                PathCode::LineTo
            } else {
                PathCode::MoveTo
            }
        }));
        if !codes[1..].contains(&PathCode::MoveTo) {
            codes[4] = PathCode::ClosePoly;
        }
        let p = Point::new;
        let square = vec![
            p(0.0, 0.0),
            p(1.0, 0.0),
            p(1.0, 1.0),
            p(0.0, 1.0),
            p(0.0, 0.0),
        ];
        Path::new(square, codes).expect("the cell outline has five vertices")
    }
}

impl Default for CellEdges {
    fn default() -> Self {
        Self::CLOSED
    }
}

impl FromStr for CellEdges {
    type Err = TableError;

    /// Parses `"closed"`, `"open"`, `"horizontal"` (bottom and top),
    /// `"vertical"` (right and left) or any combination of the letters
    /// `B`, `R`, `T` and `L`.
    fn from_str(text: &str) -> Result<Self, TableError> {
        Ok(match text {
            "closed" => Self::CLOSED,
            "open" => Self::OPEN,
            "horizontal" => Self {
                bottom: true,
                top: true,
                ..Self::OPEN
            },
            "vertical" => Self {
                right: true,
                left: true,
                ..Self::OPEN
            },
            letters => {
                let mut edges = Self::OPEN;
                for letter in letters.chars() {
                    match letter {
                        'B' => edges.bottom = true,
                        'R' => edges.right = true,
                        'T' => edges.top = true,
                        'L' => edges.left = true,
                        _ => return Err(TableError::Edges(text.to_owned())),
                    }
                }
                edges
            }
        })
    }
}

impl fmt::Display for CellEdges {
    /// The letters of the visible edges in matplotlib's order, a substring
    /// of `BRTL` (what matplotlib's `Cell.visible_edges` reads).
    ///
    /// ```
    /// use pltrs::table::CellEdges;
    /// assert_eq!(CellEdges::CLOSED.to_string(), "BRTL");
    /// assert_eq!(CellEdges::OPEN.to_string(), "");
    /// assert_eq!("LT".parse::<CellEdges>()?.to_string(), "TL");
    /// # Ok::<(), pltrs::table::TableError>(())
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.in_order()
            .iter()
            .zip(['B', 'R', 'T', 'L'])
            .filter(|(visible, _)| **visible)
            .try_for_each(|(_, letter)| write!(f, "{letter}"))
    }
}

/// The outline of a cell: the unit square with only its visible edges
/// drawn (`Cell.get_path`), placed on the cell's box and rotated as a
/// rectangle (matplotlib's `Cell` is a `Rectangle`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CellShape {
    /// The cell in the coordinates it is drawn in.
    pub(crate) bbox: Bbox,
    pub(crate) edges: CellEdges,
    /// The counter-clockwise rotation in degrees.
    pub(crate) angle: f64,
    /// Where the cell rotates around.
    pub(crate) rotation_point: RotationPoint,
}

pltrs_core::archive_struct!(CellShape {
    bbox,
    edges,
    angle,
    rotation_point
});

impl PatchShape for CellShape {
    fn path(&self) -> Path {
        self.edges.outline()
    }

    fn patch_transform(&self) -> Affine2D {
        self.rotation_point.rotated_box(&self.bbox, self.angle, 1.0)
    }
}

/// One cell of a table.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Cell {
    /// Width in axes fractions: as given, until the table placing the
    /// cell sets it (matplotlib's `_update_positions`, when the table is
    /// drawn or measured, writes the widths of fitted columns and of a
    /// table stretched over its box into its cells).
    pub(crate) width: std::cell::Cell<f64>,
    /// Height in axes fractions, as given until the table sets it.
    pub(crate) height: std::cell::Cell<f64>,
    /// Whether the width is the one the table set since it was given (a
    /// fitted column, or a table stretched over its box), which
    /// matplotlib computes as a numpy float.
    pub(crate) width_fitted: std::cell::Cell<bool>,
    /// Whether the height is the one the table set since it was given (a
    /// table stretched over its box).
    pub(crate) height_fitted: std::cell::Cell<bool>,
    pub(crate) text: String,
    /// The look of the cell's rectangle (matplotlib's `Cell` is a
    /// `Rectangle`): its face, edges, line and hatch.
    pub(crate) style: PatchStyle,
    /// The artist properties of the rectangle: whether the cell is drawn,
    /// the opacity of its face and edges (the text keeps its own), its
    /// effects, sketch, snapping, gid and url. Its transform and parent
    /// are those of its table, and it is not clipped unless asked
    /// (matplotlib's `Cell` sets `clip_on` off).
    pub(crate) base: ArtistBase,
    /// Horizontal alignment of the text, which also sets where it sits.
    pub(crate) loc: HAlign,
    pub(crate) edges: CellEdges,
    /// The font of the text as given; see [`Cell::fontsize`] for its size.
    font: FontProperties,
    /// The size the table last fitted the text to when it was drawn or
    /// measured (matplotlib's `Table._auto_set_font_size` sets every
    /// cell's font to it), which its next fitting starts from; `None`
    /// before, and once a font is given again.
    fitted_size: std::cell::Cell<Option<f64>>,
    pub(crate) text_color: Color,
    /// The vertical alignment of the text at the middle of the cell
    /// (centred, as matplotlib's `Cell` makes its text).
    pub(crate) text_valign: VAlign,
    /// The rotation of the text in degrees.
    pub(crate) text_rotation: f64,
    /// The opacity of the text.
    pub(crate) text_alpha: Option<f64>,
    /// The coordinates the cell is drawn in, its place in the table being
    /// in them (matplotlib's `Cell.set_transform`); `None` draws it in the
    /// fractions of the table's axes, as the table sets it.
    pub(crate) transform: Option<Transform>,
    /// The counter-clockwise rotation of the rectangle in degrees, its
    /// text staying upright.
    pub(crate) angle: f64,
    /// Where the rectangle rotates around.
    pub(crate) rotation_point: RotationPoint,
}

pltrs_core::archive_struct!(Cell {
    width,
    height,
    width_fitted,
    height_fitted,
    text,
    style,
    base,
    loc,
    edges,
    font,
    fitted_size,
    text_color,
    text_valign,
    text_rotation,
    text_alpha,
    transform,
    angle,
    rotation_point
});

impl Cell {
    /// A `width` × `height` cell with matplotlib's defaults: no text
    /// (aligned right), all four edges drawn, and the face and edge
    /// colours (`pltrs.table.*`: white and black), font and text colour of
    /// `rc` (`Cell(xy, width, height)`).
    pub(crate) fn new(width: f64, height: f64, rc: &RcParams) -> Self {
        let mut style = PatchStyle::from_rc(rc);
        style.facecolor = Some(rc.pltrs_table_facecolor.clone());
        style.edgecolor = Some(rc.pltrs_table_edgecolor.clone());
        let mut base = ArtistBase::with_zorder(1.0);
        base.clip_on = false;
        Self {
            width: std::cell::Cell::new(width),
            height: std::cell::Cell::new(height),
            width_fitted: std::cell::Cell::new(false),
            height_fitted: std::cell::Cell::new(false),
            text: String::new(),
            style,
            base,
            loc: HAlign::Right,
            edges: CellEdges::CLOSED,
            font: FontProperties::from_rc(rc),
            fitted_size: std::cell::Cell::new(None),
            text_color: rc.text_color.clone(),
            text_valign: VAlign::Center,
            text_rotation: 0.0,
            text_alpha: None,
            transform: None,
            angle: 0.0,
            rotation_point: RotationPoint::Xy,
        }
    }

    /// The size of the text in points: the one given, or the one the
    /// table fitted it to since (matplotlib's `Cell.get_fontsize`).
    pub(crate) fn fontsize(&self) -> f64 {
        self.fitted_size.get().unwrap_or(self.font.size)
    }

    /// The font of the text, at its [`fontsize`](Self::fontsize).
    pub(crate) fn font(&self) -> FontProperties {
        FontProperties {
            size: self.fontsize(),
            ..self.font.clone()
        }
    }

    /// Gives the text `font`, its size included.
    pub(crate) fn set_font(&mut self, font: FontProperties) {
        self.font = font;
        self.fitted_size.set(None);
    }

    /// Gives the text a size of `size` points (`base` being the size of
    /// the relative ones).
    pub(crate) fn set_fontsize(&mut self, size: f64, base: f64) {
        self.font.set_size(FontSize::Points(size), base);
        self.fitted_size.set(None);
    }

    /// Remembers that the table fitted the text to `size` points.
    pub(crate) fn fitted(&self, size: f64) {
        self.fitted_size.set(Some(size));
    }

    /// The outline of the cell placed at `bbox`, in the coordinates it is
    /// drawn in.
    pub(crate) fn shape_at(&self, bbox: &Bbox) -> CellShape {
        CellShape {
            bbox: *bbox,
            edges: self.edges,
            angle: self.angle,
            rotation_point: self.rotation_point,
        }
    }

    /// The display box of the cell placed at `bbox` in the coordinates
    /// `to_display` maps (matplotlib's `Cell.get_window_extent`: the
    /// corners of the unit square through the cell's patch transform,
    /// rotation included, then the cell's transform).
    pub(crate) fn display_box(&self, bbox: &Bbox, to_display: &Affine2D) -> Bbox {
        let to_pixels = self.shape_at(bbox).patch_transform().then(*to_display);
        let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
            .map(|(x, y)| to_pixels.transform_point(Point::new(x, y)));
        let (mut x0, mut y0, mut x1, mut y1) = (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        );
        for corner in corners {
            x0 = x0.min(corner.x);
            y0 = y0.min(corner.y);
            x1 = x1.max(corner.x);
            y1 = y1.max(corner.y);
        }
        Bbox::from_extents(x0, y0, x1, y1)
    }

    /// The maps between the cell's coordinates and display pixels: those
    /// of its transform, else the table's `maps`; `None` for a transform
    /// that cannot be resolved, is not affine or not invertible.
    pub(crate) fn maps(&self, layout: LayoutContext<'_>, maps: &AxesMaps) -> Option<AxesMaps> {
        match &self.transform {
            Some(transform) => AxesMaps::through(layout, transform),
            None => Some(maps.clone()),
        }
    }

    /// The cell's text laid out at `fontsize` points.
    fn laid_out(
        &self,
        fontsize: f64,
        dpi: f64,
        rc: &RcParams,
    ) -> Result<LaidOutText, TextDrawError> {
        let font = self
            .font
            .clone()
            .with_size(FontSize::Points(fontsize), rc.font_size);
        let style = BlockStyle {
            halign: self.loc,
            valign: self.text_valign,
            multialign: self.loc,
            rotation: self.text_rotation,
            ..BlockStyle::default()
        };
        Ok(lay_out(&self.text, &font, &style, dpi, rc)?)
    }

    /// The width the text needs at `fontsize` with its padding, in axes
    /// fractions of the axes `to_axes` maps display pixels to
    /// (`Cell.get_required_width`: the text's box where it starts, at the
    /// origin, mapped back to axes fractions).
    pub(crate) fn required_width(
        &self,
        fontsize: f64,
        layout: LayoutContext<'_>,
        rc: &RcParams,
        to_axes: &Affine2D,
    ) -> Result<f64, TextDrawError> {
        let width = if self.text.is_empty() {
            0.0
        } else {
            let text = self.laid_out(fontsize, layout.dpi(), rc)?.block.bbox;
            text.transformed(to_axes).width()
        };
        Ok(width * (1.0 + 2.0 * PAD))
    }

    /// Draws the cell placed at `placed` (in the table's axes fractions,
    /// which map to display pixels by `to_display`) with its text
    /// (`Cell.draw`), in `table`, the text being `owner`.
    pub(crate) fn draw(
        &self,
        ctx: &mut DrawContext<'_>,
        table: &ArtistBase,
        rc: &RcParams,
        placed: &PlacedCell,
        to_display: &Affine2D,
        owner: TextOwner,
    ) -> Result<(), TextDrawError> {
        if !self.base.visible {
            return Ok(());
        }
        let mut base = self.base.clone();
        base.parent = table.parent;
        base.transform = Some(Transform::Affine(*to_display));
        let shape = self.shape_at(&placed.bbox);
        PatchParts {
            base: &base,
            style: &self.style,
            rc,
            shape: &shape,
            interpolation_steps: 1,
            drawn: None,
            program_path: None,
        }
        .paint(ctx)
        .map_err(TextDrawError::Box)?;
        if self.text.is_empty() {
            return Ok(());
        }
        let anchor = self.text_anchor(&self.display_box(&placed.bbox, to_display));
        let laid_out = self.laid_out(placed.fontsize, ctx.layout().dpi(), rc)?;
        let gc = GraphicsContext {
            foreground: self.text_color.resolve()?,
            alpha: self.text_alpha,
            antialiased: rc.text_antialiased,
            ..GraphicsContext::default()
        };
        // The cell's text draws in a group of its own, as matplotlib's
        // `Text.draw` opens one.
        ctx.renderer().open_group("text", None);
        let drawn = draw_laid_out(ctx, &gc, &laid_out, anchor, 0.0, owner);
        ctx.renderer().close_group("text");
        drawn
    }

    /// Where the text of the cell drawn at `display` (display pixels) is
    /// anchored: centred vertically, and across as its alignment says,
    /// inside the padding (`Cell._set_text_position`).
    fn text_anchor(&self, display: &Bbox) -> Point {
        let y = display.y0() + display.height() / 2.0;
        let x = match self.loc {
            HAlign::Center => display.x0() + display.width() / 2.0,
            HAlign::Left => display.x0() + display.width() * PAD,
            HAlign::Right => display.x0() + display.width() * (1.0 - PAD),
        };
        Point::new(x, y)
    }

    /// The box of the text of the cell placed at `placed`, in the axes
    /// fractions `maps` map (`Cell.get_text_bounds`); an empty text has
    /// the box of no size at its anchor.
    pub(crate) fn text_box(
        &self,
        placed: &PlacedCell,
        maps: &AxesMaps,
        layout: LayoutContext<'_>,
        rc: &RcParams,
    ) -> Result<Bbox, TextDrawError> {
        let anchor = self.text_anchor(&self.display_box(&placed.bbox, &maps.to_display));
        let text = if self.text.is_empty() {
            Bbox::from_bounds(anchor.x, anchor.y, 0.0, 0.0)
        } else {
            self.laid_out(placed.fontsize, layout.dpi(), rc)?
                .block
                .bbox
                .translated(anchor.x, anchor.y)
        };
        Ok(text.transformed(&maps.to_axes))
    }
}

/// A cell placed by the table layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PlacedCell {
    /// The cell in the coordinates it is drawn in.
    pub(crate) bbox: Bbox,
    /// The width and height the table gave the cell, as computed (the
    /// box's own differ from them by their rounding).
    pub(crate) size: (f64, f64),
    /// The text size in points.
    pub(crate) fontsize: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_names_and_letters() {
        assert_eq!("closed".parse(), Ok(CellEdges::CLOSED));
        assert_eq!("open".parse(), Ok(CellEdges::OPEN));
        let horizontal: CellEdges = "horizontal".parse().unwrap();
        assert_eq!(horizontal, "BT".parse().unwrap());
        assert!(horizontal.bottom && horizontal.top && !horizontal.left);
        assert!("BX".parse::<CellEdges>().is_err());
        assert_eq!("".parse(), Ok(CellEdges::OPEN));
    }

    #[test]
    fn outlines_skip_hidden_edges() {
        let shape = |edges| CellShape {
            bbox: Bbox::UNIT,
            edges,
            angle: 0.0,
            rotation_point: RotationPoint::Xy,
        };
        let closed = shape(CellEdges::CLOSED).path();
        assert_eq!(closed.code_at(4), PathCode::ClosePoly);
        let horizontal = shape("horizontal".parse().unwrap()).path();
        assert_eq!(
            horizontal.codes().unwrap(),
            [
                PathCode::MoveTo,
                PathCode::LineTo,
                PathCode::MoveTo,
                PathCode::LineTo,
                PathCode::MoveTo
            ]
        );
    }
}
