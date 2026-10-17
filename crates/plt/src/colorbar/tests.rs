//! Colorbars built end to end, checked against values from matplotlib.

use super::*;
use crate::axes::{ImshowOptions, MappedColors, ScatterColors, ScatterOptions};
use crate::colors::{BoundaryNorm, Normalize, colormaps};
use crate::numeric::Grid;
use crate::spines::SpineSide;

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
    assert_eq!(colorbar.ax().yscale(), "function");
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
fn a_cax_is_used_as_given() {
    let fig = Figure::new();
    let cax = fig.add_axes([0.85, 0.1, 0.05, 0.8]);
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
}
