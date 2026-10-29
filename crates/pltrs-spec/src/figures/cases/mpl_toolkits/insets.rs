//! The inset tests of `test_axes_grid1.py`: zoomed and sized inset axes,
//! the marks joining them to their parents, and bbox connector patches.

use pltrs::axes::{Axes, ImshowOptions, LocatorParams, TickParams};
use pltrs::axes_grid1::anchored_artists::AnchoredSizeBar;
use pltrs::axes_grid1::{
    BboxConnectorPatch, BboxSource, Corner, InsetOptions, inset_axes, mark_inset, zoomed_inset_axes,
};
use pltrs::figure::{Figure, SubfigureOptions};
use pltrs::image::{Extent, Interpolation, Origin};
use pltrs::numeric::Grid;
use pltrs::offsetbox::{AnchorLoc, BboxToAnchor};
use pltrs::patches::PatchProps;
use pltrs::prelude::ArtistHandle;
use pltrs::rc::Aspect;
use pltrs::spines::SpineSide;
use pltrs::ticker::Nbins;
use pltrs_core::Bbox;

use super::remove_ticks_and_titles;
use crate::figures::cases::common::{bivariate_normal, framed_bivariate_normal};
use crate::figures::{CaseResult, FigureCase};

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_toolkits/inset_locator", inset_locator).sized(5.0, 4.0, 100.0),
    FigureCase::new("mpl_toolkits/inset_axes", inset_axes_demo).sized(5.0, 4.0, 100.0),
    FigureCase::new("mpl_toolkits/fill_facecolor", fill_facecolor).sized(5.0, 5.0, 100.0),
    FigureCase::new(
        "mpl_toolkits/zooming_with_inverted_axes_1",
        zooming_with_inverted_axes_1,
    ),
    FigureCase::new(
        "mpl_toolkits/zooming_with_inverted_axes_2",
        zooming_with_inverted_axes_2,
    ),
    super::classic(
        "mpl_toolkits/mark_inset_unstales_viewlim",
        mark_inset_unstales_viewlim,
    ),
    FigureCase::new(
        "mpl_toolkits/anchored_locator_base_call",
        anchored_locator_base_call,
    )
    .sized(3.0, 3.0, 100.0),
];

/// The extent of the sample image.
const EXTENT: Extent = Extent::new(-3.0, 4.0, -4.0, 3.0);

/// `imshow(z2, extent=extent, interpolation="nearest", origin="lower")`
/// of the zoom tests.
fn sample_image(ax: &Axes, image: &Grid<f64>) -> CaseResult {
    ax.imshow(
        image.clone(),
        ImshowOptions {
            extent: Some(EXTENT),
            interpolation: Some(Interpolation::Nearest),
            origin: Some(Origin::Lower),
            ..ImshowOptions::default()
        },
    )?;
    Ok(())
}

/// `test_inset_locator` and `test_inset_axes`: the sample image framed by
/// zeros, an inset (made by `make_inset`) showing a part of it, marked in
/// the parent, and a size bar.
fn zoomed_demo(fig: &Figure, make_inset: fn(&Axes) -> Axes) -> CaseResult {
    let ax = fig.subplot();
    let framed = framed_bivariate_normal();
    sample_image(&ax, &framed)?;
    let axins = make_inset(&ax);
    sample_image(&axins, &framed)?;
    axins.locator_params(LocatorParams {
        nbins: Some(Nbins::Fixed(7)),
        ..LocatorParams::default()
    })?;
    axins.set_xlim(-1.5, -0.9)?.set_ylim(-2.5, -1.9)?;
    axins.tick_params(TickParams {
        labelbottom: Some(false),
        labelleft: Some(false),
        ..TickParams::default()
    });
    mark_inset(
        &ax,
        &axins,
        Corner::UpperLeft,
        Corner::LowerRight,
        PatchProps::default().facecolor("none").edgecolor("0.5"),
    );
    ax.add_anchored_box(AnchoredSizeBar {
        pad: 0.1,
        borderpad: 0.5,
        sep: 5.0,
        frameon: false,
        ..AnchoredSizeBar::new(ax.trans_data(), 0.5, "0.5", AnchorLoc::LowerCenter)
    });
    remove_ticks_and_titles(fig);
    Ok(())
}

fn inset_locator(fig: &Figure) -> CaseResult {
    zoomed_demo(fig, |ax| {
        zoomed_inset_axes(ax, 6.0, InsetOptions::default())
    })
}

fn inset_axes_demo(fig: &Figure) -> CaseResult {
    zoomed_demo(fig, |ax| {
        let options = InsetOptions {
            bbox_to_anchor: Some(BboxToAnchor::Point(1.0, 1.0)),
            bbox_transform: Some(ax.trans_axes()),
            ..InsetOptions::default()
        };
        inset_axes(ax, 1.0.into(), 1.0.into(), options)
    })
}

