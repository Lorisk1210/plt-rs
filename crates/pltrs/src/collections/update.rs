//! Setting many properties of an existing collection at once
//! (matplotlib's `Collection.set` and `Collection.update`).

use super::{Collection, CollectionKind, CollectionProps};
use crate::artist::ArtistHandle;

impl Collection {
    /// Sets every property `props` sets, as the matching setters would
    /// one after the other (matplotlib's `Collection.set(**kwargs)`); unset
    /// ones stay as they are. `color` sets the faces and the edges, only
    /// the edges of line collections (`LineCollection.set_color`), and an
    /// `alpha` resets the colours as
    /// [`set_uniform_alpha`](Self::set_uniform_alpha) does.
    ///
    /// ```
    /// use pltrs::collections::CollectionProps;
    /// use pltrs::colors::Color;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let dots = ax.scatter([0.0, 1.0], [0.0, 1.0])?;
    /// dots.update_props(CollectionProps::default().color("r").linewidth(2.0).alpha(0.5).zorder(3.0));
    /// assert_eq!(dots.style().facecolors, Some(vec![Color::from("r")]));
    /// assert_eq!(dots.style().linewidths, Some(vec![2.0]));
    /// assert_eq!((dots.alpha(), dots.zorder()), (Some(0.5), 3.0));
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn update_props(&self, props: CollectionProps) -> &Self {
        let CollectionProps {
            mut base,
            color,
            facecolors,
            edgecolors,
            linewidths,
            linestyles,
            hatch,
            hatchcolors,
            antialiased,
            alphas,
            capstyle,
            joinstyle,
        } = props;
        if let Some(alpha) = base.alpha.take() {
            self.set_uniform_alpha(Some(alpha));
        }
        self.update_base_props(base);
        if let Some(color) = color {
            if self.kind() != CollectionKind::Line {
                self.set_facecolor(color.clone());
            }
            self.set_edgecolor(color);
        }
        if let Some(colors) = facecolors {
            self.set_facecolors(colors);
        }
        if let Some(edges) = edgecolors {
            self.set_edgecolors(edges);
        }
        if let Some(widths) = linewidths {
            self.set_linewidths(widths);
        }
        if let Some(styles) = linestyles {
            self.set_linestyles(styles);
        }
        if let Some(pattern) = hatch {
            self.set_hatch(pattern);
        }
        if let Some(colors) = hatchcolors {
            self.set_hatchcolors(colors);
        }
        if let Some(antialiased) = antialiased {
            self.set_antialiased(antialiased);
        }
        if let Some(alphas) = alphas {
            self.set_alphas(alphas);
        }
        if let Some(capstyle) = capstyle {
            self.set_capstyle(capstyle);
        }
        if let Some(joinstyle) = joinstyle {
            self.set_joinstyle(joinstyle);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::Point;
    use pltrs_render::{CapStyle, JoinStyle};

    use super::*;
    use crate::collections::{CollectionSpec, EdgeColors, HatchColors};
    use crate::colors::Color;
    use crate::figure::Figure;
    use crate::lines::LineStyle;

    #[test]
    fn every_property_set_reaches_the_collection() {
        let ax = Figure::new().subplot();
        let dots = ax.scatter([0.0, 1.0], [0.0, 1.0]).expect("a scatter");
        let props = CollectionProps {
            facecolors: Some(vec![Color::from("g"), Color::from("b")]),
            edgecolors: Some(EdgeColors::Colors(vec![Color::from("k")])),
            linestyles: Some(vec![LineStyle::Dashed]),
            hatchcolors: Some(HatchColors::Colors(vec![Color::from("m")])),
            antialiased: Some(vec![false]),
            alphas: Some(vec![0.2, 0.8]),
            ..CollectionProps::default()
        }
        .linewidth(3.0)
        .hatch("//")
        .capstyle(CapStyle::Round)
        .joinstyle(JoinStyle::Bevel)
        .label("dots")
        .visible(false);
        dots.update_props(props);
        let style = dots.style();
        assert_eq!(
            style.facecolors,
            Some(vec![Color::from("g"), Color::from("b")])
        );
        assert_eq!(style.edgecolors, EdgeColors::Colors(vec![Color::from("k")]));
        assert_eq!(style.linewidths, Some(vec![3.0]));
        assert_eq!(style.linestyles, [LineStyle::Dashed]);
        assert_eq!(style.hatch.as_deref(), Some("//"));
        assert_eq!(
            style.hatchcolors,
            HatchColors::Colors(vec![Color::from("m")])
        );
        assert_eq!(style.antialiased, Some(vec![false]));
        assert_eq!(style.alphas, Some(vec![0.2, 0.8]));
        assert_eq!(
            (style.capstyle, style.joinstyle),
            (Some(CapStyle::Round), Some(JoinStyle::Bevel))
        );
        assert_eq!(dots.label(), "dots");
        assert!(!dots.visible());
    }

    #[test]
    fn an_empty_set_changes_nothing_and_lines_keep_their_faces() {
        let ax = Figure::new().subplot();
        let dots = ax.scatter([0.0], [0.0]).expect("a scatter");
        let before = dots.style();
        dots.update_props(CollectionProps::default());
        assert_eq!(dots.style(), before);
        assert_eq!(dots.alpha(), None);

        let lines = ax.add_collection(CollectionSpec::lines(vec![vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
        ]]));
        let faces = lines.style().facecolors;
        lines.update_props(CollectionProps::default().color("r"));
        assert_eq!(lines.style().facecolors, faces, "lines are not filled");
        assert_eq!(
            lines.style().edgecolors,
            EdgeColors::Colors(vec![Color::from("r")])
        );
    }
}
