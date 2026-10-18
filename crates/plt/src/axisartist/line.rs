//! The line of an axis artist: a plain path, or an arrow pointing past the
//! end of the axis (matplotlib's `AxislineStyle` and
//! `_FancyAxislineStyle`).

use std::fmt;
use std::str::FromStr;

use plt_core::{Path, Point};
use plt_render::{CapStyle, JoinStyle};

use crate::colors::Color;
use crate::patches::{ArrowStyle, CurveEnd, DisplayPath, PatchError, PatchStyle};
use crate::rc::RcParams;
use crate::transforms::ResolvedTransform;

/// The line of an axis artist (matplotlib's `AxisArtist.line`: a
/// `PathPatch`, or a `FancyArrowPatch` with an axis line style).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LineState {
    /// The arrow style; `None` draws a plain line.
    pub(crate) style: Option<AxislineStyle>,
    /// Whether the line is clipped to the axes box.
    pub(crate) clipped: bool,
    pub(crate) visible: bool,
    /// The colour (`None` = `axes.edgecolor`, or the arrow's default).
    pub(crate) color: Option<Color>,
    /// The width in points (`None` = `axes.linewidth`, or
    /// `patch.linewidth` for arrows).
    pub(crate) linewidth: Option<f64>,
}

impl Default for LineState {
    /// A visible plain line, unclipped.
    fn default() -> Self {
        Self {
            style: None,
            clipped: false,
            visible: true,
            color: None,
            linewidth: None,
        }
    }
}

impl LineState {
    /// The line `path` (in `transform`'s coordinates) as display paths:
    /// the path itself, or the arrow of the style for tick labels of
    /// `label_size` points (matplotlib's `_draw_line`).
    pub(crate) fn display_paths(
        &self,
        path: &Path,
        transform: &ResolvedTransform,
        label_size: f64,
        rc: &RcParams,
        dpi_cor: f64,
    ) -> Result<Vec<DisplayPath>, String> {
        match &self.style {
            None => {
                let (path, affine) = transform.split_path(path);
                Ok(vec![DisplayPath::filled(path.into_owned(), affine)])
            }
            Some(style) => style
                .display_paths(
                    &transform.transform_path(path),
                    label_size,
                    self.linewidth.unwrap_or(rc.patch_linewidth),
                    dpi_cor,
                )
                .map_err(|e| e.to_string()),
        }
    }

    /// The look of the line and whether its edge takes the patch default:
    /// a `PathPatch` in `axes.edgecolor` and `axes.linewidth`, unfilled,
    /// or the arrow's `FancyArrowPatch`, with the colour and width set
    /// (matplotlib's `_init_line`).
    pub(crate) fn patch_style(&self, rc: &RcParams) -> (PatchStyle, bool) {
        let (mut style, edge_default) = match &self.style {
            None => (
                PatchStyle {
                    edgecolor: Some(rc.axes_edgecolor.clone()),
                    facecolor: Some(rc.axes_edgecolor.clone()),
                    fill: false,
                    linewidth: Some(rc.axes_linewidth),
                    capstyle: rc.lines_solid_capstyle,
                    joinstyle: rc.lines_solid_joinstyle,
                    ..PatchStyle::from_rc(rc)
                },
                false,
            ),
            Some(style) => (style.patch_style(rc), true),
        };
        if let Some(color) = &self.color {
            style.edgecolor = Some(color.clone());
            style.facecolor = Some(color.clone());
        }
        if let Some(width) = self.linewidth {
            style.linewidth = Some(width);
        }
        (style, edge_default)
    }
}

/// How the line of an axis artist is drawn (matplotlib's
/// `AxislineStyle`).
#[derive(Clone, Debug, PartialEq)]
pub enum AxislineStyle {
    /// The line ends in an open arrow head (`"->"`).
    SimpleArrow {
        /// The head size as a fraction of the tick label size.
        size: f64,
    },
    /// The line ends in a filled arrow head (`"-|>"`).
    FilledArrow {
        /// The head size as a fraction of the tick label size.
        size: f64,
        /// The head colour (`None` = `axes.edgecolor`).
        facecolor: Option<Color>,
    },
}

/// A malformed axis line style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AxislineStyleError {
    /// The style is neither `"->"` nor `"-|>"`.
    Unknown(String),
    /// An attribute is not `size=<number>` (or `facecolor=<color>` for
    /// `"-|>"`).
    Attribute(String),
}

impl fmt::Display for AxislineStyleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(name) => write!(f, "unknown axis line style {name:?}"),
            Self::Attribute(attr) => write!(f, "invalid axis line style attribute {attr:?}"),
        }
    }
}

impl std::error::Error for AxislineStyleError {}

impl FromStr for AxislineStyle {
    type Err = AxislineStyleError;

