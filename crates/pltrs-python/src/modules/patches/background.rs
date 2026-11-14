//! The backgrounds of axes and figures as patches (matplotlib's
//! `Axes.patch` and `Figure.patch`): pltrs (Rust) draws a background as
//! part of its axes or figure, and the package hands out a loose patch
//! mirroring it, whose style is copied from the background
//! (`pull_background`) and back (`push_background`).

use pltrs::artist::ArtistHandle;
use pltrs::axes::Axes;
use pltrs::figure::Figure;

use super::AnyPatch;
use crate::binding::prelude::*;
use crate::modules::path::PyPath;

/// What a background belongs to: an axes (a 3-D axes' is that of its
/// projection plane), or a figure or subfigure.
enum Owner {
    Axes(Axes),
    Figure(Figure),
}

impl FromPy<'_> for Owner {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        object
            .extract::<Axes>()
            .map(Self::Axes)
            .or_else(|_| object.extract::<Figure>().map(Self::Figure))
    }
}

/// Gives the loose `patch` the style, opacity, visibility, z-order and
/// group id of the background of `owner`, leaving it as stale as it was
/// (it changes to show what is drawn).
fn pull_background(owner: Owner, patch: AnyPatch) -> PyResult<()> {
    let patch = patch.into_style();
    let raw = patch.raw_handle();
    let stale = raw.is_stale();
    let (style, alpha, visible, gid) = match &owner {
        Owner::Axes(axes) => {
            raw.set_zorder(axes.patch_zorder());
            let visible = axes.patch_visible();
            (
                axes.patch_style(),
                axes.patch_alpha(),
                visible,
                axes.patch_gid(),
            )
        }
        Owner::Figure(figure) => (
            figure.patch_style(),
            figure.alpha(),
            figure.frameon(),
            figure.patch_gid(),
        ),
    };
    patch.update_style(&mut |current| current.clone_from(&style));
    raw.set_alpha(alpha);
    raw.set_visible(visible);
    match gid {
        Some(gid) => raw.set_gid(gid),
        None => raw.clear_gid(),
    };
    raw.set_stale(stale);
    Ok(())
}

/// Gives the background of `owner` the style, opacity, visibility,
/// z-order and group id of the loose `patch` that mirrors it.
fn push_background(owner: Owner, patch: AnyPatch) -> PyResult<()> {
    let patch = patch.into_style();
    let raw = patch.raw_handle();
    match owner {
        Owner::Axes(axes) => {
            axes.set_patch_style(patch.style())
                .set_patch_alpha(raw.alpha())
                .set_patch_zorder(raw.zorder())
                .set_patch_gid(raw.gid());
            axes.set_patch_visible(raw.visible());
        }
        Owner::Figure(figure) => {
            figure
                .set_patch_style(patch.style())
                .set_frameon(raw.visible())
                .set_patch_gid(raw.gid());
            figure.set_alpha(raw.alpha());
        }
    }
    Ok(())
}

/// Makes `outline`, a path in fractions of `figure` (a figure or
/// subfigure), the shape of its background (the patch a program gave,
/// `fig.patch = patch`), or its box again with `None`.
fn set_figure_patch_outline(figure: Figure, outline: Option<PyPath>) -> PyResult<()> {
    figure.set_patch_outline(outline.map(|outline| outline.path));
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("pull_background", pull_background)?;
    module.add("push_background", push_background)?;
    module.add("set_figure_patch_outline", set_figure_patch_outline)
}

#[cfg(test)]
mod tests {
    use pltrs::patches::{Patch, PatchProps, Rectangle};
    use pltrs_core::Point;

    use super::*;

    /// A loose unit square, as `Rectangle((0, 0), 1, 1)` makes it.
    fn unit_square() -> Patch<Rectangle> {
        Patch::loose(
            Rectangle::new(Point::new(0.0, 0.0), 1.0, 1.0),
            PatchProps::default(),
        )
    }

    fn any(patch: &Patch<Rectangle>) -> AnyPatch {
        AnyPatch(Box::new(patch.clone()))
    }

    #[test]
    fn a_patch_takes_the_style_of_an_axes_background_and_gives_it_back() {
        let ax = Figure::new().subplot();
        ax.set_patch_gid(Some("backdrop".into()));
        let patch = unit_square();
        pull_background(Owner::Axes(ax.clone()), any(&patch)).unwrap();
        assert_eq!(patch.style(), ax.patch_style());
        assert_eq!(patch.gid().as_deref(), Some("backdrop"));
        assert_eq!(patch.zorder(), 1.0);
        patch.update_style(|style| style.hatch = Some("//".into()));
        patch
            .set_alpha(Some(0.5))
            .set_visible(false)
            .set_zorder(0.9);
        push_background(Owner::Axes(ax.clone()), any(&patch)).unwrap();
        assert_eq!(ax.patch_style().hatch.as_deref(), Some("//"));
        assert_eq!(ax.patch_alpha(), Some(0.5));
        assert!(!ax.patch_visible());
        assert_eq!(ax.patch_zorder(), 0.9);
    }

    #[test]
    fn a_figure_background_follows_its_patch() {
        let fig = Figure::new();
        let patch = unit_square();
        pull_background(Owner::Figure(fig.clone()), any(&patch)).unwrap();
        assert!(patch.visible() && patch.gid().is_none());
        patch.update_style(|style| style.linewidth = Some(3.0));
        patch.set_visible(false);
        push_background(Owner::Figure(fig.clone()), any(&patch)).unwrap();
        assert_eq!(fig.linewidth(), 3.0);
        assert!(!fig.frameon());
    }
}
