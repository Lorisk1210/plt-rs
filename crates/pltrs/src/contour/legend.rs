//! Legend entries describing a contour set (matplotlib's
//! `ContourSet.legend_elements`).

use pltrs_core::Rgba;
use pltrs_render::Dashes;

use super::ContourSet;
use crate::artist::ArtistHandle;
use crate::collections::CollectionArtist;
use crate::colors::{Color, Extend};
use crate::legend::LegendHandle;
use crate::lines::{LineProps, LineStyle};
use crate::patches::PatchProps;

/// How a legend entry of a contour set is drawn
/// ([`ContourSet::legend_styles`]).
#[derive(Clone, Debug, PartialEq)]
pub enum ContourLegendStyle {
    /// A level of line contours: a line of its colour, width and dashes.
    Line {
        /// The colour of the level.
        color: Rgba,
        /// Its line width in points.
        linewidth: f64,
        /// Its dashes, as the collection draws them.
        dashes: Dashes,
    },
    /// A band of filled contours: a rectangle of its colour and hatch.
    Band {
        /// The colour of the band.
        color: Rgba,
        /// Its hatch, if it is hatched.
        hatch: Option<String>,
    },
}

impl ContourSet {
    /// Proxy artists and labels for a legend describing the contour set
    /// (matplotlib's `legend_elements`), drawn as
    /// [`legend_styles`](Self::legend_styles) describes them: for filled
    /// contours a rectangle per band, for line contours a line per level.
    /// As in matplotlib, the lines get the collection's dash tuples, so
    /// they are drawn as dashed lines (butt caps, dashes scaled once more
    /// by the line width) even when solid.
    pub fn legend_elements(
        &self,
        variable_name: &str,
        str_format: impl Fn(f64) -> String,
    ) -> (Vec<LegendHandle>, Vec<String>) {
        let (styles, labels) = self.legend_styles(variable_name, str_format);
        let handles = styles
            .into_iter()
            .map(|style| match style {
                ContourLegendStyle::Line {
                    color,
                    linewidth,
                    dashes,
                } => LegendHandle::line(
                    LineProps::default()
                        .color(Color::from(color))
                        .linewidth(linewidth)
                        .linestyle(LineStyle::Dashes {
                            offset: dashes.offset,
                            pattern: dashes.pattern,
                        }),
                ),
                ContourLegendStyle::Band { color, hatch } => LegendHandle::patch(PatchProps {
                    facecolor: Some(Color::from(color)),
                    hatch,
                    ..PatchProps::default()
                }),
            })
            .collect();
        (handles, labels)
    }

    /// How each entry of a legend describing the contour set is drawn,
    /// and its label (what matplotlib's `legend_elements` makes its
    /// artists of): for filled contours each band's colour and hatch,
    /// labelled with the band's data range such as `$0.5 < x \leq 1.0$`;
    /// for line contours each level's colour, width and dashes, labelled
    /// `$x = 0.5$`. `str_format` writes the numbers (matplotlib's default
    /// is `str`, [`pyfmt::repr`](crate::ticker::pyfmt::repr)).
    ///
    /// The labels of extended bands keep matplotlib's stray `s`
    /// (`$x \leq -1e+250s$`) and the extension sentinels as their bounds.
    pub fn legend_styles(
        &self,
        variable_name: &str,
        str_format: impl Fn(f64) -> String,
    ) -> (Vec<ContourLegendStyle>, Vec<String>) {
        let colors = self.colors();
        if !self.filled() {
            let (widths, dashes) = (self.linewidths(), self.dash_patterns());
            return self
                .levels()
                .into_iter()
                .enumerate()
                .map(|(i, level)| {
                    let style = ContourLegendStyle::Line {
                        color: colors[i % colors.len()],
                        linewidth: widths[i % widths.len()],
                        dashes: dashes[i % dashes.len()].clone(),
                    };
                    let label = format!("${variable_name} = {}$", str_format(level));
                    (style, label)
                })
                .unzip();
        }
        let bands = self.bands();
        let last = bands.len().saturating_sub(1);
        let extend = self.extend();
        let hatches = self.band_hatches();
        bands
            .into_iter()
            .enumerate()
            .map(|(i, (lower, upper))| {
                let (lower, upper) = (str_format(lower), str_format(upper));
                let label = if i == 0 && matches!(extend, Extend::Min | Extend::Both) {
                    format!(r"${variable_name} \leq {lower}s$")
                } else if i == last && matches!(extend, Extend::Max | Extend::Both) {
                    format!("${variable_name} > {upper}s$")
                } else {
                    format!(r"${lower} < {variable_name} \leq {upper}$")
                };
                let style = ContourLegendStyle::Band {
                    color: colors[i % colors.len()],
                    hatch: hatches.get(i).cloned().flatten(),
                };
                (style, label)
            })
            .unzip()
    }

    /// The hatch of every band of a filled contour set (empty when none
    /// is hatched, as matplotlib then draws the bands in one collection).
    fn band_hatches(&self) -> Vec<Option<String>> {
        let collections = self.collections();
        if collections.len() < 2 {
            return Vec::new();
        }
        collections
            .iter()
            .map(|c| c.raw().with::<CollectionArtist, _>(|c| c.hatch.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::colors::Extend;
    use crate::contour::{ContourOptions, Levels, QuadField};
    use crate::figure::Figure;
    use crate::numeric::Grid;
    use crate::ticker::pyfmt;

    fn field() -> QuadField {
        QuadField::new(Grid::from_fn(4, 5, |i, j| (i + j) as f64))
    }

    #[test]
    fn line_contours_label_their_levels() {
        let options = ContourOptions {
            levels: Levels::Values(vec![1.0, 2.5]),
            ..ContourOptions::default()
        };
        let cs = Figure::new().subplot().contour(&field(), options).unwrap();
        let (handles, labels) = cs.legend_elements("z", pyfmt::repr);
        assert_eq!(handles.len(), 2);
        assert_eq!(labels, ["$z = 1.0$", "$z = 2.5$"]);
    }

    #[test]
    fn filled_contours_label_their_bands_with_matplotlibs_stray_s() {
        let options = ContourOptions {
            levels: Levels::Values(vec![2.0, 4.0, 6.0]),
            extend: Extend::Max,
            hatches: vec![Some("/".into()), None],
            ..ContourOptions::default()
        };
        let cs = Figure::new().subplot().contourf(&field(), options).unwrap();
        let (handles, labels) = cs.legend_elements("x", pyfmt::repr);
        assert_eq!(handles.len(), 3);
        assert_eq!(
            labels,
            [
                r"$2.0 < x \leq 4.0$",
                r"$4.0 < x \leq 6.0$",
                "$x > 1e+250s$"
            ]
        );
        assert_eq!(
            cs.band_hatches(),
            [Some("/".into()), None, Some("/".into())]
        );
    }
}
