//! Anchored artists and host axes: direction arrows, anchored texts,
//! drawing areas and size bars, and parasite axes sharing an axis with
//! their host or mapping their coordinates onto it
//! (`demo_anchored_direction_arrows`, `simple_anchored_artists`,
//! `parasite_simple`, `parasite_simple2` and `simple_axisline4`).

use std::f64::consts::PI;

use plt::axes::{ErrorbarOptions, ImshowOptions};
use plt::axes_grid1::anchored_artists::{
    AnchoredDirectionArrows, AnchoredDrawingArea, AnchoredSizeBar,
};
use plt::axes_grid1::host_subplot;
use plt::fonts::FontProperties;
use plt::legend::LegendOptions;
use plt::numeric::Grid;
use plt::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions, BboxToAnchor, BoxItem};
use plt::patches::{BoxStyle, Circle};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::spines::SpineSide;
use plt::text::TextProps;
use plt::transforms::Affine2D;

use super::SEED;
use crate::figures::cases::common::{map, sized};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The superscript-bar label of the high-contrast arrows.
const BAR_TWO: &str = r"11$\overline{2}$";

fn demo_anchored_direction_arrows(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let noise = Grid::from_vec(10, 10, SpecRng::new(SEED).uniform(10 * 10, 0.0, 1.0))?;
    ax.imshow(noise, ImshowOptions::default())?;
    let arrows = |x: &str, y: &str| AnchoredDirectionArrows::new(ax.trans_axes(), x, y);
    let add = |arrows: AnchoredDirectionArrows| -> CaseResult {
        ax.add_anchored_box(AnchoredBox::try_from(arrows)?);
        Ok(())
    };
    add(arrows("X", "Y"))?;
    add(AnchoredDirectionArrows {
        loc: AnchorLoc::UpperRight,
        arrow_props: PatchProps::default()
            .edgecolor("w")
            .facecolor("none")
            .alpha(1.0)
            .linewidth(2.0),
        ..arrows("111", BAR_TWO)
    })?;
    add(AnchoredDirectionArrows {
        loc: AnchorLoc::UpperRight,
        arrow_props: PatchProps::default().edgecolor("none").facecolor("k"),
        text_props: PatchProps::default()
            .edgecolor("w")
            .facecolor("k")
            .linewidth(0.4),
        ..arrows("111", BAR_TWO)
    })?;
    let rc = plt::rc::params();
    add(AnchoredDirectionArrows {
        loc: AnchorLoc::Center,
        color: "w".into(),
        angle: 30.0,
        fontproperties: Some(FontProperties {
            families: vec!["serif".into()],
            ..FontProperties::from_rc(&rc)
        }),
        ..arrows("30", "120")
    })?;
    add(AnchoredDirectionArrows {
        loc: AnchorLoc::LowerCenter,
        length: -0.15,
        sep_x: 0.03,
        sep_y: 0.03,
        color: "r".into(),
        ..arrows("A", "B")
    })?;
    add(AnchoredDirectionArrows {
        loc: AnchorLoc::LowerLeft,
        aspect_ratio: -1.0,
        sep_x: 0.01,
        sep_y: -0.02,
        color: "orange".into(),
        ..arrows("A", " B")
    })?;
    add(AnchoredDirectionArrows {
        loc: AnchorLoc::LowerRight,
        length: -0.15,
        aspect_ratio: -1.0,
        sep_y: -0.1,
        sep_x: 0.04,
        color: "cyan".into(),
        ..arrows(" A", "B")
    })
}

/// A framed 8 pt anchored text with square corners rounded by 0.2.
fn rounded_label(ax: &Axes, text: &str, options: AnchoredOptions) -> CaseResult {
    let label = ax.add_anchored_box(AnchoredBox::text(
        text,
        TextProps::default().font(sized(8.0)),
        options,
    )?);
    let rounded: BoxStyle = "round,pad=0.,rounding_size=0.2".parse()?;
    label.patch().update_shape(|frame| frame.boxstyle = rounded);
    Ok(())
}

fn simple_anchored_artists(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_aspect(Aspect::Ratio(1.0))?;
    rounded_label(&ax, "Figure 1a", AnchoredOptions::new(AnchorLoc::UpperLeft))?;
    let above = AnchoredOptions {
        bbox_to_anchor: Some(BboxToAnchor::Point(0.0, 1.0)),
        bbox_transform: Some(ax.trans_axes()),
        ..AnchoredOptions::new(AnchorLoc::LowerLeft)
    };
    rounded_label(&ax, "Figure 1(b)", above)?;
    let area = AnchoredDrawingArea {
        pad: 0.0,
        frameon: false,
        ..AnchoredDrawingArea::new(20.0, 20.0, 0.0, 0.0, AnchorLoc::UpperRight)
    }
    .with_artist(BoxItem::patch(
        Circle::new((10.0, 10.0), 10.0),
        PatchProps::default(),
    ));
    ax.add_anchored_box(area);
    ax.add_anchored_box(AnchoredSizeBar {
        pad: 0.1,
        borderpad: 0.5,
        sep: 5.0,
        frameon: false,
        ..AnchoredSizeBar::new(
            ax.trans_data(),
            0.1,
            r"1$^{\prime}$",
            AnchorLoc::LowerCenter,
        )
    });
    Ok(())
}

