//! The radial frame a line in polar axes draws with: matplotlib's line
//! keeps its path mapped through the polar transform
//! (`Line2D._transformed_path`, a `TransformedPath`), which only a new
//! path or transform makes it map again. `PolarTransform` reads the
//! radial origin and sign of its Axes without depending on them, so a
//! change of the radial limits after a draw leaves the line drawn with the
//! origin and sign of that draw.

use std::cell::RefCell;

use crate::polar::{RadialFrame, radial_frame, with_mapped_frame};
use crate::transforms::{ResolvedTransform, Transform};

/// The radial frame a line was first drawn with, and the transform it was
/// drawn through; pickled with the line, as matplotlib's line pickles its
/// transformed path.
#[derive(Clone, Debug, Default)]
pub(crate) struct KeptPolarFrame(RefCell<Option<(Transform, RadialFrame)>>);

pltrs_core::archive_struct!(KeptPolarFrame(v0));

impl KeptPolarFrame {
    /// `resolved`, the line's `transform` resolved for a draw, mapping
    /// with the radial frame the line kept from an earlier draw through
    /// the same transform, else with its own, which the line keeps.
    pub(super) fn drawn(
        &self,
        transform: &Transform,
        resolved: ResolvedTransform,
    ) -> ResolvedTransform {
        let Some(now) = radial_frame(&resolved) else {
            return resolved;
        };
        let mut kept = self.0.borrow_mut();
        let frame = match kept.as_ref() {
            Some((kept_transform, frame)) if kept_transform == transform => *frame,
            _ => now,
        };
        *kept = Some((transform.clone(), frame));
        if frame == now {
            resolved
        } else {
            with_mapped_frame(&resolved, frame)
        }
    }

    /// Forgets the frame: the line's path changed.
    pub(super) fn forget(&self) {
        self.0.borrow_mut().take();
    }
}

#[cfg(test)]
mod tests {
    use crate::figure::Figure;

    /// Blue pixels of the figure, drawn.
    fn blue(fig: &Figure) -> usize {
        let mut canvas = fig.raster_canvas().expect("a canvas");
        fig.draw(&mut canvas).expect("drawn");
        canvas
            .pixmap()
            .data()
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[2] > 150 && p[0] < 100)
            .count()
    }

    #[test]
    fn limits_changed_after_a_draw_keep_the_frame_of_the_line() {
        let fig = Figure::with_size(4.0, 4.0, 50.0);
        let polar = fig.polar_subplot();
        let line = polar
            .plot(vec![0.0, 1.0, 2.0, 3.0], vec![3.0, 1.0, 4.0, 1.5])
            .expect("plotted");
        assert!(blue(&fig) > 0);
        // Radii beyond the origin of the new limits are not drawn when
        // the line is mapped anew; matplotlib's line keeps its mapping.
        polar.set_rlim(2.0, 0.0).expect("finite limits");
        let kept = blue(&fig);
        assert!(kept > 0, "the line keeps the frame of the first draw");
        line.set_data(vec![0.0, 1.0, 2.0, 3.0], vec![3.0, 1.0, 4.0, 1.5]);
        assert!(
            blue(&fig) < kept,
            "new data map with the limits as they are"
        );
    }
}
