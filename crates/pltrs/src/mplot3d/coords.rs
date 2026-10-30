//! The three data axes and the reordering of coordinates that places 2-D
//! data in a plane of 3-D space (matplotlib's `zdir`, `juggle_axes` and
//! `rotate_axes`).

use super::math::Vec3;

/// One of the three data axes; as a `zdir` it is the direction normal to
/// the plane 2-D data is placed in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Axis3 {
    /// The x axis.
    X,
    /// The y axis.
    Y,
    /// The z axis.
    #[default]
    Z,
}

pltrs_core::archive_enum!(Axis3 { X, Y, Z });

impl Axis3 {
    /// The three axes in order.
    pub const ALL: [Self; 3] = [Self::X, Self::Y, Self::Z];

    /// The position of the axis in `[x, y, z]`.
    pub const fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }

    /// matplotlib's name for the axis.
    pub const fn name(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Y => "y",
            Self::Z => "z",
        }
    }

    /// The unit vector along the axis (matplotlib's `get_dir_vector`).
    pub const fn unit(self) -> Vec3 {
        match self {
            Self::X => [1.0, 0.0, 0.0],
            Self::Y => [0.0, 1.0, 0.0],
            Self::Z => [0.0, 0.0, 1.0],
        }
    }

    /// Places the 2-D point `(a, b)` in the plane normal to this axis, at
    /// `c` along it (matplotlib's `juggle_axes(a, b, c, zdir)`); the
    /// coordinates may be anything, such as lists of them.
    pub fn juggle<T>(self, [a, b, c]: [T; 3]) -> [T; 3] {
        match self {
            Self::X => [c, a, b],
            Self::Y => [a, c, b],
            Self::Z => [a, b, c],
        }
    }

    /// Undoes [`Axis3::juggle`]: the 2-D point and the position along
    /// this axis of a point placed in the plane normal to it.
    pub fn unjuggle<T>(self, [x, y, z]: [T; 3]) -> [T; 3] {
        match self {
            Self::X => [y, z, x],
            Self::Y => [x, z, y],
            Self::Z => [x, y, z],
        }
    }

    /// Reorders a point so that this axis becomes the z axis
    /// (matplotlib's `rotate_axes(x, y, z, zdir)`).
    pub const fn rotate(self, [x, y, z]: Vec3) -> Vec3 {
        match self {
            Self::X => [y, z, x],
            Self::Y => [z, x, y],
            Self::Z => [x, y, z],
        }
    }

    /// Undoes [`Axis3::rotate`] (matplotlib's `rotate_axes` with a `-`
    /// prefixed `zdir`).
    pub const fn unrotate(self, [x, y, z]: Vec3) -> Vec3 {
        match self {
            Self::X => [z, x, y],
            Self::Y => [y, z, x],
            Self::Z => [x, y, z],
        }
    }
}

/// `a` rotated by `shift` places like numpy's `roll`.
pub(crate) fn roll(a: Vec3, shift: isize) -> Vec3 {
    let mut out = [0.0; 3];
    for (i, value) in a.into_iter().enumerate() {
        let target = (i as isize + shift).rem_euclid(3) as usize;
        out[target] = value;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_is_undone_by_unrotation() {
        let p = [1.0, 2.0, 3.0];
        for axis in Axis3::ALL {
            assert_eq!(axis.unrotate(axis.rotate(p)), p);
        }
        assert_eq!(Axis3::X.rotate(p), [2.0, 3.0, 1.0]);
        assert_eq!(Axis3::Y.rotate(p), [3.0, 1.0, 2.0]);
    }

    #[test]
    fn juggling_places_the_third_coordinate_along_the_axis() {
        assert_eq!(Axis3::X.juggle([1.0, 2.0, 9.0]), [9.0, 1.0, 2.0]);
        assert_eq!(Axis3::Y.juggle([1.0, 2.0, 9.0]), [1.0, 9.0, 2.0]);
        assert_eq!(Axis3::Z.juggle([1.0, 2.0, 9.0]), [1.0, 2.0, 9.0]);
        for axis in Axis3::ALL {
            assert_eq!(axis.unjuggle(axis.juggle(["a", "b", "c"])), ["a", "b", "c"]);
        }
    }

    #[test]
    fn roll_matches_numpy() {
        assert_eq!(roll([1.0, 2.0, 3.0], 1), [3.0, 1.0, 2.0]);
        assert_eq!(roll([1.0, 2.0, 3.0], -2), [3.0, 1.0, 2.0]);
        assert_eq!(roll([1.0, 2.0, 3.0], 0), [1.0, 2.0, 3.0]);
    }
}
