//! Clipping (matplotlib's `Artist.set_clip_path` and
//! `Artist.set_clip_box`): the clip regions of any artist.
//!
//! Python describes a clip path as a tuple, the first item naming its
//! kind: `("path", path, transform)` for a `Path` in the coordinates of a
//! transform, `("artist", patch)` for the outline of a patch of the same
//! figure, followed as it changes, and `("bbox", path, transform)` for the
//! bounding box of a path, which matplotlib makes of a `Rectangle`.

use pltrs::artist::{ArtistHandle, ClipPathSpec, RawHandle};
use pltrs::transforms::Transform;

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

/// Clips `artist` to the box a live transform maps the unit square to
/// (`ax.transAxes` for `ax.bbox`, a figure's `transSubfigure` for its
/// `bbox`), or to no box. Other transforms span boxes the artist cannot
/// follow, which raise `ValueError`.
fn set_clip_box(artist: RawHandle, spanning: Option<Transform>) -> PyResult<()> {
    let owner = match spanning {
        None => None,
        Some(Transform::Axes(owner) | Transform::Figure(owner)) => Some(owner),
        Some(other) => {
            return Err(PyErr::value_error(format!(
                "a clip box spanned by {other:?} is not the box of an axes or a figure"
            )));
        }
    };
    artist.set_clip_box(owner);
    Ok(())
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

/// Registers the module's functions.
/// Whether the artist is clipped to exactly its axes
/// (`_fully_clipped_to_axes`).
fn fully_clipped_to_axes(artist: RawHandle) -> PyResult<bool> {
    Ok(artist.fully_clipped_to_axes())
}

pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("fully_clipped_to_axes", fully_clipped_to_axes)?;
    module.add("set_clip_path", set_clip_path)?;
    module.add("clips_to_artist", clips_to_artist)?;
    module.add("set_clip_box", set_clip_box)?;
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
            let err = set_clip_box(raw, Some(Transform::Display)).unwrap_err();
            assert_eq!(err.class_name(py), "ValueError");
        });
    }
}
