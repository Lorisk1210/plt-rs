//! Drawing box-and-whisker plots from precomputed statistics (matplotlib's
//! `bxp`).

use std::any::Any;

use plt_core::{Path, Point};

use super::children::DataUpdate;
use super::{Axes, AxesState, PlotError};
use crate::artist::{Artist, ArtistHandle};
use crate::colors::Color;
use crate::lines::{Line2D, LineProps, LineStyle};
use crate::markers::Marker;
use crate::orientation::Orientation;
use crate::patches::{Patch, PatchArtist, PatchProps, PathShape};
use crate::rc::{ColorOrAuto, RcParams};
use crate::stats::BoxStats;
use crate::ticker::{AxisName, FixedFormatter, FixedLocator, pyfmt};
use plt_render::CapStyle;

/// How boxes are drawn (matplotlib's `patch_artist` with `boxprops`).
#[derive(Clone, Debug)]
#[allow(
    clippy::large_enum_variant,
    reason = "built once per box plot: boxing the props would only burden the callers"
)]
pub enum BoxStyle {
    /// Box outlines drawn as lines.
    Line(LineProps),
    /// Filled boxes drawn as patches.
    Patch(PatchProps),
}

/// A drawn box.
#[derive(Clone, Debug, PartialEq)]
pub enum BoxArtist {
    /// An outline ([`BoxStyle::Line`]).
    Line(Line2D),
    /// A filled patch ([`BoxStyle::Patch`]).
    Patch(Patch<PathShape>),
}

/// The artists of a box plot (matplotlib's `boxplot` result dictionary).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoxplotArtists {
    /// Two whiskers per box, lower first.
    pub whiskers: Vec<Line2D>,
    /// Two caps per box, lower first.
    pub caps: Vec<Line2D>,
    /// The boxes.
    pub boxes: Vec<BoxArtist>,
    /// The median lines.
    pub medians: Vec<Line2D>,
    /// One line of outlier markers per box.
    pub fliers: Vec<Line2D>,
    /// The means (markers or lines).
    pub means: Vec<Line2D>,
}

/// Options of [`Axes::bxp`]. Unset switches take `bxp`'s defaults
/// (notches, means and mean lines off; caps, boxes and fliers on);
/// [`Axes::boxplot`] resolves them from the `boxplot.*` rc parameters.
#[derive(Clone, Debug)]
pub struct BxpOptions {
    /// Box positions (default `1, 2, ..., n`).
    pub positions: Option<Vec<f64>>,
    /// Box widths: one for all or one per box (default from the spread
    /// of the positions).
    pub widths: Vec<f64>,
    /// Vertical boxes stand on the x axis; horizontal ones on the y axis.
    pub orientation: Orientation,
    /// How boxes are drawn (default: outlines).
    pub boxes: Option<BoxStyle>,
    /// Draw notches at the median's confidence interval.
    pub shownotches: Option<bool>,
    /// Draw the means.
    pub showmeans: Option<bool>,
    /// Draw the whisker caps.
    pub showcaps: Option<bool>,
    /// Draw the boxes.
    pub showbox: Option<bool>,
    /// Draw the outliers.
    pub showfliers: Option<bool>,
    /// Draw means as lines across the box instead of markers.
    pub meanline: Option<bool>,
    /// Style overrides of the whiskers.
    pub whiskerprops: LineProps,
    /// Style overrides of the outliers.
    pub flierprops: LineProps,
    /// Style overrides of the medians.
    pub medianprops: LineProps,
    /// Style overrides of the caps.
    pub capprops: LineProps,
    /// Style overrides of the means.
    pub meanprops: LineProps,
    /// Put ticks labelled with the box labels at the positions.
    pub manage_ticks: bool,
    /// z-order of the boxes (medians and means are drawn 0.1 above).
    pub zorder: Option<f64>,
    /// Cap widths: one for all or one per box (default half the widths).
    pub capwidths: Vec<f64>,
}

impl Default for BxpOptions {
    fn default() -> Self {
        Self {
            positions: None,
            widths: Vec::new(),
            orientation: Orientation::Vertical,
            boxes: None,
            shownotches: None,
            showmeans: None,
            showcaps: None,
            showbox: None,
            showfliers: None,
            meanline: None,
            whiskerprops: LineProps::default(),
            flierprops: LineProps::default(),
            medianprops: LineProps::default(),
            capprops: LineProps::default(),
            meanprops: LineProps::default(),
            manage_ticks: true,
            zorder: None,
            capwidths: Vec::new(),
        }
    }
}

