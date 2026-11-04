//! Adjacent images composited into one before drawing (matplotlib's
//! `_draw_list_compositing_images` and `composite_images`), for renderers
//! that do not composite images themselves — the vector backends.

use std::any::Any;

use pltrs_core::{Affine2D, Bbox, Point};
use pltrs_raster::resample::{self as raster, ResampleParams};
use pltrs_render::{ClipPath, GraphicsContext, RgbaImage};

use super::artist::AxesImageArtist;
use super::bbox_image::BboxImageArtist;
use super::figure_image::FigureImageArtist;
use super::make::RenderedImage;
use super::pcolor_image::PcolorImageArtist;
use super::{ImageError, Interpolation};
use crate::artist::{Artist, ArtistHandle, ArtistId, DrawContext, LayoutContext};
use crate::figure::Figure;
use crate::transforms::Transform;

/// An image artist that may join a composite.
trait Compositable {
    /// matplotlib's `can_composite`: the image is resampled (not drawn
    /// unsampled) through an affine map without rotation or shear.
    fn can_composite(&self, ctx: LayoutContext<'_>) -> bool;

    /// The image at `magnification` (matplotlib's `make_image`).
    fn rendered(
        &self,
        ctx: &mut DrawContext<'_>,
        magnification: f64,
    ) -> Result<Option<RenderedImage>, ImageError>;
}

/// Whether `transform` maps data without rotation or shear (matplotlib's
/// `is_affine and is_separable`).
fn affine_and_separable(ctx: LayoutContext<'_>, transform: &Transform) -> bool {
    ctx.resolve(transform).is_some_and(|resolved| {
        let [_, b, c, _, _, _] = resolved.affine_part().values();
        resolved.is_affine() && b == 0.0 && c == 0.0
    })
}

impl Compositable for AxesImageArtist {
    fn can_composite(&self, ctx: LayoutContext<'_>) -> bool {
        self.core.resampling.interpolation != Interpolation::None
            && affine_and_separable(ctx, &self.transform())
    }

    fn rendered(
        &self,
        ctx: &mut DrawContext<'_>,
        magnification: f64,
    ) -> Result<Option<RenderedImage>, ImageError> {
        self.render(ctx, magnification)
    }
}

impl Compositable for FigureImageArtist {
    fn can_composite(&self, _ctx: LayoutContext<'_>) -> bool {
        self.core.resampling.interpolation != Interpolation::None
    }

    fn rendered(
        &self,
        ctx: &mut DrawContext<'_>,
        magnification: f64,
    ) -> Result<Option<RenderedImage>, ImageError> {
        self.render(ctx, magnification)
    }
}

/// A box image maps the unit square onto the canvas
/// (`BboxTransformTo`), always without rotation or shear.
impl Compositable for BboxImageArtist {
    fn can_composite(&self, _ctx: LayoutContext<'_>) -> bool {
        self.core.resampling.interpolation != Interpolation::None
    }

    fn rendered(
        &self,
        ctx: &mut DrawContext<'_>,
        magnification: f64,
    ) -> Result<Option<RenderedImage>, ImageError> {
        self.render(ctx, magnification)
    }
}

impl Compositable for PcolorImageArtist {
    fn can_composite(&self, ctx: LayoutContext<'_>) -> bool {
        self.base()
            .transform
            .as_ref()
            .is_some_and(|transform| affine_and_separable(ctx, transform))
    }

