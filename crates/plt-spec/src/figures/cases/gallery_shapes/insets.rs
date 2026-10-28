//! Inset axes anchored in their parents: colorbars in insets, insets
//! sized in inches or percentages and anchored in boxes of various
//! coordinate systems, and zoomed insets with size bars and marks
//! (`demo_colorbar_with_inset_locator`, `inset_locator_demo` and
//! `inset_locator_demo2`).

use plt::axes::{ImshowOptions, LocatorParams, TickParams};
use plt::axes_grid1::anchored_artists::AnchoredSizeBar;
use plt::axes_grid1::{Corner, InsetOptions, SizeSpec, inset_axes, mark_inset, zoomed_inset_axes};
use plt::colorbar::{ColorbarAxes, ColorbarOptions, ColorbarTicks};
use plt::image::{Extent, Origin};
use plt::numeric::Grid;
use plt::offsetbox::{AnchorLoc, BboxToAnchor};
use plt::orientation::Orientation;
use plt::patches::Rectangle;
use plt::prelude::*;
use plt::rc::Aspect;
use plt::spines::SpineSide;
use plt::ticker::Nbins;
use plt::transforms::Transform;

use super::EXTENT;
use crate::figures::cases::common::framed_bivariate_normal;
use crate::figures::{CaseResult, FigureCase};

/// `[[1, 2], [2, 3]]`.
fn ramp() -> Grid<f64> {
    Grid::from_fn(2, 2, |r, c| (r + c + 1) as f64)
}

/// A colorbar of `image` in `cax` with ticks at 1, 2 and 3.
fn ticked_colorbar(
    fig: &Figure,
    image: &plt::image::AxesImage,
    cax: &Axes,
    orientation: Orientation,
) -> CaseResult {
    let options = ColorbarOptions {
        axes: ColorbarAxes::Cax(cax.clone()),
        orientation: Some(orientation),
        ticks: Some(ColorbarTicks::Values(vec![1.0, 2.0, 3.0])),
        ..ColorbarOptions::default()
    };
    fig.colorbar(image, options)?;
    Ok(())
}

fn demo_colorbar_with_inset_locator(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    let image1 = ax1.imshow(ramp(), ImshowOptions::default())?;
    let axins1 = inset_axes(
        ax1,
        SizeSpec::Percent(50.0),
        SizeSpec::Percent(5.0),
        InsetOptions::default(),
    );
    axins1
        .xaxis()
        .set_ticks_position(plt::axes::TickPosition::Side(SpineSide::Bottom))?;
    ticked_colorbar(fig, &image1, &axins1, Orientation::Horizontal)?;
    let image2 = ax2.imshow(ramp(), ImshowOptions::default())?;
    let beside = InsetOptions {
        loc: AnchorLoc::LowerLeft,
        bbox_to_anchor: Some(BboxToAnchor::Bounds(1.05, 0.0, 1.0, 1.0)),
        bbox_transform: Some(ax2.trans_axes()),
        borderpad: (0.0, 0.0),
    };
    let axins = inset_axes(ax2, SizeSpec::Percent(5.0), SizeSpec::Percent(50.0), beside);
    ticked_colorbar(fig, &image2, &axins, Orientation::Vertical)?;
    Ok(())
}

/// No tick labels on the left or at the bottom.
fn hide_tick_labels(axes: &[&Axes]) {
    for ax in axes {
        ax.tick_params(TickParams {
            labelleft: Some(false),
            labelbottom: Some(false),
            ..TickParams::default()
        });
    }
}

/// Inset options anchored at `loc` with `borderpad` font sizes of padding.
fn at(loc: AnchorLoc, borderpad: f64) -> InsetOptions {
    InsetOptions {
        loc,
        borderpad: (borderpad, borderpad),
        ..InsetOptions::default()
    }
}

