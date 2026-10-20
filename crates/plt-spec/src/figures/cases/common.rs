//! Small helpers several case groups share.

use std::f64::consts::PI;

use plt::artist::ArtistHandle;
use plt::axes::{Axes, AxisName, BarLabelOptions, TickLevel, TickParams};
use plt::axisartist::angle_helper::{ExtremeFinderCycle, FormatterDms, LocatorDms};
use plt::axisartist::grid_finder::{self, GridFinderOptions, GridLocator};
use plt::axisartist::grid_helper_curvelinear::GridHelperCurveLinear;
use plt::axisartist::{self, AuxTransform, AxisArtist};
use plt::collections::CollectionProps;
use plt::colors::{Color, ColorError, ColormapError, colormaps};
use plt::contour::{ContourOptions, ContourSet, Levels};
use plt::figure::{Figure, Mosaic, MosaicCell, MosaicError};
use plt::fonts::{FontProperties, FontSize};
use plt::gridspec::GridSpec;
use plt::layout::TightLayout;
use plt::legend::LegendOptions;
use plt::lines::LineProps;
use plt::mplot3d::{Axes3D, Axes3DError, Axes3DOptions, FaceColormap, FaceColors};
use plt::numeric::{Grid, arange, linspace, meshgrid};
use plt::patches::{ArrowStyle, BoxStyle, ConnectionStyle, PatchProps};
use plt::patheffects::{GcUpdate, PathEffect, Stroke, TickedStroke};
use plt::rc::Aspect;
use plt::sankey::FlowOrientation;
use plt::scale::ScaleOptions;
use plt::spines::SpineSide;
use plt::text::{
    ArrowProps, FancyArrowProps, Language, LanguageRange, LegacyArrowProps, TextBoxProps,
};
use plt::ticker::{
    AutoLocator, AutoMinorLocator, FixedLocator, IndexLocator, LinearLocator, LogLocator,
    MaxNLocator, MultipleLocator, Nbins, NullLocator, NumTicks,
};
use plt::transforms::Affine2D;

use crate::rng::SpecRng;

/// Character ranges of a language.
pub(super) fn ranges(ranges: &[(&str, usize, usize)]) -> Language {
    ranges
        .iter()
        .map(|&(tag, start, end)| LanguageRange::new(tag, start, end))
        .collect()
}

/// The default font at `points`.
pub(super) fn sized(points: f64) -> FontProperties {
    let rc = plt::rc::params();
    FontProperties::from_rc(&rc).with_size(FontSize::Points(points), rc.font_size)
}

/// `Affine2D().scale(np.pi / 180, 1) + PolarAxes.PolarTransform()`.
pub(super) fn polar_degrees() -> AuxTransform {
    AuxTransform::affine(Affine2D::scaling(PI / 180.0, 1.0)).then(AuxTransform::polar())
}

/// Takes the ticks off `ax` (`ax.set(xticks=[], yticks=[])`).
pub(super) fn tickless(ax: &Axes) {
    ax.set_xticks(Vec::<f64>::new())
        .set_yticks(Vec::<f64>::new());
}

/// The bare strip of the gallery's tick demos (the start of `setup` in
/// `tick-locators` and `tick-formatters`, and of `plot_axis` in
/// `date_formatters_locators`): no y ticks, only the bottom spine, and
/// major and minor ticks 5 and 2.5 points long.
pub(super) fn tick_strip(ax: &Axes) {
    ax.yaxis().set_major_locator(NullLocator);
    let spines = ax.spines();
    for side in [SpineSide::Left, SpineSide::Right, SpineSide::Top] {
        spines.get(side).set_visible(false);
    }
    for (which, width, length) in [(TickLevel::Major, 1.0, 5.0), (TickLevel::Minor, 0.75, 2.5)] {
        ax.tick_params(&TickParams {
            which,
            width: Some(width),
            length: Some(length),
            ..TickParams::default()
        });
    }
}

