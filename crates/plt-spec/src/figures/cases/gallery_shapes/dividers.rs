//! Axes placed by dividers: fixed and scaled cells, cells sized by the
//! data, box dividers, axes appended beside others and room made for
//! decorations (`simple_axes_divider1`, `simple_axes_divider3`,
//! `demo_fixed_size_axes`, `demo_axes_divider`, `demo_axes_hbox_divider`,
//! `demo_colorbar_with_axes_divider`, `scatter_hist_locatable_axes` and
//! `make_room_for_ylabel_using_axesgrid`'s first figure).

use plt::axes::{HistPlotOptions, ImshowOptions, TickParams, TickPosition};
use plt::axes_grid1::{
    Divider, Size, SizeSpec, make_axes_area_auto_adjustable, make_axes_locatable,
};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::image::Extent;
use plt::numeric::Grid;
use plt::orientation::Orientation;
use plt::prelude::*;
use plt::rc::Aspect;
use plt::spines::SpineSide;
use plt::stats::{Bins, HistOptions, HistOrientation};
use plt::text::{HAlign, VAlign};

use super::{EXTENT, SEED};
use crate::figures::cases::common::{arange_grid, bivariate_normal};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The rectangle the simple dividers split.
const RECT: [f64; 4] = [0.1, 0.1, 0.8, 0.8];

/// `label_axes`: centred text and neither ticks nor tick labels.
fn label_cell(ax: &Axes, text: &str) {
    ax.text(0.5, 0.5, text)
        .set_transform(ax.trans_axes())
        .set_halign(HAlign::Center)
        .set_valign(VAlign::Center);
    ax.tick_params(&TickParams {
        bottom: Some(false),
        labelbottom: Some(false),
        left: Some(false),
        labelleft: Some(false),
        ..TickParams::default()
    });
}

/// `simple_axes_divider1`'s four labelled axes in the cells of a divider.
fn four_located_axes(fig: &Figure, horizontal: Vec<Size>, vertical: Vec<Size>) -> CaseResult {
    let divider = Divider::new(fig, RECT, horizontal, vertical, false);
    for (columns, ny, text) in [
        (0..1, 0, "nx=0, ny=0"),
        (0..1, 2, "nx=0, ny=2"),
        (2..3, 2, "nx=2, ny=2"),
        (2..4, 0, "nx=2, nx1=4, ny=0"),
    ] {
        let ax = fig.add_axes(RECT)?;
        divider.place_spanning(&ax, columns, ny..ny + 1)?;
        label_cell(&ax, text);
    }
    Ok(())
}

fn simple_axes_divider1_1(fig: &Figure) -> CaseResult {
    fig.suptitle("Fixed axes sizes, fixed paddings");
    four_located_axes(
        fig,
        [1.0, 0.5, 1.5, 0.5].map(Size::Fixed).to_vec(),
        [1.5, 0.5, 1.0].map(Size::Fixed).to_vec(),
    )
}

fn simple_axes_divider1_2(fig: &Figure) -> CaseResult {
    fig.suptitle("Scalable axes sizes, fixed paddings");
    four_located_axes(
        fig,
        vec![
            Size::Scaled(1.5),
            Size::Fixed(0.5),
            Size::Scaled(1.0),
            Size::Scaled(0.5),
        ],
        vec![Size::Scaled(1.0), Size::Fixed(0.5), Size::Scaled(1.5)],
    )
}

fn simple_axes_divider3(fig: &Figure) -> CaseResult {
    let ax = (0..4)
        .map(|_| fig.add_axes(RECT))
        .collect::<Result<Vec<_>, _>>()?;
    let horizontal = vec![
        Size::axes_x(ax[0].id()),
        Size::Fixed(0.5),
        Size::axes_x(ax[1].id()),
    ];
    let vertical = vec![
        Size::axes_y(ax[0].id()),
        Size::Fixed(0.5),
        Size::axes_y(ax[2].id()),
    ];
    let divider = Divider::new(fig, RECT, horizontal, vertical, false);
    for (axes, (nx, ny)) in ax.iter().zip([(0, 0), (2, 0), (0, 2), (2, 2)]) {
        divider.place(axes, nx, ny)?;
    }
    ax[0].set_xlim(0.0, 2.0)?;
    ax[1].set_xlim(0.0, 1.0)?;
    ax[0].set_ylim(0.0, 1.0)?;
    ax[2].set_ylim(0.0, 2.0)?;
    divider.set_aspect(true);
    for axes in &ax {
        axes.tick_params(&TickParams {
            labelbottom: Some(false),
            labelleft: Some(false),
            ..TickParams::default()
        });
    }
    Ok(())
}

