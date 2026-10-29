//! Box outlines drawn around a rectangle (matplotlib's `BoxStyle`), used by
//! [`FancyBboxPatch`](super::FancyBboxPatch) and the boxes of texts.

use std::str::FromStr;

use pltrs_core::Path;

use self::outlines::Padded;
use super::StyleError;
use super::style_spec::StyleSpec;

mod outlines;

/// The padding of every box style when none is given.
pub const DEFAULT_PAD: f64 = 0.3;

/// The shape of a box outline, with its shape-specific parameters.
/// Lengths are fractions of the mutation size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoxShape {
    /// A rectangle.
    Square,
    /// The circle around the padded box.
    Circle,
    /// The ellipse through the corners of the padded box.
    Ellipse,
    /// A right-pointing arrow.
    RArrow(ArrowHead),
    /// A left-pointing arrow.
    LArrow(ArrowHead),
    /// A two-way arrow.
    DArrow(ArrowHead),
    /// Round corners of radius `rounding_size` (the padding when unset).
    Round {
        /// Corner radius.
        rounding_size: Option<f64>,
    },
    /// Rounded edges (by half the padding when unset).
    Round4 {
        /// Rounding of the edges.
        rounding_size: Option<f64>,
    },
    /// A sawtooth outline with teeth of `tooth_size` (half the padding
    /// when unset).
    Sawtooth {
        /// Size of the teeth.
        tooth_size: Option<f64>,
    },
    /// A rounded sawtooth outline.
    Roundtooth {
        /// Size of the teeth.
        tooth_size: Option<f64>,
    },
    /// An outline drawn by a function of the caller's (matplotlib's
    /// callable box styles and registered box style classes).
    Custom(CustomBox),
}

/// The outline of a [`CustomBox`]: from the boxed rectangle `x0, y0,
/// width, height` and the padding `pad` (the style's padding times the
/// mutation size), all in the same units, the closed path around it.
pub type BoxOutline = fn(x0: f64, y0: f64, width: f64, height: f64, pad: f64) -> Path;

/// A box shape drawn by a [`BoxOutline`] (matplotlib's callable box
/// styles; a registered box style class with parameters is the same
/// outline with the style's padding). The name only shows in debug output.
///
/// ```
/// use pltrs::patches::{BoxShape, BoxStyle, CustomBox};
/// use pltrs::path::Path;
/// use pltrs::transforms::Point;
/// fn diamond(x0: f64, y0: f64, width: f64, height: f64, pad: f64) -> Path {
///     let (cx, cy) = (x0 + width / 2.0, y0 + height / 2.0);
///     let (rx, ry) = (width / 2.0 + 2.0 * pad, height / 2.0 + 2.0 * pad);
///     Path::polygon([(cx - rx, cy), (cx, cy - ry), (cx + rx, cy), (cx, cy + ry)].map(Point::from))
/// }
/// let style = BoxStyle::new(BoxShape::Custom(CustomBox::new("diamond", diamond))).with_pad(0.5);
/// assert_eq!(style.path(0.0, 0.0, 4.0, 2.0, 2.0).extents().x1(), 6.0);
/// ```
#[derive(Clone, Copy)]
pub struct CustomBox {
    name: &'static str,
    outline: BoxOutline,
}

impl CustomBox {
    /// The shape `outline` draws, called `name`.
    pub const fn new(name: &'static str, outline: BoxOutline) -> Self {
        Self { name, outline }
    }

    /// The shape's name.
    pub const fn name(&self) -> &'static str {
        self.name
    }
}

impl PartialEq for CustomBox {
    /// Shapes of the same name drawn by the same function.
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && std::ptr::fn_addr_eq(self.outline, other.outline)
    }
}

impl std::fmt::Debug for CustomBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("CustomBox").field(&self.name).finish()
    }
}

/// The head of an arrow-shaped box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArrowHead {
    /// Head width relative to the shaft width (non-negative).
    pub head_width: f64,
    /// Angle at the tip in degrees (non-zero modulo 360; negative angles
    /// point the head backwards).
    pub head_angle: f64,
}

impl Default for ArrowHead {
    fn default() -> Self {
        Self {
            head_width: 1.5,
            head_angle: 90.0,
        }
    }
}

impl ArrowHead {
    fn validated(self) -> Result<Self, StyleError> {
        if self.head_width < 0.0 {
            return Err(StyleError::InvalidValue("'head_width' must be nonnegative"));
        }
        if self.head_angle % 360.0 == 0.0 {
            return Err(StyleError::InvalidValue("'head_angle' must be nonzero"));
        }
        Ok(self)
    }
}

/// A box style: a shape and the padding around the boxed rectangle
/// (`None` uses [`DEFAULT_PAD`]; texts substitute their own default).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxStyle {
    /// The outline.
    pub shape: BoxShape,
    /// Padding as a fraction of the mutation size.
    pub pad: Option<f64>,
}