fn fill_facecolor(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(1, 5)?.into_vec();
    for ax in &axes[1..4] {
        ax.yaxis().set_visible(false);
    }
    axes[4].yaxis().tick_to(SpineSide::Right)?;
    let bbox = Bbox::from_extents(0.0, 0.4, 1.0, 0.6);
    // The styles of the connector patch and of the marks, each axes.
    let styles = [
        PatchProps::default().facecolor("b"),
        PatchProps::default().facecolor("y"),
        PatchProps::default().color("g"),
        PatchProps::default().facecolor("g").fill(false),
    ];
    for (pair, style) in axes.windows(2).zip(styles) {
        let (ax, next) = (&pair[0], &pair[1]);
        let connector = BboxConnectorPatch {
            bbox1: BboxSource::transformed(bbox, ax.trans_data()),
            bbox2: BboxSource::transformed(bbox, next.trans_data()),
            loc1a: Corner::UpperRight,
            loc2a: Corner::UpperLeft,
            loc1b: Corner::LowerRight,
            loc2b: Corner::LowerLeft,
        };
        // Like matplotlib's connector patches, filled once given a face
        // colour, unless told not to be.
        let connector_style = PatchProps::default().edgecolor("r").fill(true);
        ax.add_patch(connector)
            .update_props(connector_style.overlay(style.clone()))
            .set_clip_on(false);
        let axins = zoomed_inset_axes(ax, 1.0, InsetOptions::default());
        axins.set_xlim(0.0, 0.2)?.set_ylim(0.0, 0.2)?;
        axins.set_xticks(Vec::<f64>::new());
        axins.set_yticks(Vec::<f64>::new());
        mark_inset(
            ax,
            &axins,
            Corner::UpperLeft,
            Corner::LowerRight,
            style.edgecolor("0.5"),
        );
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_zooming_with_inverted_axes`: a diagonal line in `limits`
/// (`[xmin, xmax, ymin, ymax]`) and an inset zoomed 2.5 times on
/// `inset_limits`.
fn zooming(fig: &Figure, limits: [f64; 4], inset_limits: [f64; 4]) -> CaseResult {
    let ax = fig.subplot();
    ax.plot([1.0, 2.0, 3.0], [1.0, 2.0, 3.0])?;
    ax.set_xlim(limits[0], limits[1])?
        .set_ylim(limits[2], limits[3])?;
    let options = InsetOptions {
        loc: AnchorLoc::LowerRight,
        ..InsetOptions::default()
    };
    let inset = zoomed_inset_axes(&ax, 2.5, options);
    inset
        .set_xlim(inset_limits[0], inset_limits[1])?
        .set_ylim(inset_limits[2], inset_limits[3])?;
    Ok(())
}

fn zooming_with_inverted_axes_1(fig: &Figure) -> CaseResult {
    zooming(fig, [1.0, 3.0, 1.0, 3.0], [1.1, 1.4, 1.1, 1.4])
}

fn zooming_with_inverted_axes_2(fig: &Figure) -> CaseResult {
    zooming(fig, [3.0, 1.0, 3.0, 1.0], [1.4, 1.1, 1.4, 1.1])
}

fn mark_inset_unstales_viewlim(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(1, 2)?.into_vec();
    let (inset, full) = (&axes[0], &axes[1]);
    full.plot([0.0, 5.0], [0.0, 5.0])?;
    inset.set_xlim(1.0, 2.0)?.set_ylim(1.0, 2.0)?;
    mark_inset(
        full,
        inset,
        Corner::UpperRight,
        Corner::LowerRight,
        PatchProps::default(),
    );
    Ok(())
}

fn anchored_locator_base_call(fig: &Figure) -> CaseResult {
    let subfigures = fig.subfigures(2, 1, SubfigureOptions::default())?;
    let ax = subfigures[(0, 0)].subplot();
    ax.set_aspect(Aspect::Equal)?;
    ax.set_xlim(-15.0, 15.0)?.set_ylim(-20.0, 5.0)?;
    ax.set_xticks(Vec::<f64>::new());
    ax.set_yticks(Vec::<f64>::new());
    let options = InsetOptions {
        loc: AnchorLoc::UpperLeft,
        ..InsetOptions::default()
    };
    let axins = zoomed_inset_axes(&ax, 2.0, options);
    axins.set_xticks(Vec::<f64>::new());
    axins.set_yticks(Vec::<f64>::new());
    axins.imshow(
        bivariate_normal(),
        ImshowOptions {
            extent: Some(EXTENT),
            origin: Some(Origin::Lower),
            ..ImshowOptions::default()
        },
    )?;
    Ok(())
}