/// The gallery's "Tick locators" figure: eight strips set up by `strip`
/// with their names, one per locator, in a tight layout.
pub(super) fn locator_strips(
    fig: &Figure,
    strip: impl Fn(&Axes, &str) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let axs = fig.subplots(8, 1)?;
    let ax = |i: usize| &axs[(i, 0)];
    strip(ax(0), "NullLocator()")?;
    ax(0)
        .xaxis()
        .set_major_locator(NullLocator)
        .set_minor_locator(NullLocator);
    strip(ax(1), "MultipleLocator(0.5, offset=0.2)")?;
    ax(1)
        .xaxis()
        .set_major_locator(MultipleLocator::new(0.5)?.with_offset(0.2))
        .set_minor_locator(MultipleLocator::new(0.1)?);
    strip(ax(2), "FixedLocator([0, 1, 5])")?;
    ax(2)
        .xaxis()
        .set_major_locator(FixedLocator::new(vec![0.0, 1.0, 5.0]))
        .set_minor_locator(FixedLocator::new(linspace(0.2, 0.8, 4)));
    strip(ax(3), "LinearLocator(numticks=3)")?;
    ax(3)
        .xaxis()
        .set_major_locator(LinearLocator::new(Some(3)))
        .set_minor_locator(LinearLocator::new(Some(31)));
    strip(ax(4), "IndexLocator(base=0.5, offset=0.25)")?;
    ax(4).plot_with(
        [0.0, 1.0, 2.0, 3.0, 4.0],
        [0.0; 5],
        LineProps::default().color("white"),
    )?;
    ax(4)
        .xaxis()
        .set_major_locator(IndexLocator::new(0.5, 0.25));
    strip(ax(5), "AutoLocator()")?;
    ax(5)
        .xaxis()
        .set_major_locator(AutoLocator::new())
        .set_minor_locator(AutoMinorLocator::new(None));
    strip(ax(6), "MaxNLocator(n=4)")?;
    ax(6)
        .xaxis()
        .set_major_locator(MaxNLocator::new(Nbins::Fixed(4)))
        .set_minor_locator(MaxNLocator::new(Nbins::Fixed(40)));
    strip(ax(7), "LogLocator(base=10, numticks=15)")?;
    ax(7).set_xlim(1e3, 1e10);
    ax(7).set_xscale("log")?;
    ax(7)
        .xaxis()
        .set_major_locator(LogLocator::new(10.0)?.with_numticks(NumTicks::Fixed(15)));
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// The six axes of the gallery's logit scale demo, plotting `curves`
/// (values over `x` in `[-10, 10]` and their labels) with a legend and a
/// grid: the top rows on logit scales (the middle one writing values near
/// 1 as complements of small numbers), the bottom row linear, the right
/// column zoomed on the values near 1 over positive `x`.
pub(super) fn logit_grid(
    fig: &Figure,
    x: &[f64],
    curves: &[(Vec<f64>, &str)],
) -> Result<(), Box<dyn std::error::Error>> {
    let axs = fig.subplots(3, 2)?;
    for ax in axs.as_slice() {
        for (y, label) in curves {
            ax.plot(x, y)?.set_label(*label);
        }
        ax.legend(LegendOptions::default())?;
        ax.grid(true);
    }
    let survival = ScaleOptions {
        one_half: Some("1/2".into()),
        use_overline: Some(true),
        ..ScaleOptions::default()
    };
    for (row, title) in [(0, "logit scale"), (1, "logit scale"), (2, "linear scale")] {
        for col in 0..2 {
            let ax = &axs[(row, col)];
            ax.set_title(title);
            match row {
                0 => {
                    ax.set_yscale("logit")?;
                }
                1 => {
                    ax.set_scale_with(AxisName::Y, "logit", &survival)?;
                }
                _ => {}
            }
            if col == 1 {
                ax.set_xlim(0.0, 10.0);
            }
            match (row, col) {
                (2, 0) => ax.set_ylim(0.0, 1.0),
                (2, 1) => ax.set_ylim(0.8, 1.0),
                (_, 0) => ax.set_ylim(1e-5, 1.0 - 1e-5),
                _ => ax.set_ylim(0.8, 1.0 - 5e-3),
            };
        }
    }
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// `f` applied to each value.
pub(super) fn map(values: &[f64], f: impl Fn(f64) -> f64) -> Vec<f64> {
    values.iter().copied().map(f).collect()
}

/// The surface many gallery examples sample (`_func3` of the image
/// demos): `(1 - x/2 + x⁵ + y³) e^{-(x² + y²)}`.
pub(super) fn func3(x: f64, y: f64) -> f64 {
    (1.0 - x / 2.0 + x.powf(5.0) + y.powf(3.0)) * (-(x * x + y * y)).exp()
}

/// Running sums of `values` (`np.cumsum`).
pub(super) fn cumsum(values: &[f64]) -> Vec<f64> {
    values
        .iter()
        .scan(0.0, |sum, v| {
            *sum += v;
            Some(*sum)
        })
        .collect()
}

/// `bar_label` options with only `padding` set.
pub(super) fn padded(padding: f64) -> BarLabelOptions {
    BarLabelOptions {
        padding: vec![padding],
        ..BarLabelOptions::default()
    }
}

/// The raised sine of matplotlib's `markevery` tests and demo: 200 points
/// of `sin(x) + 1.11` for `x` in `[0.11, 9.89]` (`_markevery_sine`).
pub(super) fn markevery_sine() -> (Vec<f64>, Vec<f64>) {
    let delta = 0.11;
    let x = map(&linspace(0.0, 10.0 - 2.0 * delta, 200), |x| x + delta);
    let y = map(&x, |x| x.sin() + 1.0 + delta);
    (x, y)
}

/// One axes with both limits `(0, 10)` (`plt.axis([0, 10, 0, 10])`).
pub(super) fn blank(fig: &Figure) -> Axes {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
    ax
}

/// About `n` automatic contour levels (`levels=n`).
pub(super) fn count(n: i64) -> ContourOptions {
    ContourOptions {
        levels: Levels::Count(n),
        ..ContourOptions::default()
    }
}

/// Explicit contour levels (`levels=[...]`).
pub(super) fn values(levels: &[f64]) -> ContourOptions {
    ContourOptions {
        levels: Levels::Values(levels.to_vec()),
        ..ContourOptions::default()
    }
}

/// Colours by name (`colors=[...]`).
pub(super) fn colors(names: &[&str]) -> Vec<Color> {
    names.iter().map(|&name| Color::from(name)).collect()
}

/// Hatch patterns, `None` leaving a band plain (`hatches=[...]`).
pub(super) fn hatches(patterns: &[Option<&str>]) -> Vec<Option<String>> {
    patterns.iter().map(|p| p.map(str::to_owned)).collect()
}

/// `bbox=dict(boxstyle=style, **patch)`.
pub(super) fn styled_box(
    style: &str,
    patch: PatchProps,
) -> Result<Option<TextBoxProps>, Box<dyn std::error::Error>> {
    Ok(Some(
        TextBoxProps::new(style.parse::<BoxStyle>()?).with_patch(patch),
    ))
}

/// The legacy annotation arrow of matplotlib's examples and tests,
/// `arrowprops=dict(facecolor='black', shrink=0.05)`: black, shortened by
/// 5%.
pub(super) fn black_arrow() -> ArrowProps {
    ArrowProps::Legacy(LegacyArrowProps {
        shrink: 0.05,
        patch: PatchProps::default().facecolor("black"),
        ..LegacyArrowProps::default()
    })
}

/// `arrowprops=dict(arrowstyle=style, connectionstyle=connection)`.
pub(super) fn arrow(
    style: &str,
    connection: &str,
) -> Result<FancyArrowProps, Box<dyn std::error::Error>> {
    Ok(FancyArrowProps {
        connectionstyle: connection.parse::<ConnectionStyle>()?,
        ..FancyArrowProps::new(style.parse::<ArrowStyle>()?)
    })
}

/// A mosaic of labelled cells (`"."` for an empty one), as nested lists
/// of labels give it.
pub(super) fn mosaic(rows: &[&[&str]]) -> Result<Mosaic, MosaicError> {
    let cell = |label: &&str| match *label {
        "." => MosaicCell::Empty,
        label => MosaicCell::Label(label.to_owned()),
    };
    Mosaic::new(
        rows.iter()
            .map(|row| row.iter().map(cell).collect())
            .collect(),
    )
}

/// Hides the top and right spines
/// (`ax.spines[['top', 'right']].set_visible(False)`).
pub(super) fn hide_top_right(ax: &Axes) {
    let spines = ax.spines();
    spines.top.set_visible(false);
    spines.right.set_visible(false);
}

/// The curve of the gallery's spine examples: `2 sin x` at 100 points of
/// `[0, 2π]`.
pub(super) fn double_sine() -> (Vec<f64>, Vec<f64>) {
    let x = linspace(0.0, 2.0 * PI, 100);
    let y = map(&x, |x| 2.0 * x.sin());
    (x, y)
}

/// The three axes of the gallery's `spines` example: all four spines, the
/// bottom and left ones only, and those two bounded to the data range.
pub(super) fn spine_bounds_demo(fig: &Figure) -> Result<(), Box<dyn std::error::Error>> {
    let (x, y) = double_sine();
    let axs = fig.subplots(3, 1)?;
    for (row, title) in [
        "normal spines",
        "bottom-left spines",
        "spines with bounds limited to data range",
    ]
    .into_iter()
    .enumerate()
    {
        let ax = &axs[(row, 0)];
        ax.plot(&x, &y)?;
        ax.set_title(title);
    }
    hide_top_right(&axs[(1, 0)]);
    let bounded = &axs[(2, 0)];
    let (ymin, ymax) = y
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(v), hi.max(v))
        });
    bounded
        .spines()
        .bottom
        .set_bounds(Some((x[0], x[x.len() - 1])));
    bounded.spines().left.set_bounds(Some((ymin, ymax)));
    hide_top_right(bounded);
    Ok(())
}

