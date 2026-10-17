//! Label colours (matplotlib's `labelcolor` handling in `Legend`).

use plt_core::Rgba;

use super::{LegendError, LegendLabelColor};
use crate::artist::Artist;
use crate::collections::CollectionArtist;
use crate::colors::Color;
use crate::lines::LineArtist;
use crate::markers::FillStyle;
use crate::rc::{ColorOrAuto, LabelColor, RcParams};

/// A colour getter of a handle artist, in the order matplotlib's
/// `color_getters` tries them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Getter {
    MarkerFace,
    Face,
    MarkerEdge,
    Edge,
    Color,
}

/// What a getter returned.
enum Got {
    /// The artist has no such colour.
    Missing,
    /// One colour.
    Single(Color),
    /// One colour per item of a collection.
    Array(Vec<Rgba>),
}

/// A handle as a label-colour rule sees it: its first artist, unless the
/// handle is unsupported, and whether it maps values to colours.
pub(crate) struct HandleView<'a> {
    pub(crate) artist: Option<&'a dyn Artist>,
    pub(crate) mapped: bool,
}

/// The colour each label gets (`None` keeps the text colour), for
/// `count` labels paired with `handles` in order.
pub(crate) fn label_colors(
    spec: Option<&LegendLabelColor>,
    handles: &[HandleView<'_>],
    count: usize,
    rc: &RcParams,
) -> Result<Vec<Option<Color>>, LegendError> {
    let spec = match spec {
        Some(LegendLabelColor::Single(LabelColor::Unset)) | None => match &rc.legend_labelcolor {
            LabelColor::Unset => LegendLabelColor::Single(LabelColor::Color(rc.text_color.clone())),
            other => LegendLabelColor::Single(other.clone()),
        },
        Some(spec) => spec.clone(),
    };
    let getters: &[Getter] = match &spec {
        LegendLabelColor::Single(LabelColor::LineColor) => &[
            Getter::MarkerFace,
            Getter::Face,
            Getter::MarkerEdge,
            Getter::Edge,
            Getter::Color,
        ],
        LegendLabelColor::Single(LabelColor::MarkerFaceColor) => {
            &[Getter::MarkerFace, Getter::Face]
        }
        LegendLabelColor::Single(LabelColor::MarkerEdgeColor) => {
            &[Getter::MarkerEdge, Getter::Edge]
        }
        LegendLabelColor::Single(LabelColor::Color(color)) if color.is_none() => {
            return Ok(vec![Some(Color::none()); count]);
        }
        LegendLabelColor::Single(LabelColor::Color(color)) => {
            let color = Color::from(color.resolve()?);
            return Ok(vec![Some(color); count]);
        }
        LegendLabelColor::Single(LabelColor::Unset) => return Ok(vec![None; count]),
        LegendLabelColor::Cycle(colors) => {
            let resolved = colors
                .iter()
                .map(|c| c.resolve().map(Color::from))
                .collect::<Result<Vec<_>, _>>()?;
            let mut cycled: Vec<Option<Color>> =
                resolved.into_iter().cycle().take(count).map(Some).collect();
            cycled.resize(count, None);
            return Ok(cycled);
        }
    };
    let mut colors = vec![None; count];
    for (slot, handle) in colors.iter_mut().zip(handles) {
        *slot = from_handle(handle, getters, rc);
    }
    Ok(colors)
}

/// The colour the first matching getter finds on a handle.
fn from_handle(handle: &HandleView<'_>, getters: &[Getter], rc: &RcParams) -> Option<Color> {
    if handle.mapped {
        return None;
    }
    let artist = handle.artist?;
    for &getter in getters {
        match get(artist, getter, rc) {
            Got::Missing => {}
            Got::Array(colors) => {
                let Some(&first) = colors.first() else {
                    continue;
                };
                let uniform = colors.iter().all(|&c| is_close(c, first));
                return uniform.then(|| Color::from(first));
            }
            Got::Single(color) if color.is_none() => {}
            Got::Single(color) => {
                if color.resolve().is_ok_and(|rgba| rgba.a != 0.0) {
                    return Some(color);
                }
            }
        }
    }
    None
}

/// numpy's `isclose` on every component.
fn is_close(a: Rgba, b: Rgba) -> bool {
    a.to_array()
        .iter()
        .zip(b.to_array())
        .all(|(x, y)| (x - y).abs() <= 1e-8 + 1e-5 * y.abs())
}

/// One getter of matplotlib's `Line2D`, `Patch` or `Collection`.
fn get(artist: &dyn Artist, getter: Getter, rc: &RcParams) -> Got {
    let any: &dyn std::any::Any = artist;
    if let Some(line) = any.downcast_ref::<LineArtist>() {
        return match getter {
            Getter::MarkerFace if line.fillstyle == FillStyle::None => Got::Single(Color::none()),
            Getter::MarkerFace => Got::Single(line.markerfacecolor.or(&line.color).clone()),
            Getter::MarkerEdge => Got::Single(match &line.markeredgecolor {
                ColorOrAuto::Auto => line.color.clone(),
                ColorOrAuto::Color(color) => color.clone(),
            }),
            Getter::Color => Got::Single(line.color.clone()),
            Getter::Face | Getter::Edge => Got::Missing,
        };
    }
    if let Some(view) = artist.as_patch() {
        let resolved = view
            .style
            .resolve(rc, artist.base().alpha, view.edge_default);
        return match (getter, resolved) {
            (Getter::Face, Ok(style)) => Got::Single(Color::from(style.face)),
            (Getter::Edge, Ok(style)) => Got::Single(Color::from(style.edge)),
            _ => Got::Missing,
        };
    }
    if let Some(collection) = any.downcast_ref::<CollectionArtist>() {
        return match (getter, collection.resolved_colors()) {
            (Getter::Face, Ok((faces, _))) => Got::Array(faces),
            (Getter::Edge, Ok((_, edges))) => Got::Array(edges),
            _ => Got::Missing,
        };
    }
    Got::Missing
}