/// matplotlib's `Line2D.zorder`, the default box z-order.
const LINE_ZORDER: f64 = 2.0;

/// The line styles of every part of a box, rc defaults overlaid with the
/// caller's overrides (matplotlib's `merge_kw_rc`).
struct PartStyles {
    whisker: LineProps,
    cap: LineProps,
    flier: LineProps,
    median: LineProps,
    mean: LineProps,
    boxes: BoxStyle,
}

impl PartStyles {
    fn new(options: &BxpOptions, rc: &RcParams) -> Self {
        let zorder = options.zorder.unwrap_or(LINE_ZORDER);
        let line = |color: &Color, width: f64, style: &LineStyle| {
            LineProps::default()
                .color(color.clone())
                .linewidth(width)
                .linestyle(style.clone())
                .zorder(zorder)
                .marker(Marker::default())
        };
        let butt = |props: LineProps| {
            props
                .solid_capstyle(CapStyle::Butt)
                .dash_capstyle(CapStyle::Butt)
        };
        let whisker = line(
            &rc.boxplot_whiskerprops_color,
            rc.boxplot_whiskerprops_linewidth,
            &rc.boxplot_whiskerprops_linestyle,
        );
        let cap = line(
            &rc.boxplot_capprops_color,
            rc.boxplot_capprops_linewidth,
            &rc.boxplot_capprops_linestyle,
        );
        let median = butt(line(
            &rc.boxplot_medianprops_color,
            rc.boxplot_medianprops_linewidth,
            &rc.boxplot_medianprops_linestyle,
        ))
        .zorder(zorder + 0.1);
        let flier = LineProps::default()
            .color(rc.boxplot_flierprops_color.clone())
            .marker(rc.boxplot_flierprops_marker.clone())
            .markerfacecolor(rc.boxplot_flierprops_markerfacecolor.clone())
            .markeredgecolor(ColorOrAuto::Color(
                rc.boxplot_flierprops_markeredgecolor.clone(),
            ))
            .markeredgewidth(rc.boxplot_flierprops_markeredgewidth)
            .markersize(rc.boxplot_flierprops_markersize)
            .linestyle(rc.boxplot_flierprops_linestyle.clone())
            .linewidth(rc.boxplot_flierprops_linewidth)
            .zorder(zorder);
        let mut mean = butt(
            LineProps::default()
                .color(rc.boxplot_meanprops_color.clone())
                .marker(rc.boxplot_meanprops_marker.clone())
                .markerfacecolor(ColorOrAuto::Color(
                    rc.boxplot_meanprops_markerfacecolor.clone(),
                ))
                .markeredgecolor(ColorOrAuto::Color(
                    rc.boxplot_meanprops_markeredgecolor.clone(),
                ))
                .markersize(rc.boxplot_meanprops_markersize)
                .linestyle(rc.boxplot_meanprops_linestyle.clone())
                .linewidth(rc.boxplot_meanprops_linewidth)
                .zorder(zorder + 0.1),
        );
        // A mean line hides the marker, a mean marker hides the line,
        // unless the caller styles that part explicitly.
        if options.meanline.unwrap_or(false) {
            if !options.meanprops.is_set("marker") {
                mean = mean.marker(Marker::default());
            }
        } else if !options.meanprops.is_set("linestyle") {
            mean = mean.linestyle(LineStyle::None);
        }
        let boxes = match &options.boxes {
            Some(BoxStyle::Patch(props)) => BoxStyle::Patch(
                PatchProps::default()
                    .linestyle(rc.boxplot_boxprops_linestyle.clone())
                    .linewidth(rc.boxplot_boxprops_linewidth)
                    .edgecolor(rc.boxplot_boxprops_color.clone())
                    .facecolor(rc.patch_facecolor.clone())
                    .zorder(zorder)
                    .overlay(props.clone()),
            ),
            Some(BoxStyle::Line(props)) => BoxStyle::Line(Self::box_line(rc, zorder, props)),
            None => BoxStyle::Line(Self::box_line(rc, zorder, &LineProps::default())),
        };
        Self {
            whisker: whisker.overlay(options.whiskerprops.clone()),
            cap: cap.overlay(options.capprops.clone()),
            flier: flier.overlay(options.flierprops.clone()),
            median: median.overlay(options.medianprops.clone()),
            mean: mean.overlay(options.meanprops.clone()),
            boxes,
        }
    }

    fn box_line(rc: &RcParams, zorder: f64, props: &LineProps) -> LineProps {
        LineProps::default()
            .color(rc.boxplot_boxprops_color.clone())
            .linewidth(rc.boxplot_boxprops_linewidth)
            .linestyle(rc.boxplot_boxprops_linestyle.clone())
            .zorder(zorder)
            .marker(Marker::default())
            .overlay(props.clone())
    }
}

