//! `axes_grid1`: grids of images, axes appended by a divider, zoomed
//! insets marked on their parent, and anchored scale bars and arrows.

use plt::axes::ImshowOptions;
use plt::axes_grid1::anchored_artists::{AnchoredDirectionArrows, AnchoredSizeBar};
use plt::axes_grid1::{
    CbarMode, Corner, GridRect, ImageGrid, ImageGridOptions, InsetOptions, SizeSpec,
    make_axes_locatable, mark_inset, zoomed_inset_axes,
};
use plt::colorbar::ColorbarOptions;
use plt::colors::{ColorizerOptions, colormaps};
use plt::image::{Extent, Origin};
use plt::numeric::Grid;
use plt::offsetbox::{AnchorLoc, AnchoredBox};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::spines::SpineSide;

use super::data::Samples;
use super::density::marginal_histograms;
use super::{Built, Entry};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry::figure(
        "image_grid",
        "ImageGrid: images on fixed paddings, one colorbar",
        image_grid,
    ),
    Entry::figure(
        "axes_locatable",
        "make_axes_locatable: marginal histograms appended to a scatter",
        axes_locatable,
    ),
    Entry::figure(
        "zoomed_inset",
        "zoomed_inset_axes marked on its parent",
        zoomed_inset,
    ),
    Entry::figure(
        "anchored_artists",
        "Anchored size bar and direction arrows",
        anchored_artists,
    ),
];

/// A field of ripples over `[-3, 3]²`, sharpened by `k`.
fn ripples(n: usize, k: f64) -> Grid<f64> {
    Grid::from_fn(n, n, |i, j| {
        let (x, y) = (
            j as f64 / n as f64 * 6.0 - 3.0,
            i as f64 / n as f64 * 6.0 - 3.0,
        );
        (k * x.hypot(y)).sin() * (-(x * x + y * y) / 6.0).exp()
    })
}

/// Four images in a 2 × 2 grid with fixed gaps and a single colorbar on
/// the right: the grid keeps the images square however the figure is
/// shaped.
fn image_grid() -> Built {
    let fig = Figure::with_size(6.4, 5.2, 100.0);
    let options = ImageGridOptions {
        axes_pad: (0.12, 0.12),
        share_all: true,
        cbar_location: SpineSide::Right,
        cbar_mode: Some(CbarMode::Single),
        cbar_pad: Some(SizeSpec::Inches(0.12)),
        ..ImageGridOptions::default()
    };
    let grid = ImageGrid::new(
        &fig,
        &GridRect::Rect([0.05, 0.06, 0.85, 0.86]),
        (2, 2),
        options,
    )?;
    let mut last = None;
    for (ax, k) in grid.axes().iter().zip([1.5, 2.5, 3.5, 4.5]) {
        let image = ImshowOptions {
            extent: Some(Extent::new(-3.0, 3.0, -3.0, 3.0)),
            colorizer: ColorizerOptions::default()
                .cmap(colormaps::get("cividis")?)
                .limits(Some(-1.0), Some(1.0)),
            ..ImshowOptions::default()
        };
        last = Some(ax.imshow(ripples(120, k), image)?);
    }
    let image = last.ok_or("the grid has axes")?;
    grid.cbar_axes()[0].colorbar(&image, ColorbarOptions::default())?;
    fig.suptitle("ImageGrid with a single colorbar");
    Ok(fig)
}

/// A scatter plot with histograms of x and y appended above and to its
/// right: the divider gives them fixed heights and widths, in inches,
/// and they stay aligned with the scatter's axes.
fn axes_locatable() -> Built {
    let fig = Figure::with_size(5.5, 5.5, 100.0);
    let ax = fig.subplot();
    let mut samples = Samples::new(53);
    let x = samples.normals(600, 0.0, 1.0);
    let y: Vec<f64> = x
        .iter()
        .map(|x| 0.5 * x * x + samples.normal(0.0, 0.6))
        .collect();
    ax.scatter_with(
        &x,
        &y,
        ScatterOptions {
            sizes: Some(vec![8.0]),
            alpha: 0.6.into(),
            ..ScatterOptions::default()
        },
    )?;
    ax.set_aspect(Aspect::Equal)?;
    let divider = make_axes_locatable(&ax);
    let top = divider.append_axes(
        SpineSide::Top,
        SizeSpec::Inches(1.0),
        Some(SizeSpec::Inches(0.1)),
    );
    let right = divider.append_axes(
        SpineSide::Right,
        SizeSpec::Inches(1.0),
        Some(SizeSpec::Inches(0.1)),
    );
    marginal_histograms(&ax, &top, &right, x, y, 30)?;
    // Constrained layout places the scatter only; the appended axes take
    // their room from its box, so the title goes on the figure.
    fig.suptitle("Appended axes of fixed size");
    Ok(fig)
}

/// An image with an inset enlarging a small region three times; `mark_inset`
/// outlines the enlarged region and joins it to two corners of the inset.
fn zoomed_inset() -> Built {
    let fig = Figure::with_size(6.0, 5.0, 100.0);
    let ax = fig.subplot();
    let z = ripples(300, 4.0);
    let image = || -> Result<ImshowOptions, Box<dyn std::error::Error>> {
        Ok(ImshowOptions {
            extent: Some(Extent::new(-3.0, 3.0, -3.0, 3.0)),
            origin: Some(Origin::Lower),
            colorizer: ColorizerOptions::default().cmap(colormaps::get("magma")?),
            ..ImshowOptions::default()
        })
    };
    ax.imshow(z.clone(), image()?)?;
    let options = InsetOptions {
        loc: AnchorLoc::UpperRight,
        ..InsetOptions::default()
    };
    let inset = zoomed_inset_axes(&ax, 3.0, options);
    inset.imshow(z, image()?)?;
    inset.set_xlim(-1.8, -0.8)?.set_ylim(-1.8, -0.8)?;
    inset
        .set_xticks(Vec::<f64>::new())
        .set_yticks(Vec::<f64>::new());
    let marks = PatchProps::default().facecolor("none").edgecolor("0.9");
    mark_inset(&ax, &inset, Corner::UpperLeft, Corner::LowerRight, marks);
    ax.set_title("A 3× zoom across the rings");
    Ok(fig)
}

/// A scale bar and compass arrows anchored in the corners of an image:
/// the bar measures data units, the arrows stay the same size whatever
/// the zoom.
fn anchored_artists() -> Built {
    let fig = Figure::with_size(5.5, 5.0, 100.0);
    let ax = fig.subplot();
    let options = ImshowOptions {
        extent: Some(Extent::new(0.0, 10.0, 0.0, 10.0)),
        origin: Some(Origin::Lower),
        colorizer: ColorizerOptions::default().cmap(colormaps::get("bone")?),
        ..ImshowOptions::default()
    };
    ax.imshow(ripples(200, 2.0).map(|v| v.abs()), options)?;
    let bar = AnchoredSizeBar {
        frameon: false,
        color: "white".into(),
        size_vertical: 0.15,
        ..AnchoredSizeBar::new(ax.trans_data(), 2.0, "2 km", AnchorLoc::LowerRight)
    };
    ax.add_anchored_box(bar);
    let arrows = AnchoredDirectionArrows {
        color: "white".into(),
        ..AnchoredDirectionArrows::new(ax.trans_axes(), "E", "N")
    };
    ax.add_anchored_box(AnchoredBox::try_from(arrows)?);
    ax.set_axis_off();
    ax.set_title("AnchoredSizeBar and AnchoredDirectionArrows");
    Ok(fig)
}
