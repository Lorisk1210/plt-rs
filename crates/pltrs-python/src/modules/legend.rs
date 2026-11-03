//! `_native.legend`: legends (matplotlib's `legend`), by pltrs (Rust)'s
//! `legend` module.
//!
//! Python resolves what matplotlib resolves in Python (the positional
//! forms of `legend()`, labels, handler maps keyed by handle or type) and
//! passes a legend its entries and its keyword arguments, aliases
//! resolved; this module turns them into pltrs (Rust)'s `LegendOptions`.

use pltrs::artist::{RawHandle, TypedHandle};
use pltrs::axes::Axes;
use pltrs::collections::{Collection, CollectionKind};
use pltrs::colors::Color;
use pltrs::figure::Figure;
use pltrs::fonts::FontSize;
use pltrs::legend::{
    HandlerTuple, Legend, LegendAlignment, LegendError, LegendHandle, LegendLabelColor, LegendLoc,
    LegendMode, LegendOptions, LegendShadow, LegendShadowStyle,
};
use pltrs::lines::Line2D;
use pltrs::offsetbox::BboxToAnchor;
use pltrs::patches::{FancyBboxPatch, Patch, Rectangle};
use pltrs::rc::LabelColor;
use pltrs::text::Text;
use pltrs::transforms::Transform;

use super::font_manager::FontProps;
use super::patches::PatchKwargs;
use crate::binding::prelude::*;
use crate::binding::{dict_items, new_dict};

mod anchor;
mod legend_box;
mod made;
mod perf_counter;

artist_handle!(Legend);
native_value!(LegendHandle);

impl From<LegendError> for PyErr {
    fn from(err: LegendError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

/// A legend location: a name (`"upper right"`, `"outside upper
/// center"`), a code (`0` to `10`) or a point `(x, y)`.
struct Loc(LegendLoc);

impl FromPy<'_> for Loc {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let loc = if let Ok(name) = object.extract::<String>() {
            name.parse()
                .map_err(|err: <LegendLoc as std::str::FromStr>::Err| {
                    PyErr::value_error(err.to_string())
                })?
        } else if let Ok(code) = object.extract::<i64>() {
            LegendLoc::from_code(code)?
        } else {
            let (x, y): (f64, f64) = object.extract()?;
            LegendLoc::Point(x, y)
        };
        Ok(Self(loc))
    }
}

/// A font size in points or a relative name (`"large"`).
fn font_size(object: &Bound<'_>) -> PyResult<FontSize> {
    if let Ok(name) = object.extract::<String>() {
        return name
            .parse()
            .map_err(|_| PyErr::value_error(format!("{name:?} is not a valid font size")));
    }
    Ok(FontSize::Points(object.extract()?))
}

/// A label colour rule: `"linecolor"`, `"markerfacecolor"` (`"mfc"`),
/// `"markeredgecolor"` (`"mec"`), or a colour.
fn label_color(object: &Bound<'_>) -> PyResult<LabelColor> {
    if let Ok(name) = object.extract::<String>() {
        match name.as_str() {
            "linecolor" => return Ok(LabelColor::LineColor),
            "markerfacecolor" | "mfc" => return Ok(LabelColor::MarkerFaceColor),
            "markeredgecolor" | "mec" => return Ok(LabelColor::MarkerEdgeColor),
            _ => {}
        }
    }
    Ok(LabelColor::Color(object.extract::<Color>()?))
}

/// The box a legend is placed in: a point `(x, y)` or bounds `(x, y,
/// width, height)`.
fn bbox_to_anchor(object: &Bound<'_>) -> PyResult<BboxToAnchor> {
    let values: Vec<f64> = object.extract()?;
    match *values.as_slice() {
        [x, y] => Ok(BboxToAnchor::Point(x, y)),
        [x, y, width, height] => Ok(BboxToAnchor::Bounds(x, y, width, height)),
        _ => Err(PyErr::value_error(format!("Invalid bbox: {values:?}"))),
    }
}

