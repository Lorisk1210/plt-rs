//! The figure of matplotlib's `test_determinism._save_figure`: one
//! subfigure per kind of object (markers, hatches, images, clip paths),
//! then an axes with mathtext labels.

use std::error::Error;

use pltrs::artist::{ArtistHandle, ClipPathSpec};
use pltrs::axes::{Axes, BarOptions, ImshowOptions};
use pltrs::collections::{CollectionProps, CollectionSpec};
use pltrs::figure::{Figure, SaveFormat, SaveOptions, Share, SubfigureOptions};
use pltrs::fonts::FontProperties;
use pltrs::gridspec::{GridSpec, SubplotOverrides};
use pltrs::image::Interpolation;
use pltrs::lines::LineProps;
use pltrs::markers::Marker;
use pltrs::numeric::Grid;
use pltrs::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions, AuxTransformBox, BoxItem};
use pltrs::patches::{Circle, PatchProps, PatchShape, PathShape};
use pltrs::transforms::Transform;
use pltrs_core::{Affine2D, Path, Point};

/// What drawing the figure can fail with.
pub(crate) type Failure = Box<dyn Error>;

/// The kinds of objects a figure can hold: `m`arkers, `h`atches,
/// `i`mages and clip `p`aths (the upstream `objects` letters).
const OBJECTS: [char; 4] = ['m', 'h', 'i', 'p'];

/// `plot_markers`: five lines with different markers.
fn markers(fig: &Figure) -> Result<(), Failure> {
    let ax = fig.subplot();
    let x: Vec<f64> = (0..10).map(f64::from).collect();
    for (level, marker) in [(1.0, "D"), (2.0, "x"), (3.0, "^"), (4.0, "H"), (5.0, "v")] {
        let props = LineProps::default().marker(marker.parse::<Marker>()?);
        ax.plot_with(x.clone(), vec![level; 10], props)?;
    }
    Ok(())
}

/// `plot_hatch`: two stacks of bars, each with its own hatch pattern.
fn hatches(fig: &Figure) -> Result<(), Failure> {
    let ax = fig.subplot();
    let x = [1.0, 2.0, 3.0, 4.0];
    let lower = ax.bar(x.to_vec(), x, BarOptions::default())?;
    let upper = ax.bar(
        x.to_vec(),
        [6.0; 4],
        BarOptions {
            base: x.to_vec().into(),
            ..BarOptions::default()
        },
    )?;
    ax.set_xticks(vec![1.5, 2.5, 3.5, 4.5]);
    let patterns = ["-", "+", "x", "\\", "*", "o", "O", "."];
    for (bar, pattern) in lower.patches.iter().chain(&upper.patches).zip(patterns) {
        bar.set_hatch(pattern);
    }
    Ok(())
}

/// `plot_image`: three images with different interpolations on axes
/// sharing x and y.
fn images(fig: &Figure) -> Result<(), Failure> {
    let axs = fig.subplots_shared(1, 3, Share::All, Share::All)?;
    let data = [
        (
            [[1.0, 2.0, 3.0], [2.0, 3.0, 1.0], [3.0, 1.0, 2.0]],
            Interpolation::Nearest,
        ),
        (
            [[1.0, 3.0, 2.0], [1.0, 2.0, 3.0], [3.0, 1.0, 2.0]],
            Interpolation::Bilinear,
        ),
        (
            [[2.0, 3.0, 1.0], [1.0, 2.0, 3.0], [2.0, 1.0, 3.0]],
            Interpolation::Bicubic,
        ),
    ];
    for (col, (rows, interpolation)) in data.into_iter().enumerate() {
        let options = ImshowOptions {
            interpolation: Some(interpolation),
            ..ImshowOptions::default()
        };
        axs[(0, col)].imshow(Grid::from_rows(rows)?, options)?;
    }
    Ok(())
}

