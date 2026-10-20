//! Polar and geographic axes: the example's Hammer projection, the
//! matplotlib logo, the left ventricle bullseye and radar charts
//! (`custom_projection`, `logos2`, `leftventricle_bullseye` and
//! `radar_chart`).

use std::f64::consts::PI;

use plt::axes::{BarAlign, TickParams};
use plt::colorbar::{ColorbarAxes, ColorbarOptions, ColorbarTicks, Spacing};
use plt::colors::{BoundaryNorm, Colormap, Extend, Normalize, ScalarMappable, colormaps};
use plt::fonts::{FontProperties, FontSize, FontWeight, RelativeSize};
use plt::geo::GeoProjection;
use plt::gridspec::SubplotOverrides;
use plt::layout::{ConstrainedLayout, LayoutEngine};
use plt::legend::{LegendHandle, LegendLoc, LegendOptions};
use plt::numeric::linspace_with;
use plt::orientation::Orientation;
use plt::patches::{PathShape, Rectangle};
use plt::polar::{Compass, PolarAxes, PolarFrame};
use plt::prelude::*;
use plt::rc::{Aspect, AxisBelow};
use plt::text::{HorizontalAlignment as HAlign, VerticalAlignment as VAlign, text_path};
use plt::transforms::{Point, Transform};
use plt_core::{Affine2D, Rgba};

use crate::figures::{CaseResult, FigureCase};

fn custom_projection(fig: &Figure) -> CaseResult {
    let ax = fig.geo_subplot(GeoProjection::Hammer);
    ax.plot_fmt([-1.0, 1.0, 1.0], [-1.0, -1.0, 1.0], "o-")?;
    ax.grid(true);
    Ok(())
}

/// The blue of the matplotlib logo.
const MPL_BLUE: &str = "#11557c";

/// The sizes of one logo: its height in pixels and the line widths of
/// the bars, the grid and the border.
struct LogoSizes {
    height_px: f64,
    lw_bars: f64,
    lw_grid: f64,
    lw_border: f64,
    rgrid: &'static [f64],
}

/// `create_icon_axes`: the polar bar chart of the logo at `position`.
fn icon_axes(fig: &Figure, position: [f64; 4], sizes: &LogoSizes) -> CaseResult {
    let border = sizes.lw_border.to_string();
    let ax: PolarAxes = plt::pyplot::rc_context(
        &[("axes.edgecolor", MPL_BLUE), ("axes.linewidth", &border)],
        || fig.add_polar_axes(position),
    )?;
    ax.set_axisbelow(AxisBelow::Below);
    let n = 7;
    let arc = 2.0 * PI;
    let theta: Vec<f64> = (0..n)
        .map(|i| f64::from(i) * (arc / f64::from(n)))
        .collect();
    let radii = [2.0, 6.0, 8.0, 7.0, 4.0, 5.0, 8.0];
    let width = [0.4, 0.4, 0.6, 0.8, 0.2, 0.5, 0.3].map(|w| PI / 4.0 * w);
    let bars = ax.bar(
        theta,
        radii,
        BarOptions {
            thickness: width.to_vec(),
            base: vec![0.0],
            align: BarAlign::Edge,
            edgecolor: vec![Color::from("0.3")],
            linewidth: vec![sizes.lw_bars],
            ..BarOptions::default()
        },
    )?;
    let jet = colormaps::get("jet")?;
    for (bar, r) in bars.patches.iter().zip(radii) {
        let Rgba { r, g, b, .. } = jet.sample(r / 10.0);
        bar.set_facecolor(Rgba::new(r, g, b, 0.6));
    }
    ax.tick_params(&TickParams {
        labelbottom: Some(false),
        labeltop: Some(false),
        labelleft: Some(false),
        labelright: Some(false),
        ..TickParams::default()
    });
    ax.grid_with(
        true,
        &plt::axes::GridOptions {
            linewidth: Some(sizes.lw_grid),
            color: Some("0.9".into()),
            ..plt::axes::GridOptions::default()
        },
    );
    ax.set_rmax(9.0);
    ax.set_yticks(sizes.rgrid.to_vec());
    let background = ax.add_patch_with(
        Rectangle::new((0.0, 0.0), arc, 9.58),
        PatchProps::default()
            .facecolor("white")
            .zorder(0.0)
            .clip_on(false),
    );
    // The background follows the circle like the bars: matplotlib's
    // rectangles share the unit path the bars gave 100 steps.
    background.set_interpolation_steps(100).set_in_layout(false);
    Ok(())
}

