//! Patches of the projection plane turned into 3-D patches in place
//! (matplotlib's `patch_2d_to_3d` and `pathpatch_2d_to_3d`, which change
//! the class of the patch): the patch keeps its id, its place among the
//! plane's artists, its style and its properties, and follows a 3-D
//! outline or path from then on.

use std::rc::Rc;

use pltrs_core::Path;

use super::art3d::{Art3D, Path3D, ProjectedOutline, ProjectedPath};
use super::error::Axes3DError;
use super::handle::Axes3D;
use super::math::Vec3;
use crate::artist::{Artist, ArtistHandle, ArtistLookup, KeptArtist, RawHandle};
use crate::colors::Color;
use crate::figure::FigureState;
use crate::patches::{Patch, PatchArtist, PatchShape, PatchStyle};

impl Axes3D {
    /// Turns `patch`, a patch of any shape in the projection plane, into a
    /// flat patch following the 3-D polygon `outline` (matplotlib's
    /// `patch_2d_to_3d`, the outline being the patch's first polygon
    /// placed in 3-D); `axlim_clip` hides the corners outside the view
    /// limits. The data limits stay as they are.
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::figure::Figure;
    /// use pltrs::patches::Circle;
    /// let fig = Figure::new();
    /// let ax = fig.subplot_3d();
    /// let circle = ax.projection_plane().add_patch(Circle::new((0.0, 0.0), 1.0));
    /// let outline = vec![[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, 1.0], [0.0, 0.0, 1.0]];
    /// let lifted = ax.patch_2d_to_3d(circle.raw(), outline, false)?;
    /// assert_eq!(lifted.id(), circle.id());
    /// fig.layout();
    /// assert_eq!(lifted.shape().0.len(), 4);
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `patch` is not a patch of the axes' projection plane.
    pub fn patch_2d_to_3d(
        &self,
        patch: &RawHandle,
        outline: Vec<Vec3>,
        axlim_clip: bool,
    ) -> Result<Patch<ProjectedOutline>, Axes3DError> {
        self.replace_patch(
            patch,
            ProjectedOutline(Path::default()),
            Art3D::Patch(outline),
            axlim_clip,
        )
    }

    /// Turns `patch`, a patch of any shape in the projection plane, into a
    /// patch following `path` (its outline in data coordinates, codes
    /// kept) with its vertices placed at `vertices` in 3-D (matplotlib's
    /// `pathpatch_2d_to_3d`), so that curves stay curves.
    ///
    /// # Errors
    ///
    /// When `patch` is not a patch of the axes' projection plane, or
    /// `vertices` does not have one point per vertex of `path`.
    pub fn pathpatch_2d_to_3d(
        &self,
        patch: &RawHandle,
        path: Path,
        vertices: Vec<Vec3>,
    ) -> Result<Patch<ProjectedPath>, Axes3DError> {
        if vertices.len() != path.len() {
            return Err(Axes3DError::Shape {
                argument: "zs",
                expected: path.len(),
                actual: vertices.len(),
            });
        }
        let shape = ProjectedPath(path.clone());
        self.replace_patch(
            patch,
            shape,
            Art3D::PathPatch(Path3D::new(path, vertices)),
            false,
        )
    }

    /// The path a 3-D patch of these axes is drawn with: that of its last
    /// draw, else its projection in the figure's current layout, which is
    /// brought up to date first (matplotlib's `Patch3D.get_path`, which
    /// projects the patch if it was not drawn yet).
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::figure::Figure;
    /// use pltrs::patches::Circle;
    /// let ax = Figure::new().subplot_3d();
    /// let circle = ax.projection_plane().add_patch(Circle::new((0.0, 0.0), 1.0));
    /// let outline = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]];
    /// ax.patch_2d_to_3d(circle.raw(), outline, false)?;
    /// assert_eq!(ax.patch_path(circle.raw())?.len(), 4);
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `patch` is not a 3-D patch of these axes.
    pub fn patch_path(&self, patch: &RawHandle) -> Result<Path, Axes3DError> {
        self.read_art(patch, |art| {
            matches!(art, Art3D::Patch(_) | Art3D::PathPatch(_)).then_some(())
        })?;
        self.raw().figure_state().work_on(FigureState::prepare);
        let drawn = self.with(|axes| {
            axes.artists
                .iter()
                .any(|artist| artist.id == patch.id() && artist.drawn)
        });
        if !drawn {
            self.project_artist(patch, |projection| projection)?;
        }
        (patch
            .downcast::<Patch<ProjectedOutline>>()
            .map(|p| p.shape().0))
        .or_else(|| {
            patch
                .downcast::<Patch<ProjectedPath>>()
                .map(|p| p.shape().0)
        })
        .ok_or(Axes3DError::Not3DArtist)
    }