fn parasite_simple(fig: &Figure) -> CaseResult {
    let host = host_subplot(fig, 1, 1, 1)?;
    let par = host.twinx();
    host.set_xlabel("Distance");
    host.set_ylabel("Density");
    par.set_ylabel("Temperature");
    let p1 = host.plot([0.0, 1.0, 2.0], [0.0, 1.0, 2.0])?;
    p1.set_label("Density");
    let p2 = par.plot([0.0, 1.0, 2.0], [0.0, 3.0, 2.0])?;
    p2.set_label("Temperature");
    host.legend(LegendOptions::default().labelcolor("linecolor"))?;
    host.yaxis().set_label_color(p1.rgba()?);
    par.yaxis().set_label_color(p2.rgba()?);
    Ok(())
}

/// `parasite_simple2`'s observations: name, proper motion and its error,
/// line width and its error.
const OBSERVATIONS: [(&str, f64, f64, f64, f64); 4] = [
    ("01_S1", 3.88, 0.14, 1970.0, 63.0),
    ("01_S4", 5.6, 0.82, 1622.0, 150.0),
    ("02_S1", 2.4, 0.54, 1570.0, 40.0),
    ("03_S1", 4.1, 0.62, 2380.0, 170.0),
];

fn parasite_simple2(fig: &Figure) -> CaseResult {
    let ax_kms = host_subplot(fig, 1, 1, 1)?;
    ax_kms.set_aspect(Aspect::Ratio(1.0))?;
    let pm_to_kms = 1.0 / 206_265.0 * 2300.0 * 3.085e18 / 3.15e7 / 1.0e5;
    let ax_pm = ax_kms.twin(Some(Affine2D::scaling(pm_to_kms, 1.0)))?;
    for (_, ds, dse, w, we) in OBSERVATIONS {
        let time = (2007.0 + (10.0 + 4.0 / 30.0) / 12.0) - 1988.5;
        let v = ds / time * pm_to_kms;
        let ve = dse / time * pm_to_kms;
        let options = ErrorbarOptions {
            xerr: Some(vec![ve].into()),
            yerr: Some(vec![we].into()),
            line: LineProps::default().color("k"),
            ..ErrorbarOptions::default()
        };
        ax_kms.errorbar([v], [w], options)?;
    }
    ax_kms
        .axis(SpineSide::Bottom)
        .set_label("Linear velocity at 2.3 kpc [km/s]");
    ax_kms.axis(SpineSide::Left).set_label("FWHM [km/s]");
    ax_pm
        .axis(SpineSide::Top)
        .set_label(r"Proper Motion [$''$/yr]")
        .set_label_visible(true);
    ax_pm.axis(SpineSide::Right).set_ticklabels_visible(false);
    ax_kms.set_xlim(950.0, 3700.0)?.set_ylim(950.0, 3100.0)?;
    Ok(())
}

fn simple_axisline4(fig: &Figure) -> CaseResult {
    let ax = host_subplot(fig, 1, 1, 1)?;
    let xx = arange(0.0, 2.0 * PI, 0.01);
    ax.plot(xx.clone(), map(&xx, f64::sin))?;
    let ax2 = ax.twin(None)?;
    ax2.set_xticks(vec![0.0, 0.5 * PI, PI, 1.5 * PI, 2.0 * PI]);
    ax2.set_xticklabels([
        "$0$",
        r"$\frac{1}{2}\pi$",
        r"$\pi$",
        r"$\frac{3}{2}\pi$",
        r"$2\pi$",
    ])?;
    ax2.axis(SpineSide::Right).set_ticklabels_visible(false);
    ax2.axis(SpineSide::Top).set_ticklabels_visible(true);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "gallery_shapes/demo_anchored_direction_arrows",
        demo_anchored_direction_arrows,
    ),
    FigureCase::new(
        "gallery_shapes/simple_anchored_artists",
        simple_anchored_artists,
    ),
    FigureCase::new("gallery_shapes/parasite_simple", parasite_simple),
    FigureCase::new("gallery_shapes/parasite_simple2", parasite_simple2),
    FigureCase::new("gallery_shapes/simple_axisline4", simple_axisline4),
];