/// The seed of the gallery examples' `np.random.seed(19680801)`.
pub(super) const GALLERY_SEED: u64 = 19_680_801;

/// The gallery's favourite field: a positive and a negative bump
/// (`spec_support.gallery_data.two_bumps`).
pub(super) fn two_bumps(x: f64, y: f64) -> f64 {
    let z1 = (-(x * x) - y * y).exp();
    let z2 = (-((x - 1.0) * (x - 1.0)) - (y - 1.0) * (y - 1.0)).exp();
    (z1 - z2) * 2.0
}

/// `two_bumps` on the grid of `x` and `y`, with the grid (`np.meshgrid`).
fn bumps_on(x: &[f64], y: &[f64]) -> (Grid<f64>, Grid<f64>, Grid<f64>) {
    let (xx, yy) = meshgrid(x, y);
    let z = Grid::from_fn(xx.rows(), xx.cols(), |r, c| {
        two_bumps(xx[(r, c)], yy[(r, c)])
    });
    (xx, yy, z)
}

/// `contour_demo`'s field: `two_bumps` over `[-3, 3) × [-2, 2)` in steps
/// of 0.025, with its grid (`contour_field`).
pub(super) fn contour_field() -> (Grid<f64>, Grid<f64>, Grid<f64>) {
    bumps_on(&arange(-3.0, 3.0, 0.025), &arange(-2.0, 2.0, 0.025))
}

