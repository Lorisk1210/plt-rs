//! The aspect of an axes (matplotlib's `set_aspect`, `set_adjustable`,
//! `set_anchor`, `set_box_aspect`, `apply_aspect`) and the data limits it
//! adjusts (`dataLim`): pltrs (Rust) keeps
//! them and applies them whenever the axes is laid out. The Python
//! methods have checked matplotlib's arguments and handle `share`.

use pltrs::axes::{Adjustable, Aspect, AspectError, Axes};
use pltrs::transforms::{Anchor, Bbox};

use crate::binding::prelude::*;

/// The adjustable called `name` (`"box"` or `"datalim"`).
fn adjustable(name: &str) -> PyResult<Adjustable> {
    match name {
        "box" => Ok(Adjustable::Box),
        "datalim" => Ok(Adjustable::Datalim),
        _ => Err(PyErr::value_error(format!(
            "{name:?} is not a valid value for adjustable; supported values are 'box', 'datalim'"
        ))),
    }
}

/// Fixes the ratio of y to x data units on screen, `None` filling the
/// axes (`'auto'`).
fn set_aspect(axes: Axes, ratio: Option<f64>) -> PyResult<()> {
    axes.set_aspect(ratio.map_or(Aspect::Auto, Aspect::Ratio))?;
    Ok(())
}

/// The ratio of y to x data units on screen, `None` for `'auto'`.
fn get_aspect(axes: Axes) -> PyResult<Option<f64>> {
    Ok(match axes.aspect() {
        Aspect::Auto => None,
        Aspect::Equal => Some(1.0),
        Aspect::Ratio(ratio) => Some(ratio),
    })
}

/// Sets what keeps the aspect: `"box"` or `"datalim"`.
fn set_adjustable(axes: Axes, name: String) -> PyResult<()> {
    match axes.set_adjustable(adjustable(&name)?) {
        Ok(_) => Ok(()),
        Err(AspectError::FixedDataRatio(_)) => Err(PyErr::value_error(
            "Cannot set Axes adjustable to 'datalim' for Axes which override 'get_data_ratio'",
        )),
        Err(other) => Err(other.into()),
    }
}

/// What keeps the aspect: `"box"` or `"datalim"`.
fn get_adjustable(axes: Axes) -> PyResult<&'static str> {
    Ok(match axes.adjustable() {
        Adjustable::Box => "box",
        Adjustable::Datalim => "datalim",
    })
}

/// The anchor named `code` (`"C"`, `"SW"`...), or at the fractions `(x,
/// y)` of the original box without a code.
pub(in crate::modules) fn anchor(code: Option<String>, (x, y): (f64, f64)) -> PyResult<Anchor> {
    match code {
        Some(code) => Anchor::from_code(&code)
            .ok_or_else(|| PyErr::value_error(format!("unknown anchor {code:?}"))),
        None => Ok(Anchor::Fraction(x, y)),
    }
}

/// Places the shrunk box at the anchor named `code` (`"C"`, `"SW"`...),
/// or at the fractions `(x, y)` of the original box.
fn set_anchor(axes: Axes, code: Option<String>, fractions: (f64, f64)) -> PyResult<()> {
    axes.set_anchor(anchor(code, fractions)?);
    Ok(())
}

/// The anchor: its code, or `None` and its fractions.
fn get_anchor(axes: Axes) -> PyResult<(Option<&'static str>, (f64, f64))> {
    let anchor = axes.anchor();
    let code = match anchor {
        Anchor::C => Some("C"),
        Anchor::SW => Some("SW"),
        Anchor::S => Some("S"),
        Anchor::SE => Some("SE"),
        Anchor::E => Some("E"),
        Anchor::NE => Some("NE"),
        Anchor::N => Some("N"),
        Anchor::NW => Some("NW"),
        Anchor::W => Some("W"),
        Anchor::Fraction(..) => None,
    };
    Ok((code, anchor.coefficients()))
}

