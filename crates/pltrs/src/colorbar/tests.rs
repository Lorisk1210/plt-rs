//! Colorbars built end to end, checked against values from matplotlib.

use super::*;
use crate::artist::ArtistId;
use crate::axes::{
    ImshowOptions, MappedColors, ScatterColors, ScatterOptions, TriShading, TriValues, Tripcolor,
    TripcolorOptions,
};
use crate::collections::CollectionArtist;
use crate::colors::{
    BoundaryNorm, Colorize, ColorizerOptions, LogNorm, Normalize, ScalarMappable, colormaps,
};
use crate::contour::{ContourOptions, Levels, QuadField};
use crate::numeric::Grid;
use crate::scale::ScaleName;
use crate::spines::SpineSide;
use crate::tri::Triangulation;

fn close(actual: (f64, f64, f64, f64), expected: [f64; 4]) -> bool {
    let actual = [actual.0, actual.1, actual.2, actual.3];
    actual
        .iter()
        .zip(expected)
        .all(|(a, e)| (a - e).abs() < 1e-9)
}

/// A 6.4 × 4.8 in figure with a 4 × 4 ramp image in one subplot.
fn image_figure() -> (Figure, Axes, crate::image::AxesImage) {
    let fig = Figure::new();
    let ax = fig.subplot();
    let data = Grid::from_fn(4, 4, |r, c| (r * 4 + c) as f64 / 15.0 - 0.5);
    let image = ax.imshow(data, ImshowOptions::default()).unwrap();
    (fig, ax, image)
}

#[test]
fn the_current_axes_stays_current() {
    let (fig, ax, image) = image_figure();
    let colorbar = fig.colorbar(&image, ColorbarOptions::default()).unwrap();
    assert_eq!(fig.gca(), ax);
    assert_ne!(colorbar.ax(), &ax);
}

#[test]
fn default_colorbar_takes_the_right_of_the_cell() {
    let (fig, ax, image) = image_figure();
    let colorbar = fig.colorbar(&image, ColorbarOptions::default()).unwrap();
    // matplotlib: [0.78375, 0.11, 0.028875, 0.77] and the image shrunk
    // towards it.
    assert!(close(
        colorbar.ax().position().bounds(),
        [0.783_75, 0.11, 0.028_875, 0.77]
    ));
    assert!(close(ax.position().bounds(), [0.1675, 0.11, 0.5775, 0.77]));
    assert_eq!(colorbar.ax().ylim(), (-0.5, 0.5));
    assert_eq!(colorbar.ax().xlim(), (0.0, 1.0));
    assert_eq!(colorbar.boundaries().len(), 257);
    assert_eq!(colorbar.long_axis().label_position(), SpineSide::Right);
    assert!(colorbar.ax().spines().iter().all(|s| !s.visible()));
}

#[test]
fn horizontal_colorbars_go_below() {
    let (fig, _, image) = image_figure();
    let options = ColorbarOptions {
        orientation: Some(Orientation::Horizontal),
        ..ColorbarOptions::default()
    };
    let colorbar = fig.colorbar(&image, options).unwrap();
    assert_eq!(colorbar.orientation(), Orientation::Horizontal);
    assert_eq!(colorbar.ax().xlim(), (-0.5, 0.5));
    let (_, y, _, h) = colorbar.ax().position().bounds();
    assert!(y > 0.11 && y + h < 0.3);
}

#[test]
fn extensions_shrink_the_axes() {
    let (fig, _, image) = image_figure();
    let options = ColorbarOptions {
        extend: Some(Extend::Max),
        extendrect: true,
        ..ColorbarOptions::default()
    };
    let colorbar = fig.colorbar(&image, options).unwrap();
    assert_eq!(colorbar.extend(), Extend::Max);
    let (_, y, _, h) = colorbar.ax().position().bounds();
    assert!((y - 0.11).abs() < 1e-12 && (h - 0.77 / 1.05).abs() < 1e-12);
    assert_eq!(colorbar.boundaries().len(), 258);
}