/// A star clipped to a square with a round hole, as a collection on `ax1`
/// and as a patch on `ax2` (from matplotlib's `test_artist.py`).
fn clipped_stars(ax1: &Axes, ax2: &Axes) -> Result<(), Failure> {
    let exterior =
        Path::unit_rectangle().transformed(&Affine2D::scaling(4.0, 4.0).translate(-2.0, -2.0));
    // matplotlib reverses the circle's vertices, keeping its codes.
    let circle = Path::unit_circle();
    let reversed = circle.vertices().iter().rev().copied().collect();
    let interior = Path::new(reversed, circle.codes().unwrap_or_default().to_vec())?;
    let clip = Path::compound(&[exterior, interior]);
    let star = Path::unit_regular_star(6, 0.5).transformed(&Affine2D::scaling(2.6, 2.6));
    let props = CollectionProps::default()
        .linewidth(5.0)
        .edgecolor("blue")
        .facecolor("red")
        .alpha(0.7)
        .hatch("*");
    let collection =
        ax1.add_collection(CollectionSpec::paths(vec![star.clone()]).with_props(props));
    collection.set_clip_path(Some(ClipPathSpec::Path {
        path: clip.clone(),
        transform: ax1.trans_data(),
    }));
    let patch = ax2.add_patch_with(
        PathShape(star),
        PatchProps::default()
            .linewidth(5.0)
            .edgecolor("blue")
            .facecolor("red")
            .alpha(0.7)
            .hatch("*"),
    );
    patch.set_clip_path(Some(ClipPathSpec::Path {
        path: clip,
        transform: ax2.trans_data(),
    }));
    ax1.set_xlim(-3.0, 3.0)?;
    ax1.set_ylim(-3.0, 3.0)?;
    Ok(())
}

/// `plot_paths`: a polar image clipped by a circle, a text path in an
/// anchored box and a grid of clipped stars, side by side. (The upstream
/// figure fills the text path with a photograph; pltrs draws its
/// outline.)
fn paths(fig: &Figure) -> Result<(), Failure> {
    let parts = fig.subfigures(1, 3, SubfigureOptions::default())?;

    let polar = parts[(0, 0)].polar_subplot();
    let image = polar.imshow(Grid::from_rows([[2.0]])?, ImshowOptions::default())?;
    image.set_clip_path(Circle::new((0.0, 1.0), 0.3333).path(), Transform::Display);

    let ax = parts[(0, 1)].subplot();
    let font = FontProperties::from_rc(&pltrs::rc::params());
    let text = pltrs::text::text_path(Point::ORIGIN, "!?", Some(150.0), &font)?;
    let outline = AuxTransformBox::new(Transform::Display).with_artist(BoxItem::patch(
        PathShape(text),
        PatchProps::default().edgecolor("k").facecolor("none"),
    ));
    ax.add_anchored_box(AnchoredBox::new(
        outline,
        AnchoredOptions {
            borderpad: (0.2, 0.2),
            ..AnchoredOptions::new(AnchorLoc::UpperLeft)
        },
    ));

    let grid = GridSpec::with_ratios(
        2,
        2,
        None,
        None,
        SubplotOverrides {
            wspace: Some(0.0),
            hspace: Some(0.0),
            ..SubplotOverrides::default()
        },
    )?;
    let axs = parts[(0, 2)].subplots_on(&grid, Share::All, Share::All)?;
    for row in 0..2 {
        clipped_stars(&axs[(row, 0)], &axs[(row, 1)])?;
    }
    Ok(())
}

/// Draws the figure with the `objects` (letters of [`OBJECTS`]) and saves
/// it as `format` with the default metadata, as `_save_figure` does with
/// `svg.hashsalt = "asdf"`.
pub(crate) fn save_figure(objects: &str, format: SaveFormat) -> Result<Vec<u8>, Failure> {
    pltrs::rc::context(|| {
        pltrs::rc::set("svg.hashsalt", "asdf")?;
        let count = objects.len() + 1;
        let fig = Figure::with_size(7.0, 3.0 * count as f64, 100.0);
        let parts = fig.subfigures(count, 1, SubfigureOptions::default())?;
        fig.subplots_adjust(SubplotOverrides {
            bottom: Some(0.15),
            ..SubplotOverrides::default()
        })?;
        let mut next = parts.as_slice().iter();
        let mut part = || next.next().ok_or("one subfigure per object");
        for (letter, draw) in OBJECTS
            .into_iter()
            .zip([markers, hatches, images, paths] as [fn(&Figure) -> Result<(), Failure>; 4])
        {
            if objects.contains(letter) {
                draw(part()?)?;
            }
        }
        let ax = part()?.subplot();
        let x: Vec<f64> = (0..5).map(f64::from).collect();
        ax.plot(x.clone(), x)?;
        let label = r"A string $1+2+\sigma$";
        ax.set_title(label);
        ax.set_xlabel(label);
        ax.set_ylabel(label);
        Ok(fig.to_format(format, SaveOptions::default())?)
    })
}
