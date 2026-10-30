//! `_native.artist`: what every artist has (matplotlib's `Artist`), on
//! the native object of any artist handle, and artists made outside any
//! figure: their adoption by an axes or a figure (matplotlib's
//! `add_artist`, `add_line`, `add_patch`, `add_image`, `add_collection`).

use pltrs::artist::{
    Artist, ArtistBase, ArtistHandle, AttachError, Custom, DrawContext, RawHandle,
};
use pltrs::axes::{AddAs, Axes};
use pltrs::figure::Figure;
use pltrs_render::Sketch;

use crate::binding::prelude::*;

mod clip;
mod extent;
mod filter;
mod picking;

pub(super) use clip::ClipSpec;
pub(super) use picking::PyPicker;

impl From<AttachError> for PyErr {
    /// matplotlib's `RuntimeError` for an artist of another figure
    /// (`Artist.set_figure`), its `ValueError` for one already in an axes
    /// (the `Artist.axes` setter), `ValueError` otherwise.
    fn from(err: AttachError) -> Self {
        match err {
            AttachError::OtherFigure => {
                PyErr::runtime_error("Can not put single artist in more than one figure")
            }
            AttachError::NotRemoved => PyErr::value_error(
                "Can not reset the Axes. You are probably trying to reuse an artist in more \
                 than one Axes which is not supported",
            ),
            other => PyErr::value_error(other.to_string()),
        }
    }
}

/// The artist of a bare `matplotlib.artist.Artist()`: the common
/// properties, drawing nothing.
#[derive(Clone, Debug, Default)]
struct PlainArtist {
    base: ArtistBase,
}

impl Artist for PlainArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, _ctx: &mut DrawContext<'_>) {}
}

/// A bare artist outside any figure (`Artist()`).
fn new() -> PyResult<RawHandle> {
    Ok(Custom::loose(PlainArtist::default()).raw().clone())
}

/// Whether the artist was made outside any figure and is not added yet.
fn is_loose(artist: RawHandle) -> PyResult<bool> {
    Ok(artist.is_loose())
}

/// Adds a ready-made artist to `axes` as matplotlib's `add_<kind>` does
/// (`"artist"`, `"line"`, `"patch"`, `"image"` or `"collection"`, the
/// last with `autolim`), returning its handle in the axes' figure.
fn adopt_into_axes(
    axes: Axes,
    artist: RawHandle,
    kind: String,
    autolim: bool,
) -> PyResult<RawHandle> {
    let add_as = match kind.as_str() {
        "artist" => AddAs::Artist,
        "line" => AddAs::Line,
        "patch" => AddAs::Patch,
        "image" => AddAs::Image,
        "collection" => AddAs::Collection { autolim },
        _ => return Err(PyErr::value_error(format!("no add_{kind} method"))),
    };
    Ok(axes.adopt_artist(&artist, add_as)?)
}

/// Adds a ready-made artist to `figure` (a figure or subfigure;
/// `Figure.add_artist(artist, clip)`), returning its handle there.
fn adopt_into_figure(figure: Figure, artist: RawHandle, clip: bool) -> PyResult<RawHandle> {
    Ok(figure.adopt_artist(&artist, clip)?)
}

/// The figure or subfigure the artist is directly in
/// (`get_figure(root=False)`).
fn parent_figure(artist: RawHandle) -> PyResult<Figure> {
    Ok(artist.parent_figure())
}

/// The axes the artist is drawn in, if any (`artist.axes`).
fn axes(artist: RawHandle) -> PyResult<Option<Axes>> {
    Ok(artist.axes())
}

/// The artist's id in its figure: how pltrs's Python classes find the
/// one object they keep per artist.
fn id(artist: RawHandle) -> PyResult<u32> {
    Ok(artist.id().index())
}

/// Whether the artist is drawn.
fn get_visible(artist: RawHandle) -> PyResult<bool> {
    Ok(artist.visible())
}

