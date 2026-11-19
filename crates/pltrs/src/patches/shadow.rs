//! Drop shadows of patches (matplotlib's `Shadow`).

use pltrs_core::{Affine2D, Path, Rgba};

use super::shape::ShapeFrame;
use super::{
    DisplayPath, Patch, PatchArtist, PatchError, PatchShape, PatchStyle, PathShape, ShapeError,
};
use crate::artist::{Artist, ArtistBase, ArtistHandle};
use crate::colors::ColorError;
use crate::transforms::{ResolvedTransform, Transform};

/// The outline of another patch, offset by `(ox, oy)` and painted in a
/// darker shade of its face colour, half transparent, just below it.
///
/// As in matplotlib, the offset is given in points but applied in the
/// source patch's coordinates (before its data transform).
#[derive(Clone, Debug, PartialEq)]
pub struct Shadow<S> {
    source: S,
    offset: (f64, f64),
    style: PatchStyle,
    transform: Option<Transform>,
    zorder: f64,
    /// Whether the shadow is clipped to its axes, as its source is.
    clip_on: bool,
}

pltrs_core::archive_struct!(Shadow<S> { source, offset, style, transform, zorder, clip_on });

/// Why a shadow cannot be made.
#[derive(Clone, Debug, PartialEq)]
pub enum ShadowError {
    /// The shade is outside `[0, 1]`.
    Shape(ShapeError),
    /// The source's face colour is invalid.
    Color(ColorError),
}

