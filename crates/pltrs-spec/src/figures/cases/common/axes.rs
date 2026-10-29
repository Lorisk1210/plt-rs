//! Axes several case groups set up alike: tick strips and decorations,
//! mosaics, 3-D and curvilinear-grid axes, and their axis artists.

use std::f64::consts::PI;

use pltrs::artist::ArtistHandle;
use pltrs::axes::{Axes, AxisName, LocatorParams, PlotError, TickLevel, TickParams, TickPosition};
use pltrs::axisartist::angle_helper::{ExtremeFinderCycle, FormatterDms, LocatorDms};
use pltrs::axisartist::grid_finder::{self, GridFinderOptions, GridLocator};
use pltrs::axisartist::grid_helper_curvelinear::GridHelperCurveLinear;
use pltrs::axisartist::{self, AuxTransform, AxisArtist};
use pltrs::colors::{Color, Colorize, ColorizerOptions, ColormapError, colormaps};
use pltrs::figure::{Figure, Mosaic, MosaicCell, MosaicError};
use pltrs::fonts::FontSize;
use pltrs::gridspec::{GridSpec, SubplotSpec};
use pltrs::layout::{LayoutEngine, LayoutEngineError, TightLayout};
use pltrs::legend::LegendOptions;
use pltrs::lines::{LineProps, LineStyle, MarkEvery};
use pltrs::mplot3d::{Axes3D, Axes3DError, Axes3DOptions, FaceColors};
use pltrs::numeric::linspace;
use pltrs::rc::Aspect;
use pltrs::scale::ScaleOptions;
use pltrs::spines::SpineSide;
use pltrs::ticker::{
    AutoLocator, AutoMinorLocator, FixedLocator, IndexLocator, LinearLocator, LogLocator,
    MaxNLocator, MultipleLocator, Nbins, NullLocator, NumTicks,
};
use pltrs::transforms::Affine2D;

use super::data::double_sine;

/// `Affine2D().scale(np.pi / 180, 1) + PolarAxes.PolarTransform()`.
pub(crate) fn polar_degrees() -> AuxTransform {
    AuxTransform::affine(Affine2D::scaling(PI / 180.0, 1.0)).then(AuxTransform::polar())
}

/// Takes the ticks off `ax` (`ax.set(xticks=[], yticks=[])`).
pub(crate) fn tickless(ax: &Axes) {
    ax.set_xticks(Vec::<f64>::new())
        .set_yticks(Vec::<f64>::new());
}

/// The bare strip of the gallery's tick demos (the start of `setup` in
/// `tick-locators` and `tick-formatters`, and of `plot_axis` in
/// `date_formatters_locators`): no y ticks, only the bottom spine, and
/// major and minor ticks 5 and 2.5 points long.
pub(crate) fn tick_strip(ax: &Axes) {
    ax.yaxis().set_major_locator(NullLocator);
    let spines = ax.spines();
    for side in [SpineSide::Left, SpineSide::Right, SpineSide::Top] {
        spines.get(side).set_visible(false);
    }
    for (which, width, length) in [(TickLevel::Major, 1.0, 5.0), (TickLevel::Minor, 0.75, 2.5)] {
        ax.tick_params(TickParams {
            which,
            width: Some(width),
            length: Some(length),
            ..TickParams::default()
        });
    }
}

/// The gallery's "Tick locators" figure: eight strips set up by `strip`
/// with their names, one per locator, in a tight layout.
pub(crate) fn locator_strips(
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
    ax(7).set_xlim(1e3, 1e10)?;
    ax(7).set_xscale("log")?;
    ax(7)
        .xaxis()
        .set_major_locator(LogLocator::new(10.0)?.with_numticks(NumTicks::Fixed(15)));
    fig.tight_layout(TightLayout::default())?;
    Ok(())
}

