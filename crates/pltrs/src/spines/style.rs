//! A spine's paint, as a patch has it: matplotlib's `Spine` is a `Patch`,
//! whose getters and setters work on it.

use super::{Spine, SpineArtist};
use crate::artist::BaseProps;
use crate::colors::ColorError;
use crate::patches::{PatchColors, PatchStyle};

impl Spine {
    /// The edge, face and hatch properties of the spine (matplotlib's
    /// `Patch` getters on a spine).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// assert!(ax.spines().left.style().fill);
    /// ```
    pub fn style(&self) -> PatchStyle {
        self.raw.with::<SpineArtist, _>(|s| s.style.clone())
    }

    /// Changes the edge, face and hatch properties with `f`.
    pub fn update_style(&self, f: impl FnOnce(&mut PatchStyle)) -> &Self {
        self.raw.with_mut::<SpineArtist, _>(|s| f(&mut s.style));
        self
    }

    /// The spine width in points: the one set, else `axes.linewidth`
    /// (matplotlib's `get_linewidth`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// assert_eq!(ax.spines().left.set_linewidth(3.0).linewidth(), 3.0);
    /// ```
    pub fn linewidth(&self) -> f64 {
        self.raw
            .with::<SpineArtist, _>(|s| s.style.linewidth.unwrap_or(s.rc.axes_linewidth))
    }

    /// The colours the spine is painted with, defaults and alpha applied
    /// (matplotlib's `get_facecolor`, `get_edgecolor` and
    /// `get_hatchcolor` of a spine, which draws an edge by default).
    ///
    /// # Errors
    ///
    /// When a colour cannot be resolved.
    pub fn colors(&self) -> Result<PatchColors, ColorError> {
        self.raw.with::<SpineArtist, _>(|s| {
            let resolved = s.style.resolve(&s.rc, s.base.alpha, true)?;
            Ok(PatchColors {
                face: resolved.face,
                edge: resolved.edge,
                hatch: resolved.hatch_color,
            })
        })
    }

    /// Sets the properties every artist has (label, z-order, visibility,
    /// clipping) that `props` sets.
    pub fn update_base(&self, props: BaseProps) -> &Self {
        self.raw
            .with_mut::<SpineArtist, _>(|s| props.apply(&mut s.base));
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::colors::Rgba;
    use crate::prelude::*;

    #[test]
    fn a_spine_is_painted_as_a_patch() {
        let ax = Figure::new().subplot();
        let spine = &ax.spines().left;
        assert_eq!(spine.linewidth(), 0.8, "axes.linewidth");
        spine.set_color("red");
        let colors = spine.colors().unwrap();
        assert_eq!(colors.edge, Rgba::new(1.0, 0.0, 0.0, 1.0));
        assert_eq!(colors.face, Rgba::new(0.0, 0.0, 0.0, 0.0), "no face");
        spine.update_style(|style| style.hatch = Some("/".into()));
        assert_eq!(spine.style().hatch.as_deref(), Some("/"));
    }
}
