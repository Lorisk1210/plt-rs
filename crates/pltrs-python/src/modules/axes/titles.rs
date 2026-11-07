//! The titles of an axes (matplotlib's `set_title`, `ax.title`,
//! `ax._left_title`, `ax._right_title`, `titleOffsetTrans` and
//! `_update_title_position`).

use pltrs::axes::{Axes, TitleLocation, TitlePlacement};
use pltrs::text::Text;

use crate::binding::prelude::*;

/// The title location `loc` names (`"left"`, `"center"` or `"right"`),
/// with matplotlib's `ValueError` for any other.
fn location(loc: &str) -> PyResult<TitleLocation> {
    loc.parse().map_err(|_| {
        PyErr::value_error(format!(
            "{loc:?} is not a valid value for loc; supported values are 'left', 'center', \
             'right'"
        ))
    })
}

/// The title text at `loc`.
fn title_text(axes: Axes, loc: String) -> PyResult<Text> {
    Ok(axes.title_text(location(&loc)?))
}

/// Sets the title at `loc` to `text` with the `axes.title*` defaults, its
/// gap above the axes to `pad` points (`axes.titlepad` when `None`) and
/// its height to `y` in axes fractions, which stops lifting the titles
/// above decorations on top (`axes.titley` when `None`); returns it.
fn set_title(
    axes: Axes,
    loc: String,
    text: String,
    pad: Option<f64>,
    y: Option<f64>,
) -> PyResult<Text> {
    let location = location(&loc)?;
    let title = axes.set_title_at(location, text);
    if let Some(pad) = pad {
        axes.set_title_pad(pad);
    }
    if let Some(y) = y {
        axes.set_title_y(location, y);
    }
    Ok(title)
}

/// The gap between the axes top and its titles in points.
fn get_title_pad(axes: Axes) -> PyResult<f64> {
    Ok(axes.title_pad())
}

/// Lifts the titles above what rises over the top of the axes: those of
/// every axes as the figure is laid out, or the axes' own alone when the
/// program places them (an `Axes` subclass' `_update_title_position`
/// calling its base's, which may be measuring the figure); titles at a
/// fixed height stay put.
fn update_title_position(axes: Axes) -> PyResult<()> {
    match axes.title_placement() {
        TitlePlacement::Auto => {
            axes.update_title_position();
        }
        TitlePlacement::Program => {
            axes.lift_titles();
        }
        TitlePlacement::Fixed => {}
    }
    Ok(())
}

/// The name of a title placement.
fn placement_name(placement: TitlePlacement) -> &'static str {
    match placement {
        TitlePlacement::Auto => "auto",
        TitlePlacement::Fixed => "fixed",
        TitlePlacement::Program => "program",
    }
}

/// How the height of the titles is chosen: `"auto"`, `"fixed"` or
/// `"program"`.
fn title_placement(axes: Axes) -> PyResult<&'static str> {
    Ok(placement_name(axes.title_placement()))
}

/// Chooses how the height of the titles is chosen (see `title_placement`).
fn set_title_placement(axes: Axes, placement: String) -> PyResult<()> {
    let placement = [
        TitlePlacement::Auto,
        TitlePlacement::Fixed,
        TitlePlacement::Program,
    ]
    .into_iter()
    .find(|&known| placement_name(known) == placement)
    .ok_or_else(|| PyErr::value_error(format!("unknown title placement {placement:?}")))?;
    axes.set_title_placement(placement);
    Ok(())
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("title_text", title_text)?;
    module.add("set_title", set_title)?;
    module.add("get_title_pad", get_title_pad)?;
    module.add("update_title_position", update_title_position)?;
    module.add("title_placement", title_placement)?;
    module.add("set_title_placement", set_title_placement)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::figure::Figure;

    #[test]
    fn titles_take_their_pad_and_height() {
        let ax = Figure::new().subplot();
        let title = set_title(ax.clone(), "left".into(), "t".into(), Some(30.0), Some(1.2))
            .expect("a valid location");
        assert_eq!(title.text(), "t");
        assert_eq!(title.position().y, 1.2);
        assert_eq!(get_title_pad(ax.clone()).expect("a pad"), 30.0);
        assert_eq!(
            title_text(ax.clone(), "left".into())
                .expect("a title")
                .text(),
            "t"
        );
        assert!(title_text(ax, "top".into()).is_err());
    }

    #[test]
    fn placements_go_by_their_names() {
        let ax = Figure::new().subplot();
        assert_eq!(title_placement(ax.clone()).expect("a placement"), "auto");
        set_title_placement(ax.clone(), "program".into()).expect("a placement");
        assert_eq!(ax.title_placement(), TitlePlacement::Program);
        assert_eq!(title_placement(ax.clone()).expect("a placement"), "program");
        assert!(set_title_placement(ax, "high".into()).is_err());
    }
}