/// `image_demo`'s field: `two_bumps` over `[-3, 3)²` in steps of 0.025
/// (`image_field`).
pub(super) fn image_field() -> Grid<f64> {
    let x = arange(-3.0, 3.0, 0.025);
    bumps_on(&x, &x).2
}

/// A 256 × 256 MRI-like slice in place of `s1045.ima.gz` (`scan`): a
/// bright ellipse with darker rings inside, values up to 1000.
pub(super) fn scan() -> Grid<f64> {
    let v = linspace(-1.0, 1.0, 256);
    Grid::from_fn(256, 256, |r, c| {
        let (x, y) = (v[c], v[r]);
        let (ex, ey) = (x / 0.75, y / 0.9);
        let head = (-(ex * ex + ey * ey).powf(4.0)).exp();
        let folds = 0.5 + 0.5 * (14.0 * (x - 0.1).hypot(y + 0.05)).cos();
        1000.0 * head * (0.6 + 0.4 * folds)
    })
}

/// A rough 344 × 403 elevation model in place of
/// `jacksboro_fault_dem.npz` (`terrain`): ridges with noise on top.
pub(super) fn terrain() -> Grid<f64> {
    let noise = SpecRng::new(GALLERY_SEED).normal(344 * 403, 0.0, 12.0);
    Grid::from_fn(344, 403, |y, x| {
        let (xf, yf) = (x as f64, y as f64);
        let ridges = 600.0 + 250.0 * (xf / 37.0).sin() * (yf / 53.0).cos() + 0.5 * (xf - yf);
        ridges + noise[y * 403 + x]
    })
}

/// Sankey flow orientations from matplotlib's codes `-1`, `0` and `1`.
pub(super) fn flow_orientations(codes: &[i8]) -> Vec<FlowOrientation> {
    codes
        .iter()
        .map(|code| match code {
            1 => FlowOrientation::Top,
            -1 => FlowOrientation::Bottom,
            _ => FlowOrientation::Side,
        })
        .collect()
}

/// Sankey flow labels, `None` for Python's `None`.
pub(super) fn flow_labels(texts: &[Option<&str>]) -> Vec<Option<String>> {
    texts.iter().map(|t| t.map(str::to_owned)).collect()
}

/// Sankey flow labels that all show text.
pub(super) fn shown_labels(texts: &[&str]) -> Vec<Option<String>> {
    texts.iter().map(|&t| Some(t.to_owned())).collect()
}