    /// Replaces `patch` by a patch of `shape` with its style and common
    /// properties, under the same id, with the 3-D geometry `art`.
    fn replace_patch<S: PatchShape>(
        &self,
        patch: &RawHandle,
        shape: S,
        art: Art3D,
        axlim_clip: bool,
    ) -> Result<Patch<S>, Axes3DError> {
        let figure = self.raw().figure_state();
        if !Rc::ptr_eq(figure, patch.figure_state()) {
            return Err(Axes3DError::Not3DArtist);
        }
        let plane = self.plane().id();
        let id = patch.id();
        let mut state = figure.borrow_mut();
        let lifted = {
            let old = state
                .artist(id)
                .filter(|artist| artist.base().parent == Some(plane))
                .ok_or(Axes3DError::Not3DArtist)?;
            let view = old.as_patch().ok_or(Axes3DError::Not3DArtist)?;
            let mut lifted = PatchArtist::new(shape, crate::rc::params());
            lifted.style = settled_edge(view.style.clone(), view.edge_default, S::EDGE_DEFAULT);
            *lifted.base_mut() = old.base().clone();
            lifted
        };
        state.store.take(id);
        state.store.put_back(id, KeptArtist::new(lifted));
        state.stale = true;
        drop(state);
        self.with_mut(|axes| axes.artists.retain(|artist| artist.id != id));
        self.register(id, art, axlim_clip);
        Ok(Patch::from_raw(patch.clone()))
    }
}

/// `style` with its default edge settled as the shape it was made with
/// (`edge_default`) draws it, where a shape whose default is
/// `new_default` would draw it otherwise: matplotlib resolves the default
/// edge colour when a patch is made, before its class changes.
fn settled_edge(mut style: PatchStyle, edge_default: bool, new_default: bool) -> PatchStyle {
    let rc = crate::rc::params();
    if style.edgecolor.is_none()
        && style.fill
        && !rc.patch_force_edgecolor
        && edge_default != new_default
    {
        style.edgecolor = Some(if edge_default {
            rc.patch_edgecolor.clone()
        } else {
            Color::none()
        });
    }
    style
}

#[cfg(test)]
mod tests {
    use pltrs_core::PathCode;

    use super::*;
    use crate::figure::Figure;
    use crate::patches::{Arc, Circle};

    #[test]
    fn path_patches_keep_their_curves_and_their_place() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let plane = ax.projection_plane();
        let arc = Arc::new((0.5, 0.5), 0.5, 0.9).between(10.0, 130.0);
        let patch = plane.add_patch(arc.clone());
        let path = arc.path().transformed(&arc.patch_transform());
        let vertices = path.vertices().iter().map(|v| [v.x, v.y, 0.0]).collect();
        let lifted = ax.pathpatch_2d_to_3d(patch.raw(), path, vertices).unwrap();
        fig.layout();
        assert!(
            lifted
                .shape()
                .0
                .resolved_codes()
                .contains(&PathCode::Curve4)
        );
        assert_eq!(plane.patches(), [patch.raw().clone()]);
        assert_eq!(lifted.colors().unwrap().face.a, 0.0, "arcs stay unfilled");
    }

    #[test]
    fn lifted_circles_keep_no_edge_and_only_plane_patches_lift() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let circle = Circle::new((0.0, 0.0), 1.0);
        let patch = ax.projection_plane().add_patch(circle.clone());
        let path = circle.path().transformed(&circle.patch_transform());
        let vertices = path.vertices().iter().map(|v| [v.x, v.y, 0.0]).collect();
        let lifted = ax.pathpatch_2d_to_3d(patch.raw(), path, vertices).unwrap();
        assert_eq!(lifted.colors().unwrap().edge.a, 0.0, "a circle has no edge");
        assert!(
            ax.patch_2d_to_3d(ax.projection_plane().raw(), Vec::new(), false)
                .is_err()
        );
        let other = Figure::new().subplot_3d();
        assert!(
            other
                .patch_2d_to_3d(patch.raw(), Vec::new(), false)
                .is_err()
        );
        assert!(
            ax.pathpatch_2d_to_3d(patch.raw(), Path::unit_rectangle(), Vec::new())
                .is_err()
        );
    }
}