/// `make_logo`: the icon, with the word "matplotlib" beside it when
/// `with_text`. Neither Calibri nor Carlito is available, so the text is
/// in the default font, as in the example without them.
fn logo(fig: &Figure, sizes: &LogoSizes, with_text: bool) -> CaseResult {
    fig.set_alpha(Some(0.0));
    if with_text {
        let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]);
        ax.set_aspect(Aspect::Equal)?;
        ax.set_axis_off();
        let font = FontProperties::from_rc(&plt::rc::params());
        let path = text_path(
            Point::new(0.0, 0.0),
            "matplotlib",
            Some(sizes.height_px * 0.8),
            &font,
        )?;
        let skew = Transform::Affine(Affine2D::identity().skew_deg(4.25, 0.0));
        ax.add_patch_with(
            PathShape(path),
            PatchProps::default()
                .color(MPL_BLUE)
                .linewidth(0.0)
                .transform(skew.then(ax.trans_data())),
        );
        ax.autoscale(None, None);
    }
    let position = if with_text {
        [0.535, 0.12, 0.17, 0.75]
    } else {
        [0.03, 0.03, 0.94, 0.94]
    };
    icon_axes(fig, position, sizes)
}

/// The sizes of the large logos.
const LARGE: LogoSizes = LogoSizes {
    height_px: 110.0,
    lw_bars: 0.7,
    lw_grid: 0.5,
    lw_border: 1.0,
    rgrid: &[1.0, 3.0, 5.0, 7.0],
};

fn logos2_1(fig: &Figure) -> CaseResult {
    logo(fig, &LARGE, false)
}

fn logos2_2(fig: &Figure) -> CaseResult {
    let small = LogoSizes {
        height_px: 32.0,
        lw_bars: 0.3,
        lw_grid: 0.3,
        lw_border: 0.3,
        rgrid: &[5.0],
    };
    logo(fig, &small, false)
}

fn logos2_3(fig: &Figure) -> CaseResult {
    logo(fig, &LARGE, true)
}

/// `bullseye_plot`: the 17 segments of the left ventricle coloured by
/// `data` through the colormap and norm of `colors`, the segments of
/// `seg_bold` outlined in bold.
fn bullseye_plot(
    ax: &PolarAxes,
    data: &[f64],
    seg_bold: &[usize],
    colors: &ScalarMappable,
) -> CaseResult {
    let r = linspace(0.2, 1.0, 4);
    ax.set_ylim(0.0, 1.0);
    ax.set_xticklabels(Vec::<String>::new())?;
    ax.set_yticklabels(Vec::<String>::new())?;
    ax.grid(false);
    let rings = [
        (0, 6, r[2], r[3]),
        (6, 12, r[1], r[2]),
        (12, 16, r[0], r[1]),
        (16, 17, 0.0, r[0]),
    ];
    let ring_bars = |start: usize, stop: usize, r_in: f64, r_out: f64, options: BarOptions| {
        let n = stop - start;
        let dtheta = 2.0 * PI / n as f64;
        let theta: Vec<f64> = (0..n).map(|i| i as f64 * dtheta + PI / 2.0).collect();
        ax.bar(
            theta,
            vec![r_out - r_in; n],
            BarOptions {
                thickness: vec![dtheta],
                base: vec![r_in],
                ..options
            },
        )
    };
    for &(start, stop, r_in, r_out) in &rings {
        let values: Vec<Option<f64>> = data[start..stop].iter().copied().map(Some).collect();
        let colors = colors
            .clone()
            .to_rgba(&values, None)?
            .into_iter()
            .map(Color::from)
            .collect();
        ring_bars(
            start,
            stop,
            r_in,
            r_out,
            BarOptions {
                color: colors,
                ..BarOptions::default()
            },
        )?;
    }
    for &(start, stop, r_in, r_out) in &rings[..3] {
        let widths = (start..stop)
            .map(|i| {
                if seg_bold.contains(&(i + 1)) {
                    4.0
                } else {
                    2.0
                }
            })
            .collect();
        ring_bars(
            start,
            stop,
            r_in,
            r_out,
            BarOptions {
                color: vec![Color::from("none")],
                edgecolor: vec![Color::from("k")],
                linewidth: widths,
                props: PatchProps::default().clip_on(false),
                ..BarOptions::default()
            },
        )?;
    }
    let bold = if seg_bold.contains(&17) { 4.0 } else { 2.0 };
    ax.plot_with(
        linspace(0.0, 2.0 * PI, 50),
        vec![r[0]; 50],
        LineProps::fmt("k")?.linewidth(bold),
    )?;
    Ok(())
}