#[test]
fn boundary_norms_tick_their_boundaries() {
    let fig = Figure::new();
    let ax = fig.subplot();
    let norm = BoundaryNorm::new([0.0, 1.0, 5.0, 10.0], 3, Extend::Neither).unwrap();
    let mappable = ScalarMappable::with_norm(colormaps::get("viridis").unwrap(), Box::new(norm));
    let options = ColorbarOptions {
        axes: ColorbarAxes::Parent(ax),
        ..ColorbarOptions::default()
    };
    let colorbar = fig.colorbar(&mappable, options).unwrap();
    assert_eq!(colorbar.ax().yscale(), ScaleName::Function);
    let ticks = colorbar.ax().ticks(AxisName::Y).unwrap();
    let locs: Vec<f64> = ticks.major.iter().map(|t| t.loc).collect();
    assert_eq!(locs, [0.0, 1.0, 5.0, 10.0]);
}

#[test]
fn several_parents_share_one_colorbar() {
    let fig = Figure::with_size(5.0, 5.0, 100.0);
    let axs = fig.subplots(2, 1).unwrap();
    let mappable = ScalarMappable::with_norm(
        colormaps::get("viridis").unwrap(),
        Box::new(Normalize::new(-1.0, 1.0)),
    );
    let options = ColorbarOptions {
        axes: ColorbarAxes::Parents(axs.as_slice().to_vec()),
        ..ColorbarOptions::default()
    };
    let colorbar = fig.colorbar(&mappable, options).unwrap();
    // matplotlib: [0.78375, 0.11, 0.0385, 0.77].
    assert!(close(
        colorbar.ax().position().bounds(),
        [0.783_75, 0.11, 0.0385, 0.77]
    ));
}

#[test]
fn a_projection_plane_given_as_parent_stands_for_its_3d_axes() {
    let mappable = ScalarMappable::new(colormaps::get("viridis").unwrap());
    let place = |given: bool| {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let options = ColorbarOptions {
            axes: if given {
                ColorbarAxes::Parents(vec![ax.projection_plane()])
            } else {
                ColorbarAxes::Parent(ax.projection_plane())
            },
            shrink: 0.5,
            ..ColorbarOptions::default()
        };
        let colorbar = fig.colorbar(&mappable, options).unwrap();
        (colorbar.ax().position().bounds(), ax.position().bounds())
    };
    // matplotlib, with `ax=` the 3-D axes of a 6.4 × 4.8 in figure.
    let (cax, ax) = place(true);
    assert!(close(cax, [0.783_75, 0.3025, 0.014_437_5, 0.385]));
    assert!(close(ax, [0.1675, 0.11, 0.5775, 0.77]));
    let (one_cax, one_ax) = place(false);
    assert!(close(one_cax, [cax.0, cax.1, cax.2, cax.3]));
    assert!(close(one_ax, [ax.0, ax.1, ax.2, ax.3]));
}

#[test]
fn a_cax_is_used_as_given() {
    let fig = Figure::new();
    let cax = fig
        .add_axes([0.85, 0.1, 0.05, 0.8])
        .expect("a valid rectangle");
    let mappable = ScalarMappable::with_norm(
        colormaps::get("viridis").unwrap(),
        Box::new(Normalize::new(0.0, 10.0)),
    );
    let options = ColorbarOptions {
        axes: ColorbarAxes::Cax(cax.clone()),
        label: "speed".into(),
        ..ColorbarOptions::default()
    };
    let colorbar = fig.colorbar(&mappable, options).unwrap();
    assert_eq!(colorbar.ax(), &cax);
    assert!(close(cax.position().bounds(), [0.85, 0.1, 0.05, 0.8]));
    assert_eq!(cax.label(AxisName::Y), "speed");
}

#[test]
fn colorbars_need_an_axes() {
    let fig = Figure::new();
    let mappable = ScalarMappable::new(colormaps::get("viridis").unwrap());
    assert_eq!(
        fig.colorbar(&mappable, ColorbarOptions::default()).err(),
        Some(ColorbarError::NoAxes)
    );
    let empty = ColorbarOptions {
        axes: ColorbarAxes::Parents(Vec::new()),
        ..ColorbarOptions::default()
    };
    assert_eq!(
        fig.colorbar(&mappable, empty).err(),
        Some(ColorbarError::NoAxes)
    );
}

#[test]
fn scatter_colorbars_follow_the_mapped_values() {
    let fig = Figure::new();
    let ax = fig.subplot();
    let options = ScatterOptions {
        colors: ScatterColors::from(MappedColors::new(vec![1.0, 3.0, 2.0])),
        ..ScatterOptions::default()
    };
    let points = ax
        .scatter_with(vec![0.0, 1.0, 2.0], vec![0.0, 1.0, 2.0], options)
        .unwrap();
    let colorbar = fig.colorbar(&points, ColorbarOptions::default()).unwrap();
    assert_eq!(colorbar.ax().ylim(), (1.0, 3.0));
    points.set_clim(None, Some(5.0));
    assert_eq!(colorbar.ax().ylim(), (1.0, 5.0));
}