    /// Parses `"->"` or `"-|>"`, optionally followed by attributes such
    /// as `",size=1.5"`.
    fn from_str(text: &str) -> Result<Self, AxislineStyleError> {
        let mut parts = text.split(',').map(str::trim);
        let name = parts.next().unwrap_or_default();
        let mut size = 1.0;
        let mut facecolor = None;
        for attr in parts {
            let invalid = || AxislineStyleError::Attribute(attr.to_owned());
            let (key, value) = attr.split_once('=').ok_or_else(invalid)?;
            match (key.trim(), name) {
                ("size", _) => size = value.trim().parse().map_err(|_| invalid())?,
                ("facecolor", "-|>") => facecolor = Some(Color::from(value.trim())),
                _ => return Err(invalid()),
            }
        }
        match name {
            "->" => Ok(Self::SimpleArrow { size }),
            "-|>" => Ok(Self::FilledArrow { size, facecolor }),
            _ => Err(AxislineStyleError::Unknown(name.to_owned())),
        }
    }
}

impl AxislineStyle {
    /// The head size as a fraction of the tick label size.
    fn size(&self) -> f64 {
        match self {
            Self::SimpleArrow { size } | Self::FilledArrow { size, .. } => *size,
        }
    }

    /// The `FancyArrowPatch` look of the arrowed line under `rc`.
    pub(crate) fn patch_style(&self, rc: &RcParams) -> PatchStyle {
        let facecolor = match self {
            Self::SimpleArrow { .. } => None,
            Self::FilledArrow { facecolor, .. } => Some(
                facecolor
                    .clone()
                    .unwrap_or_else(|| rc.axes_edgecolor.clone()),
            ),
        };
        PatchStyle {
            facecolor,
            capstyle: CapStyle::Round,
            joinstyle: JoinStyle::Round,
            ..PatchStyle::from_rc(rc)
        }
    }

    /// The arrow along `line` (display pixels): the line extended past its
    /// end by the mutation size, then the arrow style's pieces
    /// (`_FancyAxislineStyle.SimpleArrow.draw`). `label_size` is the tick
    /// label size in points, `linewidth` the edge width in points.
    pub(crate) fn display_paths(
        &self,
        line: &Path,
        label_size: f64,
        linewidth: f64,
        dpi_cor: f64,
    ) -> Result<Vec<DisplayPath>, PatchError> {
        let mutation_scale = label_size * self.size();
        let extended = extend(line, mutation_scale);
        let head = match self {
            Self::SimpleArrow { .. } => CurveEnd::Head,
            Self::FilledArrow { .. } => CurveEnd::FilledHead,
        };
        Ok(ArrowStyle::curve(CurveEnd::Plain, head)
            .paths(
                &extended,
                mutation_scale * dpi_cor,
                linewidth * dpi_cor,
                1.0,
            )?
            .into_iter()
            .map(|(path, filled)| DisplayPath::display(path, filled))
            .collect())
    }
}

/// `path` with one more vertex, `length` (pixels) beyond its last one in
/// the direction of its last segment (matplotlib's `_extend_path`).
fn extend(path: &Path, length: f64) -> Path {
    let vertices = path.vertices();
    let [.., from, to] = vertices else {
        return path.clone();
    };
    let theta = (to.y - from.y).atan2(to.x - from.x);
    let tip = Point::new(to.x + theta.cos() * length, to.y + theta.sin() * length);
    let mut extended = vertices.to_vec();
    extended.push(tip);
    Path::polyline(extended)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_parse_with_attributes() {
        assert_eq!(
            "-|>".parse::<AxislineStyle>(),
            Ok(AxislineStyle::FilledArrow {
                size: 1.0,
                facecolor: None
            })
        );
        assert_eq!(
            "->, size=1.5".parse::<AxislineStyle>(),
            Ok(AxislineStyle::SimpleArrow { size: 1.5 })
        );
        assert!(matches!(
            "<-".parse::<AxislineStyle>(),
            Err(AxislineStyleError::Unknown(_))
        ));
        assert!(matches!(
            "->,facecolor=red".parse::<AxislineStyle>(),
            Err(AxislineStyleError::Attribute(_))
        ));
    }

    #[test]
    fn lines_extend_along_their_last_segment() {
        let line = Path::polyline(vec![Point::new(0.0, 0.0), Point::new(0.0, 10.0)]);
        let extended = extend(&line, 5.0);
        let tip = extended.vertices()[2];
        assert!(tip.x.abs() < 1e-12 && (tip.y - 15.0).abs() < 1e-12);
        let arrow = AxislineStyle::FilledArrow {
            size: 1.0,
            facecolor: None,
        };
        let paths = arrow.display_paths(&line, 10.0, 1.0, 1.0).unwrap();
        assert_eq!(paths.len(), 2, "the line and the filled head");
        assert!(!paths[0].filled && paths[1].filled);
    }
}