/// A legend's shadow: a bool, or a dict of `Shadow` arguments (`ox`,
/// `oy`, `shade` and the shadow patch's properties), which turns it on.
fn legend_shadow(object: &Bound<'_>) -> PyResult<LegendShadow> {
    if object.type_name() != "dict" {
        return Ok(object.extract::<bool>()?.into());
    }
    let mut style = LegendShadowStyle::default();
    let props = new_dict(object.py())?;
    for (key, value) in dict_items(object)? {
        match key.extract::<String>()?.as_str() {
            "ox" => style.ox = value.extract()?,
            "oy" => style.oy = value.extract()?,
            "shade" => style.shade = value.extract()?,
            _ => props.set_item(key, value)?,
        }
    }
    style.props = props.extract::<PatchKwargs>()?.0;
    Ok(style.into())
}

/// The options of a dict of `Legend` keyword arguments, aliases resolved.
struct LegendKwargs(LegendOptions);

impl FromPy<'_> for LegendKwargs {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let mut options = LegendOptions::default();
        for (key, value) in dict_items(object)? {
            let key: String = key.extract()?;
            if value.is_none() {
                continue;
            }
            options = match key.as_str() {
                "loc" => LegendOptions {
                    loc: Some(value.extract::<Loc>()?.0),
                    ..options
                },
                "ncols" => options.ncols(value.extract::<usize>()?),
                "prop" => options.prop(value.extract::<FontProps>()?.0),
                "fontsize" => options.fontsize(font_size(&value)?),
                "title_fontsize" => options.title_fontsize(font_size(&value)?),
                "labelcolor" => LegendOptions {
                    labelcolor: Some(match value.extract::<String>() {
                        Ok(_) => LegendLabelColor::Single(label_color(&value)?),
                        Err(_) => LegendLabelColor::Cycle(value.extract::<Vec<Color>>()?),
                    }),
                    ..options
                },
                "numpoints" => options.numpoints(value.extract::<usize>()?),
                "scatterpoints" => options.scatterpoints(value.extract::<usize>()?),
                "scatteryoffsets" => options.scatteryoffsets(value.extract::<Vec<f64>>()?),
                "markerscale" => options.markerscale(value.extract::<f64>()?),
                "markerfirst" => options.markerfirst(value.extract::<bool>()?),
                "reverse" => options.reverse(value.extract::<bool>()?),
                "frameon" => options.frameon(value.extract::<bool>()?),
                "fancybox" => options.fancybox(value.extract::<bool>()?),
                "shadow" => options.shadow(legend_shadow(&value)?),
                "framealpha" => options.framealpha(value.extract::<f64>()?),
                "facecolor" => options.facecolor(value.extract::<Color>()?),
                "edgecolor" => options.edgecolor(value.extract::<Color>()?),
                "linewidth" => options.linewidth(value.extract::<f64>()?),
                "mode" => match value.extract::<String>()?.as_str() {
                    "expand" => options.mode(LegendMode::Expand),
                    mode => {
                        return Err(PyErr::value_error(format!(
                            "{mode:?} is not a valid value for mode; supported values are \
                             'expand'"
                        )));
                    }
                },
                "title" => options.title(value.str()?),
                "alignment" => options.alignment(match value.extract::<String>()?.as_str() {
                    "center" => LegendAlignment::Center,
                    "left" => LegendAlignment::Left,
                    "right" => LegendAlignment::Right,
                    alignment => {
                        return Err(PyErr::value_error(format!(
                            "{alignment:?} is not a valid value for alignment; supported \
                             values are 'center', 'left', 'right'"
                        )));
                    }
                }),
                "borderpad" => options.borderpad(value.extract::<f64>()?),
                "labelspacing" => options.labelspacing(value.extract::<f64>()?),
                "handlelength" => options.handlelength(value.extract::<f64>()?),
                "handleheight" => options.handleheight(value.extract::<f64>()?),
                "handletextpad" => options.handletextpad(value.extract::<f64>()?),
                "borderaxespad" => options.borderaxespad(value.extract::<f64>()?),
                "columnspacing" => options.columnspacing(value.extract::<f64>()?),
                "bbox_to_anchor" => options.bbox_to_anchor(bbox_to_anchor(&value)?),
                "bbox_transform" => options.bbox_transform(value.extract::<Transform>()?),
                "draggable" => LegendOptions {
                    draggable: value.extract::<bool>()?,
                    ..options
                },
                _ => {
                    return Err(PyErr::new(
                        ExceptionKind::TypeError,
                        format!("Legend.__init__() got an unexpected keyword argument '{key}'"),
                    ));
                }
            };
        }
        Ok(Self(options))
    }
}