    fn rendered(
        &self,
        ctx: &mut DrawContext<'_>,
        magnification: f64,
    ) -> Result<Option<RenderedImage>, ImageError> {
        self.render(ctx, magnification)
    }
}

/// The artist as an image that may join a composite, if it is an image.
fn as_image(artist: &dyn Artist) -> Option<&dyn Compositable> {
    let any = artist as &dyn Any;
    any.downcast_ref::<AxesImageArtist>()
        .map(|a| a as &dyn Compositable)
        .or_else(|| {
            any.downcast_ref::<FigureImageArtist>()
                .map(|a| a as &dyn Compositable)
        })
        .or_else(|| {
            any.downcast_ref::<BboxImageArtist>()
                .map(|a| a as &dyn Compositable)
        })
        .or_else(|| {
            any.downcast_ref::<PcolorImageArtist>()
                .map(|a| a as &dyn Compositable)
        })
}

/// `image` upside down: matplotlib's image arrays start with their bottom
/// row, pltrs images with their top row.
fn flip_rows(image: &RgbaImage) -> RgbaImage {
    let row = (image.width() * 4).max(1);
    let data: Vec<u8> = image
        .data()
        .chunks_exact(row)
        .rev()
        .flatten()
        .copied()
        .collect();
    RgbaImage::new(image.width(), image.height(), data).expect("flipping keeps the size")
}

/// One image of a composite (what matplotlib's `make_image` gives for it)
/// and its uniform opacity, which compositing applies (again) as
/// matplotlib does (`_get_scalar_alpha`).
#[derive(Clone, Debug, PartialEq)]
pub struct CompositePart {
    /// The image's pixels, first row at the top.
    pub pixels: RgbaImage,
    /// Where its lower left corner goes, in display pixels at the
    /// renderer's own resolution (before `magnification`).
    pub origin: Point,
    /// The opacity applied over its pixels.
    pub alpha: f64,
}

/// Composites images into one, each placed at its magnified position, in
/// order (matplotlib's `image.composite_images`): the composite and its
/// lower left corner, unmagnified, or `None` when there is nothing to
/// composite (no parts, or no pixels).
///
/// ```
/// use pltrs::image::{CompositePart, RgbaImage, composite_images};
/// use pltrs_core::Point;
/// let red = RgbaImage::new(2, 1, vec![255, 0, 0, 255, 255, 0, 0, 255])?;
/// let blue = RgbaImage::new(1, 1, vec![0, 0, 255, 255])?;
/// let parts = [
///     CompositePart { pixels: red, origin: Point::new(0.0, 0.0), alpha: 1.0 },
///     CompositePart { pixels: blue, origin: Point::new(1.0, 1.0), alpha: 1.0 },
/// ];
/// let (image, origin) = composite_images(&parts, 1.0)?.expect("pixels");
/// assert_eq!((image.width(), image.height()), (2, 2));
/// assert_eq!(origin, Point::new(0.0, 0.0));
/// assert_eq!(&image.data()[4..8], &[0, 0, 255, 255]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When the composite is too large to allocate.
pub fn composite_images(
    parts: &[CompositePart],
    magnification: f64,
) -> Result<Option<(RgbaImage, Point)>, ImageError> {
    let placed: Vec<(&RgbaImage, f64, f64, f64)> = parts
        .iter()
        .map(|part| {
            (
                &part.pixels,
                part.origin.x * magnification,
                part.origin.y * magnification,
                part.alpha,
            )
        })
        .collect();
    let Some(bbox) = placed
        .iter()
        .map(|&(part, x, y, _)| {
            Bbox::from_extents(x, y, x + part.width() as f64, y + part.height() as f64)
        })
        .reduce(|a, b| a.union(&b))
    else {
        return Ok(None);
    };
    let (width, height) = (bbox.width() as usize, bbox.height() as usize);
    if width == 0 || height == 0 {
        return Ok(None);
    }
    let mut output = raster::Grid::filled(width, height, [0u8; 4])?;
    for (part, x, y, alpha) in placed {
        // matplotlib's image arrays have their first row at the bottom.
        let bottom_up = flip_rows(part);
        let pixels: Vec<[u8; 4]> = bottom_up.data().as_chunks::<4>().0.to_vec();
        let source = raster::Grid::from_vec(part.width(), part.height(), pixels)?;
        let shift = Affine2D::translation(x - bbox.x0(), y - bbox.y0());
        raster::resample(
            &source,
            &mut output,
            &ResampleParams::new(shift)
                .with_resample(false)
                .with_alpha(alpha),
        )?;
    }
    let data: Vec<u8> = output.into_vec().into_iter().flatten().collect();
    let canvas = RgbaImage::new(width, height, data).expect("the canvas holds its own pixels");
    Ok(Some((
        flip_rows(&canvas),
        Point::new(bbox.x0() / magnification, bbox.y0() / magnification),
    )))
}

impl Figure {
    /// Keeps adjacent images apart (`Some(true)`) or composites them into
    /// one (`Some(false)`) in every part of the figure, or lets the output
    /// format decide (`None`: only vector formats composite); matplotlib's
    /// `suppressComposite`.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// assert_eq!(fig.suppress_composite(), None);
    /// fig.set_suppress_composite(Some(true));
    /// assert_eq!(fig.suppress_composite(), Some(true));
    /// ```
    pub fn set_suppress_composite(&self, suppress: Option<bool>) -> &Self {
        self.raw().figure_state().borrow_mut().suppress_composite = suppress;
        self
    }

    /// Whether adjacent images are kept apart or composited (`None` = as
    /// the output format prefers).
    pub fn suppress_composite(&self) -> Option<bool> {
        self.raw().figure_state().borrow().suppress_composite
    }
}

/// Draws the children of a figure or axes in order, compositing adjacent
/// images into one when the renderer asks for it (matplotlib's
/// `_draw_list_compositing_images`); `clip` is the parent's box and clip
/// path, which a composite is clipped to.
pub(crate) struct Compositor {
    /// Whether images are composited at all.
    enabled: bool,
    magnification: f64,
    clip: (Option<Bbox>, Option<ClipPath>),
    group: Vec<ArtistId>,
}

impl Compositor {
    /// A compositor for `children` of a parent clipped to `clip`.
    pub(crate) fn new(
        ctx: &mut DrawContext<'_>,
        children: &[ArtistId],
        clip: (Option<Bbox>, Option<ClipPath>),
    ) -> Self {
        let store = ctx.layout().store();
        let has_images = children
            .iter()
            .any(|&id| store.get(id).is_some_and(|a| as_image(a).is_some()));
        let suppressed = match ctx.layout().suppress_composite() {
            Some(suppress) => suppress,
            None => ctx.renderer().option_image_nocomposite(),
        };
        Self {
            enabled: has_images && !suppressed,
            magnification: ctx.renderer().image_magnification(),
            clip,
            group: Vec::new(),
        }
    }