/// Fixes the ratio of the box's height to its width (with its twins), or
/// frees it (`None`).
fn set_box_aspect(axes: Axes, ratio: Option<f64>) -> PyResult<()> {
    axes.set_box_aspect(ratio)?;
    Ok(())
}

/// The ratio of the box's height to its width, if fixed.
fn get_box_aspect(axes: Axes) -> PyResult<Option<f64>> {
    Ok(axes.box_aspect())
}

/// Draws the axes and its twins at their original positions again, until
/// the aspect is next applied (`reset_position`).
fn reset_position(axes: Axes) -> PyResult<()> {
    axes.reset_position();
    Ok(())
}

/// Data limits as `((x0, y0, x1, y1), (minpos_x, minpos_y))`.
type Limits = ((f64, f64, f64, f64), (f64, f64));

/// The data limits, `(x0, y0, x1, y1)`, and the smallest positive x and y
/// of the data (matplotlib's `dataLim` and its `minpos`).
fn data_lim(axes: Axes) -> PyResult<Limits> {
    let limits = axes.data_lim();
    let bbox = limits.bbox;
    Ok((
        (bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1()),
        (limits.minpos.x, limits.minpos.y),
    ))
}

/// Applies the aspect now, with the axes where its locators put it or,
/// given, at `position` (`(x0, y0, width, height)` in figure fractions).
fn apply_aspect(axes: Axes, position: Option<(f64, f64, f64, f64)>) -> PyResult<()> {
    match position {
        Some((x0, y0, width, height)) => {
            axes.apply_aspect_at(Bbox::from_bounds(x0, y0, width, height))
        }
        None => axes.apply_aspect(),
    };
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_aspect", set_aspect)?;
    module.add("get_aspect", get_aspect)?;
    module.add("set_adjustable", set_adjustable)?;
    module.add("get_adjustable", get_adjustable)?;
    module.add("set_anchor", set_anchor)?;
    module.add("get_anchor", get_anchor)?;
    module.add("set_box_aspect", set_box_aspect)?;
    module.add("get_box_aspect", get_box_aspect)?;
    module.add("data_lim", data_lim)?;
    module.add("reset_position", reset_position)?;
    module.add("apply_aspect", apply_aspect)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::figure::Figure;

    #[test]
    fn the_aspect_round_trips() {
        let ax = Figure::new().subplot();
        assert_eq!(get_aspect(ax.clone()).unwrap(), None);
        set_aspect(ax.clone(), Some(2.0)).unwrap();
        assert_eq!(get_aspect(ax.clone()).unwrap(), Some(2.0));
        set_adjustable(ax.clone(), "datalim".into()).unwrap();
        assert_eq!(get_adjustable(ax.clone()).unwrap(), "datalim");
        assert!(set_adjustable(ax.clone(), "frame".into()).is_err());
        set_anchor(ax.clone(), Some("NE".into()), (0.0, 0.0)).unwrap();
        assert_eq!(get_anchor(ax.clone()).unwrap(), (Some("NE"), (1.0, 1.0)));
        set_anchor(ax.clone(), None, (0.25, 0.75)).unwrap();
        assert_eq!(get_anchor(ax.clone()).unwrap(), (None, (0.25, 0.75)));
        assert!(set_anchor(ax.clone(), Some("middle".into()), (0.0, 0.0)).is_err());
        set_box_aspect(ax.clone(), Some(1.0)).unwrap();
        assert_eq!(get_box_aspect(ax.clone()).unwrap(), Some(1.0));
        apply_aspect(ax.clone(), None).unwrap();
        apply_aspect(ax.clone(), Some((0.0, 0.0, 1.0, 1.0))).unwrap();
        let (_, minpos) = data_lim(ax).unwrap();
        assert_eq!(minpos, (f64::INFINITY, f64::INFINITY), "no data");
    }
}
