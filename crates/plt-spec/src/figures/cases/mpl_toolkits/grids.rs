//! The other image tests of `test_axes_grid1.py`: twins of host axes,
//! anchored artists, image grids with colorbars, and RGB axes.

use plt::axes::{Axes, ImshowOptions};
use plt::axes_grid1::anchored_artists::{
    AnchoredAuxTransformBox, AnchoredDirectionArrows, AnchoredDrawingArea, AnchoredSizeBar,
};
use plt::axes_grid1::{
    CbarMode, GridDirection, GridRect, ImageGrid, ImageGridOptions, LabelMode, RgbAxes, SizeSpec,
    host_subplot,
};
use plt::colorbar::ColorbarOptions;
use plt::colors::colormaps;
use plt::figure::Figure;
use plt::fonts::{FontProperties, FontSize};
use plt::gridspec::{GridSpec, GridSpecError, SubplotOverrides};
use plt::image::{ColorizerOptions, Interpolation};
use plt::numeric::Grid;
use plt::offsetbox::{AnchorLoc, AnchoredBox, BoxItem};
use plt::patches::{Circle, Ellipse, PatchProps};
use plt::spines::SpineSide;
use plt::text::{HorizontalAlignment, VerticalAlignment};

use super::remove_ticks_and_titles;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `savefig(bbox_inches="tight")`.
const TIGHT: &[(&str, &str)] = &[("savefig.bbox", "tight")];

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "mpl_toolkits/twin_axes_empty_and_removed",
        twin_axes_empty_and_removed,
    )
    .sized(8.0, 6.0, 100.0)
    .with_rc(&[
        ("font.size", "8"),
        ("xtick.labelsize", "8"),
        ("ytick.labelsize", "8"),
    ]),
    FigureCase::new(
        "mpl_toolkits/anchored_direction_arrows",
        anchored_direction_arrows,
    ),
    FigureCase::new(
        "mpl_toolkits/anchored_direction_arrows_many_args",
        anchored_direction_arrows_many_args,
    ),
    FigureCase::new(
        "mpl_toolkits/image_grid_each_left_label_mode_all",
        image_grid_each_left_label_mode_all,
    )
    .sized(3.0, 3.0, 100.0)
    .with_rc(TIGHT),
    FigureCase::new(
        "mpl_toolkits/image_grid_single_bottom",
        image_grid_single_bottom,
    )
    .sized(2.5, 1.5, 100.0)
    .with_rc(TIGHT),
    FigureCase::new("mpl_toolkits/image_grid", image_grid)
        .sized(4.0, 4.0, 100.0)
        .with_rc(TIGHT),
    FigureCase::new("mpl_toolkits/anchored_artists", anchored_artists).sized(3.0, 3.0, 100.0),
    super::classic("mpl_toolkits/rgb_axes", rgb_axes),
    FigureCase::new(
        "mpl_toolkits/imagegrid_cbar_mode_edge",
        imagegrid_cbar_mode_edge,
    )
    .sized(18.0, 9.0, 100.0),
];

/// What `test_twin_axes_empty_and_removed` does to a host and its twin.
const MODIFIERS: [&str; 5] = [
    "",
    "host invisible",
    "twin removed",
    "twin invisible",
    "twin removed\nhost invisible",
];

/// Text centred at `(0.5, 0.5)` of `ax`'s data coordinates.
fn centered_text(ax: &Axes, text: &str) {
    ax.text(0.5, 0.5, text)
        .set_halign(HorizontalAlignment::Center)
        .set_valign(VerticalAlignment::Center);
}