/// `demo_fixed_size_axes`: a line plot in the middle cell.
fn fixed_size_axes(fig: &Figure, horizontal: Vec<Size>, vertical: Vec<Size>) -> CaseResult {
    let divider = Divider::new(fig, [0.0, 0.0, 1.0, 1.0], horizontal, vertical, false);
    let (x, y, w, h) = divider.position().bounds();
    let ax = fig.add_axes([x, y, w, h])?;
    divider.place(&ax, 1, 1)?;
    ax.plot_y([1.0, 2.0, 3.0]);
    Ok(())
}

fn demo_fixed_size_axes_1(fig: &Figure) -> CaseResult {
    fixed_size_axes(
        fig,
        vec![Size::Fixed(1.0), Size::Fixed(4.5)],
        vec![Size::Fixed(0.7), Size::Fixed(5.0)],
    )
}

fn demo_fixed_size_axes_2(fig: &Figure) -> CaseResult {
    fixed_size_axes(
        fig,
        vec![Size::Fixed(1.0), Size::Scaled(1.0), Size::Fixed(0.2)],
        vec![Size::Fixed(0.7), Size::Scaled(1.0), Size::Fixed(0.5)],
    )
}

/// The sample image over the examples' extent.
fn sample_image(ax: &Axes) -> Result<plt::image::AxesImage, Box<dyn std::error::Error>> {
    let [left, right, bottom, top] = EXTENT;
    let options = ImshowOptions {
        extent: Some(Extent::new(left, right, bottom, top)),
        ..ImshowOptions::default()
    };
    Ok(ax.imshow(bivariate_normal(), options)?)
}

/// No tick labels on the right of `axis`.
fn no_right_labels(axis: &plt::axes::Axis) {
    axis.set_tick_params(&TickParams {
        labelright: Some(false),
        ..TickParams::default()
    });
}

/// A colorbar of `image` drawn into `cax`.
fn colorbar_in(fig: &Figure, image: &plt::image::AxesImage, cax: &Axes) -> CaseResult {
    let options = ColorbarOptions {
        axes: ColorbarAxes::Cax(cax.clone()),
        ..ColorbarOptions::default()
    };
    fig.colorbar(image, options)?;
    Ok(())
}

fn demo_axes_divider(fig: &Figure) -> CaseResult {
    let ax = fig.add_subplot(2, 2, 1)?;
    let image = sample_image(&ax)?;
    let colorbar = fig.colorbar(&image, ColorbarOptions::default())?;
    no_right_labels(&colorbar.ax().yaxis());

    let spec = GridSpec::new(2, 2)?.numbered(2)?;
    let divider = Divider::subplot(fig, &spec, Vec::new(), Vec::new(), true);
    let ax = fig.add_subplot(1, 1, 1)?;
    divider.place(&ax, 0, 0)?;
    let ax_cb = fig.add_subplot(1, 1, 1)?;
    divider.place(&ax_cb, 2, 0)?;
    divider.set_horizontal(vec![
        Size::axes_x(ax.id()),
        Size::Fixed(0.05),
        Size::Fixed(0.2),
    ]);
    divider.set_vertical(vec![Size::axes_y(ax.id())]);
    let image = sample_image(&ax)?;
    colorbar_in(fig, &image, &ax_cb)?;
    no_right_labels(&ax_cb.yaxis());

    let ax = fig.add_subplot(2, 2, 3)?;
    let ax_cb = make_axes_locatable(&ax).append_axes(
        SpineSide::Right,
        SizeSpec::Percent(5.0),
        Some(0.05.into()),
    );
    let image = sample_image(&ax)?;
    colorbar_in(fig, &image, &ax_cb)?;
    ax_cb
        .yaxis()
        .set_ticks_position(TickPosition::Side(SpineSide::Right))?;
    no_right_labels(&ax_cb.yaxis());

    let ax = fig.add_subplot(2, 2, 4)?;
    let ax2 = make_axes_locatable(&ax).append_axes(
        SpineSide::Right,
        SizeSpec::Percent(100.0),
        Some(0.05.into()),
    );
    sample_image(&ax)?;
    sample_image(&ax2)?;
    ax2.yaxis().set_tick_params(&TickParams {
        labelleft: Some(false),
        ..TickParams::default()
    });
    Ok(())
}