impl BoxStyle {
    /// A style of `shape` with the default padding.
    pub const fn new(shape: BoxShape) -> Self {
        Self { shape, pad: None }
    }

    /// The same style with an explicit padding.
    #[must_use]
    pub const fn with_pad(self, pad: f64) -> Self {
        Self {
            pad: Some(pad),
            ..self
        }
    }

    /// The padding in use.
    pub fn pad(&self) -> f64 {
        self.pad.unwrap_or(DEFAULT_PAD)
    }

    /// The outline around the rectangle at `(x0, y0)` of `width × height`,
    /// with lengths scaled by `mutation_size`.
    pub fn path(&self, x0: f64, y0: f64, width: f64, height: f64, mutation_size: f64) -> Path {
        let pad = self.pad() * mutation_size;
        let padded = Padded {
            x0: x0 - pad,
            y0: y0 - pad,
            dx: width + 2.0 * pad,
            dy: height + 2.0 * pad,
        };
        match self.shape {
            BoxShape::Square => padded.square(),
            BoxShape::Circle => padded.circle(),
            BoxShape::Ellipse => padded.ellipse(),
            BoxShape::RArrow(head) => padded.right_arrow(head),
            BoxShape::LArrow(head) => {
                let mut path = padded.right_arrow(head);
                for p in path.vertices_mut() {
                    p.x = 2.0 * x0 + width - p.x;
                }
                path
            }
            BoxShape::DArrow(head) => padded.double_arrow(head),
            BoxShape::Round { rounding_size } => padded.round(
                rounding_size
                    .filter(|&r| r != 0.0)
                    .map_or(pad, |r| r * mutation_size),
            ),
            BoxShape::Round4 { rounding_size } => padded.round4(
                rounding_size
                    .filter(|&r| r != 0.0)
                    .map_or(pad / 2.0, |r| r * mutation_size),
            ),
            BoxShape::Sawtooth { tooth_size } => {
                Path::closed(padded.sawtooth(self.tooth(tooth_size, mutation_size)))
            }
            BoxShape::Roundtooth { tooth_size } => {
                padded.roundtooth(self.tooth(tooth_size, mutation_size))
            }
            BoxShape::Custom(custom) => (custom.outline)(x0, y0, width, height, pad),
        }
    }

    fn tooth(&self, tooth_size: Option<f64>, mutation_size: f64) -> f64 {
        tooth_size.map_or(self.pad() * 0.5 * mutation_size, |t| t * mutation_size)
    }
}

impl From<BoxShape> for BoxStyle {
    fn from(shape: BoxShape) -> Self {
        Self::new(shape)
    }
}

impl FromStr for BoxStyle {
    type Err = StyleError;