/// A stroke `width` points wide in `color` (`Stroke(linewidth=width,
/// foreground=color)`).
pub(super) fn stroke(width: f64, color: &str) -> Result<Stroke, ColorError> {
    Ok(Stroke::new(
        GcUpdate::default().linewidth(width).foreground(color)?,
    ))
}

/// `[withStroke(linewidth=width, foreground=color)]`.
pub(super) fn with_stroke(width: f64, color: &str) -> Result<Vec<PathEffect>, ColorError> {
    Ok(vec![PathEffect::with_stroke(stroke(width, color)?)])
}

/// Sets `effects` on every collection of `contours` (matplotlib's
/// `ContourSet.set(path_effects=...)`).
pub(super) fn contour_effects(contours: &ContourSet, effects: &[PathEffect]) {
    for collection in contours.collections() {
        collection.set_path_effects(effects.to_vec());
    }
}

/// `[withTickedStroke(angle=angle, spacing=spacing, length=length)]`.
pub(super) fn ticked_stroke(angle: f64, spacing: f64, length: f64) -> Vec<PathEffect> {
    vec![PathEffect::with_ticked_stroke(TickedStroke {
        angle,
        spacing,
        length,
        ..TickedStroke::default()
    })]
}

/// `np.mgrid[start:stop:n*1j]` along one axis: like `linspace`, but the
/// last value is computed rather than pinned to `stop`.
pub(super) fn mgrid_count(start: f64, stop: f64, n: usize) -> Vec<f64> {
    let step = (stop - start) / (n - 1) as f64;
    (0..n).map(|i| i as f64 * step + start).collect()
}

/// numpy's `hanning(len)`, normalised to sum to 1.
fn hanning_weights(len: usize) -> Vec<f64> {
    let span = (len - 1) as f64;
    let window: Vec<f64> = (0..len)
        .map(|k| 0.5 + 0.5 * (PI * (2.0 * k as f64 - span) / span).cos())
        .collect();
    let total: f64 = window.iter().sum();
    window.iter().map(|w| w / total).collect()
}

/// `smooth1d` of matplotlib's agg filter test and example: a moving
/// average weighted by a Hanning window of `weights.len()` samples, the
/// ends extended by point reflection.
fn smooth1d(x: &[f64], weights: &[f64]) -> Vec<f64> {
    let len = weights.len();
    let (first, last) = (x[0], x[x.len() - 1]);
    let reflected: Vec<f64> = (2..=len)
        .rev()
        .map(|i| 2.0 * first - x[i])
        .chain(x.iter().copied())
        .chain((0..len - 1).map(|i| 2.0 * last - x[x.len() - 1 - i]))
        .collect();
    let start = len - 1 - (len - 1) / 2;
    (0..x.len())
        .map(|i| {
            let window = &reflected[start + i..start + i + len];
            window.iter().zip(weights).map(|(s, w)| s * w).sum()
        })
        .collect()
}

/// `smooth2d` of matplotlib's agg filter test and example: [`smooth1d`]
/// down every column of the `width`-wide row-major `values`, then along
/// every row.
pub(super) fn smooth2d(values: &[f64], width: usize, sigma: f64) -> Vec<f64> {
    let weights = hanning_weights((sigma as usize).max(3) * 2 + 1);
    let height = values.len() / width;
    let mut smoothed = values.to_vec();
    for col in 0..width {
        let column: Vec<f64> = (0..height).map(|row| values[row * width + col]).collect();
        for (row, value) in smooth1d(&column, &weights).into_iter().enumerate() {
            smoothed[row * width + col] = value;
        }
    }
    smoothed
        .chunks(width)
        .flat_map(|row| smooth1d(row, &weights))
        .collect()
}

/// `cbook.get_sample_data("axes_grid/bivariate_normal.npy")`: ten times
/// the difference of two bivariate normal densities, the standard one and
/// one centred at `(1, 1)` with deviations 1.5 and 0.5, on the 15 × 15
/// grid `x = -3, -2.5, ..., 4`, `y = -4, -3.5, ..., 3`.
pub(super) fn bivariate_normal() -> Grid<f64> {
    let density = |x: f64, y: f64, (sigma_x, sigma_y): (f64, f64), (mu_x, mu_y): (f64, f64)| {
        let z = (x - mu_x).powi(2) / sigma_x.powi(2) + (y - mu_y).powi(2) / sigma_y.powi(2);
        (-z / 2.0).exp() / (2.0 * std::f64::consts::PI * sigma_x * sigma_y)
    };
    Grid::from_fn(15, 15, |r, c| {
        let (x, y) = (-3.0 + 0.5 * c as f64, -4.0 + 0.5 * r as f64);
        (density(x, y, (1.0, 1.0), (0.0, 0.0)) - density(x, y, (1.5, 0.5), (1.0, 1.0))) * 10.0
    })
}