/// One value for all boxes or one per box.
fn per_box(argument: &'static str, values: &[f64], n: usize) -> Result<Vec<f64>, PlotError> {
    match values {
        [value] => Ok(vec![*value; n]),
        _ if values.len() == n => Ok(values.to_vec()),
        _ => Err(PlotError::DatasetCount {
            argument,
            len: values.len(),
            datasets: n,
        }),
    }
}

impl Axes {
    /// Draws a box plot from precomputed statistics (matplotlib's `bxp`).
    ///
    /// # Errors
    ///
    /// When `positions`, `widths` or `capwidths` hold neither one value
    /// (widths) nor one per box.
    pub fn bxp(
        &self,
        stats: &[BoxStats],
        options: &BxpOptions,
    ) -> Result<BoxplotArtists, PlotError> {
        let rc = crate::rc::params();
        // `boxplot.vertical = False` lays every box plot out horizontally,
        // as matplotlib's (deprecated) `vert` default does.
        let horizontal;
        let options = if rc.boxplot_vertical {
            options
        } else {
            horizontal = BxpOptions {
                orientation: Orientation::Horizontal,
                ..options.clone()
            };
            &horizontal
        };
        let styles = PartStyles::new(options, &rc);
        let n = stats.len();
        let default_positions = options.positions.is_none();
        let positions = match &options.positions {
            Some(p) if p.len() != n => {
                return Err(PlotError::DatasetCount {
                    argument: "positions",
                    len: p.len(),
                    datasets: n,
                });
            }
            Some(p) => p.clone(),
            None => (1..=n).map(|i| i as f64).collect(),
        };
        let widths = if options.widths.is_empty() {
            let low = positions.iter().copied().fold(f64::INFINITY, f64::min);
            let high = positions.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            vec![(0.15 * (high - low)).clamp(0.15, 0.5); n]
        } else {
            per_box("widths", &options.widths, n)?
        };
        let capwidths = if options.capwidths.is_empty() {
            widths.iter().map(|w| 0.5 * w).collect()
        } else {
            per_box("capwidths", &options.capwidths, n)?
        };
        let mut artists = BoxplotArtists::default();
        for (i, box_stats) in stats.iter().enumerate() {
            self.draw_box(
                box_stats,
                &BoxGeometry {
                    position: positions[i],
                    width: widths[i],
                    capwidth: capwidths[i],
                },
                options,
                &styles,
                &mut artists,
            )?;
        }
        if options.manage_ticks {
            let labels = stats
                .iter()
                .zip(&positions)
                .map(|(s, &p)| match &s.label {
                    Some(label) => label.clone(),
                    None if default_positions => format!("{p:.0}"),
                    None => pyfmt::repr(p),
                })
                .collect();
            self.manage_box_ticks(&positions, labels, options.orientation, &artists.medians);
        }
        Ok(artists)
    }