/// The six axes of the gallery's logit scale demo, plotting `curves`
/// (values over `x` in `[-10, 10]` and their labels) with a legend and a
/// grid: the top rows on logit scales (the middle one writing values near
/// 1 as complements of small numbers), the bottom row linear, the right
/// column zoomed on the values near 1 over positive `x`.
pub(crate) fn logit_grid(
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
                    ax.set_scale_with(AxisName::Y, "logit", survival.clone())?;
                }
                _ => {}
            }
            if col == 1 {
                ax.set_xlim(0.0, 10.0)?;
            }
            match (row, col) {
                (2, 0) => ax.set_ylim(0.0, 1.0)?,
                (2, 1) => ax.set_ylim(0.8, 1.0)?,
                (_, 0) => ax.set_ylim(1e-5, 1.0 - 1e-5)?,
                _ => ax.set_ylim(0.8, 1.0 - 5e-3)?,
            };
        }
    }
    fig.tight_layout(TightLayout::default())?;
    Ok(())
}

/// One axes with both limits `(0, 10)` (`plt.axis([0, 10, 0, 10])`).
pub(crate) fn blank(fig: &Figure) -> Result<Axes, PlotError> {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 10.0)?.set_ylim(0.0, 10.0)?;
    Ok(ax)
}

/// A mosaic of labelled cells (`"."` for an empty one), as nested lists
/// of labels give it.
pub(crate) fn mosaic(rows: &[&[&str]]) -> Result<Mosaic, MosaicError> {
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
pub(crate) fn hide_top_right(ax: &Axes) {
    let spines = ax.spines();
    spines.top.set_visible(false);
    spines.right.set_visible(false);
}

/// The three axes of the gallery's `spines` example: all four spines, the
/// bottom and left ones only, and those two bounded to the data range.
pub(crate) fn spine_bounds_demo(fig: &Figure) -> Result<(), Box<dyn std::error::Error>> {
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

/// `fig.subplots(nrows, ncols, subplot_kw={"projection": "3d"})`, in
/// row-major order: the cells of one grid, which layout engines lay out
/// together.
pub(crate) fn subplots_3d(
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
pub(crate) fn face_cmap(name: &str) -> Result<FaceColors, ColormapError> {
    Ok(FaceColors::from(
        ColorizerOptions::default().cmap(colormaps::get(name)?),
    ))
}

/// The polar grid in degrees of matplotlib's `demo_curvelinear_grid` in
/// a rectangular box, its radii located by `grid_locator2` (the default
/// locator when `None`).
pub(crate) fn polar_grid_helper(
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
pub(crate) fn polar_box(
    fig: &Figure,
    (rows, cols, index): (usize, usize, usize),
) -> Result<axisartist::AxisArtistAxes, Box<dyn std::error::Error>> {
    let helper = polar_grid_helper(Some(Box::new(grid_finder::GridMaxNLocator::new(5))));
    let ax =
        axisartist::AxisArtistAxes::with_grid_helper(fig.add_subplot(rows, cols, index)?, helper);
    ax.ax().set_aspect(Aspect::Equal)?;
    ax.ax().set_xlim(-5.0, 12.0)?.set_ylim(-5.0, 10.0)?;
    Ok(ax)
}

/// The axis artist under `name` (`ax.axis[name]`).
pub(crate) fn axis_artist(
    ax: &axisartist::AxisArtistAxes,
    name: &str,
) -> Result<AxisArtist, &'static str> {
    ax.axis(name).ok_or("no such axis artist")
}

/// `axis.label.set_text(text); axis.label.set_visible(True)`.
pub(crate) fn show_label(axis: &AxisArtist, text: &str) {
    axis.label().set_text(text).set_visible(true);
}

/// Writes the name of what a tick strip shows over its axis, in blue
/// monospace.
pub(crate) fn strip_title(ax: &Axes, title: &str) {
    ax.text(0.0, 0.2, title)
        .set_transform(ax.trans_axes())
        .set_fontsize(14.0)
        .set_family(["Monospace"])
        .set_color(Color::from("tab:blue"));
}

/// `setup` of the gallery's `tick-locators` and of the axis ticks guide:
/// the bare [`tick_strip`] over `[0, 5]` with ticks at the bottom, named
/// `title`.
pub(crate) fn named_tick_strip(ax: &Axes, title: &str) -> Result<(), Box<dyn std::error::Error>> {
    tick_strip(ax);
    ax.xaxis()
        .set_ticks_position(TickPosition::Side(SpineSide::Bottom))?;
    ax.set_xlim(0.0, 5.0)?.set_ylim(0.0, 1.0)?;
    strip_title(ax, title);
    Ok(())
}

/// `ax.set_xlabel`, `ax.set_ylabel` and `ax.set_title` in `fontsize`
/// points, as the layout tests and guides label their axes.
pub(crate) fn decorate(ax: &Axes, fontsize: f64) {
    ax.set_xlabel("x-label").set_ylabel("y-label");
    for axis in [ax.xaxis(), ax.yaxis()] {
        axis.set_label_fontsize(FontSize::Points(fontsize));
    }
    ax.set_title("Title").set_fontsize(fontsize);
}

/// The `example_plot` of matplotlib's layout tests and guides: a line
/// with at most three tick intervals, labelled in `fontsize` points unless
/// `nodec`, in which case the tick labels are blanked instead.
pub(crate) fn example_plot(
    ax: &Axes,
    fontsize: f64,
    nodec: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    ax.plot_y([1.0, 2.0]);
    ax.locator_params(LocatorParams {
        nbins: Some(Nbins::Fixed(3)),
        ..LocatorParams::default()
    })?;
    if nodec {
        ax.set_xticklabels(Vec::<String>::new())?;
        ax.set_yticklabels(Vec::<String>::new())?;
    } else {
        decorate(ax, fontsize);
    }
    Ok(())
}

/// The only axes of a figure laid out by the constrained layout engine.
pub(crate) fn constrained(fig: &Figure) -> Result<Axes, LayoutEngineError> {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    Ok(fig.subplot())
}

/// Two axes in a constrained figure, one above the other.
pub(crate) fn stacked_pair(fig: &Figure) -> Result<(Axes, Axes), Box<dyn std::error::Error>> {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(2, 1)?;
    Ok((axs[(0, 0)].clone(), axs[(1, 0)].clone()))
}

/// How a `markevery` grid sets up each of its axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MarkeveryAxes {
    /// Plain rectilinear axes.
    Linear,
    /// Log scales on both axes before plotting.
    Log,
    /// Rectilinear axes zoomed to `x` 6–6.7, `y` 1.1–1.7 after plotting.
    Zoomed,
    /// Polar axes.
    Polar,
}

/// One axes of a `markevery` grid, in the cell `spec`, set up as `kind`
/// and titled `title` if given, plotting `y` against `x` as `o-` with 4 pt
/// markers at `markevery`.
pub(crate) fn markevery_panel(
    fig: &Figure,
    spec: &SubplotSpec,
    kind: MarkeveryAxes,
    title: Option<String>,
    (x, y): (&[f64], &[f64]),
    markevery: Option<MarkEvery>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ax = if kind == MarkeveryAxes::Polar {
        (*fig.add_polar_subplot_spec(spec)?).clone()
    } else {
        fig.add_subplot_spec(spec)?
    };
    if let Some(title) = title {
        ax.set_title(title);
    }
    if kind == MarkeveryAxes::Log {
        ax.set_xscale("log")?.set_yscale("log")?;
    }
    let style = LineProps::fmt("o")?
        .linestyle(LineStyle::Solid)
        .markersize(4.0)
        .markevery(markevery);
    ax.plot_with(x, y, style)?;
    if kind == MarkeveryAxes::Zoomed {
        ax.set_xlim(6.0, 6.7)?.set_ylim(1.1, 1.7)?;
    }
    Ok(())
}