/// `demo_axes_hbox_divider`'s two images, placed by a box divider of the
/// figure's 111 cell made by `boxed` from their sizes.
fn boxed_images(
    fig: &Figure,
    grid: (usize, usize),
    boxed: impl FnOnce(&Axes, &Axes) -> Result<Divider, Box<dyn std::error::Error>>,
    second_cell: (isize, isize),
) -> CaseResult {
    let axs = fig.subplots(grid.0, grid.1)?;
    let (ax1, ax2) = (&axs.as_slice()[0], &axs.as_slice()[1]);
    ax1.imshow(arange_grid(4, 5), ImshowOptions::default())?;
    ax2.imshow(arange_grid(5, 4), ImshowOptions::default())?;
    let divider = boxed(ax1, ax2)?;
    divider.place(ax1, 0, 0)?;
    divider.place(ax2, second_cell.0, second_cell.1)?;
    Ok(())
}

fn demo_axes_hbox_divider_1(fig: &Figure) -> CaseResult {
    boxed_images(
        fig,
        (1, 2),
        |ax1, ax2| {
            let horizontal = vec![
                Size::axes_x(ax1.id()),
                Size::Fixed(0.5),
                Size::axes_x(ax2.id()),
            ];
            let vertical = vec![
                Size::axes_y(ax1.id()),
                Size::Scaled(1.0),
                Size::axes_y(ax2.id()),
            ];
            Ok(Divider::hbox(
                fig,
                &GridSpec::new(1, 1)?.numbered(1)?,
                horizontal,
                vertical,
            ))
        },
        (2, 0),
    )
}

fn demo_axes_hbox_divider_2(fig: &Figure) -> CaseResult {
    boxed_images(
        fig,
        (2, 1),
        |ax1, ax2| {
            let horizontal = vec![
                Size::axes_x(ax1.id()),
                Size::Scaled(1.0),
                Size::axes_x(ax2.id()),
            ];
            let vertical = vec![
                Size::axes_y(ax1.id()),
                Size::Fixed(0.5),
                Size::axes_y(ax2.id()),
            ];
            Ok(Divider::vbox(
                fig,
                &GridSpec::new(1, 1)?.numbered(1)?,
                horizontal,
                vertical,
            ))
        },
        (0, 2),
    )
}

fn demo_colorbar_with_axes_divider(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    fig.subplots_adjust(&SubplotOverrides {
        wspace: Some(0.5),
        ..SubplotOverrides::default()
    })?;
    let data = || Grid::from_fn(2, 2, |r, c| (r * 2 + c + 1) as f64);
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    let image1 = ax1.imshow(data(), ImshowOptions::default())?;
    let cax1 = make_axes_locatable(ax1).append_axes(
        SpineSide::Right,
        SizeSpec::Percent(7.0),
        Some(SizeSpec::Percent(2.0)),
    );
    colorbar_in(fig, &image1, &cax1)?;
    let image2 = ax2.imshow(data(), ImshowOptions::default())?;
    let cax2 = make_axes_locatable(ax2).append_axes(
        SpineSide::Top,
        SizeSpec::Percent(7.0),
        Some(SizeSpec::Percent(2.0)),
    );
    let horizontal = ColorbarOptions {
        axes: ColorbarAxes::Cax(cax2.clone()),
        orientation: Some(Orientation::Horizontal),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&image2, horizontal)?;
    cax2.xaxis()
        .set_ticks_position(TickPosition::Side(SpineSide::Top))?;
    Ok(())
}