/// A horizontal colorbar of `mappable` in an inset below `ax`.
fn colorbar_below(
    fig: &Figure,
    ax: &PolarAxes,
    mappable: &ScalarMappable,
    options: ColorbarOptions,
) -> CaseResult {
    fig.colorbar(
        mappable,
        ColorbarOptions {
            axes: ColorbarAxes::Cax(ax.inset_axes([0.0, -0.15, 1.0, 0.1])),
            orientation: Some(Orientation::Horizontal),
            ..options
        },
    )?;
    Ok(())
}

fn leftventricle_bullseye(fig: &Figure) -> CaseResult {
    let data: Vec<f64> = (1..=17).map(f64::from).collect();
    fig.set_layout_engine(LayoutEngine::Constrained(ConstrainedLayout {
        wspace: 0.1,
        w_pad: 0.2,
        ..ConstrainedLayout::from_rc(&plt::rc::params())
    }));
    let axs: Vec<PolarAxes> = (1..=3)
        .map(|i| fig.add_polar_subplot(1, 3, i))
        .collect::<Result<_, _>>()?;
    let labelled = |label: &str| ColorbarOptions {
        label: label.into(),
        ..ColorbarOptions::default()
    };
    let viridis = colormaps::get("viridis")?;
    let norm = Normalize::new(1.0, 17.0);
    let first = ScalarMappable::with_norm(viridis, Box::new(norm));
    colorbar_below(fig, &axs[0], &first, labelled("Some units"))?;
    let cool = colormaps::get("cool")?;
    let second = ScalarMappable::with_norm(cool, Box::new(norm));
    colorbar_below(fig, &axs[1], &second, labelled("Some other units"))?;
    let listed = Colormap::listed("from_list", vec!["r", "g", "b", "c"])?
        .with_over("0.35")?
        .with_under("0.75")?;
    let bounds = [2.0, 3.0, 7.0, 9.0, 15.0];
    let boundary = BoundaryNorm::new(bounds, listed.n(), Extend::Neither)?;
    let third = ScalarMappable::with_norm(listed, Box::new(boundary));
    colorbar_below(
        fig,
        &axs[2],
        &third,
        ColorbarOptions {
            extend: Some(Extend::Both),
            ticks: Some(ColorbarTicks::Values(bounds.to_vec())),
            spacing: Spacing::Proportional,
            ..labelled("Discrete intervals, some other units")
        },
    )?;
    bullseye_plot(&axs[0], &data, &[], &first)?;
    axs[0].set_title("Bulls Eye (AHA)");
    bullseye_plot(&axs[1], &data, &[], &second)?;
    axs[1].set_title("Bulls Eye (AHA)");
    bullseye_plot(&axs[2], &data, &[3, 5, 6, 11, 12, 16], &third)?;
    axs[2].set_title("Segments [3, 5, 6, 11, 12, 16] in bold");
    Ok(())
}

/// The five source profiles of one scenario of the radar chart, over its
/// nine species.
type Profiles = [[f64; 9]; 5];

/// The Denver Aerosol Sources and Health study's profiles of the radar
/// chart example, by scenario.
const RADAR_DATA: [(&str, Profiles); 4] = [
    (
        "Basecase",
        [
            [0.88, 0.01, 0.03, 0.03, 0.00, 0.06, 0.01, 0.00, 0.00],
            [0.07, 0.95, 0.04, 0.05, 0.00, 0.02, 0.01, 0.00, 0.00],
            [0.01, 0.02, 0.85, 0.19, 0.05, 0.10, 0.00, 0.00, 0.00],
            [0.02, 0.01, 0.07, 0.01, 0.21, 0.12, 0.98, 0.00, 0.00],
            [0.01, 0.01, 0.02, 0.71, 0.74, 0.70, 0.00, 0.00, 0.00],
        ],
    ),
    (
        "With CO",
        [
            [0.88, 0.02, 0.02, 0.02, 0.00, 0.05, 0.00, 0.05, 0.00],
            [0.08, 0.94, 0.04, 0.02, 0.00, 0.01, 0.12, 0.04, 0.00],
            [0.01, 0.01, 0.79, 0.10, 0.00, 0.05, 0.00, 0.31, 0.00],
            [0.00, 0.02, 0.03, 0.38, 0.31, 0.31, 0.00, 0.59, 0.00],
            [0.02, 0.02, 0.11, 0.47, 0.69, 0.58, 0.88, 0.00, 0.00],
        ],
    ),
    (
        "With O3",
        [
            [0.89, 0.01, 0.07, 0.00, 0.00, 0.05, 0.00, 0.00, 0.03],
            [0.07, 0.95, 0.05, 0.04, 0.00, 0.02, 0.12, 0.00, 0.00],
            [0.01, 0.02, 0.86, 0.27, 0.16, 0.19, 0.00, 0.00, 0.00],
            [0.01, 0.03, 0.00, 0.32, 0.29, 0.27, 0.00, 0.00, 0.95],
            [0.02, 0.00, 0.03, 0.37, 0.56, 0.47, 0.87, 0.00, 0.00],
        ],
    ),
    (
        "CO & O3",
        [
            [0.87, 0.01, 0.08, 0.00, 0.00, 0.04, 0.00, 0.00, 0.01],
            [0.09, 0.95, 0.02, 0.03, 0.00, 0.01, 0.13, 0.06, 0.00],
            [0.01, 0.02, 0.71, 0.24, 0.13, 0.16, 0.00, 0.50, 0.00],
            [0.01, 0.03, 0.00, 0.28, 0.24, 0.23, 0.00, 0.44, 0.88],
            [0.02, 0.00, 0.18, 0.45, 0.64, 0.55, 0.86, 0.00, 0.16],
        ],
    ),
];

