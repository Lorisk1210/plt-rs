//! What a 3-D axis is as an artist: its visibility and the gids of the
//! groups it draws in SVG output (matplotlib's `get_visible` and `get_gid`
//! of an `axis3d.Axis`, of its texts and of its ticks).

use super::axis_handle::Axis3D;
use crate::axes::AxisGroup;
use crate::colors::Color;

impl Axis3D {
    /// Counts the axis in tight boxes, or leaves it out (matplotlib's
    /// `Axis.set_visible`: a 3-D axis is drawn either way).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// ax.zaxis().set_visible(false);
    /// assert!(!ax.zaxis().visible() && ax.xaxis().visible());
    /// assert_eq!(ax.zaxis().tight_bbox(false)?, None);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_visible(&self, visible: bool) -> &Self {
        let dir = self.dir;
        self.axes
            .with_mut(|axes| axes.axis_mut(dir).visible = visible);
        self
    }

    /// Whether tight boxes count the axis.
    pub fn visible(&self) -> bool {
        self.axes.with(|axes| axes.axis(self.dir).visible)
    }

    /// The gid of `group` in SVG output: the axis (whose pane, gridlines
    /// and axis groups are named after it), its label, its offset text,
    /// one of its ticks or a part of one.
    ///
    /// ```
    /// use pltrs::axes::AxisGroup;
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let ax = fig.subplot_3d();
    /// ax.yaxis().set_gid(AxisGroup::Axis, Some("depth".into()));
    /// assert_eq!(ax.yaxis().gid(AxisGroup::Axis).as_deref(), Some("depth"));
    /// assert!(fig.to_svg()?.contains("id=\"depth\""));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn gid(&self, group: AxisGroup) -> Option<String> {
        self.axes
            .with(|axes| axes.axis(self.dir).gids.get(group).map(str::to_owned))
    }

    /// Gives `group` the gid `gid` (see [`Axis3D::gid`]), or takes its
    /// gid away (`None`).
    pub fn set_gid(&self, group: AxisGroup, gid: Option<String>) -> &Self {
        let dir = self.dir;
        self.axes
            .with_mut(|axes| axes.axis_mut(dir).gids.set(group, gid));
        self
    }

    /// The width in points and the colour the axis line is drawn with
    /// (matplotlib's `axis.line.get_linewidth` and `get_color`).
    pub fn line_style(&self) -> (f64, Color) {
        self.axes.with(|axes| {
            let style = &axes.axis(self.dir).style;
            (style.line_width, style.line_color.clone())
        })
    }

    /// Draws the axis line `width` points wide in `color` (matplotlib's
    /// `axis.line.set_linewidth` and `set_color`).
    ///
    /// ```
    /// use pltrs::colors::Color;
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// ax.zaxis().set_line_style(2.0, Color::from("red"));
    /// assert_eq!(ax.zaxis().line_style(), (2.0, Color::from("red")));
    /// ```
    pub fn set_line_style(&self, width: f64, color: Color) -> &Self {
        let dir = self.dir;
        self.axes.with_mut(|axes| {
            let style = &mut axes.axis_mut(dir).style;
            style.line_width = width;
            style.line_color = color;
        });
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::axes::{AxisGroup, TickPart};
    use crate::figure::Figure;

    #[test]
    fn panes_gridlines_and_axes_draw_before_the_axes_group() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        ax.plot([0.0, 1.0], [0.0, 1.0], [0.0, 1.0]).unwrap();
        ax.xaxis().set_gid(AxisGroup::Axis, Some("x".into()));
        let mark = AxisGroup::TickPart {
            minor: false,
            index: 1,
            part: TickPart::Tick1Line,
        };
        ax.zaxis().set_gid(mark, Some("z-mark".into()));
        let svg = fig.to_svg().unwrap();
        let at = |id: &str| svg.find(&format!("<g id=\"{id}\"")).expect(id);
        // The background, then the x pane named after the axis.
        assert!(at("patch_2") < at("x") && at("x") < at("pane3d_1"));
        assert!(at("pane3d_2") < at("Line3DCollection_1"));
        assert!(at("Line3DCollection_1") < at("grid3d_1") && at("grid3d_2") < at("axis3d_1"));
        assert!(at("axis3d_2") < at("z-mark") && at("z-mark") < at("axes_1"));
        assert_eq!(
            svg.matches("<g id=\"x\"").count(),
            3,
            "pane, gridlines and axis"
        );
    }
}