fn twin_axes_empty_and_removed(fig: &Figure) -> CaseResult {
    let generators = ["twinx", "twiny", "twin"];
    let (rows, cols) = (MODIFIERS.len() + 1, generators.len());
    centered_text(host_subplot(fig, rows, cols, 2)?.axes(), "host_subplot");
    let panels = MODIFIERS.iter().flat_map(|modifier| {
        generators
            .iter()
            .map(move |generator| (modifier, generator))
    });
    for (index, (modifier, generator)) in (cols + 1..).zip(panels) {
        let host = host_subplot(fig, rows, cols, index)?;
        let twin = match *generator {
            "twinx" => host.twinx(),
            "twiny" => host.twiny(),
            _ => host.twin(None)?,
        };
        if modifier.contains("twin invisible") {
            for side in SpineSide::ALL {
                twin.axis(side).set_visible(false);
            }
        }
        if modifier.contains("twin removed") {
            twin.remove();
        }
        if modifier.contains("host invisible") {
            for side in SpineSide::ALL {
                host.axis(side).set_visible(false);
            }
        }
        let text = if modifier.is_empty() {
            (*generator).to_owned()
        } else {
            format!("{generator}\n{modifier}")
        };
        centered_text(&host, &text);
    }
    fig.subplots_adjust(&SubplotOverrides {
        wspace: Some(0.5),
        hspace: Some(1.0),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}

/// `imshow(data)`, with `interpolation` when given.
fn show(ax: &Axes, data: Grid<f64>, interpolation: Option<Interpolation>) -> CaseResult {
    ax.imshow(
        data,
        ImshowOptions {
            interpolation,
            ..ImshowOptions::default()
        },
    )?;
    Ok(())
}

fn anchored_direction_arrows(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    show(
        &ax,
        Grid::from_fn(10, 10, |_, _| 0.0),
        Some(Interpolation::Nearest),
    )?;
    ax.add_anchored_box(AnchoredBox::try_from(AnchoredDirectionArrows::new(
        ax.trans_axes(),
        "X",
        "Y",
    ))?);
    Ok(())
}

fn anchored_direction_arrows_many_args(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    show(&ax, Grid::from_fn(10, 10, |_, _| 1.0), None)?;
    ax.add_anchored_box(AnchoredBox::try_from(AnchoredDirectionArrows {
        loc: AnchorLoc::UpperRight,
        color: "red".into(),
        aspect_ratio: -0.5,
        pad: 0.6,
        borderpad: 2.0,
        frameon: true,
        alpha: 0.7,
        sep_x: -0.06,
        sep_y: -0.08,
        back_length: 0.1,
        head_width: 9.0,
        head_length: 10.0,
        tail_width: 5.0,
        ..AnchoredDirectionArrows::new(ax.trans_axes(), "A", "B")
    })?);
    Ok(())
}

/// `np.arange(100).reshape((10, 10))`.
fn arange_10x10() -> Grid<f64> {
    Grid::from_fn(10, 10, |r, c| (r * 10 + c) as f64)
}

/// The one cell of a 1 × 1 subplot grid (`111` or `(1, 1, 1)`).
fn whole_subplot() -> Result<GridRect, GridSpecError> {
    Ok(GridRect::Subplot(GridSpec::new(1, 1)?.cell(0, 0)?))
}

fn image_grid_each_left_label_mode_all(fig: &Figure) -> CaseResult {
    let options = ImageGridOptions {
        axes_pad: (0.5, 0.3),
        cbar_mode: Some(CbarMode::Each),
        cbar_location: SpineSide::Left,
        cbar_size: SizeSpec::Percent(15.0),
        label_mode: LabelMode::All,
        ..ImageGridOptions::default()
    };
    let grid = ImageGrid::new(fig, &whole_subplot()?, (3, 2), &options)?;
    for (ax, cax) in grid.axes().iter().zip(grid.cbar_axes()) {
        let image = ax.imshow(
            arange_10x10(),
            ImshowOptions {
                interpolation: Some(Interpolation::None),
                ..ImshowOptions::default()
            },
        )?;
        cax.colorbar(&image, ColorbarOptions::default())?;
    }
    Ok(())
}

fn image_grid_single_bottom(fig: &Figure) -> CaseResult {
    let options = ImageGridOptions {
        axes_pad: (0.2, 0.15),
        cbar_mode: Some(CbarMode::Single),
        cbar_pad: Some(0.3),
        cbar_location: SpineSide::Bottom,
        cbar_size: SizeSpec::Percent(10.0),
        label_mode: LabelMode::One,
        ..ImageGridOptions::default()
    };
    let grid = ImageGrid::new(fig, &GridRect::Rect([0.0, 0.0, 1.0, 1.0]), (1, 3), &options)?;
    let mut last = None;
    for ax in grid.axes() {
        last = Some(ax.imshow(
            arange_10x10(),
            ImshowOptions {
                interpolation: Some(Interpolation::None),
                ..ImshowOptions::default()
            },
        )?);
    }
    let image = last.ok_or("the grid has axes")?;
    grid.cbar_axes()[0].colorbar(&image, ColorbarOptions::default())?;
    Ok(())
}

fn image_grid(fig: &Figure) -> CaseResult {
    let options = ImageGridOptions {
        axes_pad: (0.1, 0.1),
        ..ImageGridOptions::default()
    };
    let grid = ImageGrid::new(fig, &whole_subplot()?, (2, 2), &options)?;
    for ax in grid.axes() {
        show(ax, arange_10x10(), Some(Interpolation::Nearest))?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn anchored_artists(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let area = AnchoredDrawingArea {
        pad: 0.0,
        frameon: false,
        ..AnchoredDrawingArea::new(40.0, 20.0, 0.0, 0.0, AnchorLoc::UpperRight)
    }
    .with_artist(BoxItem::patch(
        Circle::new((10.0, 10.0), 10.0),
        PatchProps::default(),
    ))
    .with_artist(BoxItem::patch(
        Circle::new((30.0, 10.0), 5.0),
        PatchProps::default().facecolor("r"),
    ));
    ax.add_anchored_box(area);
    let ellipse = Ellipse {
        angle: 30.0,
        ..Ellipse::new((0.0, 0.0), 0.1, 0.4)
    };
    let aux = AnchoredAuxTransformBox::new(ax.trans_data(), AnchorLoc::UpperLeft)
        .with_artist(BoxItem::patch(ellipse, PatchProps::default().color("cyan")));
    ax.add_anchored_box(aux);
    let rc = plt::rc::params();
    ax.add_anchored_box(AnchoredSizeBar {
        pad: 0.3,
        borderpad: 0.4,
        sep: 4.0,
        fill_bar: Some(true),
        frameon: false,
        label_top: true,
        fontproperties: Some(
            FontProperties::from_rc(&rc).with_size(FontSize::Points(20.0), rc.font_size),
        ),
        size_vertical: 0.05,
        color: "green".into(),
        ..AnchoredSizeBar::new(ax.trans_data(), 0.2, "0.2 units", AnchorLoc::LowerRight)
    });
    remove_ticks_and_titles(fig);
    Ok(())
}

fn rgb_axes(fig: &Figure) -> CaseResult {
    let axes = RgbAxes::new(fig, [0.1, 0.1, 0.8, 0.8], 0.1);
    let mut rng = SpecRng::new(19_680_801);
    let mut channel = || {
        let values = rng.uniform(25, 0.0, 1.0);
        Grid::from_fn(5, 5, |r, c| values[r * 5 + c])
    };
    let (red, green, blue) = (channel(), channel(), channel());
    axes.imshow_rgb(&red, &green, &blue, || ImshowOptions {
        interpolation: Some(Interpolation::None),
        ..ImshowOptions::default()
    })?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// The images of each grid of `test_imagegrid_cbar_mode_edge`, with their
/// colormaps: `arr`, `arr.T`, `np.hypot(arr, arr.T)` and
/// `np.arctan2(arr, arr.T)` for `arr = np.arange(16).reshape((4, 4))`.
fn edge_images() -> [(Grid<f64>, &'static str); 4] {
    let arr = |r: usize, c: usize| (r * 4 + c) as f64;
    [
        (Grid::from_fn(4, 4, arr), "nipy_spectral"),
        (Grid::from_fn(4, 4, |r, c| arr(c, r)), "hot"),
        (
            Grid::from_fn(4, 4, |r, c| arr(r, c).hypot(arr(c, r))),
            "jet",
        ),
        (
            Grid::from_fn(4, 4, |r, c| arr(r, c).atan2(arr(c, r))),
            "hsv",
        ),
    ]
}

fn imagegrid_cbar_mode_edge(fig: &Figure) -> CaseResult {
    let locations = [
        SpineSide::Left,
        SpineSide::Right,
        SpineSide::Top,
        SpineSide::Bottom,
    ];
    let layouts = [GridDirection::Row, GridDirection::Column]
        .into_iter()
        .flat_map(|direction| locations.map(|location| (direction, location)));
    for (position, (direction, cbar_location)) in (1..).zip(layouts) {
        let options = ImageGridOptions {
            direction,
            cbar_location,
            cbar_size: SizeSpec::Percent(20.0),
            cbar_mode: Some(CbarMode::Edge),
            ..ImageGridOptions::default()
        };
        let rect = GridRect::Subplot(GridSpec::new(2, 4)?.numbered(position)?);
        let grid = ImageGrid::new(fig, &rect, (2, 2), &options)?;
        let mut images = Vec::new();
        for (ax, (data, cmap)) in grid.axes().iter().zip(edge_images()) {
            images.push(ax.imshow(
                data,
                ImshowOptions {
                    colors: ColorizerOptions::default().cmap(colormaps::get(cmap)?),
                    ..ImshowOptions::default()
                },
            )?);
        }
        // Each colorbar axes serves two axes: the second colorbar drawn
        // into it replaces the first.
        for (index, image) in images.iter().enumerate() {
            let cax = grid.cax(index).ok_or("every axes has a colorbar axes")?;
            cax.ax().clear();
            cax.colorbar(image, ColorbarOptions::default())?;
        }
    }
    remove_ticks_and_titles(fig);
    Ok(())
}