/// The colormap name of the colorbar's colour blocks, and how many
/// collections its axes holds.
fn solids(colorbar: &Colorbar) -> (String, usize) {
    let id = colorbar.read(|state| state.drawn.solids).expect("filled");
    let children = colorbar
        .ax()
        .raw()
        .with::<AxesState, _>(|axes| axes.children.clone());
    let figure = colorbar.ax().raw().figure_state().borrow();
    let collection = |id: ArtistId| figure.store.get_as::<CollectionArtist>(id);
    let name = collection(id)
        .and_then(|c| c.mapping.as_ref())
        .map(|mapped| mapped.source.mappable.cmap().name().to_owned())
        .unwrap_or_default();
    let count = children
        .into_iter()
        .filter(|&c| collection(c).is_some())
        .count();
    (name, count)
}

#[test]
fn colorbars_follow_limits_and_colormaps() {
    let (fig, _, image) = image_figure();
    let colorbar = fig.colorbar(&image, ColorbarOptions::default()).unwrap();
    colorbar.set_ticks(vec![0.0, 0.25]);
    image.set_clim(Some(-0.25), Some(0.5));
    assert_eq!(colorbar.ax().ylim(), (-0.25, 0.5));
    image.set_cmap(colormaps::get("magma").unwrap());
    assert_eq!(
        solids(&colorbar),
        ("magma".to_owned(), 1),
        "old blocks removed"
    );
    let ticks = colorbar.ax().ticks(AxisName::Y).unwrap();
    let locs: Vec<f64> = ticks.major.iter().map(|t| t.loc).collect();
    assert_eq!(locs, [0.0, 0.25], "ticks set on the colorbar stay");
}

#[test]
fn a_new_norm_brings_its_scale() {
    let fig = Figure::new();
    let ax = fig.subplot();
    let data = Grid::from_fn(4, 4, |r, c| 1.0 + (r * 4 + c) as f64 * 10.0);
    let image = ax.imshow(data, ImshowOptions::default()).unwrap();
    let colorbar = fig.colorbar(&image, ColorbarOptions::default()).unwrap();
    assert_eq!(colorbar.ax().yscale(), ScaleName::Linear);
    image.set_norm(Box::new(LogNorm::default()));
    assert_eq!(
        image.clim(),
        (Some(1.0), Some(151.0)),
        "autoscaled to the data"
    );
    assert_eq!(colorbar.ax().yscale(), ScaleName::Log);
    assert_eq!(colorbar.ax().ylim(), (1.0, 151.0));
}

#[test]
fn a_retuned_norm_keeps_the_ticks_a_replaced_one_resets() {
    let (fig, _, image) = image_figure();
    let colorbar = fig.colorbar(&image, ColorbarOptions::default()).unwrap();
    colorbar.set_ticks(vec![-0.25, 0.25]);
    image.retune_norm(Box::new(Normalize::new(-1.0, 1.0)));
    assert_eq!(colorbar.ax().ylim(), (-1.0, 1.0), "the new limits show");
    assert_eq!(colorbar.long_axis().ticklocs(false).unwrap(), [-0.25, 0.25]);
    image.set_norm(Box::new(Normalize::new(-1.0, 1.0)));
    assert_ne!(colorbar.long_axis().ticklocs(false).unwrap(), [-0.25, 0.25]);
}

#[test]
fn contour_colorbars_follow_recolouring() {
    let fig = Figure::new();
    let ax = fig.subplot();
    let field = QuadField::new(Grid::from_fn(4, 5, |i, j| (i + j) as f64));
    let options = ContourOptions {
        levels: Levels::Values(vec![0.0, 2.0, 4.0, 7.0]),
        ..ContourOptions::default()
    };
    let fills = ax.contourf(&field, options).unwrap();
    let colorbar = fig.colorbar(&fills, ColorbarOptions::default()).unwrap();
    fills.set_cmap(colormaps::get("plasma").unwrap());
    fills.set_clim(Some(-5.0), None);
    assert_eq!(solids(&colorbar).0, "plasma");
    assert_eq!(colorbar.ax().ylim(), (0.0, 7.0), "the levels stay");
    let lines = ax.contour(&field, ContourOptions::default()).unwrap();
    let lined = fig.colorbar(&lines, ColorbarOptions::default()).unwrap();
    lines.set_cmap(colormaps::get("gray").unwrap());
    assert_eq!(
        lined.read(|state| state.drawn.lines.len()),
        1,
        "the contour lines are redrawn, not added"
    );
}

