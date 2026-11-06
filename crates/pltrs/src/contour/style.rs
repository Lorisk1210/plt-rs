//! Colours, line widths and line styles of the levels (matplotlib's
//! `ContourSet.__init__` colour handling, `_process_colors`,
//! `_process_linewidths` and `_process_linestyles`).

use pltrs_core::Rgba;

use super::{ContourError, ContourOptions};
use crate::colors::{Color, Colormap, Extend, LogNorm, NoNorm, Norm, Normalize, ScalarMappable};
use crate::image::ScalarKind;
use crate::lines::LineStyle;
use crate::rc::RcParams;

/// The colour of every level (lines) or band (fills), before alpha.
#[derive(Clone, Debug)]
pub(crate) struct ColorPlan {
    /// One colour per level or band.
    pub(crate) colors: Vec<Rgba>,
    /// The colormap and norm the colours come from (`None` when no colour
    /// is mapped).
    pub(crate) mapping: Option<ScalarMappable>,
    /// The value mapped to each colour (matplotlib's `cvalues`): the
    /// layers, or colour indices for explicit colours.
    pub(crate) cvalues: Vec<f64>,
    /// Whether the `cvalues` are layers or colour indices.
    pub(crate) cvalue_kind: ScalarKind,
    /// The norm limits after autoscaling to the levels.
    pub(crate) norm_limits: (Option<f64>, Option<f64>),
    /// Whether the colormap has a single colour (negative levels of line
    /// contours are then drawn in `negative_linestyles`).
    pub(crate) monochrome: bool,
}

/// What [`map_colors`] needs to know about the levels.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LevelColoring<'a> {
    pub(crate) levels: &'a [f64],
    pub(crate) layers: &'a [f64],
    pub(crate) filled: bool,
    pub(crate) logscale: bool,
    pub(crate) extend: Extend,
}

/// Maps the layers through the colormap and norm, or indexes the explicit
/// `colors` (with optional under/over colours for extended ends).
pub(crate) fn map_colors(
    options: &mut ContourOptions,
    coloring: &LevelColoring<'_>,
    rc: &RcParams,
) -> Result<ColorPlan, ContourError> {
    let colorizer = std::mem::take(&mut options.colorizer);
    if options.colors.is_some() && colorizer.cmap.is_some() {
        return Err(ContourError::ColorsAndCmap);
    }
    if let Some(colors) = &options.colors {
        return indexed_colors(colors, coloring);
    }
    let cmap = colorizer.colormap(rc)?;
    let mut norm: Box<dyn Norm> = match colorizer.norm {
        Some(norm) => norm.resolve(),
        None if coloring.logscale => Box::new(LogNorm::default()),
        None => Box::new(Normalize::default()),
    };
    if let Some(vmin) = colorizer.vmin {
        norm.set_vmin(Some(vmin));
    }
    if let Some(vmax) = colorizer.vmax {
        norm.set_vmax(Some(vmax));
    }
    let levels: Vec<Option<f64>> = coloring.levels.iter().copied().map(Some).collect();
    norm.autoscale_none(&levels);
    let layers = super::set::mapped_values(coloring.layers);
    let colors = norm
        .normalize_masked(&layers, None)?
        .into_iter()
        .map(|value| cmap.lookup(value, None))
        .collect();
    Ok(ColorPlan {
        colors,
        norm_limits: (norm.vmin(), norm.vmax()),
        monochrome: cmap.is_monochrome(),
        cvalues: coloring.layers.to_vec(),
        cvalue_kind: ScalarKind::Float,
        mapping: Some(ScalarMappable::with_norm(cmap, norm)),
    })
}

