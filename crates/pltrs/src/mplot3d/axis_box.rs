//! The box a 3-D axis is drawn on as a program gives it: matplotlib's
//! `Axis3D.draw`, `draw_pane` and `draw_grid` ask the axis for it
//! (`_get_coord_info`, and `_calc_centers_deltas` for its centre and the
//! offsets of the ticks and labels), so a program replacing those methods
//! on an axis (qutip shrinks the box of its histograms) moves the pane,
//! the axis line, the ticks and the labels of that axis.

use super::coords::Axis3;
use super::geometry::BoxGeometry;
use super::handle::Axes3D;
use super::math::Vec3;
use super::state::Axes3DState;
use crate::artist::ArtistHandle;

/// The box one axis of a 3-D axes is drawn on, in transformed coordinates
/// (the axis scales applied), as matplotlib's `_get_coord_info` and
/// `_calc_centers_deltas` give it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisBox {
    /// The low corner of the box (`mins`).
    pub mins: Vec3,
    /// The high corner of the box (`maxs`).
    pub maxs: Vec3,
    /// The eight corners projected (`bounds_proj`), in the order of
    /// matplotlib's `_transformed_cube`.
    pub corners: [Vec3; 8],
    /// For each axis, whether its high plane is the far one (`highs`).
    pub highs: [bool; 3],
    /// The centre of the box (`centers`).
    pub centers: Vec3,
    /// How far the ticks and labels stand off the box (`deltas`).
    pub deltas: Vec3,
}

impl From<AxisBox> for BoxGeometry {
    fn from(given: AxisBox) -> Self {
        Self {
            mins: given.mins,
            maxs: given.maxs,
            corners: given.corners,
            highs: given.highs,
            centers: given.centers,
            deltas: given.deltas,
        }
    }
}

impl Axes3D {
    /// Draws `axis` on `given` from the next draw on, instead of the box
    /// of the axes' limits (`None`): what a program's `_get_coord_info`
    /// answers in matplotlib's draws, which a program hands over as the
    /// figure is drawn (the change leaves the figure as fresh as it was).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::{Axis3, AxisBox};
    /// let fig = Figure::new();
    /// let ax = fig.subplot_3d();
    /// let corner = [0.0, 0.0, 0.0];
    /// ax.set_axis_box(Axis3::X, Some(AxisBox {
    ///     mins: [0.25; 3],
    ///     maxs: [0.75; 3],
    ///     corners: [corner; 8],
    ///     highs: [true; 3],
    ///     centers: [0.5; 3],
    ///     deltas: [0.04; 3],
    /// }));
    /// let mut canvas = fig.raster_canvas()?;
    /// fig.draw(&mut canvas)?;
    /// ax.set_axis_box(Axis3::X, None);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_axis_box(&self, axis: Axis3, given: Option<AxisBox>) -> &Self {
        self.raw().with_mut_unseen(|axes: &mut Axes3DState| {
            axes.axis_mut(axis).program_box = given.map(BoxGeometry::from);
        });
        self
    }
}