fn radar_chart(fig: &Figure) -> CaseResult {
    let spokes = 9;
    let theta = linspace_with(0.0, 2.0 * PI, spokes, false);
    let spoke_labels = [
        "Sulfate", "Nitrate", "EC", "OC1", "OC2", "OC3", "OP", "CO", "O3",
    ];
    let axs: Vec<PolarAxes> = (1..=4)
        .map(|i| fig.add_polar_subplot(2, 2, i))
        .collect::<Result<_, _>>()?;
    fig.subplots_adjust(&SubplotOverrides {
        wspace: Some(0.25),
        hspace: Some(0.20),
        top: Some(0.85),
        bottom: Some(0.05),
        ..SubplotOverrides::default()
    })?;
    let colors = ["b", "r", "g", "m", "y"];
    let mut first_lines = Vec::new();
    for (ax, (title, profiles)) in axs.iter().zip(&RADAR_DATA) {
        ax.set_frame(PolarFrame::Polygon(spokes))?
            .set_theta_zero_location(Compass::N, 0.0);
        ax.set_rgrids(&[0.2, 0.4, 0.6, 0.8], None);
        // matplotlib's `y` (1.0) overrides the `position` given with it.
        ax.set_title(*title)
            .set_weight(FontWeight::BOLD)
            .set_fontsize_relative(RelativeSize::Medium)
            .set_halign(HAlign::Center)
            .set_valign(VAlign::Center);
        for (profile, color) in profiles.iter().zip(colors) {
            // The example's `plot` closes the line.
            let closed_theta: Vec<f64> = theta.iter().chain(&theta[..1]).copied().collect();
            let closed: Vec<f64> = profile.iter().chain(&profile[..1]).copied().collect();
            let line = ax.plot_with(closed_theta, closed, LineProps::default().color(color))?;
            if first_lines.len() < colors.len() {
                first_lines.push(LegendHandle::from(&line));
            }
            ax.fill(
                &theta,
                profile,
                PatchProps::default()
                    .facecolor(color)
                    .alpha(0.25)
                    .label("_nolegend_"),
            )?;
        }
        let degrees: Vec<f64> = theta.iter().map(|t| t.to_degrees()).collect();
        ax.set_thetagrids_labeled(&degrees, spoke_labels)?;
    }
    axs[0].legend(LegendOptions {
        handles: Some(first_lines),
        labels: Some((1..=5).map(|i| format!("Factor {i}")).collect()),
        loc: Some(LegendLoc::Point(0.9, 0.95)),
        labelspacing: Some(0.1),
        fontsize: Some(FontSize::Relative(RelativeSize::Small)),
        ..LegendOptions::default()
    })?;
    fig.text(
        0.5,
        0.965,
        "5-Factor Solution Profiles Across Four Scenarios",
    )
    .set_halign(HAlign::Center)
    .set_color("black")
    .set_weight(FontWeight::BOLD)
    .set_fontsize_relative(RelativeSize::Large);
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_misc/radar_chart", radar_chart).sized(9.0, 9.0, 100.0),
    FigureCase::new("gallery_misc/custom_projection", custom_projection),
    FigureCase::new("gallery_misc/logos2_1", logos2_1).sized(1.1, 1.1, 100.0),
    FigureCase::new("gallery_misc/logos2_2", logos2_2).sized(0.32, 0.32, 100.0),
    FigureCase::new("gallery_misc/logos2_3", logos2_3).sized(5.5, 1.1, 100.0),
    FigureCase::new(
        "gallery_misc/leftventricle_bullseye",
        leftventricle_bullseye,
    )
    .sized(10.0, 5.0, 100.0),
];