    /// Draws child `id`, or keeps it for a composite.
    pub(crate) fn draw(&mut self, ctx: &mut DrawContext<'_>, id: ArtistId) {
        if self.enabled && Self::joins(ctx, id) {
            self.group.push(id);
        } else {
            self.flush(ctx);
            ctx.draw_child(id);
        }
    }

    /// Whether child `id` is a visible image that can be composited and
    /// is clipped to no path.
    fn joins(ctx: &DrawContext<'_>, id: ArtistId) -> bool {
        let layout = ctx.layout();
        layout.store().get(id).is_some_and(|artist| {
            let base = artist.base();
            base.visible
                && base.clip_on
                && ctx.base_gc(base).clip_path.is_none()
                && as_image(artist).is_some_and(|image| image.can_composite(*layout))
        })
    }

    /// Draws the images kept: one as it is, several as one composite.
    pub(crate) fn flush(&mut self, ctx: &mut DrawContext<'_>) {
        match std::mem::take(&mut self.group).as_slice() {
            [] => {}
            [single] => ctx.draw_child(*single),
            several => {
                let store = ctx.layout().store();
                let mut parts = Vec::new();
                for &id in several {
                    let Some(artist) = store.get(id) else {
                        continue;
                    };
                    let Some(image) = as_image(artist) else {
                        continue;
                    };
                    let alpha = artist.base().alpha.unwrap_or(1.0);
                    match image.rendered(ctx, self.magnification) {
                        Ok(Some(image)) => parts.push(CompositePart {
                            pixels: image.pixels,
                            origin: image.origin,
                            alpha,
                        }),
                        Ok(None) => {}
                        Err(err) => ctx.report_kind(id, err.draw_kind(), err),
                    }
                }
                match composite_images(&parts, self.magnification) {
                    Ok(Some((pixels, origin))) => {
                        let gc = GraphicsContext {
                            clip_rect: self.clip.0,
                            clip_path: self.clip.1.clone(),
                            ..GraphicsContext::default()
                        };
                        let origin =
                            Point::new(origin.x.round_ties_even(), origin.y.round_ties_even());
                        ctx.renderer().draw_image(&gc, origin, &pixels, None);
                    }
                    Ok(None) => {}
                    Err(err) => ctx.report_kind(several[0], err.draw_kind(), err),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::Bbox;

    use crate::axes::ImshowOptions;
    use crate::figure::Figure;
    use crate::image::{BboxImageOptions, Extent, Interpolation};
    use crate::numeric::Grid;

    /// The `<image` elements of an SVG of two images side by side, with
    /// compositing left to the format or forced either way.
    fn svg_images(suppress: Option<bool>) -> usize {
        let fig = Figure::new();
        fig.set_suppress_composite(suppress);
        let ax = fig.subplot();
        for left in [0.0, 2.0] {
            let options = ImshowOptions {
                extent: Some(Extent::new(left, left + 1.0, 0.0, 1.0)),
                ..ImshowOptions::default()
            };
            ax.imshow(Grid::filled(2, 2, left), options).unwrap();
        }
        ax.set_xlim(0.0, 3.0).expect("finite limits");
        fig.to_svg().unwrap().matches("<image").count()
    }

    #[test]
    fn suppressing_composites_keeps_images_apart() {
        assert_eq!(svg_images(None), 1, "vector formats composite");
        assert_eq!(svg_images(Some(true)), 2);
        assert_eq!(svg_images(Some(false)), 1);
    }

    #[test]
    fn adjacent_box_images_composite_unless_unsampled() {
        let svg_images = |interpolation: Option<Interpolation>| {
            let fig = Figure::new();
            let ax = fig.subplot();
            for bottom in [0.1, 0.6] {
                let options = BboxImageOptions {
                    interpolation,
                    ..BboxImageOptions::default()
                };
                ax.add_bbox_image(
                    Grid::from_fn(1, 4, |_, j| j as f64),
                    Bbox::from_extents(0.1, bottom, 0.9, bottom + 0.3),
                    ax.trans_axes(),
                    options,
                )
                .unwrap();
            }
            fig.to_svg().unwrap().matches("<image").count()
        };
        assert_eq!(svg_images(None), 1);
        assert_eq!(svg_images(Some(Interpolation::None)), 2);
    }
}
