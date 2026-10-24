//! The artists a legend draws for its handles (matplotlib's
//! `legend_handles`, `get_lines` and `get_patches`): proxies its handlers
//! made in the look of the artists the entries stand for.

use super::Legend;
use crate::artist::RawHandle;
use crate::lines::Line2D;

impl Legend {
    /// The artist drawn for every listed handle — the first one when its
    /// handler draws several — or `None` for a handle no handler draws
    /// (matplotlib's `legend_handles`). The built-in handlers draw a
    /// [`Line2D`] for lines and line collections, a `Patch<Rectangle>` for
    /// patches, bars and polygon or patch collections, and a
    /// [`Collection`](crate::collections::Collection) for scatters:
    /// [`RawHandle::downcast`] gives typed access.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::artist::RawHandle;
    /// use plt::legend::LegendOptions;
    /// use plt::lines::Line2D;
    /// use plt::markers::Marker;
    /// let ax = Figure::new().subplot();
    /// ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::fmt("v")?.label("down"))?;
    /// let legend = ax.legend(LegendOptions::default())?;
    /// let proxy: Line2D = legend.handles()[0].as_ref().and_then(RawHandle::downcast).unwrap();
    /// assert_eq!(proxy.marker(), "v".parse::<Marker>()?);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn handles(&self) -> Vec<Option<RawHandle>> {
        let ids = self.read(|legend| legend.handles.clone());
        ids.into_iter()
            .map(|id| id.map(|id| self.handle(id)))
            .collect()
    }

    /// The handle artists that are lines (matplotlib's `get_lines`).
    pub fn lines(&self) -> Vec<Line2D> {
        self.handles()
            .iter()
            .flatten()
            .filter_map(RawHandle::downcast)
            .collect()
    }

    /// The handle artists that are patches, whatever their shape
    /// (matplotlib's `get_patches`); the built-in handlers draw
    /// rectangles, which [`RawHandle::downcast`] turns into
    /// `Patch<Rectangle>`.
    pub fn patches(&self) -> Vec<RawHandle> {
        self.handles()
            .into_iter()
            .flatten()
            .filter(RawHandle::is_patch)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use plt_core::{Point, Rgba};

    use super::*;
    use crate::artist::ArtistHandle;
    use crate::axes::FillBetweenOptions;
    use crate::collections::{Collection, CollectionPatch, CollectionProps, CollectionSpec};
    use crate::figure::Figure;
    use crate::legend::{LegendHandle, LegendLabelColor, LegendOptions};
    use crate::lines::LineProps;
    use crate::patches::{Circle, Patch, PatchProps, Rectangle};

    #[test]
    fn proxies_follow_the_entries_in_order() {
        let ax = Figure::new().subplot();
        let props = LineProps::fmt("o-").unwrap().label("line");
        ax.plot_with([0.0, 1.0], [0.0, 1.0], props).unwrap();
        let mut fill = FillBetweenOptions::default();
        fill.props.base.label = Some("fill".to_owned());
        ax.fill_between([0.0, 1.0], [1.0, 2.0], [0.0, 0.0], fill)
            .unwrap();
        ax.scatter(vec![0.5], vec![0.5]).unwrap().set_label("dots");
        let legend = ax.legend(LegendOptions::default()).unwrap();
        let handles = legend.handles();
        let line: Line2D = handles[0].as_ref().and_then(RawHandle::downcast).unwrap();
        assert_eq!(line.marker(), "o".parse().unwrap());
        let patch = handles[1].clone().unwrap();
        assert!(patch.downcast::<Patch<Rectangle>>().is_some());
        let dots = handles[2]
            .as_ref()
            .and_then(RawHandle::downcast::<Collection>);
        assert!(dots.is_some());
        assert_eq!(legend.lines(), [line]);
        assert_eq!(legend.patches(), [patch]);
    }

    #[test]
    fn handles_without_a_handler_have_no_proxy() {
        let ax = Figure::new().subplot();
        let text = ax.text(0.0, 0.0, "text");
        let line = ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        let options = LegendOptions::default()
            .handles([LegendHandle::from(&text), LegendHandle::from(&line)])
            .labels(["text", "line"]);
        let legend = ax.legend(options).unwrap();
        let handles = legend.handles();
        assert!(handles[0].is_none());
        assert!(
            handles[1]
                .as_ref()
                .and_then(RawHandle::downcast::<Line2D>)
                .is_some()
        );
    }

    #[test]
    fn handles_drawing_no_artist_have_no_proxy() {
        // A line collection without edge colours draws nothing (matplotlib
        // fails on it), so it makes no entry.
        let ax = Figure::new().subplot();
        let segments = vec![vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)]];
        let invisible =
            CollectionSpec::lines(segments).with_props(CollectionProps::default().color("none"));
        ax.add_collection(invisible).set_label("invisible");
        let options = LegendOptions::default().labelcolor(LegendLabelColor::from("linecolor"));
        let legend = ax.legend(options).unwrap();
        assert_eq!(legend.labels(), Vec::<String>::new());
        assert_eq!(legend.handles(), [None]);
    }

    #[test]
    fn patch_collections_are_drawn_as_rectangles() {
        let ax = Figure::new().subplot();
        let circle = CollectionPatch::new(&Circle::new((0.0, 0.0), 1.0), PatchProps::default());
        let spec = CollectionSpec::patches(&[circle], false).unwrap();
        let patches =
            ax.add_collection(spec.with_props(CollectionProps::default().facecolor("red")));
        patches.set_label("patches");
        let legend = ax.legend(LegendOptions::default()).unwrap();
        assert_eq!(legend.labels(), ["patches"]);
        let proxy = legend.patches()[0].downcast::<Patch<Rectangle>>().unwrap();
        assert_eq!(proxy.colors().unwrap().face, Rgba::new(1.0, 0.0, 0.0, 1.0));
    }
}