    /// Draws the parts of one box.
    fn draw_box(
        &self,
        stats: &BoxStats,
        geometry: &BoxGeometry,
        options: &BxpOptions,
        styles: &PartStyles,
        artists: &mut BoxplotArtists,
    ) -> Result<(), PlotError> {
        let orientation = options.orientation;
        let plot = |along: &[f64], values: &[f64], props: &LineProps| {
            let (xs, ys) = match orientation {
                Orientation::Vertical => (along, values),
                Orientation::Horizontal => (values, along),
            };
            self.plot_with(xs, ys, props.clone())
        };
        let pos = geometry.position;
        let (box_left, box_right) = (pos - geometry.width * 0.5, pos + geometry.width * 0.5);
        let (cap_left, cap_right) = (pos - geometry.capwidth * 0.5, pos + geometry.capwidth * 0.5);
        let (box_x, box_y, med_x) = if options.shownotches.unwrap_or(false) {
            let (notch_left, notch_right) =
                (pos - geometry.width * 0.25, pos + geometry.width * 0.25);
            (
                vec![
                    box_left,
                    box_right,
                    box_right,
                    notch_right,
                    box_right,
                    box_right,
                    box_left,
                    box_left,
                    notch_left,
                    box_left,
                    box_left,
                ],
                vec![
                    stats.q1, stats.q1, stats.cilo, stats.med, stats.cihi, stats.q3, stats.q3,
                    stats.cihi, stats.med, stats.cilo, stats.q1,
                ],
                [notch_left, notch_right],
            )
        } else {
            (
                vec![box_left, box_right, box_right, box_left, box_left],
                vec![stats.q1, stats.q1, stats.q3, stats.q3, stats.q1],
                [box_left, box_right],
            )
        };
        if options.showbox.unwrap_or(true) {
            let drawn = match &styles.boxes {
                BoxStyle::Line(props) => BoxArtist::Line(plot(&box_x, &box_y, props)?),
                BoxStyle::Patch(props) => {
                    BoxArtist::Patch(self.box_patch(&box_x, &box_y, orientation, props))
                }
            };
            artists.boxes.push(drawn);
        }
        for (quartile, end) in [(stats.q1, stats.whislo), (stats.q3, stats.whishi)] {
            artists
                .whiskers
                .push(plot(&[pos, pos], &[quartile, end], &styles.whisker)?);
        }
        if options.showcaps.unwrap_or(true) {
            for end in [stats.whislo, stats.whishi] {
                artists
                    .caps
                    .push(plot(&[cap_left, cap_right], &[end, end], &styles.cap)?);
            }
        }
        artists
            .medians
            .push(plot(&med_x, &[stats.med, stats.med], &styles.median)?);
        if options.showmeans.unwrap_or(false) {
            let mean = if options.meanline.unwrap_or(false) {
                plot(
                    &[box_left, box_right],
                    &[stats.mean, stats.mean],
                    &styles.mean,
                )?
            } else {
                plot(&[pos], &[stats.mean], &styles.mean)?
            };
            artists.means.push(mean);
        }
        if options.showfliers.unwrap_or(true) {
            let along = vec![pos; stats.fliers.len()];
            artists
                .fliers
                .push(plot(&along, &stats.fliers, &styles.flier)?);
        }
        Ok(())
    }

    /// A filled box, added without touching the data limits (matplotlib's
    /// `add_artist`).
    fn box_patch(
        &self,
        along: &[f64],
        values: &[f64],
        orientation: Orientation,
        props: &PatchProps,
    ) -> Patch<PathShape> {
        let corners = along.iter().zip(values).map(|(&a, &v)| {
            let (x, y) = orientation.place(a, v);
            Point::new(x, y)
        });
        let mut patch = PatchArtist::new(PathShape(Path::polygon(corners)), crate::rc::params());
        props.clone().apply(&mut patch);
        patch
            .base_mut()
            .transform
            .get_or_insert_with(|| self.trans_data());
        Patch::from_raw(self.adopt(patch, &DataUpdate::default()))
    }

    /// matplotlib's `manage_ticks`: the data limits reach half a unit
    /// past the outer boxes, medians stick there, and fixed ticks with the
    /// box labels are appended at the positions.
    fn manage_box_ticks(
        &self,
        positions: &[f64],
        labels: Vec<String>,
        orientation: Orientation,
        medians: &[Line2D],
    ) {
        let name = match orientation {
            Orientation::Vertical => AxisName::X,
            Orientation::Horizontal => AxisName::Y,
        };
        let low = positions.iter().copied().fold(f64::INFINITY, f64::min) - 0.5;
        let high = positions.iter().copied().fold(f64::NEG_INFINITY, f64::max) + 0.5;
        for (median, &p) in medians.iter().zip(positions) {
            let (mut x, mut y) = median.sticky_edges();
            match name {
                AxisName::X => x.extend([p - 0.5, p + 0.5]),
                AxisName::Y => y.extend([p - 0.5, p + 0.5]),
            }
            median.set_sticky_edges(x, y);
        }
        let (locs, seq) = self.raw().with_mut::<AxesState, _>(|axes| {
            axes.extend_data_interval(name, low, high);
            let axis = axes.axis(name);
            let locator: &dyn Any = axis.major_locator.as_ref();
            let formatter: &dyn Any = axis.major_formatter.as_ref();
            let mut locs = locator
                .downcast_ref::<FixedLocator>()
                .map(|fixed| fixed.locs().to_vec())
                .unwrap_or_default();
            let mut seq = formatter
                .downcast_ref::<FixedFormatter>()
                .map(|fixed| fixed.seq().to_vec())
                .unwrap_or_default();
            locs.extend_from_slice(positions);
            seq.extend(labels);
            (locs, seq)
        });
        let axis = self.axis_of(name);
        axis.set_major_locator(FixedLocator::new(locs));
        axis.set_major_formatter(FixedFormatter::new(seq));
        self.update_data(&DataUpdate {
            autoscale: (true, true),
            ..DataUpdate::default()
        });
    }
}

/// Where one box sits.
struct BoxGeometry {
    position: f64,
    width: f64,
    capwidth: f64,
}