/// Shows or hides the artist.
fn set_visible(artist: RawHandle, visible: bool) -> PyResult<()> {
    artist.set_visible(visible);
    Ok(())
}

/// The drawing order.
fn get_zorder(artist: RawHandle) -> PyResult<f64> {
    Ok(artist.zorder())
}

/// Sets the drawing order.
fn set_zorder(artist: RawHandle, zorder: f64) -> PyResult<()> {
    artist.set_zorder(zorder);
    Ok(())
}

/// The opacity override, `None` when unset.
fn get_alpha(artist: RawHandle) -> PyResult<Option<f64>> {
    Ok(artist.alpha())
}

/// Sets the opacity override (`None` removes it).
fn set_alpha(artist: RawHandle, alpha: Option<f64>) -> PyResult<()> {
    artist.set_alpha(alpha);
    Ok(())
}

/// The legend label.
fn get_label(artist: RawHandle) -> PyResult<String> {
    Ok(artist.label())
}

/// Sets the legend label.
fn set_label(artist: RawHandle, label: String) -> PyResult<()> {
    artist.set_label(label);
    Ok(())
}

/// Declares a boolean property of every artist: a getter and a setter
/// of `ArtistHandle` reached by name.
macro_rules! flags {
    ($($(#[$doc:meta])* $get:ident, $set:ident => $field:ident, $set_field:ident;)*) => {$(
        $(#[$doc])*
        fn $get(artist: RawHandle) -> PyResult<bool> {
            Ok(artist.$field())
        }

        $(#[$doc])*
        fn $set(artist: RawHandle, on: bool) -> PyResult<()> {
            artist.$set_field(on);
            Ok(())
        }
    )*};
}

flags! {
    /// Whether the artist is clipped (`clip_on`).
    get_clip_on, set_clip_on => clip_on, set_clip_on;
    /// Whether the artist is rasterized in vector output (`rasterized`).
    get_rasterized, set_rasterized => rasterized, set_rasterized;
    /// Whether the artist is left out of normal draws (`animated`).
    get_animated, set_animated => animated, set_animated;
    /// Whether layout engines account for the artist (`in_layout`).
    get_in_layout, set_in_layout => in_layout, set_in_layout;
    /// Whether a status bar shows the data under the pointer (`mouseover`).
    get_mouseover, set_mouseover => mouseover, set_mouseover;
}

/// The pixel snapping preference, `None` letting the renderer decide.
fn get_snap(artist: RawHandle) -> PyResult<Option<bool>> {
    Ok(artist.snap())
}

/// Sets the pixel snapping preference.
fn set_snap(artist: RawHandle, snap: Option<bool>) -> PyResult<()> {
    artist.set_snap(snap);
    Ok(())
}

/// The group id used in vector output (`get_gid`).
fn get_gid(artist: RawHandle) -> PyResult<Option<String>> {
    Ok(artist.gid())
}

/// Sets the group id used in vector output, `None` removing it.
fn set_gid(artist: RawHandle, gid: Option<String>) -> PyResult<()> {
    match gid {
        Some(gid) => artist.set_gid(gid),
        None => artist.clear_gid(),
    };
    Ok(())
}

/// The hyperlink used in vector output (`get_url`).
fn get_url(artist: RawHandle) -> PyResult<Option<String>> {
    Ok(artist.url())
}

/// Sets the hyperlink used in vector output, `None` removing it.
fn set_url(artist: RawHandle, url: Option<String>) -> PyResult<()> {
    match url {
        Some(url) => artist.set_url(url),
        None => artist.clear_url(),
    };
    Ok(())
}

/// The sketch parameters `(scale, length, randomness)`, if any
/// (`get_sketch_params`).
fn get_sketch_params(artist: RawHandle) -> PyResult<Option<(f64, f64, f64)>> {
    Ok(artist
        .sketch_params()
        .map(|sketch| (sketch.scale, sketch.length, sketch.randomness)))
}

/// Draws the artist's lines wiggled with `(scale, length, randomness)`,
/// or straight for `None` (`set_sketch_params`).
fn set_sketch_params(artist: RawHandle, sketch: Option<(f64, f64, f64)>) -> PyResult<()> {
    artist.set_sketch_params(sketch.map(|(scale, length, randomness)| Sketch {
        scale,
        length,
        randomness,
    }));
    Ok(())
}

/// Removes the artist from its figure.
fn remove(artist: RawHandle) -> PyResult<()> {
    artist.remove();
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("new", new)?;
    module.add("is_loose", is_loose)?;
    module.add("adopt_into_axes", adopt_into_axes)?;
    module.add("adopt_into_figure", adopt_into_figure)?;
    module.add("axes", axes)?;
    module.add("parent_figure", parent_figure)?;
    module.add("id", id)?;
    module.add("get_clip_on", get_clip_on)?;
    module.add("set_clip_on", set_clip_on)?;
    module.add("get_rasterized", get_rasterized)?;
    module.add("set_rasterized", set_rasterized)?;
    module.add("get_animated", get_animated)?;
    module.add("set_animated", set_animated)?;
    module.add("get_in_layout", get_in_layout)?;
    module.add("set_in_layout", set_in_layout)?;
    module.add("get_mouseover", get_mouseover)?;
    module.add("set_mouseover", set_mouseover)?;
    module.add("get_snap", get_snap)?;
    module.add("set_snap", set_snap)?;
    module.add("get_visible", get_visible)?;
    module.add("set_visible", set_visible)?;
    module.add("get_zorder", get_zorder)?;
    module.add("set_zorder", set_zorder)?;
    module.add("get_alpha", get_alpha)?;
    module.add("set_alpha", set_alpha)?;
    module.add("get_label", get_label)?;
    module.add("set_label", set_label)?;
    module.add("get_gid", get_gid)?;
    module.add("set_gid", set_gid)?;
    module.add("get_url", get_url)?;
    module.add("set_url", set_url)?;
    module.add("get_sketch_params", get_sketch_params)?;
    module.add("set_sketch_params", set_sketch_params)?;
    module.add("remove", remove)?;
    clip::register(module)?;
    extent::register(module)?;
    filter::register(module)?;
    picking::register(module)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{extension, with_python};
    use pltrs::lines::{Line2D, LineProps};

    #[test]
    fn adopted_artists_convert_to_their_typed_handles() {
        with_python(|py| {
            extension(py);
            let ax = Figure::new().subplot();
            let line = Line2D::loose(vec![0.0, 1.0], vec![1.0, 0.0], LineProps::default());
            assert!(is_loose(line.raw().clone()).unwrap());
            let adopted =
                adopt_into_axes(ax.clone(), line.raw().clone(), "line".into(), true).unwrap();
            let object = adopted.clone().into_py(py).unwrap();
            let typed: Line2D = object.extract().unwrap();
            assert_eq!(ax.lines(), [typed]);
            let err = object.extract::<Axes>().unwrap_err();
            assert_eq!(err.class_name(py), "TypeError");
            let err =
                adopt_into_axes(ax.clone(), adopted.clone(), "line".into(), true).unwrap_err();
            assert_eq!(err.class_name(py), "ValueError", "already in the axes");
            let err = adopt_into_axes(ax, adopted, "table".into(), true).unwrap_err();
            assert!(err.message(py).contains("no add_table method"));
        });
    }

    #[test]
    fn artists_of_another_figure_raise_matplotlibs_error() {
        with_python(|py| {
            extension(py);
            let line = Figure::new().subplot().plot([0.0], [0.0]).unwrap();
            let err = adopt_into_figure(Figure::new(), line.raw().clone(), false).unwrap_err();
            assert_eq!(err.class_name(py), "RuntimeError");
            assert!(err.message(py).contains("more than one figure"));
            let plain = new().unwrap();
            assert!(adopt_into_figure(Figure::new(), plain, true).is_ok());
        });
    }
}