/// Explicit colours: a listed colormap indexed by level (band) number,
/// where extended bands index past the ends (matplotlib's `NoNorm` path).
fn indexed_colors(
    sequence: &[Color],
    coloring: &LevelColoring<'_>,
) -> Result<ColorPlan, ContourError> {
    let (extend_min, extend_max) = (coloring.extend.extends_min(), coloring.extend.extends_max());
    let ncolors = coloring.levels.len() - usize::from(coloring.filled);
    let total = ncolors + usize::from(extend_min) + usize::from(extend_max);
    // Enough colours for the extended ends: the first and last give the
    // under and over colours.
    let under_over = sequence.len() == total && (extend_min || extend_max);
    let skip = usize::from(under_over && extend_min);
    let mut norm = NoNorm::default();
    let levels: Vec<Option<f64>> = coloring.levels.iter().copied().map(Some).collect();
    norm.autoscale_none(&levels);
    let norm_limits = (norm.vmin(), norm.vmax());
    let available = &sequence[skip.min(sequence.len())..];
    if ncolors == 0 || available.is_empty() {
        return Ok(ColorPlan {
            colors: Vec::new(),
            mapping: None,
            cvalues: Vec::new(),
            cvalue_kind: ScalarKind::Integer,
            norm_limits,
            monochrome: true,
        });
    }
    let listed: Vec<Color> = available.iter().cycle().take(ncolors).cloned().collect();
    let mut cmap = Colormap::listed("from_list", listed)?;
    if under_over && extend_min {
        cmap = cmap.with_under(sequence[0].clone())?;
    }
    if under_over && extend_max {
        cmap = cmap.with_over(sequence[sequence.len() - 1].clone())?;
    }
    let first = if coloring.filled && extend_min { -1 } else { 0 };
    let count = coloring.levels.len() - usize::from(coloring.filled)
        + usize::from(coloring.filled && extend_min)
        + usize::from(coloring.filled && extend_max);
    let indices: Vec<i64> = (first..).take(count).collect();
    Ok(ColorPlan {
        colors: indices.iter().map(|&i| cmap.sample_index(i)).collect(),
        cvalues: indices.iter().map(|&i| i as f64).collect(),
        cvalue_kind: ScalarKind::Integer,
        norm_limits,
        monochrome: cmap.is_monochrome(),
        mapping: Some(ScalarMappable::with_norm(cmap, Box::new(norm))),
    })
}

/// The line width of every level: `linewidths` cycled over the levels, or
/// `contour.linewidth` (falling back to `lines.linewidth`).
pub(crate) fn linewidths(linewidths: Option<&[f64]>, nlevels: usize, rc: &RcParams) -> Vec<f64> {
    match linewidths {
        Some(widths) if !widths.is_empty() => {
            widths.iter().copied().cycle().take(nlevels).collect()
        }
        _ => vec![rc.contour_linewidth.unwrap_or(rc.lines_linewidth); nlevels],
    }
}

/// The inputs of [`linestyles`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct LineStyling<'a> {
    pub(crate) levels: &'a [f64],
    pub(crate) zrange: Option<(f64, f64)>,
    pub(crate) monochrome: bool,
    pub(crate) negative: &'a LineStyle,
}