/// A legend entry: an artist, a native legend handle (one the axes found,
/// see [`handles_labels`]), `("tuple", entries, ndivide, pad)` for
/// several handles in one entry (matplotlib's tuple handles and
/// `HandlerTuple`), or `("made", artists)` for the artists a handler of
/// the program's made (see [`made`]).
struct Entry(LegendHandle);

impl FromPy<'_> for Entry {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok(handle) = object.extract::<LegendHandle>() {
            return Ok(Self(handle));
        }
        if let Ok((kind, artists)) = object.extract::<(String, Vec<(RawHandle, String)>)>()
            && kind == "made"
        {
            return made::made_handle(artists).map(Self);
        }
        if let Ok((kind, entries, ndivide, pad)) =
            object.extract::<(String, Vec<Entry>, Option<usize>, Option<f64>)>()
            && kind == "tuple"
        {
            let handles = entries.into_iter().map(|entry| entry.0);
            return Ok(Self(LegendHandle::tuple(
                handles,
                HandlerTuple { ndivide, pad },
            )));
        }
        let artist: RawHandle = object.extract()?;
        LegendHandle::of_artist(&artist).map(Self).ok_or_else(|| {
            PyErr::value_error("pltrs: a legend cannot show this artist made outside any figure")
        })
    }
}

/// A legend of `entries` labelled `labels` (both as Python resolved them)
/// on `parent`, an axes or a figure (`Legend(parent, handles, labels,
/// **kwargs)`): it replaces the axes' legend, and adds to the figure's.
fn new(
    parent: RawHandle,
    entries: Vec<Entry>,
    labels: Vec<String>,
    kwargs: LegendKwargs,
) -> PyResult<Legend> {
    let options = kwargs
        .0
        .handles(entries.into_iter().map(|entry| entry.0))
        .labels(labels);
    // A 3-D axes places its legend on its projection plane.
    let axes = Axes::from_raw_handle(&parent).or_else(|| {
        pltrs::mplot3d::Axes3D::from_raw_handle(&parent).map(|axes| axes.projection_plane())
    });
    if let Some(axes) = axes {
        return Ok(axes.legend(options)?);
    }
    match Figure::from_raw_handle(&parent) {
        Some(figure) => Ok(figure.legend(options)?),
        None => Err(PyErr::type_error(
            "Legend needs either Axes or FigureBase as parent",
        )),
    }
}

