//! Parasite axes drawn by axis artists (the parasites of matplotlib's
//! `axisartist.HostAxes`, and of `host_subplot(..., axes_class=
//! axisartist.Axes)`): twins and auxiliary axes whose axis lines are axis
//! artists of their own, which the host draws among its artists.

use super::axes::Axes;
use super::handles::AxisArtist;
use crate::artist::ArtistHandle;
use crate::axes_grid1::{HostAxes, ParasiteError, ParasiteLink};
use crate::spines::SpineSide;
use crate::ticker::AxisName;

impl Axes {
    /// Adds a parasite whose coordinates relate to this axes' as `link`
    /// says and whose axis lines are axis artists, all hidden until shown
    /// one by one, like its gridlines (matplotlib's
    /// `get_aux_axes(viewlim_mode=...)` on an axisartist host; share an
    /// axis with [`crate::axes::Axes::share_x`] for its `sharex=host`).
    ///
    /// ```
    /// use plt::axes_grid1::ParasiteLink;
    /// use plt::axisartist;
    /// use plt::prelude::*;
    /// let fig = Figure::new();
    /// let host = axisartist::Axes::new(fig.subplot());
    /// let par = host.add_parasite_axes(ParasiteLink::Independent)?;
    /// let right = par.axis("right").expect("an edge");
    /// assert!(!right.visible());
    /// right.set_visible(true);
    /// assert_eq!(fig.axes().len(), 1, "parasites are drawn by their host");
    /// # Ok::<(), plt::axes_grid1::ParasiteError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the transform of a [`ParasiteLink::Transform`] rotates, shears
    /// or cannot be inverted.
    pub fn add_parasite_axes(&self, link: ParasiteLink) -> Result<Self, ParasiteError> {
        let parasite = HostAxes::new(self.ax().clone()).aux_axes(link)?;
        let axes = Self::new(parasite.axes().clone());
        // matplotlib's `ParasiteAxesBase.clear` hides every child.
        for artist in axes.axis_artists() {
            artist.set_visible(false);
        }
        if let Some(gridlines) = axes.gridlines() {
            gridlines.set_visible(false);
        }
        Ok(axes)
    }

    /// A parasite sharing the x axis whose `right` axis artist replaces
    /// this axes' (matplotlib's `HostAxes.twinx` on an axisartist host);
    /// like any axis artist on the right, it starts without tick labels
    /// and label.
    #[must_use]
    pub fn twinx(&self) -> Self {
        self.twin_sharing(AxisName::X)
    }

    /// A parasite sharing the y axis whose `top` axis artist replaces this
    /// axes' (matplotlib's `HostAxes.twiny` on an axisartist host); like
    /// any axis artist on top, it starts without tick labels and label.
    #[must_use]
    pub fn twiny(&self) -> Self {
        self.twin_sharing(AxisName::Y)
    }

    /// `twinx` (sharing x) or `twiny` (sharing y).
    fn twin_sharing(&self, shared: AxisName) -> Self {
        let twin = self
            .add_parasite_axes(ParasiteLink::Independent)
            .expect("independent parasites need no transform");
        twin.ax()
            .share(shared, self.ax())
            .expect("a new axes of this figure shares nothing yet");
        let side = match shared {
            AxisName::X => SpineSide::Right,
            AxisName::Y => SpineSide::Top,
        };
        let edge = |axes: &Self| -> AxisArtist {
            axes.axis(side.name())
                .expect("axisartist axes have an artist on each edge")
        };
        edge(self).set_visible(false);
        edge(&twin).set_visible(true);
        twin
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::legend::LegendOptions;

    #[test]
    fn twins_take_over_one_edge_of_the_host() {
        let fig = Figure::new();
        let host = Axes::new(fig.subplot());
        let twin = host.twinx();
        let visible = |axes: &Axes, name: &str| axes.axis(name).expect("an edge").visible();
        assert!(!visible(&host, "right") && visible(&host, "left"));
        assert!(visible(&twin, "right"));
        assert!(!visible(&twin, "left") && !visible(&twin, "top") && !visible(&twin, "bottom"));
        let right = twin.axis("right").expect("an edge");
        assert!(!right.major_ticklabels().visible() && !right.label().visible());
        host.ax().set_xlim(0.0, 2.0).expect("finite limits");
        assert_eq!(twin.ax().xlim(), (0.0, 2.0), "the x axis is shared");
        let over = host.twiny();
        assert!(!visible(&host, "top") && visible(&over, "top"));
    }

    #[test]
    fn parasites_hide_their_artists_and_join_the_host_legend() {
        let fig = Figure::new();
        let host = Axes::new(fig.subplot());
        let par = host
            .add_parasite_axes(ParasiteLink::Independent)
            .expect("a parasite");
        assert!(par.axis_artists().iter().all(|artist| !artist.visible()));
        assert!(
            par.gridlines()
                .is_some_and(|gridlines| !gridlines.visible())
        );
        let offset = par.new_fixed_axis("right2", SpineSide::Right, (60.0, 0.0));
        assert!(offset.visible() && offset.label().visible());
        host.ax()
            .plot([0.0, 1.0], [0.0, 1.0])
            .expect("a line")
            .set_label("host");
        par.ax()
            .plot([0.0, 1.0], [5.0, 3.0])
            .expect("a line")
            .set_label("parasite");
        let legend = host
            .ax()
            .legend(LegendOptions::default())
            .expect("a legend");
        assert_eq!(legend.labels(), ["host", "parasite"]);
        assert_eq!(fig.axes().len(), 1);
    }
}
