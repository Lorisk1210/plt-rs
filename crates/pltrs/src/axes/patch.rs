//! The background patch of an axes (matplotlib's `Axes.patch`): its face,
//! its edge (none by default) and its z-order.

use super::Axes;
use super::state::AxesState;
use crate::colors::Color;

impl Axes {
    /// Sets the background colour (matplotlib's `set_facecolor`).
    pub fn set_facecolor(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.raw
            .with_mut::<AxesState, _>(|a| a.patch.facecolor = color);
        self
    }

    /// The background colour.
    pub fn facecolor(&self) -> Color {
        self.raw.with::<AxesState, _>(|a| a.patch.facecolor.clone())
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
            .with_mut::<AxesState, _>(|a| a.patch.edgecolor = color);
        self
    }

    /// The edge colour of the background patch.
    pub fn patch_edgecolor(&self) -> Color {
        self.raw.with::<AxesState, _>(|a| a.patch.edgecolor.clone())
    }

    /// Sets the line width of the background patch's edge in points
    /// (matplotlib's `ax.patch.set_linewidth`; 0 by default).
    pub fn set_patch_linewidth(&self, width: f64) -> &Self {
        self.raw
            .with_mut::<AxesState, _>(|a| a.patch.linewidth = width);
        self
    }

    /// The line width of the background patch's edge in points.
    pub fn patch_linewidth(&self) -> f64 {
        self.raw.with::<AxesState, _>(|a| a.patch.linewidth)
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
