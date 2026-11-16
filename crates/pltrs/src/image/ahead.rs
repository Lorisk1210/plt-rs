//! Images made where matplotlib's draw makes them, ahead of pltrs' paint:
//! a figure whose drawing calls the program goes through its artists in
//! drawing order before it paints them (see `transform_sites`), calling
//! the program as matplotlib's draw does; an image whose norm is the
//! program's is made there, its norm called among the others, and the
//! paint draws what was made.

use pltrs_core::Bbox;

use super::ImageError;
use super::artist::AxesImageArtist;
use super::make::{Placement, RenderedImage};
use crate::artist::{Artist, ArtistId, LayoutContext};
use crate::figure::FigureState;

/// An image made ahead of its paint, with the boxes it was made for.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MadeAhead {
    /// The image's display box (matplotlib's `out_bbox`).
    out_bbox: Bbox,
    /// The box it was clipped to.
    clip: Bbox,
    /// The magnification it was made at.
    magnification: f64,
    /// The image made, or why it could not be.
    made: Result<Option<RenderedImage>, ImageError>,
}

impl MadeAhead {
    /// The image `made` for `placement`.
    fn new(placement: &Placement, made: Result<Option<RenderedImage>, ImageError>) -> Self {
        Self {
            out_bbox: placement.out_bbox,
            clip: placement.clip,
            magnification: placement.magnification,
            made,
        }
    }

    /// The image made for `placement`, if it was made for the same boxes
    /// at the same magnification.
    pub(crate) fn for_placement(
        self,
        placement: &Placement,
    ) -> Option<Result<Option<RenderedImage>, ImageError>> {
        #[expect(
            clippy::float_cmp,
            reason = "only the very magnification the image was made at matches"
        )]
        let same_magnification = placement.magnification == self.magnification;
        (same_magnification && self.out_bbox == placement.out_bbox && self.clip == placement.clip)
            .then_some(self.made)
    }
}

impl FigureState {
    /// `AxesImage.draw` of the image `id`, as far as it calls the
    /// program: made through a norm of the program's, at the figure's
    /// resolution magnified as a print's renderer magnifies images, on the
    /// canvas the figure is drawn on (the window printed, else the whole
    /// figure), and kept for the paint; the figure is lent meanwhile, as
    /// the norm may read it.
    pub(crate) fn program_image_drawn(&mut self, id: ArtistId) {
        let Some(image) = self.store.shared_as::<AxesImageArtist>(id) else {
            return;
        };
        if !image.base().visible || !image.norm_calls_program() {
            return;
        }
        let Some(canvas) = self.frame_patch_bbox(self.root) else {
            return;
        };
        let magnification = self.print_image_magnification;
        let Some(placement) = image.placement(LayoutContext::new(self), canvas, magnification)
        else {
            return;
        };
        let alpha = image.core.alpha(image.base().alpha);
        if let Some(made) = self.lend(|_| image.core.make_image(&placement, alpha.as_ref())) {
            image
                .made_ahead
                .replace(Some(MadeAhead::new(&placement, made)));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::artist::ArtistHandle;
    use crate::axes::ImshowOptions;
    use crate::colors::{ArrayForm, Norm, NormError, Normalized};
    use crate::figure::Figure;
    use crate::image::Interpolation;
    use crate::numeric::Grid;

    /// A linear norm over `[0, 1]` standing for one of the program's,
    /// noting the shape of every array it normalises.
    #[derive(Clone, Debug, Default)]
    struct Program(Rc<RefCell<Vec<Vec<usize>>>>);

    impl Norm for Program {
        fn vmin(&self) -> Option<f64> {
            Some(0.0)
        }

        fn vmax(&self) -> Option<f64> {
            Some(1.0)
        }

        fn set_vmin(&mut self, _: Option<f64>) {}

        fn set_vmax(&mut self, _: Option<f64>) {}

        fn clip(&self) -> bool {
            false
        }

        fn set_clip(&mut self, _: bool) {}

        fn autoscale_none(&mut self, _: &[Option<f64>]) {}

        fn normalize_masked(
            &mut self,
            data: &[Option<f64>],
            _: Option<bool>,
        ) -> Result<Vec<Normalized>, NormError> {
            self.0.borrow_mut().push(vec![data.len()]);
            Ok(data
                .iter()
                .map(|v| v.map_or(Normalized::Masked, Normalized::Fraction))
                .collect())
        }

        fn normalize_shaped(
            &mut self,
            data: &[Option<f64>],
            clip: Option<bool>,
            form: ArrayForm<'_>,
        ) -> Result<Vec<Normalized>, NormError> {
            let normalized = self.normalize_masked(data, clip);
            if let Some(last) = self.0.borrow_mut().last_mut() {
                *last = form.shape.to_vec();
            }
            normalized
        }

        fn inverse(&self, values: &[f64]) -> Result<Vec<Option<f64>>, NormError> {
            Ok(values.iter().copied().map(Some).collect())
        }

        fn calls_program(&self) -> bool {
            true
        }
    }

    #[test]
    fn images_made_ahead_are_painted_as_made() {
        let fig = Figure::with_size(2.0, 2.0, 50.0);
        let ax = fig.subplot();
        let options = ImshowOptions {
            interpolation: Some(Interpolation::Nearest),
            ..ImshowOptions::default()
        };
        let image = ax
            .imshow(Grid::from_fn(2, 3, |r, c| (r + c) as f64 / 3.0), options)
            .unwrap();
        let norm = Program::default();
        let shapes = Rc::clone(&norm.0);
        image.set_norm(Box::new(norm));
        shapes.borrow_mut().clear();
        fig.to_png().unwrap();
        let painted = shapes.take();
        assert_eq!(painted.len(), 1, "{painted:?}");
        assert_eq!(painted[0].len(), 2, "the array of the output's shape");
        let figure = Rc::clone(image.raw().figure_state());
        figure.work_on(|state| state.program_image_drawn(image.id()));
        assert_eq!(shapes.take(), painted, "made ahead as the paint makes it");
        fig.to_png().unwrap();
        assert!(shapes.borrow().is_empty(), "painted as made");
        fig.to_png().unwrap();
        assert_eq!(shapes.take(), painted, "made again by the next paint");
    }
}