/// Inset options anchored at `loc` of `bbox` in `transform` coordinates,
/// without padding unless `borderpad` says otherwise.
fn anchored(
    loc: AnchorLoc,
    bbox: BboxToAnchor,
    transform: Transform,
    borderpad: f64,
) -> InsetOptions {
    InsetOptions {
        bbox_to_anchor: Some(bbox),
        bbox_transform: Some(transform),
        ..at(loc, borderpad)
    }
}

fn inset_locator_demo_1(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    let percent = SizeSpec::Percent;
    hide_tick_labels(&[
        &inset_axes(ax, 1.3.into(), 0.9.into(), InsetOptions::default()),
        &inset_axes(
            ax,
            percent(30.0),
            percent(40.0),
            at(AnchorLoc::LowerLeft, 0.5),
        ),
        &inset_axes(
            ax2,
            percent(30.0),
            1.0.into(),
            at(AnchorLoc::UpperLeft, 0.5),
        ),
        &inset_axes(
            ax2,
            percent(20.0),
            percent(20.0),
            at(AnchorLoc::LowerRight, 1.0),
        ),
    ]);
    Ok(())
}

/// A dashed cyan outline of `width` points around `rect`, placed by
/// `transform` (data coordinates when `None`).
fn dashed_outline(ax: &Axes, rect: Rectangle, width: Option<f64>, transform: Option<Transform>) {
    let mut props = PatchProps::default()
        .linestyle(LineStyle::Dashed)
        .edgecolor("c")
        .facecolor("none");
    if let Some(width) = width {
        props = props.linewidth(width);
    }
    if let Some(transform) = transform {
        props = props.transform(transform);
    }
    ax.add_patch_with(rect, props);
}

fn inset_locator_demo_2(fig: &Figure) -> CaseResult {
    let ax = fig.add_subplot(1, 2, 1)?;
    let box_in_axes = BboxToAnchor::Bounds(0.2, 0.4, 0.6, 0.5);
    let options = anchored(AnchorLoc::LowerLeft, box_in_axes, ax.trans_axes(), 0.5);
    inset_axes(
        &ax,
        SizeSpec::Percent(50.0),
        SizeSpec::Percent(75.0),
        options,
    );
    let outline = Rectangle::new((0.2, 0.4), 0.6, 0.5);
    dashed_outline(&ax, outline, None, Some(ax.trans_axes()));
    ax.set_xlim(0.0, 10.0)?.set_ylim(0.0, 10.0)?;
    let ax2 = fig.add_subplot(2, 2, 2)?;
    let axins2 = inset_axes(
        &ax2,
        SizeSpec::Percent(30.0),
        SizeSpec::Percent(50.0),
        InsetOptions::default(),
    );
    let ax3 = fig.add_subplot(2, 2, 4)?;
    let corner = BboxToAnchor::Bounds(0.7, 0.5, 0.3, 0.5);
    let full = SizeSpec::Percent(100.0);
    let options = anchored(AnchorLoc::UpperRight, corner, ax3.trans_axes(), 0.5);
    let axins3 = inset_axes(&ax3, full, full, options);
    dashed_outline(&ax2, Rectangle::new((0.0, 0.0), 1.0, 1.0), Some(2.0), None);
    dashed_outline(&ax3, Rectangle::new((0.7, 0.5), 0.3, 0.5), Some(2.0), None);
    hide_tick_labels(&[&axins2, &axins3, &ax2, &ax3]);
    Ok(())
}

