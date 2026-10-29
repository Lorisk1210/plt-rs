//! Contours, triangulations and vector fields on the current axes
//! (matplotlib's `contour`, `contourf`, `clabel`, `tricontour`,
//! `tricontourf`, `tripcolor`, `triplot`, `quiver`, `quiverkey`, `barbs`
//! and `streamplot`). The plots that colour-map become the current image.

use super::mappables::make_current;
use super::registry::gca;
use crate::axes::{
    PlotError, TriValues, Tripcolor, TripcolorError, TripcolorOptions, TriplotLines,
};
use crate::collections::Collection;
use crate::contour::{ClabelOptions, ContourError, ContourOptions, ContourSet, QuadField};
use crate::lines::LineProps;
use crate::numeric::Number;
use crate::quiver::{
    BarbsOptions, Quiver, QuiverKey, QuiverKeyOptions, QuiverOptions, VectorField,
};
use crate::streamplot::{StreamField, StreamplotError, StreamplotOptions, StreamplotSet};
use crate::text::Text;
use crate::tri::Triangulation;

/// Draws contour lines of a gridded field on the current axes and makes
/// them the current image (matplotlib's `contour`).
///
/// # Errors
///
/// As [`Axes::contour`](crate::axes::Axes::contour).
pub fn contour(field: &QuadField, options: ContourOptions) -> Result<ContourSet, ContourError> {
    let ax = gca();
    let contours = ax.contour(field, options)?;
    make_current(&ax, &contours);
    Ok(contours)
}

/// Draws filled contours of a gridded field on the current axes and makes
/// them the current image (matplotlib's `contourf`).
///
/// # Errors
///
/// As [`Axes::contourf`](crate::axes::Axes::contourf).
pub fn contourf(field: &QuadField, options: ContourOptions) -> Result<ContourSet, ContourError> {
    let ax = gca();
    let contours = ax.contourf(field, options)?;
    make_current(&ax, &contours);
    Ok(contours)
}

/// Labels the contour lines of `contours` (matplotlib's `clabel`, which is
/// `ContourSet.clabel`). It takes the contour set, as matplotlib's pyplot
/// `clabel(CS)` does, where [`ContourSet::clabel`] is a method of it.
///
/// # Errors
///
/// As [`ContourSet::clabel`].
pub fn clabel(contours: &ContourSet, options: ClabelOptions) -> Result<Vec<Text>, ContourError> {
    contours.clabel(options)
}

/// Draws contour lines of values on a triangulation on the current axes
/// and makes them the current image (matplotlib's `tricontour`).
///
/// # Errors
///
/// As [`Axes::tricontour`](crate::axes::Axes::tricontour).
pub fn tricontour(
    tri: &Triangulation,
    z: impl IntoIterator<Item: Number>,
    options: ContourOptions,
) -> Result<ContourSet, ContourError> {
    let ax = gca();
    let contours = ax.tricontour(tri, z, options)?;
    make_current(&ax, &contours);
    Ok(contours)
}

/// Draws filled contours of values on a triangulation on the current axes
/// and makes them the current image (matplotlib's `tricontourf`).
///
/// # Errors
///
/// As [`Axes::tricontourf`](crate::axes::Axes::tricontourf).
pub fn tricontourf(
    tri: &Triangulation,
    z: impl IntoIterator<Item: Number>,
    options: ContourOptions,
) -> Result<ContourSet, ContourError> {
    let ax = gca();
    let contours = ax.tricontourf(tri, z, options)?;
    make_current(&ax, &contours);
    Ok(contours)
}

/// Draws a pseudocolour plot of a triangulation on the current axes and
/// makes it the current image (matplotlib's `tripcolor`).
///
/// # Errors
///
/// As [`Axes::tripcolor`](crate::axes::Axes::tripcolor).
pub fn tripcolor(
    tri: &Triangulation,
    values: TriValues,
    options: TripcolorOptions,
) -> Result<Tripcolor, TripcolorError> {
    let ax = gca();
    let plot = ax.tripcolor(tri, values, options)?;
    make_current(&ax, &plot);
    Ok(plot)
}

/// Draws the edges and nodes of a triangulation on the current axes
/// (matplotlib's `triplot`).
///
/// # Errors
///
/// As [`Axes::triplot`](crate::axes::Axes::triplot).
pub fn triplot(
    tri: &Triangulation,
    format: &str,
    props: LineProps,
) -> Result<TriplotLines, PlotError> {
    gca().triplot(tri, format, props)
}

/// Draws a field of arrows on the current axes and makes them the current
/// image (matplotlib's `quiver`).
///
/// # Errors
///
/// As [`Axes::quiver`](crate::axes::Axes::quiver).
pub fn quiver(field: VectorField, options: QuiverOptions) -> Result<Quiver, PlotError> {
    let ax = gca();
    let arrows = ax.quiver(field, options)?;
    make_current(&ax, arrows.collection());
    Ok(arrows)
}