/// A legend candidate as Python gets it: its native handle, the artist it
/// shows with that artist's kind, and its label.
type Candidate = (LegendHandle, Option<(RawHandle, &'static str)>, String);

/// matplotlib's `_loc` as parts: the code, or the point.
type LocParts = (Option<u8>, Option<(f64, f64)>);

/// What a legend of the axes `parent` can show, hidden labels included
/// (matplotlib's `_get_legend_handles`): for each candidate its native
/// handle, the artist it shows if it is one artist, with that artist's
/// kind (`"Line2D"`, `"LineCollection"`, `"Collection"`, `"Text"`,
/// `"Rectangle"` or `"Artist"`), and its label.
fn candidates(parent: Axes) -> PyResult<Vec<Candidate>> {
    Ok(parent
        .legend_candidates()
        .into_iter()
        .map(|(handle, label)| {
            let artist = handle.artist(&parent).map(|raw| {
                let kind = artist_kind(&raw);
                (raw, kind)
            });
            (handle, artist, label)
        })
        .collect())
}

/// The kind of artist `raw` reaches, as legends tell them apart.
fn artist_kind(raw: &RawHandle) -> &'static str {
    if raw.downcast::<Line2D>().is_some() {
        "Line2D"
    } else if let Some(collection) = raw.downcast::<Collection>() {
        if collection.kind() == CollectionKind::Line {
            "LineCollection"
        } else {
            "Collection"
        }
    } else if raw.downcast::<Text>().is_some() {
        "Text"
    } else if raw.downcast::<Patch<Rectangle>>().is_some() {
        "Rectangle"
    } else {
        "Artist"
    }
}

/// The legend's entry texts.
fn get_texts(legend: Legend) -> PyResult<Vec<Text>> {
    Ok(legend.texts())
}

/// The artist drawn for each listed handle, with its kind (see
/// [`artist_kind`]), or `None` for a handle no handler draws
/// (matplotlib's `legend_handles`).
fn legend_handles(legend: Legend) -> PyResult<Vec<Option<(RawHandle, &'static str)>>> {
    Ok(legend
        .handles()
        .into_iter()
        .map(|handle| {
            handle.map(|raw| {
                let kind = artist_kind(&raw);
                (raw, kind)
            })
        })
        .collect())
}

/// The frame (matplotlib's `get_frame` and `legendPatch`).
fn get_frame(legend: Legend) -> PyResult<Patch<FancyBboxPatch>> {
    Ok(legend.frame())
}

/// The title text.
fn get_title(legend: Legend) -> PyResult<Text> {
    Ok(legend.title())
}

/// Sets the title; an empty one hides it.
fn set_title(legend: Legend, title: String) -> PyResult<()> {
    legend.set_title(title);
    Ok(())
}

/// Whether the frame is drawn.
fn get_frame_on(legend: Legend) -> PyResult<bool> {
    Ok(legend.frame_on())
}

/// Shows or hides the frame.
fn set_frame_on(legend: Legend, on: bool) -> PyResult<()> {
    legend.set_frame_on(on);
    Ok(())
}

/// Moves the legend to `loc`, or back to the default location with
/// `None`.
fn set_loc(legend: Legend, loc: Option<Loc>) -> PyResult<()> {
    match loc {
        Some(loc) => legend.set_loc(loc.0)?,
        None => legend.reset_loc()?,
    };
    Ok(())
}

/// matplotlib's `_loc`: the code of a named location, or the point.
fn loc(legend: Legend) -> PyResult<LocParts> {
    Ok(match legend.loc() {
        LegendLoc::Point(x, y) => (None, Some((x, y))),
        loc => (loc.code(), None),
    })
}

/// How the title and the entries line up (`"center"`, `"left"`,
/// `"right"`).
fn get_alignment(legend: Legend) -> PyResult<&'static str> {
    Ok(match legend.alignment() {
        LegendAlignment::Center => "center",
        LegendAlignment::Left => "left",
        LegendAlignment::Right => "right",
    })
}

/// Lines up the title and the entries.
fn set_alignment(legend: Legend, alignment: String) -> PyResult<()> {
    legend.set_alignment(match alignment.as_str() {
        "center" => LegendAlignment::Center,
        "left" => LegendAlignment::Left,
        "right" => LegendAlignment::Right,
        _ => {
            return Err(PyErr::value_error(format!(
                "{alignment:?} is not a valid value for alignment; supported values are \
                 'center', 'left', 'right'"
            )));
        }
    });
    Ok(())
}

