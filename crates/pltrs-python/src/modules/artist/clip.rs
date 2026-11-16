//! Clipping (matplotlib's `Artist.set_clip_path` and
//! `Artist.set_clip_box`): the clip regions of any artist.
//!
//! Python describes a clip path as a tuple, the first item naming its
//! kind: `("path", path, transform)` for a `Path` in the coordinates of a
//! transform, `("artist", patch)` for the outline of a patch of the same
//! figure, followed as it changes, and `("bbox", path, transform)` for the
//! bounding box of a path, which matplotlib makes of a `Rectangle`.

use pltrs::artist::{ArtistHandle, ClipPathSpec, RawHandle};
use pltrs::axes::Axes;
use pltrs::transforms::Transform;
use pltrs_core::Path;

use super::super::path::PyPath;
use crate::binding::prelude::*;

/// A clip path described by Python (see the module documentation).
pub(crate) struct ClipSpec(pub(crate) ClipPathSpec);

impl FromPy<'_> for ClipSpec {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let items: Vec<Bound<'_>> = object.extract()?;
        let kind: String = items
            .first()
            .ok_or_else(|| PyErr::value_error("an empty clip path description"))?
            .extract()?;
        let spec = match (kind.as_str(), &items[1..]) {
            ("path", [path, transform]) => ClipPathSpec::Path {
                path: path.extract::<PyPath>()?.path,
                transform: transform.extract()?,
            },
            ("bbox", [path, transform]) => ClipPathSpec::BoundingBox {
                path: path.extract::<PyPath>()?.path,
                transform: transform.extract()?,
            },
            ("artist", [patch]) => ClipPathSpec::Artist(patch.extract::<RawHandle>()?.id()),
            _ => {
                return Err(PyErr::value_error(format!(
                    "a malformed clip path description of kind {kind:?}"
                )));
            }
        };
        Ok(Self(spec))
    }
}

/// Clips `artist` to the region `spec` describes, or to none.
fn set_clip_path(artist: RawHandle, spec: Option<ClipSpec>) -> PyResult<()> {
    artist.set_clip_path(spec.map(|spec| spec.0));
    Ok(())
}

/// Whether the clip path of `artist` is the outline of another artist,
/// which matplotlib keeps as a `TransformedPatchPath`.
fn clips_to_artist(artist: RawHandle) -> PyResult<bool> {
    Ok(matches!(artist.clip_path(), Some(ClipPathSpec::Artist(_))))
}

/// Clips `artist` to the box a live transform maps the unit square to, or
/// to no box: the box of an axes or a figure it follows as they are laid
/// out (`ax.transAxes` for `ax.bbox`, a figure's `transSubfigure` for its
/// `bbox`), and any other box as the unit square through its transform
/// (`fig.bbox`, whose transform a print cut to its tight box leaves at the
/// canvas origin).
fn set_clip_box(artist: RawHandle, spanning: Option<Transform>) -> PyResult<()> {
    match spanning {
        None => {
            artist.set_clip_box(None);
        }
        Some(Transform::Axes(owner) | Transform::Figure(owner)) => {
            artist.set_clip_box(Some(owner));
        }
        Some(transform) => {
            artist.set_clip_region(Some(ClipPathSpec::BoundingBox {
                path: Path::unit_rectangle(),
                transform,
            }));
        }
    }
    Ok(())
}

/// Clips `artist` to the bounding box Python describes as `("bbox", path,
/// transform)` in place of its clip box (a `TransformedBbox` or a `Bbox` in
/// pixels given to `set_clip_box`, a `Rectangle` given to
/// `set_clip_path`), leaving its clip path.
fn set_clip_region(artist: RawHandle, region: ClipSpec) -> PyResult<()> {
    match region.0 {
        spec @ ClipPathSpec::BoundingBox { .. } => {
            artist.set_clip_region(Some(spec));
            Ok(())
        }
        _ => Err(PyErr::value_error(
            "a clip box is the bounding box of a path",
        )),
    }
}

/// Whether the artist is clipped to the box of an axes or a figure, and
/// the box's transform from the unit square (`get_clip_box`).
fn get_clip_box(artist: RawHandle) -> PyResult<Option<Transform>> {
    Ok(artist.clip_box().map(|owner| {
        if artist.figure().id() == owner || artist.parent_figure().id() == owner {
            Transform::Figure(owner)
        } else {
            Transform::Axes(owner)
        }
    }))
}

/// Whether `axes` clips what it clips to another region than its box (a
/// patch of its own, polar and geographic axes; `_artist_clipping`).
fn clips_to_patch_shape(axes: Axes) -> PyResult<bool> {
    Ok(axes.clips_to_patch_shape())
}

/// Whether the artist is clipped to exactly its axes
/// (`_fully_clipped_to_axes`).
fn fully_clipped_to_axes(artist: RawHandle) -> PyResult<bool> {
    Ok(artist.fully_clipped_to_axes())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("fully_clipped_to_axes", fully_clipped_to_axes)?;
    module.add("clips_to_patch_shape", clips_to_patch_shape)?;
    module.add("set_clip_path", set_clip_path)?;
    module.add("clips_to_artist", clips_to_artist)?;
    module.add("set_clip_box", set_clip_box)?;
    module.add("set_clip_region", set_clip_region)?;
    module.add("get_clip_box", get_clip_box)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{extension, with_python};
    use pltrs::figure::Figure;

    #[test]
    fn clip_boxes_name_their_axes_or_figure() {
        with_python(|py| {
            extension(py);
            let fig = Figure::new();
            let ax = fig.subplot();
            let line = ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
            let raw = line.raw().clone();
            set_clip_box(raw.clone(), Some(Transform::Axes(ax.id()))).unwrap();
            assert_eq!(
                get_clip_box(raw.clone()).unwrap(),
                Some(Transform::Axes(ax.id()))
            );
            set_clip_box(raw.clone(), Some(Transform::Figure(fig.id()))).unwrap();
            assert_eq!(
                get_clip_box(raw.clone()).unwrap(),
                Some(Transform::Figure(fig.id()))
            );
            set_clip_box(raw.clone(), None).unwrap();
            assert_eq!(get_clip_box(raw.clone()).unwrap(), None);
            // Any other box clips as a region of its own, in place of the
            // axes' or figure's box.
            set_clip_box(raw.clone(), Some(Transform::Display)).unwrap();
            assert_eq!(get_clip_box(raw.clone()).unwrap(), None);
            assert!(matches!(
                raw.clip_region(),
                Some(ClipPathSpec::BoundingBox { .. })
            ));
            let err = set_clip_region(raw, ClipSpec(ClipPathSpec::Artist(ax.id()))).unwrap_err();
            assert_eq!(err.class_name(py), "ValueError");
        });
    }
}