/// Adds a key arrow of `length` labelled `label` at `position` for a
/// quiver plot on the current axes (matplotlib's `quiverkey`).
pub fn quiverkey(
    quiver: &Quiver,
    position: (f64, f64),
    length: f64,
    label: impl Into<String>,
    options: QuiverKeyOptions,
) -> QuiverKey {
    gca().quiverkey(quiver, position, length, label, options)
}

/// Draws wind barbs on the current axes (matplotlib's `barbs`).
///
/// # Errors
///
/// As [`Axes::barbs`](crate::axes::Axes::barbs).
pub fn barbs(field: VectorField, options: BarbsOptions) -> Result<Collection, PlotError> {
    gca().barbs(field, options)
}

/// Draws streamlines of a vector field on the current axes and makes them
/// the current image (matplotlib's `streamplot`).
///
/// # Errors
///
/// As [`Axes::streamplot`](crate::axes::Axes::streamplot).
pub fn streamplot(
    field: &StreamField,
    options: StreamplotOptions,
) -> Result<StreamplotSet, StreamplotError> {
    let ax = gca();
    let streams = ax.streamplot(field, options)?;
    make_current(&ax, &streams.lines);
    Ok(streams)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::ColorizingArtist;
    use crate::axes::TriShading;
    use crate::numeric::{Grid, meshgrid};
    use crate::pyplot::{clim, close_all, gci};

    fn field() -> QuadField {
        QuadField::new(Grid::from_fn(5, 5, |row, col| (row * col) as f64))
    }

    fn triangulation() -> Triangulation {
        Triangulation::new(
            &[0.0, 1.0, 0.0, 1.0],
            &[0.0, 0.0, 1.0, 1.0],
            vec![[0, 1, 2], [1, 3, 2]],
        )
        .unwrap()
    }

    #[test]
    fn contours_become_current_and_take_labels() {
        close_all();
        contourf(&field(), ContourOptions::default()).unwrap();
        let lines = contour(&field(), ContourOptions::default()).unwrap();
        let Some(ColorizingArtist::Contour(current)) = gci() else {
            panic!("the line contours are current");
        };
        assert_eq!(current.collections(), lines.collections());
        let labels = clabel(&lines, ClabelOptions::default()).unwrap();
        assert_ne!(labels.len(), 0);
        close_all();
    }

    #[test]
    fn triangulations() {
        close_all();
        let tri = triangulation();
        let z = [0.0, 1.0, 1.0, 2.0];
        tricontour(&tri, z, ContourOptions::default()).unwrap();
        tricontourf(&tri, z, ContourOptions::default()).unwrap();
        let Tripcolor::Flat(cells) = tripcolor(
            &tri,
            TriValues::Auto(z.to_vec()),
            TripcolorOptions::default(),
        )
        .unwrap() else {
            panic!("flat shading by default");
        };
        assert!(matches!(gci(), Some(ColorizingArtist::Collection(c)) if c == cells));
        let gouraud = TripcolorOptions {
            shading: TriShading::Gouraud,
            ..TripcolorOptions::default()
        };
        let Tripcolor::Gouraud(mesh) =
            tripcolor(&tri, TriValues::Auto(z.to_vec()), gouraud).unwrap()
        else {
            panic!("Gouraud shading draws a mesh");
        };
        assert!(matches!(gci(), Some(ColorizingArtist::TriMesh(m)) if m == mesh));
        clim(Some(0.0), Some(4.0)).unwrap();
        assert_eq!(mesh.clim(), (Some(0.0), Some(4.0)));
        triplot(&tri, "k-", LineProps::default()).unwrap();
        close_all();
    }

    #[test]
    fn vector_fields() {
        close_all();
        let (x, y) = meshgrid(&[0.0, 1.0, 2.0], &[0.0, 1.0]);
        let arrows = quiver(
            VectorField::on_mesh(&x, &y, &x, &y),
            QuiverOptions::default(),
        )
        .unwrap();
        assert!(
            matches!(gci(), Some(ColorizingArtist::Collection(c)) if &c == arrows.collection())
        );
        quiverkey(
            &arrows,
            (0.9, 0.9),
            1.0,
            "1 m/s",
            QuiverKeyOptions::default(),
        );
        barbs(
            VectorField::on_mesh(&x, &y, &x, &y),
            BarbsOptions::default(),
        )
        .unwrap();
        let stream = StreamField::new(vec![0.0, 1.0, 2.0], vec![0.0, 1.0], x, y);
        let streams = streamplot(&stream, StreamplotOptions::default()).unwrap();
        assert!(matches!(gci(), Some(ColorizingArtist::Collection(c)) if c == streams.lines));
        close_all();
    }
}
