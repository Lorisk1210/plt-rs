//! Patches placed in 3-D with their path codes kept, so that curves stay
//! curves (matplotlib's `PathPatch3D` and `pathpatch_2d_to_3d`).

use super::art3d::{Art3D, Path3D, ProjectedPath};
use super::coords::Axis3;
use super::error::Axes3DError;
use super::handle::Axes3D;
use super::plot::broadcast;
use crate::artist::Artist;
use crate::colors::Color;
use crate::patches::{Patch, PatchArtist, PatchProps, PatchShape};

impl Axes3D {
    /// Places the outline of `shape` in 3-D and adds it: the shape's path
    /// through its patch transform (in data coordinates) lies in the plane
    /// normal to `zdir`, each vertex at its `zs` along it (one position
    /// per vertex, or one for all), and keeps its codes (matplotlib's
    /// `add_artist(PathPatch3D(path, zs=zs, zdir=zdir))`, or
    /// `pathpatch_2d_to_3d` of a patch added with `add_patch`). The patch
    /// keeps the style the shape gives it (an arc is not filled), restyled
    /// by `props`, draws an edge by default only if the shape does (a
    /// [`PathShape`](crate::patches::PathShape) like matplotlib's
    /// `PathPatch3D` does, a filled circle does not), and leaves the data
    /// limits alone.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Axis3;
    /// use pltrs::patches::{Arc, PatchProps};
    /// let ax = Figure::new().subplot_3d();
    /// let arc = Arc::new((0.5, 0.5), 0.5, 0.9).between(10.0, 130.0);
    /// let before = ax.xlim();
    /// ax.add_path_patch3d(&arc, PatchProps::default(), &[0.0], Axis3::Z)?;
    /// assert_eq!(ax.xlim(), before);
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `zs` has neither one value nor one per vertex.
    pub fn add_path_patch3d<S: PatchShape>(
        &self,
        shape: &S,
        props: PatchProps,
        zs: &[f64],
        zdir: Axis3,
    ) -> Result<Patch<ProjectedPath>, Axes3DError> {
        let path = shape.path().transformed(&shape.patch_transform());
        let zs = broadcast("zs", zs, path.len())?;
        let vertices = (path.vertices().iter().zip(&zs))
            .map(|(v, &z)| zdir.juggle([v.x, v.y, z]))
            .collect();
        let rc = crate::rc::params();
        let styled = PatchArtist::new(shape.clone(), rc.clone());
        let mut patch = PatchArtist::new(ProjectedPath(path.clone()), rc);
        patch.style = styled.style.clone();
        *patch.base_mut() = styled.base().clone();
        props.apply(&mut patch);
        // matplotlib settles the default edge when the 2-D patch is made,
        // before it turns into a path patch, which would draw one.
        if !S::EDGE_DEFAULT
            && patch.style.edgecolor.is_none()
            && patch.style.fill
            && !crate::rc::params().patch_force_edgecolor
        {
            patch.style.edgecolor = Some(Color::none());
        }
        let art = Art3D::PathPatch(Path3D::new(path, vertices));
        Ok(Patch::from_raw(self.add_artist(patch, art, false)))
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::{Path, PathCode};

    use super::*;
    use crate::figure::Figure;
    use crate::patches::{Arc, Circle, PathShape};

    #[test]
    fn path_patches_keep_their_codes_and_style() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let arc = Arc::new((0.5, 0.5), 0.5, 0.9)
            .with_angle(20.0)
            .between(10.0, 130.0);
        let curve = ax
            .add_path_patch3d(&arc, PatchProps::default(), &[0.0], Axis3::Z)
            .unwrap();
        fig.layout();
        let projected = curve.shape().0;
        assert!(projected.resolved_codes().contains(&PathCode::Curve4));
        assert_eq!(curve.colors().unwrap().face.a, 0.0, "arcs are not filled");
        let square = PathShape(Path::unit_rectangle());
        assert!(
            ax.add_path_patch3d(&square, PatchProps::default(), &[0.0, 1.0], Axis3::Y)
                .is_err(),
            "two positions for five vertices"
        );
        let lifted = ax
            .add_path_patch3d(
                &square,
                PatchProps::default(),
                &[0.0, 0.5, 0.7, 1.0, 0.0],
                Axis3::Y,
            )
            .unwrap();
        fig.layout();
        assert!(lifted.shape().0.vertices().iter().all(|p| p.is_finite()));
    }

    #[test]
    fn lifted_shapes_keep_their_default_edges() {
        let ax = Figure::new().subplot_3d();
        let edge_alpha = |patch: Patch<ProjectedPath>| patch.colors().unwrap().edge.a;
        let circle = Circle::new((5.0, 5.0), 3.0);
        let lifted = ax.add_path_patch3d(&circle, PatchProps::default(), &[0.0], Axis3::X);
        assert_eq!(
            edge_alpha(lifted.unwrap()),
            0.0,
            "a filled circle has no edge"
        );
        let outlined = PatchProps::default().edgecolor("k");
        let lifted = ax.add_path_patch3d(&circle, outlined, &[0.0], Axis3::X);
        assert_eq!(edge_alpha(lifted.unwrap()), 1.0);
        let path = PathShape(circle.path().transformed(&circle.patch_transform()));
        let lifted = ax.add_path_patch3d(&path, PatchProps::default(), &[0.0], Axis3::X);
        assert_eq!(edge_alpha(lifted.unwrap()), 1.0, "path patches have edges");
    }
}