/// [`bivariate_normal`] framed by zeros, as the zoom examples show it: a
/// 150 × 150 grid holding the sample data from row and column 30 on
/// (`Z2 = np.zeros((150, 150)); Z2[30:30+ny, 30:30+nx] = Z`).
pub(super) fn framed_bivariate_normal() -> Grid<f64> {
    let z = bivariate_normal();
    Grid::from_fn(150, 150, |r, c| {
        r.checked_sub(30)
            .zip(c.checked_sub(30))
            .and_then(|(r, c)| z.get(r, c).copied())
            .unwrap_or(0.0)
    })
}

/// `fig.subplots(nrows, ncols, subplot_kw={"projection": "3d"})`, in
/// row-major order: the cells of one grid, which layout engines lay out
/// together.
pub(super) fn subplots_3d(
    fig: &Figure,
    nrows: usize,
    ncols: usize,
) -> Result<Vec<Axes3D>, Axes3DError> {
    let grid = GridSpec::new(nrows, ncols)?;
    (0..nrows * ncols)
        .map(|i| {
            let cell = grid.cell(i / ncols, i % ncols)?;
            fig.add_subplot_spec_3d(&cell, Axes3DOptions::default())
        })
        .collect()
}

/// Surface faces coloured by the colormap `name` (`cmap=name`).
pub(super) fn face_cmap(name: &str) -> Result<FaceColors, ColormapError> {
    Ok(FaceColors::Mapped(Box::new(FaceColormap::new(
        colormaps::get(name)?,
    ))))
}

/// `lw=linewidth, antialiased=False`.
pub(super) fn aliased(linewidth: f64) -> CollectionProps {
    CollectionProps {
        antialiased: Some(vec![false]),
        ..CollectionProps::default().linewidth(linewidth)
    }
}

/// The polar grid in degrees of matplotlib's `demo_curvelinear_grid` in
/// a rectangular box, its radii located by `grid_locator2` (the default
/// locator when `None`).
pub(super) fn polar_grid_helper(
    grid_locator2: Option<Box<dyn GridLocator>>,
) -> GridHelperCurveLinear {
    GridHelperCurveLinear::new(
        polar_degrees(),
        GridFinderOptions {
            extreme_finder: Some(Box::new(ExtremeFinderCycle {
                lat_minmax: Some((0.0, f64::INFINITY)),
                ..ExtremeFinderCycle::new(20, 20)
            })),
            grid_locator1: Some(Box::new(LocatorDms::new(12))),
            grid_locator2,
            tick_formatter1: Some(Box::new(FormatterDms)),
            ..GridFinderOptions::default()
        },
    )
}

/// The `setup_axes` of the axisartist gallery's polar demos: axisartist
/// axes in cell `index` of a `rows × cols` grid with the polar grid of
/// [`polar_grid_helper`] (radii by `MaxNLocator(5)`), equal aspect and
/// limits `(-5, 12)` × `(-5, 10)`.
pub(super) fn polar_box(
    fig: &Figure,
    (rows, cols, index): (usize, usize, usize),
) -> Result<axisartist::Axes, Box<dyn std::error::Error>> {
    let helper = polar_grid_helper(Some(Box::new(grid_finder::MaxNLocator::new(5))));
    let ax = axisartist::Axes::with_grid_helper(fig.add_subplot(rows, cols, index)?, helper);
    ax.ax().set_aspect(Aspect::Equal)?;
    ax.ax().set_xlim(-5.0, 12.0).set_ylim(-5.0, 10.0);
    Ok(ax)
}

/// The axis artist under `name` (`ax.axis[name]`).
pub(super) fn axis_artist(ax: &axisartist::Axes, name: &str) -> Result<AxisArtist, &'static str> {
    ax.axis(name).ok_or("no such axis artist")
}

/// `axis.label.set_text(text); axis.label.set_visible(True)`.
pub(super) fn show_label(axis: &AxisArtist, text: &str) {
    axis.label().set_text(text).set_visible(true);
}
