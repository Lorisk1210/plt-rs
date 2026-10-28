//! Resolved marker geometry (matplotlib's `MarkerStyle`).

use std::str::FromStr;

use plt_core::{Affine2D, Path};
use plt_render::{CapStyle, JoinStyle};

use super::shapes::Shape;
use super::spec::{FillStyle, Marker, MarkerError};

/// A marker resolved into the paths and transforms used to draw it.
///
/// Marker paths live in a unit coordinate system (a marker of size 1 spans
/// about one unit); [`transform`](Self::transform) maps them there and the
/// line or collection then scales by the marker size in pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct MarkerStyle {
    marker: Marker,
    fillstyle: FillStyle,
    user_transform: Option<Affine2D>,
    user_capstyle: Option<CapStyle>,
    user_joinstyle: Option<JoinStyle>,
    shape: Shape,
}

impl MarkerStyle {
    /// Resolves `marker` with the given fill style.
    pub fn new(marker: impl Into<Marker>, fillstyle: FillStyle) -> Result<Self, MarkerError> {
        Self::with_styles(marker, fillstyle, None, None)
    }

    /// Resolves `marker`, overriding the marker's own cap and join styles.
    ///
    /// As in matplotlib, a join style override only applies to markers that
    /// choose a join style themselves (polygons, stars, carets, ...); round
    /// markers always use round joins.
    pub fn with_styles(
        marker: impl Into<Marker>,
        fillstyle: FillStyle,
        capstyle: Option<CapStyle>,
        joinstyle: Option<JoinStyle>,
    ) -> Result<Self, MarkerError> {
        let marker = marker.into();
        let shape = Shape::build(&marker, fillstyle, capstyle, joinstyle)?;
        Ok(Self {
            marker,
            fillstyle,
            user_transform: None,
            user_capstyle: capstyle,
            user_joinstyle: joinstyle,
            shape,
        })
    }

    /// The marker specification.
    pub fn marker(&self) -> &Marker {
        &self.marker
    }

    /// The fill style.
    pub fn fillstyle(&self) -> FillStyle {
        self.fillstyle
    }

    /// The primary path (the face for filled markers).
    pub fn path(&self) -> &Path {
        &self.shape.path
    }

    /// Transform of [`path`](Self::path), including any user transform.
    pub fn transform(&self) -> Affine2D {
        self.with_user(self.shape.transform)
    }

    /// The alternate half of a half-filled marker.
    pub fn alt_path(&self) -> Option<&Path> {
        self.shape.alt.as_ref().map(|(path, _)| path)
    }

    /// Transform of [`alt_path`](Self::alt_path), including any user transform.
    pub fn alt_transform(&self) -> Option<Affine2D> {
        self.shape.alt.as_ref().map(|(_, t)| self.with_user(*t))
    }

    /// The user-supplied part of the transform.
    pub fn user_transform(&self) -> Option<Affine2D> {
        self.user_transform
    }

    /// Join style used when stroking the marker.
    pub fn joinstyle(&self) -> JoinStyle {
        self.shape.joinstyle
    }

    /// Cap style used when stroking the marker.
    pub fn capstyle(&self) -> CapStyle {
        self.shape.capstyle
    }

    /// Whether the marker has a face to fill.
    pub fn is_filled(&self) -> bool {
        self.shape.filled
    }

    /// Marker size (in pixels) at or above which the marker is snapped to
    /// the pixel grid; `None` never snaps, infinity always defers to the
    /// renderer's automatic snapping.
    pub fn snap_threshold(&self) -> Option<f64> {
        self.shape.snap_threshold
    }

    /// Whether the marker draws anything at all.
    pub fn is_visible(&self) -> bool {
        !self.shape.path.is_empty()
    }

    /// A copy with `transform` appended to the user transform.
    #[must_use]
    pub fn transformed(&self, transform: Affine2D) -> Self {
        let mut marker = self.clone();
        marker.user_transform = Some(self.user_transform.map_or(transform, |t| t.then(transform)));
        marker
    }

    /// A copy rotated by `degrees`.
    #[must_use]
    pub fn rotated_deg(&self, degrees: f64) -> Self {
        self.transformed(Affine2D::identity().rotate_deg(degrees))
    }

    /// A copy rotated by `radians`.
    #[must_use]
    pub fn rotated(&self, radians: f64) -> Self {
        self.transformed(Affine2D::identity().rotate(radians))
    }

    /// A copy scaled by `(sx, sy)`.
    #[must_use]
    pub fn scaled(&self, sx: f64, sy: f64) -> Self {
        self.transformed(Affine2D::scaling(sx, sy))
    }

    /// The user cap-style override.
    pub fn user_capstyle(&self) -> Option<CapStyle> {
        self.user_capstyle
    }

    /// The user join-style override.
    pub fn user_joinstyle(&self) -> Option<JoinStyle> {
        self.user_joinstyle
    }