/// Whether the legend can be dragged with the mouse.
fn get_draggable(legend: Legend) -> PyResult<bool> {
    Ok(legend.draggable())
}

/// Keeps `legend` on `axes` as an ordinary artist, so that the next
/// `legend()` adds a legend beside it (`ax.add_artist(legend)`).
fn keep(axes: Axes, legend: Legend) -> PyResult<()> {
    axes.keep_legend(&legend);
    Ok(())
}

/// The axes legend, if any.
fn axes_legend(axes: Axes) -> PyResult<Option<Legend>> {
    Ok(axes.current_legend())
}

/// Makes `legend`, a legend made for `axes`, its axes legend again
/// (`ax.legend_ = legend`); whether it is a legend of `axes`.
fn set_axes_legend(axes: Axes, legend: Legend) -> PyResult<bool> {
    Ok(axes.set_current_legend(&legend))
}

/// The figure legends, in the order they were made.
fn figure_legends(figure: Figure) -> PyResult<Vec<Legend>> {
    Ok(figure.legends())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    perf_counter::register();
    module.add("new", new)?;
    module.add("candidates", candidates)?;
    module.add("get_texts", get_texts)?;
    module.add("legend_handles", legend_handles)?;
    module.add("get_frame", get_frame)?;
    module.add("legend_box", legend_box::legend_box)?;
    module.add("get_title", get_title)?;
    module.add("set_title", set_title)?;
    module.add("get_frame_on", get_frame_on)?;
    module.add("set_frame_on", set_frame_on)?;
    module.add("set_loc", set_loc)?;
    module.add("loc", loc)?;
    module.add("get_alignment", get_alignment)?;
    module.add("set_alignment", set_alignment)?;
    module.add("get_draggable", get_draggable)?;
    anchor::register(module)?;
    made::register(module)?;
    module.add("keep", keep)?;
    module.add("axes_legend", axes_legend)?;
    module.add("set_axes_legend", set_axes_legend)?;
    module.add("figure_legends", figure_legends)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, extension, with_python};
    use pltrs::artist::ArtistHandle;

    #[test]
    fn keyword_arguments_become_legend_options() {
        with_python(|py| {
            extension(py);
            let kwargs = eval(
                py,
                "",
                "{'loc': 'upper left', 'ncols': 2, 'fontsize': 'small', 'title': 'T', \
                 'framealpha': None, 'labelcolor': 'mfc', 'bbox_to_anchor': (1, 0.5)}",
            );
            let options = kwargs.extract::<LegendKwargs>().unwrap().0;
            assert_eq!(options.ncols, 2);
            assert_eq!(options.title.as_deref(), Some("T"));
            assert_eq!(options.framealpha, None, "None leaves the default");
            assert_eq!(options.bbox_to_anchor, Some(BboxToAnchor::Point(1.0, 0.5)));
            let bad = eval(py, "", "{'colour': 'r'}").extract::<LegendKwargs>();
            assert!(bad.is_err_and(|err| err.message(py).contains("'colour'")));
        });
    }

    #[test]
    fn legends_are_made_on_axes_and_figures() {
        with_python(|py| {
            extension(py);
            let fig = Figure::new();
            let ax = fig.subplot();
            let line = ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
            let entry = || {
                line.raw()
                    .clone()
                    .into_py(py)
                    .unwrap()
                    .extract::<Entry>()
                    .unwrap()
            };
            let kwargs = || LegendKwargs(LegendOptions::default());
            let legend = new(ax.raw().clone(), vec![entry()], vec!["a".into()], kwargs()).unwrap();
            assert_eq!(legend.labels(), ["a"]);
            assert_eq!(axes_legend(ax).unwrap(), Some(legend));
            new(fig.raw().clone(), vec![entry()], vec!["b".into()], kwargs()).unwrap();
            assert_eq!(figure_legends(fig).unwrap().len(), 1);
        });
    }
}