fn inset_locator_demo_3(fig: &Figure) -> CaseResult {
    let ax = fig.add_subplot(1, 3, 1)?;
    let full = SizeSpec::Percent(100.0);
    let beside = BboxToAnchor::Bounds(1.05, 0.6, 0.5, 0.4);
    let options = anchored(AnchorLoc::UpperLeft, beside, ax.trans_axes(), 0.0);
    let axins = inset_axes(&ax, full, full, options);
    axins.tick_params(TickParams {
        left: Some(false),
        right: Some(true),
        labelleft: Some(false),
        labelright: Some(true),
        ..TickParams::default()
    });
    let point = BboxToAnchor::Point(0.33, 0.25);
    let options = anchored(AnchorLoc::LowerLeft, point, ax.trans_axes(), 0.0);
    inset_axes(&ax, 0.5.into(), 0.4.into(), options);
    let ax2 = fig.add_subplot(1, 3, 3)?;
    ax2.set_xscale("log")?;
    ax2.set_xlim(1e-6, 1e6)?.set_ylim(-2.0, 6.0)?;
    let in_data = BboxToAnchor::Bounds(1e-2, 2.0, 1e3, 3.0);
    let options = anchored(AnchorLoc::UpperLeft, in_data, ax2.trans_data(), 0.0);
    inset_axes(&ax2, full, full, options);
    let blended = Transform::blended(Transform::Figure(fig.id()), ax2.trans_axes());
    let whole = BboxToAnchor::Bounds(0.0, 0.0, 1.0, 1.0);
    let options = anchored(AnchorLoc::LowerCenter, whole, blended, 0.0);
    inset_axes(
        &ax2,
        SizeSpec::Percent(16.0),
        SizeSpec::Percent(34.0),
        options,
    );
    Ok(())
}

/// A frameless scale bar `size` data units long under the axes' middle.
fn size_bar(ax: &Axes, size: f64, label: &str) {
    ax.add_anchored_box(AnchoredSizeBar {
        pad: 0.1,
        borderpad: 0.5,
        sep: 5.0,
        frameon: false,
        ..AnchoredSizeBar::new(ax.trans_data(), size, label, AnchorLoc::LowerCenter)
    });
}

/// At most seven ticks per axis, without labels (set on the locators
/// themselves, which leaves the view limits alone).
fn seven_bins(axins: &Axes) -> CaseResult {
    let seven = LocatorParams {
        nbins: Some(Nbins::Fixed(7)),
        ..LocatorParams::default()
    };
    axins.yaxis().set_major_locator_params(seven.clone())?;
    axins.xaxis().set_major_locator_params(seven)?;
    hide_tick_labels(&[axins]);
    Ok(())
}

fn inset_locator_demo2(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    ax.set_aspect(Aspect::Ratio(1.0))?;
    let axins = zoomed_inset_axes(ax, 0.5, InsetOptions::default());
    seven_bins(&axins)?;
    size_bar(ax, 0.5, "0.5");
    size_bar(&axins, 0.5, "0.5");
    let framed = framed_bivariate_normal();
    let [left, right, bottom, top] = EXTENT;
    let options = || ImshowOptions {
        extent: Some(Extent::new(left, right, bottom, top)),
        origin: Some(Origin::Lower),
        ..ImshowOptions::default()
    };
    ax2.imshow(framed.clone(), options())?;
    let axins2 = zoomed_inset_axes(ax2, 6.0, InsetOptions::default());
    axins2.imshow(framed, options())?;
    axins2.set_xlim(-1.5, -0.9)?.set_ylim(-2.5, -1.9)?;
    seven_bins(&axins2)?;
    let marks = PatchProps::default().facecolor("none").edgecolor("0.5");
    mark_inset(ax2, &axins2, Corner::UpperLeft, Corner::LowerRight, marks);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "gallery_shapes/demo_colorbar_with_inset_locator",
        demo_colorbar_with_inset_locator,
    )
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("gallery_shapes/inset_locator_demo_1", inset_locator_demo_1)
        .sized(5.5, 2.8, 100.0),
    FigureCase::new("gallery_shapes/inset_locator_demo_2", inset_locator_demo_2)
        .sized(5.5, 2.8, 100.0),
    FigureCase::new("gallery_shapes/inset_locator_demo_3", inset_locator_demo_3)
        .sized(5.5, 2.8, 100.0),
    FigureCase::new("gallery_shapes/inset_locator_demo2", inset_locator_demo2)
        .sized(6.0, 3.0, 100.0),
];