    fn with_user(&self, transform: Affine2D) -> Affine2D {
        self.user_transform
            .map_or(transform, |user| transform.then(user))
    }
}

impl FromStr for MarkerStyle {
    type Err = MarkerError;

    /// Parses a marker code with the fill style of `markers.fillstyle`
    /// (matplotlib's `MarkerStyle(marker)`).
    fn from_str(code: &str) -> Result<Self, MarkerError> {
        Self::new(
            code.parse::<Marker>()?,
            crate::rc::params().markers_fillstyle,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plt_core::Point;

    use crate::markers::{MarkerSymbol, PolygonStyle};

    fn style(marker: MarkerSymbol, fill: FillStyle) -> MarkerStyle {
        MarkerStyle::new(marker, fill).expect("valid marker")
    }

    #[test]
    fn circle_is_filled_with_round_join() {
        let m = style(MarkerSymbol::Circle, FillStyle::Full);
        assert!(m.is_filled());
        assert_eq!(m.joinstyle(), JoinStyle::Round);
        assert_eq!(m.transform(), Affine2D::scaling(0.5, 0.5));
        assert_eq!(m.snap_threshold(), Some(f64::INFINITY));
        assert!(m.alt_path().is_none());
    }

    #[test]
    fn user_joinstyle_only_overrides_markers_that_choose_one() {
        let circle = MarkerStyle::with_styles(
            MarkerSymbol::Circle,
            FillStyle::Full,
            None,
            Some(JoinStyle::Miter),
        )
        .expect("valid");
        assert_eq!(circle.joinstyle(), JoinStyle::Round);
        let square = MarkerStyle::with_styles(
            MarkerSymbol::Square,
            FillStyle::Full,
            None,
            Some(JoinStyle::Bevel),
        )
        .expect("valid");
        assert_eq!(square.joinstyle(), JoinStyle::Bevel);
    }

    #[test]
    fn half_filled_markers_have_alternates() {
        let m = style(MarkerSymbol::Circle, FillStyle::Top);
        assert!(m.alt_path().is_some());
        let alt = m.alt_transform().expect("alternate");
        let p = alt.transform_point(Point::new(1.0, 0.0));
        assert!(
            (p - Point::new(0.0, -0.5)).length() < 1e-12,
            "alt half is rotated 180°"
        );
    }

    #[test]
    fn thin_diamond_scales_both_halves() {
        let m = style(MarkerSymbol::ThinDiamond, FillStyle::Left);
        assert_eq!(m.transform(), m.alt_transform().expect("alternate"));
    }

    #[test]
    fn unfilled_markers() {
        for symbol in [
            MarkerSymbol::Plus,
            MarkerSymbol::X,
            MarkerSymbol::TriUp,
            MarkerSymbol::CaretDown,
        ] {
            assert!(!style(symbol, FillStyle::Full).is_filled(), "{symbol:?}");
        }
        assert!(!style(MarkerSymbol::Nothing, FillStyle::Full).is_visible());
    }

    #[test]
    fn custom_vertices_are_normalised() {
        let m = MarkerStyle::new(
            Marker::Vertices(vec![Point::new(-2.0, 0.0), Point::new(4.0, 1.0)]),
            FillStyle::Full,
        )
        .expect("valid");
        assert_eq!(m.transform(), Affine2D::scaling(0.125, 0.125));
        assert_eq!(
            MarkerStyle::new(Marker::Vertices(Vec::new()), FillStyle::Full),
            Err(MarkerError::EmptyShape)
        );
    }

    #[test]
    fn user_transforms_compose() {
        let m = style(MarkerSymbol::Square, FillStyle::Full)
            .scaled(2.0, 2.0)
            .rotated_deg(90.0);
        let p = m.transform().transform_point(Point::new(1.0, 0.5));
        assert!((p - Point::new(0.0, 1.0)).length() < 1e-12);
    }

    #[test]
    fn polygon_tuples() {
        let star = MarkerStyle::new(Marker::polygon(5, PolygonStyle::Star, 0.0), FillStyle::Full)
            .expect("valid");
        assert_eq!(star.joinstyle(), JoinStyle::Bevel);
        assert_eq!(star.path().len(), 11);
        let asterisk = MarkerStyle::new(
            Marker::polygon(4, PolygonStyle::Asterisk, 45.0),
            FillStyle::Full,
        )
        .expect("valid");
        assert!(!asterisk.is_filled());
        assert_eq!(
            MarkerStyle::new(
                Marker::polygon(0, PolygonStyle::Regular, 0.0),
                FillStyle::Full
            ),
            Err(MarkerError::NoSides)
        );
    }

    #[test]
    fn parsed_styles_take_the_rc_fillstyle() {
        let parsed: MarkerStyle = "D".parse().expect("a marker code");
        assert_eq!(parsed.fillstyle(), FillStyle::Full);
        assert_eq!(parsed.marker(), &Marker::Symbol(MarkerSymbol::Diamond));
        assert!("not a marker".parse::<MarkerStyle>().is_err());
    }
}