#[test]
fn gouraud_meshes_have_colorbars() {
    let fig = Figure::new();
    let ax = fig.subplot();
    let tri = Triangulation::delaunay(&[0.0, 1.0, 0.0, 1.0], &[0.0, 0.0, 1.0, 1.0]).unwrap();
    let options = TripcolorOptions {
        shading: TriShading::Gouraud,
        ..TripcolorOptions::default()
    };
    let plot = ax
        .tripcolor(&tri, TriValues::Auto(vec![0.0, 1.0, 2.0, 3.0]), options)
        .unwrap();
    let colorbar = fig.colorbar(&plot, ColorbarOptions::default()).unwrap();
    assert_eq!(colorbar.ax().ylim(), (0.0, 3.0));
    let Tripcolor::Gouraud(mesh) = plot else {
        panic!("Gouraud shading draws a mesh");
    };
    mesh.set_clim(None, Some(6.0));
    assert_eq!(colorbar.ax().ylim(), (0.0, 6.0));
}

#[test]
fn colorbars_in_other_figures_keep_a_copy() {
    let (_, _, image) = image_figure();
    let other = Figure::new();
    let options = ColorbarOptions {
        axes: ColorbarAxes::Cax(
            other
                .add_axes([0.85, 0.1, 0.05, 0.8])
                .expect("a valid rectangle"),
        ),
        ..ColorbarOptions::default()
    };
    let colorbar = other.colorbar(&image, options).unwrap();
    image.set_clim(Some(-2.0), Some(2.0));
    assert_eq!(colorbar.ax().ylim(), (-0.5, 0.5));
}

#[test]
fn colorbars_take_room_in_the_figure_of_their_parent() {
    // matplotlib's test_colorbar_wrong_figure: asked of another figure,
    // the colorbar goes beside the image in the image's figure.
    let (fig, _, image) = image_figure();
    fig.set_layout_engine(crate::layout::LayoutEngine::constrained())
        .unwrap();
    let tight = Figure::new();
    tight
        .set_layout_engine(crate::layout::LayoutEngine::tight())
        .unwrap();
    let colorbar = tight.colorbar(&image, ColorbarOptions::default()).unwrap();
    assert_eq!(tight.axes(), []);
    assert_eq!(fig.axes().len(), 2);
    image.set_clim(Some(-2.0), Some(2.0));
    assert_eq!(colorbar.ax().ylim(), (-2.0, 2.0), "it follows the image");
    assert!(fig.render_raster().is_ok());
}

#[test]
fn limits_opacity_and_dividers_read_back() {
    let fig = Figure::new();
    let ax = fig.subplot();
    let norm = BoundaryNorm::new([0.0, 1.0, 2.0, 3.0], 3, Extend::Neither).unwrap();
    let mappable = ScalarMappable::with_norm(colormaps::get("viridis").unwrap(), Box::new(norm));
    let options = ColorbarOptions {
        drawedges: true,
        extend: Some(Extend::Max),
        ..ColorbarOptions::beside(&ax)
    };
    let colorbar = fig.colorbar(&mappable, options).unwrap();
    assert_eq!((colorbar.vmin(), colorbar.vmax()), (0.0, 3.0));
    assert_eq!(colorbar.alpha(), None);
    colorbar.ax().set_ylim(0.5, 2.5).expect("finite limits");
    // The edges inside the view, then the extended top of the view.
    let at: Vec<f64> = colorbar
        .divider_segments()
        .iter()
        .map(|segment| segment.vertices()[0].y)
        .collect();
    assert_eq!(at.len(), 3);
    assert!(
        at.iter()
            .zip([1.0, 2.0, 2.5])
            .all(|(a, e)| (a - e).abs() < 1e-12)
    );
}

