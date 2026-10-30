//! matplotlib's class for each kind of pltrs (Rust) artist, which the
//! package gives the object of an artist it has not seen before
//! (`_handles.wrap_any`: the children of an axes, say).

use pltrs::artist::{RawHandle, TypedHandle};
use pltrs::axes::Axes;
use pltrs::collections::{Collection, CollectionKind, EllipseCollection, EventCollection, TriMesh};
use pltrs::contour::ContourSet;
use pltrs::image::{AxesImage, BboxImage, PcolorImage};
use pltrs::legend::Legend;
use pltrs::lines::Line2D;
use pltrs::patches::{
    Annulus, Arc, Arrow, Circle, CirclePolygon, ConnectionPatch, Ellipse, FancyArrow,
    FancyArrowPatch, FancyBboxPatch, Patch, PathShape, Polygon, Rectangle, RegularPolygon,
    StepPatch, Wedge,
};
use pltrs::quiver::{Quiver, QuiverKey};
use pltrs::spines::Spine;
use pltrs::table::Table;
use pltrs::text::{Annotation, Text};

use crate::binding::prelude::*;

/// A test of the kind of an artist, with the name of matplotlib's class
/// for the artists passing it.
type ClassTest = (fn(&RawHandle) -> bool, &'static str);

/// Whether `raw` reaches an artist of handle type `H`.
fn is<H: TypedHandle>(raw: &RawHandle) -> bool {
    H::from_raw_handle(raw).is_some()
}

/// matplotlib's class for a collection of `kind`.
fn collection_class(kind: CollectionKind) -> &'static str {
    match kind {
        CollectionKind::Generic => "Collection",
        CollectionKind::Patch => "PatchCollection",
        CollectionKind::Path => "PathCollection",
        CollectionKind::Poly => "PolyCollection",
        CollectionKind::Line => "LineCollection",
        CollectionKind::QuadMesh => "QuadMesh",
    }
}

/// matplotlib's class for a patch, by shape; `None` for an artist that is
/// no patch of these shapes.
fn patch_class(raw: &RawHandle) -> Option<&'static str> {
    let classes: [ClassTest; 16] = [
        (is::<Patch<Rectangle>>, "Rectangle"),
        (is::<Patch<Ellipse>>, "Ellipse"),
        (is::<Patch<Circle>>, "Circle"),
        (is::<Patch<Polygon>>, "Polygon"),
        (is::<Patch<Wedge>>, "Wedge"),
        (is::<Patch<PathShape>>, "PathPatch"),
        (is::<Patch<RegularPolygon>>, "RegularPolygon"),
        (is::<Patch<CirclePolygon>>, "CirclePolygon"),
        (is::<Patch<StepPatch>>, "StepPatch"),
        (is::<Patch<FancyArrow>>, "FancyArrow"),
        (is::<Patch<Arrow>>, "Arrow"),
        (is::<Patch<Arc>>, "Arc"),
        (is::<Patch<Annulus>>, "Annulus"),
        (is::<Patch<FancyBboxPatch>>, "FancyBboxPatch"),
        (is::<Patch<FancyArrowPatch>>, "FancyArrowPatch"),
        (is::<Patch<ConnectionPatch>>, "ConnectionPatch"),
    ];
    classes
        .into_iter()
        .find_map(|(test, class)| test(raw).then_some(class))
}

/// The name of matplotlib's class for the artist `raw` reaches: `"Line2D"`,
/// `"Annotation"`, `"PathCollection"`, `"Rectangle"`, ..., or `"Artist"`
/// for a kind matplotlib has no more specific class for.
fn class_name(raw: &RawHandle) -> &'static str {
    // The collections of a class of their own come before the others,
    // which are told apart by their kind.
    let classes: [ClassTest; 15] = [
        (is::<Line2D>, "Line2D"),
        (is::<Annotation>, "Annotation"),
        (is::<Text>, "Text"),
        (is::<PcolorImage>, "PcolorImage"),
        (is::<AxesImage>, "AxesImage"),
        (is::<BboxImage>, "BboxImage"),
        (is::<EventCollection>, "EventCollection"),
        (is::<EllipseCollection>, "EllipseCollection"),
        (is::<Quiver>, "Quiver"),
        (is::<TriMesh>, "TriMesh"),
        (is::<QuiverKey>, "QuiverKey"),
        (is::<ContourSet>, "QuadContourSet"),
        (is::<Legend>, "Legend"),
        (is::<Table>, "Table"),
        (is::<Spine>, "Spine"),
    ];
    classes
        .into_iter()
        .find_map(|(test, class)| test(raw).then_some(class))
        .or_else(|| patch_class(raw))
        .or_else(|| Collection::from_raw_handle(raw).map(|c| collection_class(c.kind())))
        .unwrap_or_else(|| if is::<Axes>(raw) { "Axes" } else { "Artist" })
}

/// The name of matplotlib's class for the artist `artist` (see
/// [`class_name`]).
fn get_class_name(artist: RawHandle) -> PyResult<&'static str> {
    Ok(class_name(&artist))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("class_name", get_class_name)
}

#[cfg(test)]
mod tests {
    use pltrs::artist::ArtistHandle;
    use pltrs::figure::Figure;

    use super::*;

    #[test]
    fn artists_are_named_by_matplotlibs_classes() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let line = ax.plot([0.0, 1.0], [0.0, 1.0]).expect("equal lengths");
        let dots = ax.scatter(vec![0.0], vec![1.0]).expect("equal lengths");
        let note = ax.text(0.5, 0.5, "note");
        assert_eq!(class_name(line.raw()), "Line2D");
        assert_eq!(class_name(dots.raw()), "PathCollection");
        assert_eq!(class_name(note.raw()), "Text");
        assert_eq!(class_name(ax.raw()), "Axes");
        assert_eq!(class_name(fig.raw()), "Artist");
    }
}