    /// Parses matplotlib's description, e.g. `"round,pad=0.2"` or
    /// `"rarrow,head_width=2"` (names are case-insensitive).
    fn from_str(text: &str) -> Result<Self, StyleError> {
        let mut spec = StyleSpec::parse(text)?;
        let pad = spec.take("pad");
        let arrow = |spec: &mut StyleSpec| {
            let defaults = ArrowHead::default();
            ArrowHead {
                head_width: spec.take("head_width").unwrap_or(defaults.head_width),
                head_angle: spec.take("head_angle").unwrap_or(defaults.head_angle),
            }
            .validated()
        };
        let shape = match spec.name.as_str() {
            "square" => BoxShape::Square,
            "circle" => BoxShape::Circle,
            "ellipse" => BoxShape::Ellipse,
            "rarrow" => BoxShape::RArrow(arrow(&mut spec)?),
            "larrow" => BoxShape::LArrow(arrow(&mut spec)?),
            "darrow" => BoxShape::DArrow(arrow(&mut spec)?),
            "round" => BoxShape::Round {
                rounding_size: spec.take("rounding_size"),
            },
            "round4" => BoxShape::Round4 {
                rounding_size: spec.take("rounding_size"),
            },
            "sawtooth" => BoxShape::Sawtooth {
                tooth_size: spec.take("tooth_size"),
            },
            "roundtooth" => BoxShape::Roundtooth {
                tooth_size: spec.take("tooth_size"),
            },
            _ => return Err(spec.unknown()),
        };
        spec.finish()?;
        Ok(Self { shape, pad })
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::{PathCode, Point};

    use super::*;

    fn style(text: &str) -> BoxStyle {
        text.parse().expect("valid style")
    }

    #[test]
    fn parses_names_and_parameters() {
        assert_eq!(style("Square"), BoxStyle::new(BoxShape::Square));
        assert_eq!(
            style("round,pad=0.6,rounding_size=0.8"),
            BoxStyle::new(BoxShape::Round {
                rounding_size: Some(0.8)
            })
            .with_pad(0.6)
        );
        assert!("rarrow,head_width=-1".parse::<BoxStyle>().is_err());
        assert!("larrow,head_angle=360".parse::<BoxStyle>().is_err());
        assert!("square,rounding_size=1".parse::<BoxStyle>().is_err());
        assert!("hexagon".parse::<BoxStyle>().is_err());
    }

    #[test]
    fn square_is_padded() {
        let path = style("square,pad=0.5").path(0.0, 0.0, 4.0, 2.0, 2.0);
        let extents = path.extents();
        assert_eq!(
            (extents.x0(), extents.y0(), extents.x1(), extents.y1()),
            (-1.0, -1.0, 5.0, 3.0)
        );
        assert_eq!(path.len(), 5);
    }

    #[test]
    fn arrows_point_the_right_way() {
        let right = style("rarrow").path(0.0, 0.0, 4.0, 2.0, 1.0).extents();
        let left = style("larrow").path(0.0, 0.0, 4.0, 2.0, 1.0).extents();
        assert!((right.x1() - 5.6).abs() < 1e-12, "tip past the box");
        assert!((left.x0() + 1.6).abs() < 1e-12, "mirrored tip");
        let both = style("darrow").path(0.0, 0.0, 4.0, 2.0, 1.0).extents();
        assert!((both.x0() + 1.6).abs() < 1e-12 && (both.x1() - 5.6).abs() < 1e-12);
        let backwards = style("rarrow,head_angle=-90").path(0.0, 0.0, 4.0, 2.0, 1.0);
        assert!(backwards.extents().x1() <= 4.3 + 1e-12);
    }

    #[test]
    fn rounded_and_toothed_outlines_close() {
        for name in [
            "round",
            "round4",
            "sawtooth",
            "roundtooth",
            "circle",
            "ellipse",
        ] {
            let path = style(name).path(0.0, 0.0, 4.0, 2.0, 10.0);
            assert_eq!(path.code_at(path.len() - 1), PathCode::ClosePoly, "{name}");
        }
        let teeth = style("sawtooth").path(0.0, 0.0, 4.0, 2.0, 10.0);
        assert!(teeth.len() > 20);
    }

    #[test]
    fn tiny_teeth_are_bounded() {
        // Found by fuzzing: `sawtooth,pad=1e-9` asked for billions of teeth.
        for name in ["sawtooth,pad=1e-9", "roundtooth,tooth_size=1e-300"] {
            let path = style(name).path(0.0, 0.0, 40.0, 20.0, 10.0);
            assert!(path.len() < 200_000, "{name}: {}", path.len());
        }
    }

    /// A box with a point on its left, as in matplotlib's annotation guide.
    fn angled(x0: f64, y0: f64, width: f64, height: f64, pad: f64) -> Path {
        let (x0, y0) = (x0 - pad, y0 - pad);
        let (x1, y1) = (x0 + width + 2.0 * pad, y0 + height + 2.0 * pad);
        Path::closed(
            [
                (x0, y0),
                (x1, y0),
                (x1, y1),
                (x0, y1),
                (x0 - pad, f64::midpoint(y0, y1)),
                (x0, y0),
            ]
            .map(|(x, y)| Point::new(x, y))
            .to_vec(),
        )
    }

    #[test]
    fn custom_outlines_get_the_scaled_padding() {
        let angled_box = BoxShape::Custom(CustomBox::new("angled", angled));
        let path = BoxStyle::new(angled_box).path(0.0, 0.0, 4.0, 2.0, 10.0);
        assert_eq!(path.extents().x0(), -6.0, "default padding of 0.3");
        let padded = BoxStyle::new(angled_box)
            .with_pad(0.5)
            .path(0.0, 0.0, 4.0, 2.0, 10.0);
        assert_eq!(padded.extents().x0(), -10.0);
        assert_eq!(padded.code_at(padded.len() - 1), PathCode::ClosePoly);
    }

    #[test]
    fn custom_shapes_compare_by_name_and_function() {
        let angled_box = CustomBox::new("angled", angled);
        assert_eq!(angled_box, CustomBox::new("angled", angled));
        assert_ne!(angled_box, CustomBox::new("other", angled));
        assert_eq!(format!("{angled_box:?}"), "CustomBox(\"angled\")");
        assert_eq!(angled_box.name(), "angled");
    }

    #[test]
    fn degenerate_teeth_leave_plain_edges() {
        let flat = style("sawtooth,pad=0").path(0.0, 0.0, 4.0, 2.0, 10.0);
        assert!(flat.vertices().iter().all(|p| p.is_finite()));
        let tiny = style("roundtooth").path(0.0, 0.0, 0.0, 0.0, 1.0);
        assert_eq!(tiny.code_at(tiny.len() - 1), PathCode::ClosePoly);
    }
}