#[test]
fn only_a_colorbar_of_a_bare_mapping_takes_the_alpha_option() {
    // matplotlib's Colorbar(alpha=...): kept for a bare ScalarMappable,
    // replaced by the opacity of a drawn mappable.
    let fig = Figure::new();
    let ax = fig.subplot();
    let mappable = ScalarMappable::with_norm(
        colormaps::get("viridis").unwrap(),
        Box::new(Normalize::new(0.0, 1.0)),
    );
    let options = ColorbarOptions {
        alpha: Some(0.5),
        ..ColorbarOptions::beside(&ax)
    };
    let bare = fig.colorbar(&mappable, options).unwrap();
    assert_eq!(bare.alpha(), Some(0.5));
    assert_eq!(bare.solids().unwrap().alpha(), Some(0.5));
    let (fig, ax, image) = image_figure();
    let drawn = fig
        .colorbar(
            &image,
            ColorbarOptions {
                alpha: Some(0.5),
                ..ColorbarOptions::beside(&ax)
            },
        )
        .unwrap();
    assert_eq!(drawn.alpha(), None);
}

#[test]
fn hatched_contours_get_hatched_blocks_and_extensions() {
    let fig = Figure::new();
    let ax = fig.subplot();
    let field = QuadField::new(Grid::from_fn(4, 5, |i, j| (i + j) as f64));
    let options = ContourOptions {
        levels: Levels::Values(vec![1.0, 3.0, 5.0]),
        extend: crate::colors::Extend::Both,
        hatches: vec![Some("/".into()), None, Some("x".into())],
        ..ContourOptions::default()
    };
    let contours = ax.contourf(&field, options).unwrap();
    let colorbar = fig.colorbar(&contours, ColorbarOptions::default()).unwrap();
    let (solids, patches, extensions) = colorbar.read(|state| {
        (
            state.drawn.solids,
            state.drawn.solids_patches.clone(),
            state.drawn.extensions.clone(),
        )
    });
    assert!(solids.is_none(), "patches replace the mesh");
    let figure = fig.raw().figure_state().borrow();
    let hatch = |id: ArtistId| {
        figure
            .store
            .get(id)
            .and_then(crate::artist::Artist::as_patch)
            .and_then(|patch| patch.style.hatch.clone())
    };
    // Bands: under (/), [1, 3] (none), [3, 5] (x), over (/).
    let inner: Vec<_> = patches.into_iter().map(hatch).collect();
    assert_eq!(inner, [None, Some("x".into())]);
    let ends: Vec<_> = extensions.into_iter().map(hatch).collect();
    assert_eq!(ends, [Some("/".into()), Some("/".into())]);
}

#[test]
fn removed_mappables_keep_their_colorbar() {
    // matplotlib's colorbar reads the mappable wherever it is: its
    // advanced hillshading example colours a shaded image with the
    // colorbar of an image it removed.
    let (fig, ax, image) = image_figure();
    image.remove();
    let colorbar = fig.colorbar(&image, ColorbarOptions::beside(&ax)).unwrap();
    assert_eq!((colorbar.vmin(), colorbar.vmax()), (-0.5, 0.5));
}

#[test]
fn explicit_contour_colours_index_the_colorbar_colormap() {
    // matplotlib's test_given_colors_levels_and_extends: the integer
    // values of a `NoNorm` contour set pick their colours directly.
    let fig = Figure::new();
    let ax = fig.subplot();
    let field = QuadField::new(Grid::from_fn(3, 4, |i, j| (i * 4 + j) as f64));
    let names = ["yellow", "pink", "blue", "black"];
    let options = ContourOptions {
        levels: Levels::Values(vec![2.0, 4.0, 8.0, 10.0]),
        colors: Some(names.iter().map(|&n| Color::from(n)).collect()),
        extend: Extend::Max,
        ..ContourOptions::default()
    };
    let fills = ax.contourf(&field, options).unwrap();
    let colorbar = fig.colorbar(&fills, ColorbarOptions::default()).unwrap();
    let id = colorbar.read(|state| state.drawn.solids).expect("filled");
    let figure = colorbar.ax().raw().figure_state().borrow();
    let faces = figure
        .store
        .get_as::<CollectionArtist>(id)
        .map(|solids| solids.resolved_colors().unwrap().0)
        .unwrap();
    let expected: Vec<_> = names[..3]
        .iter()
        .map(|&n| Color::from(n).resolve().unwrap())
        .collect();
    assert_eq!(faces, expected);
}