fn scatter_hist_locatable_axes(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let x = rng.normal(1000, 0.0, 1.0);
    let y = rng.normal(1000, 0.0, 1.0);
    let ax = fig.subplot();
    ax.scatter(x.clone(), y.clone())?;
    ax.set_aspect(Aspect::Ratio(1.0))?;
    let divider = make_axes_locatable(&ax);
    let ax_histx = divider.append_axes(SpineSide::Top, 1.2, Some(0.1.into()));
    ax_histx.share_x(&ax)?;
    let ax_histy = divider.append_axes(SpineSide::Right, 1.2, Some(0.1.into()));
    ax_histy.share_y(&ax)?;
    ax_histx.xaxis().set_tick_params(&TickParams {
        labelbottom: Some(false),
        ..TickParams::default()
    });
    ax_histy.yaxis().set_tick_params(&TickParams {
        labelleft: Some(false),
        ..TickParams::default()
    });
    let binwidth = 0.25;
    let xymax = x.iter().chain(&y).fold(0.0_f64, |m, v| m.max(v.abs()));
    let lim = (f64::from((xymax / binwidth) as i32) + 1.0) * binwidth;
    let bins = arange(-lim, lim + binwidth, binwidth);
    let hist = |orientation| HistPlotOptions {
        layout: HistOptions {
            bins: Some(Bins::Edges(bins.clone())),
            orientation,
            ..HistOptions::default()
        },
        ..HistPlotOptions::default()
    };
    ax_histx.hist(&[&x], hist(HistOrientation::Vertical))?;
    ax_histy.hist(&[&y], hist(HistOrientation::Horizontal))?;
    ax_histx.set_yticks(vec![0.0, 50.0, 100.0]);
    ax_histy.set_xticks(vec![0.0, 50.0, 100.0]);
    Ok(())
}

/// One tick at 0.5 labelled `very long label` on the y axis.
fn long_label(ax: &Axes) -> CaseResult {
    ax.set_yticks(vec![0.5])
        .set_yticklabels(["very long label"])?;
    Ok(())
}

fn make_room_for_ylabel_using_axesgrid_1(fig: &Figure) -> CaseResult {
    let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0])?;
    long_label(&ax)?;
    make_axes_area_auto_adjustable(&ax, &[&ax], 0.1, &SpineSide::ALL);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "gallery_shapes/simple_axes_divider1_1",
        simple_axes_divider1_1,
    )
    .sized(6.0, 6.0, 100.0),
    FigureCase::new(
        "gallery_shapes/simple_axes_divider1_2",
        simple_axes_divider1_2,
    )
    .sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_shapes/simple_axes_divider3", simple_axes_divider3)
        .sized(5.5, 4.0, 100.0),
    FigureCase::new(
        "gallery_shapes/demo_fixed_size_axes_1",
        demo_fixed_size_axes_1,
    )
    .sized(6.0, 6.0, 100.0),
    FigureCase::new(
        "gallery_shapes/demo_fixed_size_axes_2",
        demo_fixed_size_axes_2,
    )
    .sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_shapes/demo_axes_divider", demo_axes_divider).sized(6.0, 6.0, 100.0),
    FigureCase::new(
        "gallery_shapes/demo_axes_hbox_divider_1",
        demo_axes_hbox_divider_1,
    ),
    FigureCase::new(
        "gallery_shapes/demo_axes_hbox_divider_2",
        demo_axes_hbox_divider_2,
    ),
    FigureCase::new(
        "gallery_shapes/demo_colorbar_with_axes_divider",
        demo_colorbar_with_axes_divider,
    ),
    FigureCase::new(
        "gallery_shapes/scatter_hist_locatable_axes",
        scatter_hist_locatable_axes,
    )
    .sized(5.5, 5.5, 100.0),
    FigureCase::new(
        "gallery_shapes/make_room_for_ylabel_using_axesgrid_1",
        make_room_for_ylabel_using_axesgrid_1,
    ),
];