/// The line style of every level: `linestyles` cycled over the levels, or
/// solid with negative levels in the negative style when monochrome.
pub(crate) fn linestyles(
    linestyles: Option<&[LineStyle]>,
    styling: &LineStyling<'_>,
) -> Vec<LineStyle> {
    let nlevels = styling.levels.len();
    if let Some(styles) = linestyles.filter(|s| !s.is_empty()) {
        return styles.iter().cycle().take(nlevels).cloned().collect();
    }
    // "Negative" allows for rounding just below zero.
    let eps = styling
        .zrange
        .filter(|_| styling.monochrome)
        .map(|(zmin, zmax)| -(zmax - zmin) * 1e-15);
    styling
        .levels
        .iter()
        .map(|&level| match eps {
            Some(eps) if level < eps => styling.negative.clone(),
            _ => LineStyle::Solid,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::{ColorizerOptions, colormaps};

    fn coloring<'a>(
        levels: &'a [f64],
        layers: &'a [f64],
        filled: bool,
        extend: Extend,
    ) -> LevelColoring<'a> {
        LevelColoring {
            levels,
            layers,
            filled,
            logscale: false,
            extend,
        }
    }

    #[test]
    fn colormap_colors_follow_the_layers() {
        let mut options = ContourOptions::default();
        let rc = RcParams::default();
        let plan = map_colors(
            &mut options,
            &coloring(&[0.0, 1.0], &[0.0, 1.0], false, Extend::Neither),
            &rc,
        )
        .unwrap();
        let viridis = colormaps::get("viridis").unwrap();
        assert_eq!(plan.colors, [viridis.sample(0.0), viridis.sample(1.0)]);
        assert_eq!(plan.norm_limits, (Some(0.0), Some(1.0)));
        assert!(!plan.monochrome);
    }

    #[test]
    fn bands_up_to_an_infinite_level_take_the_bad_colour() {
        // nilearn's filled contours above a single level: matplotlib's
        // `set_array` masks the infinite layer.
        let mut options = ContourOptions::default();
        options.colorizer.vmin = Some(0.0);
        options.colorizer.vmax = Some(1.0);
        let rc = RcParams::default();
        let plan = map_colors(
            &mut options,
            &coloring(
                &[0.4, f64::INFINITY],
                &[f64::INFINITY],
                true,
                Extend::Neither,
            ),
            &rc,
        )
        .unwrap();
        assert_eq!(plan.colors, [Rgba::TRANSPARENT]);
    }

    #[test]
    fn explicit_colors_index_bands_and_extremes() {
        let mut options = ContourOptions {
            colors: Some(["b", "c", "y", "m"].map(Color::from).to_vec()),
            ..ContourOptions::default()
        };
        let rc = RcParams::default();
        let levels = [0.0, 0.5, 1.0];
        let plan = map_colors(
            &mut options,
            &coloring(&levels, &[0.0; 4], true, Extend::Both),
            &rc,
        )
        .unwrap();
        let resolved: Vec<Rgba> = ["b", "c", "y", "m"]
            .iter()
            .map(|c| Color::from(*c).resolve().unwrap())
            .collect();
        assert_eq!(plan.colors, resolved);
        assert_eq!(plan.cvalues, [-1.0, 0.0, 1.0, 2.0], "band indices");
        let mut single = ContourOptions {
            colors: Some(vec![Color::from("k")]),
            ..ContourOptions::default()
        };
        let plan = map_colors(
            &mut single,
            &coloring(&levels, &levels, false, Extend::Neither),
            &rc,
        )
        .unwrap();
        assert!(plan.monochrome && plan.colors.len() == 3);
        let mut both = ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("gray").unwrap()),
                ..ColorizerOptions::default()
            },
            colors: Some(vec![Color::from("k")]),
            ..ContourOptions::default()
        };
        assert!(matches!(
            map_colors(
                &mut both,
                &coloring(&levels, &levels, false, Extend::Neither),
                &rc
            ),
            Err(ContourError::ColorsAndCmap)
        ));
    }

    #[test]
    fn widths_and_styles() {
        let rc = RcParams::default();
        assert_eq!(linewidths(None, 2, &rc), [1.5, 1.5]);
        assert_eq!(linewidths(Some(&[1.0, 2.0]), 3, &rc), [1.0, 2.0, 1.0]);
        let styling = LineStyling {
            levels: &[-1.0, 0.0, 1.0],
            zrange: Some((-2.0, 2.0)),
            monochrome: true,
            negative: &LineStyle::Dashed,
        };
        assert_eq!(
            linestyles(None, &styling),
            [LineStyle::Dashed, LineStyle::Solid, LineStyle::Solid]
        );
        let colored = LineStyling {
            monochrome: false,
            ..styling
        };
        assert_eq!(linestyles(None, &colored), [const { LineStyle::Solid }; 3]);
        assert_eq!(
            linestyles(Some(&[LineStyle::Dotted]), &colored),
            [const { LineStyle::Dotted }; 3]
        );
    }
}
