//! The background patch of an axes (matplotlib's `Axes.patch`): its face,
//! its edge (none by default), the rest of its patch style and its
//! z-order.

use pltrs_core::Path;

use super::Axes;
use super::state::{AxesState, PatchOutline};
use crate::colors::Color;
use crate::patches::PatchStyle;
use crate::transforms::Transform;

impl Axes {
    /// Sets the background colour (matplotlib's `set_facecolor`).
    pub fn set_facecolor(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.raw
            .with_mut::<AxesState, _>(|a| a.patch.style.facecolor = Some(color));
        self
    }

    /// The background colour (`patch.facecolor` when the patch's was
    /// reset).
    pub fn facecolor(&self) -> Color {
        self.raw.with::<AxesState, _>(|a| {
            (a.patch.style.facecolor.clone()).unwrap_or_else(|| a.rc.patch_facecolor.clone())
        })
    }

    /// The style of the background patch: its colours, edge width, line
    /// style, hatch, caps, joins and anti-aliasing, as a patch's (see
    /// [`Patch::style`](crate::patches::Patch::style)).
    pub fn patch_style(&self) -> PatchStyle {
        self.raw.with::<AxesState, _>(|a| a.patch.style.clone())
    }

    /// Gives the background patch the style of a patch (matplotlib's
    /// `ax.patch` setters); the background stays where the axes puts it.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::patches::PatchStyle;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// ax.set_patch_style(PatchStyle {
    ///     hatch: Some("//".into()),
    ///     ..ax.patch_style()
    /// });
    /// assert_eq!(ax.patch_style().hatch.as_deref(), Some("//"));
    /// assert!(fig.to_svg()?.contains("<pattern"), "the hatch is drawn");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_patch_style(&self, style: PatchStyle) -> &Self {
        self.raw.with_mut::<AxesState, _>(|a| a.patch.style = style);
        self
    }

    /// Makes the closed path `outline`, in axes fractions, the shape of
    /// the background and what the artists the axes clips are clipped to,
    /// in place of the axes box (matplotlib's `_gen_axes_patch` of an
    /// `Axes` subclass making a circle, say); with `as_box` they are
    /// clipped to its bounding box instead, as matplotlib clips to a
    /// `Rectangle` patch. `None` gives the axes the projection's
    /// background again.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::path::Path;
    /// use pltrs::transforms::Point;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// ax.set_patch_outline(Some(Path::circle(Point::new(0.5, 0.5), 0.5)), false);
    /// assert!(ax.patch_outline().is_some_and(|outline| outline.has_curves()));
    /// fig.to_png()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_patch_outline(&self, outline: Option<Path>, as_box: bool) -> &Self {
        let transform = Transform::Axes(self.raw.id());
        self.raw.with_mut::<AxesState, _>(|a| {
            a.patch_outline = outline.map(|path| PatchOutline {
                path,
                transform,
                clipped_to_box: false,
                clips_as_box: as_box,
            });
        });
        self
    }

    /// The outline replacing the projection's background (see
    /// [`set_patch_outline`](Self::set_patch_outline)), in the coordinates
    /// it was given in.
    pub fn patch_outline(&self) -> Option<Path> {
        self.raw
            .with::<AxesState, _>(|a| a.patch_outline.as_ref().map(|o| o.path.clone()))
    }

    /// The opacity of the background patch replacing that of its colours
    /// (matplotlib's `ax.patch.get_alpha`), `None` by default.
    pub fn patch_alpha(&self) -> Option<f64> {
        self.raw.with::<AxesState, _>(|a| a.patch.alpha)
    }

    /// Sets the opacity of the background patch (matplotlib's
    /// `ax.patch.set_alpha`), replacing that of its colours.
    pub fn set_patch_alpha(&self, alpha: Option<f64>) -> &Self {
        self.raw.with_mut::<AxesState, _>(|a| a.patch.alpha = alpha);
        self
    }

    /// Sets the z-order of the background patch (matplotlib's
    /// `ax.patch.set_zorder`). It matters only where a parasite axes
    /// shares the patch and draws it among its children; as the
    /// background the patch is always drawn first.
    pub fn set_patch_zorder(&self, zorder: f64) -> &Self {
        self.raw
            .with_mut::<AxesState, _>(|a| a.patch.zorder = zorder);
        self
    }

    /// The z-order of the background patch (1 by default).
    pub fn patch_zorder(&self) -> f64 {
        self.raw.with::<AxesState, _>(|a| a.patch.zorder)
    }

    /// Sets the edge colour of the background patch (matplotlib's
    /// `ax.patch.set_edgecolor`; `"none"` by default). Only drawn with a
    /// line width (see [`Axes::set_patch_linewidth`]).
    pub fn set_patch_edgecolor(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.raw
            .with_mut::<AxesState, _>(|a| a.patch.style.edgecolor = Some(color));
        self
    }

    /// The edge colour of the background patch (a patch's default edge
    /// when it was reset: `patch.edgecolor` when edges are forced, else
    /// none).
    pub fn patch_edgecolor(&self) -> Color {
        self.raw
            .with::<AxesState, _>(|a| a.patch.style.edge_spec(&a.rc, false))
    }

    /// Sets the line width of the background patch's edge in points
    /// (matplotlib's `ax.patch.set_linewidth`; 0 by default).
    pub fn set_patch_linewidth(&self, width: f64) -> &Self {
        self.raw
            .with_mut::<AxesState, _>(|a| a.patch.style.linewidth = Some(width));
        self
    }

    /// The line width of the background patch's edge in points
    /// (`patch.linewidth` when it was reset).
    pub fn patch_linewidth(&self) -> f64 {
        self.raw
            .with::<AxesState, _>(|a| a.patch.style.linewidth.unwrap_or(a.rc.patch_linewidth))
    }

    /// Names the SVG group of the background patch `gid` (matplotlib's
    /// `ax.patch.set_gid`), or numbers it as a `patch` (`None`); the
    /// background of a 3-D axes is that of its projection plane.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// fig.subplot().set_patch_gid(Some("backdrop".into()));
    /// assert!(fig.to_svg()?.contains("<g id=\"backdrop\">"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_patch_gid(&self, gid: Option<String>) -> &Self {
        self.raw.with_mut::<AxesState, _>(|a| a.patch.gid = gid);
        self
    }

    /// The id of the background patch's SVG group, if it was given one.
    pub fn patch_gid(&self) -> Option<String> {
        self.raw.with::<AxesState, _>(|a| a.patch.gid.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;

    #[test]
    fn the_background_has_no_edge_unless_given_one() {
        let ax = Figure::new().subplot();
        assert_eq!(
            (ax.patch_edgecolor().to_string(), ax.patch_linewidth()),
            ("none".to_owned(), 0.0)
        );
        ax.set_patch_edgecolor("red").set_patch_linewidth(2.0);
        assert_eq!(ax.patch_linewidth(), 2.0);
        let svg = ax.figure().to_svg().unwrap();
        assert!(svg.contains("stroke: #ff0000"), "the edge is drawn");
    }
}