impl std::fmt::Display for ShadowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Shape(err) => err.fmt(f),
            Self::Color(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for ShadowError {}

impl<S: PatchShape> Shadow<S> {
    /// The shadow of `source` painted with `style` (the source's
    /// properties), offset by `(ox, oy)` points, its colour
    /// `(1 - shade)` times the source's face colour. The shadow inherits
    /// `transform`, the source's coordinate system; with `None` (a source
    /// never added anywhere) it takes the data coordinates of the axes it
    /// is added to.
    pub fn new(
        source: S,
        style: &PatchStyle,
        transform: Option<Transform>,
        (ox, oy): (f64, f64),
        shade: f64,
    ) -> Result<Self, ShadowError> {
        if !(0.0..=1.0).contains(&shade) {
            return Err(ShadowError::Shape(ShapeError::ShadeOutOfRange));
        }
        let face = style
            .resolve(&crate::rc::params(), None, S::EDGE_DEFAULT)
            .map_err(ShadowError::Color)?
            .face;
        let rgb = |c: f64| (1.0 - shade) * c;
        let color = Rgba::opaque(rgb(face.r), rgb(face.g), rgb(face.b));
        let mut style = style.clone();
        style.facecolor = Some(color.into());
        style.edgecolor = Some(color.into());
        Ok(Self {
            source,
            offset: (ox, oy),
            style,
            transform,
            zorder: 1.0,
            clip_on: true,
        })
    }

    /// The shadow of a patch already in a figure, with matplotlib's default
    /// shade of 0.7, drawn just below it and clipped like it.
    pub fn of(patch: &Patch<S>, ox: f64, oy: f64) -> Result<Self, ShadowError> {
        Self::of_shaded(patch, ox, oy, 0.7)
    }

    /// [`Shadow::of`] with an explicit shade.
    pub fn of_shaded(patch: &Patch<S>, ox: f64, oy: f64, shade: f64) -> Result<Self, ShadowError> {
        let (source, style, transform, zorder, clip_on) =
            patch.raw().with::<PatchArtist<S>, _>(|p| {
                (
                    p.shape.clone(),
                    p.style.clone(),
                    p.base().transform.clone(),
                    p.base().zorder,
                    p.base().clip_on,
                )
            });
        Ok(Self {
            zorder,
            clip_on,
            ..Self::new(source, &style, transform, (ox, oy), shade)?
        })
    }

    /// The shadowed outline.
    pub fn source(&self) -> &S {
        &self.source
    }

    /// Shadows `source` from now on: matplotlib's `Shadow` reads the path
    /// and the patch transform of its patch each time it draws, so that it
    /// follows the patch's later changes of geometry.
    ///
    /// ```
    /// use pltrs::patches::{Patch, PatchProps, PatchShape, Rectangle, Shadow};
    /// let rect = Patch::loose(Rectangle::new((1.0, 2.0), 3.0, 4.0), PatchProps::default());
    /// let mut shadow = Shadow::of(&rect, 1.0, 1.0)?;
    /// shadow.set_source(Rectangle::new((0.0, 0.0), 1.0, 1.0));
    /// assert_eq!(shadow.path(), Rectangle::new((0.0, 0.0), 1.0, 1.0).path());
    /// # Ok::<(), pltrs::patches::ShadowError>(())
    /// ```
    pub fn set_source(&mut self, source: S) {
        self.source = source;
    }
}

impl Shadow<PathShape> {
    /// [`Shadow::of_shaded`] drawn from a copy of the outline of `patch`
    /// placed by its patch transform ([`Patch::placed_outline`]), whatever
    /// its shape: the shadow a program makes of a patch whose shape it does
    /// not know (matplotlib's `Shadow(patch, ox, oy, shade=...)`).
    ///
    /// ```
    /// use pltrs::patches::{Patch, PatchProps, PatchShape, Rectangle, Shadow};
    /// let rect = Patch::loose(Rectangle::new((1.0, 2.0), 3.0, 4.0), PatchProps::default());
    /// let shadow = Shadow::outline_of(&rect, 1.0, 1.0, 0.7)?;
    /// assert_eq!(shadow.source().path(), rect.shape().placed_path());
    /// # Ok::<(), pltrs::patches::ShadowError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `shade` is outside `[0, 1]` or the face colour of `patch` is
    /// invalid.
    pub fn outline_of<S: PatchShape>(
        patch: &Patch<S>,
        ox: f64,
        oy: f64,
        shade: f64,
    ) -> Result<Self, ShadowError> {
        let Shadow {
            offset,
            style,
            transform,
            zorder,
            clip_on,
            ..
        } = Shadow::of_shaded(patch, ox, oy, shade)?;
        Ok(Self {
            source: PathShape(patch.placed_outline()),
            offset,
            style,
            transform,
            zorder,
            clip_on,
        })
    }
}

impl<S: PatchShape> PatchShape for Shadow<S> {
    const MAPS_OUTLINE_ALONE: bool = false;

    fn path(&self) -> Path {
        self.source.path()
    }

    fn patch_transform(&self) -> Affine2D {
        self.source.patch_transform()
    }

    fn init_style(&self, style: &mut PatchStyle, base: &mut ArtistBase) {
        style.clone_from(&self.style);
        base.alpha = Some(0.5);
        base.zorder = self.zorder.next_down();
        base.transform.clone_from(&self.transform);
        base.clip_on = self.clip_on;
    }

    /// The source's outline, offset as the shadow was last drawn: not at
    /// all before its first draw, as matplotlib's `Shadow` sets its offset
    /// when it draws (an extent measured before shows the outline in
    /// place).
    fn display_paths(&self, frame: &ShapeFrame<'_>) -> Result<Vec<DisplayPath>, PatchError> {
        let (ox, oy) = self.offset;
        let shift = frame.drawn_dpi_cor.unwrap_or(0.0);
        let placed = ResolvedTransform::affine(
            self.source
                .patch_transform()
                .translate(ox * shift, oy * shift),
        )
        .then(frame.transform.clone());
        let outline = self.source.path();
        let (path, affine) = placed.split_path(&outline);
        Ok(vec![DisplayPath::filled(path.into_owned(), affine)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patches::Rectangle;
    use crate::rc::RcParams;

    #[test]
    fn shadows_darken_the_face() {
        let mut style = PatchStyle::from_rc(&RcParams::matplotlib());
        style.facecolor = Some("white".into());
        let shadow = Shadow::new(
            Rectangle::new((0.0, 0.0), 1.0, 1.0),
            &style,
            None,
            (2.0, -2.0),
            0.3,
        )
        .unwrap();
        let mut base = ArtistBase::default();
        let mut painted = PatchStyle::from_rc(&RcParams::matplotlib());
        shadow.init_style(&mut painted, &mut base);
        let face = painted.facecolor.unwrap().resolve().unwrap();
        assert!((face.r - 0.7).abs() < 1e-12);
        assert_eq!(face.a, 1.0);
        assert_eq!(base.alpha, Some(0.5));
        assert!(base.zorder < 1.0);
        assert!(matches!(
            Shadow::new(
                Rectangle::new((0.0, 0.0), 1.0, 1.0),
                &style,
                None,
                (0.0, 0.0),
                1.5
            ),
            Err(ShadowError::Shape(ShapeError::ShadeOutOfRange))
        ));
    }

    #[test]
    fn outline_shadows_take_the_source_placed_and_styled() {
        let fig = crate::figure::Figure::new();
        let ax = fig.subplot();
        let rect = ax.add_patch(Rectangle::new((1.0, 0.0), 2.0, 1.0));
        let exact = Shadow::of(&rect, 1.0, 1.0).unwrap();
        let outline = Shadow::outline_of(&rect, 1.0, 1.0, 0.7).unwrap();
        assert_eq!(outline.style, exact.style);
        assert_eq!(
            (outline.zorder, outline.clip_on, outline.offset),
            (exact.zorder, exact.clip_on, exact.offset)
        );
        assert_eq!(outline.patch_transform(), Affine2D::IDENTITY);
        assert_eq!(outline.path(), exact.source().placed_path());
    }

    #[test]
    fn shadows_are_clipped_like_their_source() {
        let fig = crate::figure::Figure::new();
        let ax = fig.subplot();
        let free = ax.add_patch_with(
            Rectangle::new((0.0, 0.0), 1.0, 1.0),
            crate::patches::PatchProps::default().clip_on(false),
        );
        let clipped = ax.add_patch(Rectangle::new((0.0, 0.0), 1.0, 1.0));
        let clip_on = |shadow: &crate::patches::Patch<Shadow<Rectangle>>| {
            shadow.raw().with_base(|base| base.clip_on)
        };
        assert!(!clip_on(
            &ax.add_patch(Shadow::of(&free, 1.0, 1.0).unwrap())
        ));
        assert!(clip_on(
            &ax.add_patch(Shadow::of(&clipped, 1.0, 1.0).unwrap())
        ));
    }
}