#[test]
fn singular_limits_widened_by_the_colorbar_reach_the_mappable() {
    // matplotlib's colorbar changes the norm it shares with the mappable
    // (test_colorbar_single_scatter).
    let fig = Figure::new();
    let ax = fig.subplot();
    let options = ScatterOptions {
        colors: ScatterColors::from(MappedColors::new(vec![50.0])),
        ..ScatterOptions::default()
    };
    let points = ax.scatter_with(vec![0.0], vec![0.0], options).unwrap();
    assert_eq!(points.clim(), (Some(50.0), Some(50.0)));
    fig.colorbar(&points, ColorbarOptions::default()).unwrap();
    let (vmin, vmax) = points.clim();
    assert!((vmin.unwrap() - 45.0).abs() < 1e-9);
    assert!((vmax.unwrap() - 55.0).abs() < 1e-9);
}

#[test]
fn surfaces_of_3d_axes_take_room_from_the_3d_axes() {
    use crate::mplot3d::{FaceColors, SurfaceOptions};
    // matplotlib's test_surface3d: the colorbar maps the surface's mean
    // heights and steals its room from the 3-D axes, whose square box
    // keeps to the right of what is left.
    let fig = Figure::new();
    let ax = fig.subplot_3d();
    let x = Grid::from_fn(3, 3, |_, c| c as f64);
    let y = Grid::from_fn(3, 3, |r, _| r as f64);
    let z = Grid::from_fn(3, 3, |r, c| (r + c) as f64);
    let options = SurfaceOptions {
        colors: FaceColors::from(
            ColorizerOptions::default().cmap(colormaps::get("coolwarm").unwrap()),
        ),
        ..SurfaceOptions::default()
    };
    let surface = ax.plot_surface(&x, &y, &z, options).unwrap();
    let colorbar = fig.colorbar(&surface, ColorbarOptions::default()).unwrap();
    fig.layout();
    assert_eq!(colorbar.ax().ylim(), (1.0, 3.0));
    assert!(close(ax.position().bounds(), [0.1675, 0.11, 0.5775, 0.77]));
    assert!(close(
        colorbar.ax().position().bounds(),
        [0.783_75, 0.11, 0.028_875, 0.77]
    ));
}

#[test]
fn update_normal_follows_another_mappable_from_then_on() {
    let (fig, ax, image) = image_figure();
    let colorbar = fig.colorbar(&image, ColorbarOptions::default()).unwrap();
    colorbar.set_ticks(vec![0.0, 1.0]);
    colorbar.update_normal(&image).unwrap();
    assert_eq!(
        colorbar.long_axis().ticklocs(false).unwrap(),
        [0.0, 1.0],
        "the same artist keeps the ticks"
    );
    let other = ax
        .imshow(
            Grid::from_fn(2, 2, |r, c| 10.0 * (r + c) as f64),
            ImshowOptions::default(),
        )
        .unwrap();
    colorbar.update_normal(&other).unwrap();
    assert_eq!(colorbar.ax().ylim(), (0.0, 20.0));
    assert_ne!(colorbar.long_axis().ticklocs(false).unwrap(), [0.0, 1.0]);
    other.set_clim(Some(5.0), Some(15.0));
    assert_eq!(colorbar.ax().ylim(), (5.0, 15.0), "it follows the new one");
    image.set_clim(Some(-1.0), Some(1.0));
    assert_eq!(colorbar.ax().ylim(), (5.0, 15.0), "and no longer the first");
}

#[test]
fn update_normal_resets_the_ticks_of_a_bare_mapping_for_a_new_kind_of_norm() {
    let fig = Figure::new();
    let ax = fig.subplot();
    let viridis = colormaps::get("viridis").unwrap();
    let mut sm = ScalarMappable::with_norm(viridis, Box::new(Normalize::new(1.0, 10.0)));
    let colorbar = fig.colorbar(&sm, ColorbarOptions::beside(&ax)).unwrap();
    colorbar.set_ticks(vec![2.0, 4.0]);
    sm.set_clim(Some(2.0), Some(8.0));
    colorbar.update_normal(&sm).unwrap();
    assert_eq!(colorbar.ax().ylim(), (2.0, 8.0));
    assert_eq!(colorbar.long_axis().ticklocs(false).unwrap(), [2.0, 4.0]);
    sm.set_norm(Box::new(LogNorm::new(1.0, 100.0)));
    colorbar.update_normal(&sm).unwrap();
    assert_eq!(colorbar.ax().yscale(), ScaleName::Log);
    assert_ne!(colorbar.long_axis().ticklocs(false).unwrap(), [2.0, 4.0]);
}
