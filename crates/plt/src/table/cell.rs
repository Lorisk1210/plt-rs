//! Table cells (matplotlib's `table.Cell`): a rectangle with some of its
//! edges drawn and a text inside.

use std::str::FromStr;

use plt_core::{Affine2D, Bbox, Path, PathCode, Point};
use plt_render::GraphicsContext;
use plt_text::{BlockStyle, HAlign, VAlign};

use super::TableError;
use crate::artist::{ArtistBase, DrawContext, LayoutContext};
use crate::colors::Color;
use crate::fonts::{FontProperties, FontSize};
use crate::patches::{PatchParts, PatchShape, PatchStyle};
use crate::rc::RcParams;
use crate::text::{LaidOutText, TextDrawError, draw_laid_out, lay_out};
use crate::transforms::{Transform, unit_to_bbox};

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
    /// use plt::table::CellEdges;
    /// use plt_core::PathCode::{ClosePoly, LineTo, MoveTo};
    /// assert_eq!(
    ///     CellEdges::CLOSED.outline().resolved_codes(),
    ///     [MoveTo, LineTo, LineTo, LineTo, ClosePoly]
    /// );
    /// let top_only: CellEdges = "T".parse()?;
    /// assert_eq!(
    ///     top_only.outline().resolved_codes(),
    ///     [MoveTo, MoveTo, MoveTo, LineTo, MoveTo]
    /// );
    /// # Ok::<(), plt::table::TableError>(())
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

/// The outline of a cell: the unit square with only its visible edges
/// drawn (`Cell.get_path`), placed on the cell's box.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CellShape {
    /// The cell in axes fractions.
    pub(crate) bbox: Bbox,
    pub(crate) edges: CellEdges,
}

impl PatchShape for CellShape {
    fn path(&self) -> Path {
        self.edges.outline()
    }

    fn patch_transform(&self) -> Affine2D {
        unit_to_bbox(&self.bbox)
    }
}

/// One cell of a table.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Cell {
    /// Width in axes fractions.
    pub(crate) width: f64,
    /// Height in axes fractions.
    pub(crate) height: f64,
    pub(crate) text: String,
    pub(crate) facecolor: Color,
    pub(crate) edgecolor: Color,
    /// Edge width in points (`None` = `patch.linewidth`).
    pub(crate) linewidth: Option<f64>,
    /// Opacity of the face and edges (the text keeps its own).
    pub(crate) alpha: Option<f64>,
    /// Horizontal alignment of the text, which also sets where it sits.
    pub(crate) loc: HAlign,
    pub(crate) edges: CellEdges,
    pub(crate) font: FontProperties,
    pub(crate) text_color: Color,
}

impl Cell {
    /// A `width` × `height` cell with matplotlib's defaults: no text
    /// (aligned right), a white face, all four edges drawn in black and
    /// the font and text colour of `rc` (`Cell(xy, width, height)`).
    pub(crate) fn new(width: f64, height: f64, rc: &RcParams) -> Self {
        Self {
            width,
            height,
            text: String::new(),
            facecolor: Color::from("w"),
            edgecolor: Color::from("k"),
            linewidth: None,
            alpha: None,
            loc: HAlign::Right,
            edges: CellEdges::CLOSED,
            font: FontProperties::from_rc(rc),
            text_color: rc.text_color.clone(),
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
            valign: VAlign::Center,
            multialign: self.loc,
            ..BlockStyle::default()
        };
        Ok(lay_out(&self.text, &font, &style, dpi, rc)?)
    }

    /// The width the text needs at `fontsize` with its padding, in axes
    /// fractions of an axes `axes_width` pixels wide
    /// (`Cell.get_required_width`).
    pub(crate) fn required_width(
        &self,
        fontsize: f64,
        layout: LayoutContext<'_>,
        rc: &RcParams,
        axes_width: f64,
    ) -> Result<f64, TextDrawError> {
        let width = if self.text.is_empty() {
            0.0
        } else {
            self.laid_out(fontsize, layout.dpi(), rc)?
                .block
                .bbox
                .width()
        };
        Ok(width / axes_width * (1.0 + 2.0 * PAD))
    }

    /// Draws the cell placed at `placed` (in the table's axes fractions,
    /// which map to display pixels by `to_display`) with its text
    /// (`Cell.draw`).
    pub(crate) fn draw(
        &self,
        ctx: &mut DrawContext<'_>,
        table: &ArtistBase,
        rc: &RcParams,
        placed: &PlacedCell,
        to_display: &Affine2D,
    ) -> Result<(), TextDrawError> {
        let mut base = ArtistBase::default();
        base.clip_on = false;
        base.parent = table.parent;
        base.transform = Some(Transform::Affine(*to_display));
        base.alpha = self.alpha;
        let mut style = PatchStyle::from_rc(rc);
        style.facecolor = Some(self.facecolor.clone());
        style.edgecolor = Some(self.edgecolor.clone());
        if let Some(width) = self.linewidth {
            style.linewidth = Some(width);
        }
        let shape = CellShape {
            bbox: placed.bbox,
            edges: self.edges,
        };
        PatchParts {
            base: &base,
            style: &style,
            rc,
            shape: &shape,
            interpolation_steps: 1,
            drawn: None,
        }
        .paint(ctx)
        .map_err(TextDrawError::Box)?;
        if self.text.is_empty() {
            return Ok(());
        }
        let display = placed.bbox.transformed(to_display);
        let y = display.y0() + display.height() / 2.0;
        let x = match self.loc {
            HAlign::Center => display.x0() + display.width() / 2.0,
            HAlign::Left => display.x0() + display.width() * PAD,
            HAlign::Right => display.x0() + display.width() * (1.0 - PAD),
        };
        let laid_out = self.laid_out(placed.fontsize, ctx.layout().dpi(), rc)?;
        let gc = GraphicsContext {
            foreground: self.text_color.resolve()?,
            antialiased: rc.text_antialiased,
            ..GraphicsContext::default()
        };
        draw_laid_out(ctx, &gc, &laid_out, Point::new(x, y), 0.0);
        Ok(())
    }
}

/// A cell placed by the table layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PlacedCell {
    /// The cell in axes fractions.
    pub(crate) bbox: Bbox,
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
        let closed = CellShape {
            bbox: Bbox::UNIT,
            edges: CellEdges::CLOSED,
        }
        .path();
        assert_eq!(closed.code_at(4), PathCode::ClosePoly);
        let horizontal = CellShape {
            bbox: Bbox::UNIT,
            edges: "horizontal".parse().unwrap(),
        }
        .path();
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
