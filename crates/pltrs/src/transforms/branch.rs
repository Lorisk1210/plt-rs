//! Which transforms a transform ends with (matplotlib's `contains_branch`
//! and `contains_branch_separately`).
//!
//! matplotlib answers these by flattening a chain of composite transforms
//! and comparing its tails with the branch. An axes' data transform is
//! itself a chain ending with the axes box (`transScale + transLimits +
//! transAxes`), so it contains `transAxes`; the identity disappears from
//! every chain, as matplotlib's composition drops `IdentityTransform`.

use super::Transform;
use crate::artist::ArtistId;

/// One link of a flattened transform chain.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Link<'a> {
    /// The part of an axes' data transform before its box: the scales,
    /// limits and projection (`transScale + transLimits`).
    DataToAxes(ArtistId),
    /// The box of an axes (`transAxes`), the end of its data transform.
    AxesBox(ArtistId),
    /// Any other transform, compared as a whole; a blend is one link.
    Whole(&'a Transform),
}

impl Transform {
    /// The links of the chain, the first applied first.
    fn links(&self) -> Vec<Link<'_>> {
        let mut links = Vec::new();
        self.push_links(&mut links);
        links
    }

    /// Appends the links of the chain to `links`.
    fn push_links<'a>(&'a self, links: &mut Vec<Link<'a>>) {
        match self {
            Self::Display => {}
            Self::Composite(first, second) => {
                first.push_links(links);
                second.push_links(links);
            }
            Self::Data(axes) => links.extend([Link::DataToAxes(*axes), Link::AxesBox(*axes)]),
            Self::Axes(axes) => links.push(Link::AxesBox(*axes)),
            other => links.push(Link::Whole(other)),
        }
    }

    /// Whether this transform ends with `branch`, the whole transform
    /// included (matplotlib's `contains_branch`): applying it means
    /// applying `branch` last. Composites compare link by link, so `(a +
    /// b) + c` and `a + (b + c)` both contain `b + c`.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::transforms::{Affine2D, Transform};
    /// let ax = Figure::new().subplot();
    /// let data = Transform::Data(ax.id());
    /// let shifted = Transform::Affine(Affine2D::translation(1.0, 2.0)).then(data.clone());
    /// assert!(shifted.contains_branch(&data));
    /// assert!(data.contains_branch(&Transform::Axes(ax.id())));
    /// assert!(!data.contains_branch(&shifted));
    /// ```
    pub fn contains_branch(&self, branch: &Self) -> bool {
        let (links, branch) = (self.links(), branch.links());
        if branch.is_empty() {
            // Only the identity contains the identity.
            return links.is_empty();
        }
        links.ends_with(&branch)
    }

    /// Whether the x and the y outputs pass through `branch` last
    /// (matplotlib's `contains_branch_separately`): both when the
    /// transform [contains](Self::contains_branch) it, each component of
    /// a blend it ends with on its own, neither otherwise.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::transforms::Transform;
    /// let ax = Figure::new().subplot();
    /// let data = Transform::Data(ax.id());
    /// let x_data = Transform::blended(data.clone(), Transform::Axes(ax.id()));
    /// assert_eq!(x_data.contains_branch_separately(&data), (true, false));
    /// ```
    pub fn contains_branch_separately(&self, branch: &Self) -> (bool, bool) {
        if self.contains_branch(branch) {
            return (true, true);
        }
        match self.links().last() {
            Some(Link::Whole(Self::Blended(x, y))) => {
                (x.contains_branch(branch), y.contains_branch(branch))
            }
            _ => (false, false),
        }
    }

    /// Whether the x and y outputs pass through the data coordinates of
    /// `axes` last (matplotlib's `contains_branch_separately(transData)`),
    /// i.e. which directions of points in this system are data values.
    pub fn data_directions(&self, axes: ArtistId) -> (bool, bool) {
        self.contains_branch_separately(&Self::Data(axes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::transforms::Affine2D;

    fn shift(dx: f64) -> Transform {
        Transform::Affine(Affine2D::translation(dx, 0.0))
    }

    #[test]
    fn chains_contain_their_tails() {
        let fig = Figure::new();
        let ax = fig.subplot().id();
        let data = Transform::Data(ax);
        let left = shift(1.0).then(shift(2.0)).then(data.clone());
        let right = shift(1.0).then(shift(2.0).then(data.clone()));
        let tail = shift(2.0).then(data.clone());
        for chain in [&left, &right] {
            assert!(chain.contains_branch(chain));
            assert!(chain.contains_branch(&tail));
            assert!(chain.contains_branch(&data));
            assert!(chain.contains_branch(&Transform::Axes(ax)));
            assert!(!chain.contains_branch(&shift(1.0).then(data.clone())));
            assert!(!chain.contains_branch(&shift(2.0)));
        }
        assert!(!data.contains_branch(&left));
        assert!(!Transform::Axes(ax).contains_branch(&data));
    }

    #[test]
    fn other_axes_and_other_systems_differ() {
        let fig = Figure::new();
        let [a, b] = [1, 2].map(|i| fig.add_subplot(1, 2, i).expect("a cell").id());
        assert!(!Transform::Data(a).contains_branch(&Transform::Data(b)));
        assert!(!Transform::Inches.contains_branch(&Transform::Figure(fig.id())));
        assert!(
            Transform::Inverted(Box::new(Transform::Data(a)))
                .contains_branch(&Transform::Inverted(Box::new(Transform::Data(a))))
        );
        assert!(!shift(f64::NAN).contains_branch(&shift(f64::NAN)));
    }

    #[test]
    fn the_identity_vanishes_from_chains() {
        let fig = Figure::new();
        let data = Transform::Data(fig.subplot().id());
        assert!(Transform::Display.then(data.clone()).contains_branch(&data));
        assert!(data.clone().then(Transform::Display).contains_branch(&data));
        assert!(Transform::Display.contains_branch(&Transform::Display));
        assert!(!data.contains_branch(&Transform::Display));
    }

    #[test]
    fn blends_contain_branches_per_direction() {
        let fig = Figure::new();
        let ax = fig.subplot().id();
        let data = Transform::Data(ax);
        let x_data = Transform::blended(data.clone(), Transform::Axes(ax));
        assert!(!x_data.contains_branch(&data));
        assert_eq!(x_data.contains_branch_separately(&data), (true, false));
        assert_eq!(
            shift(1.0)
                .then(x_data.clone())
                .contains_branch_separately(&data),
            (true, false)
        );
        assert_eq!(
            x_data.clone().then(shift(1.0)).data_directions(ax),
            (false, false)
        );
        let y_data = Transform::blended(Transform::Axes(ax), shift(3.0).then(data.clone()));
        assert_eq!(y_data.data_directions(ax), (false, true));
        assert_eq!(shift(1.0).then(data).data_directions(ax), (true, true));
        assert_eq!(Transform::Axes(ax).data_directions(ax), (false, false));
    }
}
