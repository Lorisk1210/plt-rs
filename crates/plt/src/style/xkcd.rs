//! matplotlib's xkcd sketch style (`pyplot.xkcd`): wobbly lines outlined
//! in white, a comic font and heavier strokes.

use std::fmt;

use plt_core::Rgba;

use crate::colors::Color;
use crate::patheffects::{GcUpdate, PathEffect, Stroke};
use crate::rc::RcParams;

/// The wobble of the xkcd style (the arguments of `pyplot.xkcd`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct XkcdSketch {
    /// Amplitude of the wiggle perpendicular to lines, in pixels.
    pub scale: f64,
    /// Length of the wiggle along lines, in pixels.
    pub length: f64,
    /// Factor by which the length randomly shrinks and expands.
    pub randomness: f64,
}

impl Default for XkcdSketch {
    /// matplotlib's defaults: `scale=1, length=100, randomness=2`.
    fn default() -> Self {
        Self {
            scale: 1.0,
            length: 100.0,
            randomness: 2.0,
        }
    }
}

/// Why the xkcd style cannot be applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XkcdError {
    /// `text.usetex` is on: TeX text cannot take the style.
    Usetex,
}

impl fmt::Display for XkcdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usetex => f.write_str("xkcd mode is not compatible with text.usetex = True"),
        }
    }
}

impl std::error::Error for XkcdError {}

/// The families the xkcd style asks for; without them installed, text
/// falls back to the default font (`DejaVu Sans`), as in matplotlib.
const XKCD_FAMILIES: [&str; 4] = ["xkcd", "xkcd Script", "Comic Neue", "Comic Sans MS"];

/// Sets the rc parameters `pyplot.xkcd` sets in `rc`: the comic font at
/// 14 pt, the `sketch` wobble (`path.sketch`), a white 4 pt outline under
/// every path (`path.effects`), heavier axes, lines and major ticks, no
/// grid and plain hyphens as minus signs.
///
/// ```
/// use plt::rc::RcParams;
/// use plt::style::{XkcdSketch, apply_xkcd};
/// let mut rc = RcParams::matplotlib();
/// apply_xkcd(&mut rc, XkcdSketch::default())?;
/// assert_eq!(rc.path_sketch, Some([1.0, 100.0, 2.0]));
/// assert_eq!(rc.font_size, 14.0);
/// # Ok::<(), plt::style::XkcdError>(())
/// ```
///
/// # Errors
///
/// [`XkcdError::Usetex`] when `text.usetex` is on.
pub fn apply_xkcd(rc: &mut RcParams, sketch: XkcdSketch) -> Result<(), XkcdError> {
    if rc.text_usetex {
        return Err(XkcdError::Usetex);
    }
    let outline = Stroke::new(GcUpdate {
        linewidth: Some(4.0),
        foreground: Some(Rgba::WHITE.into()),
        ..GcUpdate::default()
    });
    rc.font_family = XKCD_FAMILIES.map(str::to_owned).to_vec();
    rc.font_size = 14.0;
    rc.path_sketch = Some([sketch.scale, sketch.length, sketch.randomness]);
    rc.path_effects = vec![PathEffect::with_stroke(outline)];
    rc.axes_linewidth = 1.5;
    rc.lines_linewidth = 2.0;
    rc.figure_facecolor = Color::from("white");
    rc.grid_linewidth = 0.0;
    rc.axes_grid = false;
    rc.axes_unicode_minus = false;
    rc.axes_edgecolor = Color::from("black");
    rc.xtick_major_size = 8.0;
    rc.xtick_major_width = 3.0;
    rc.ytick_major_size = 8.0;
    rc.ytick_major_width = 3.0;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usetex_refuses_the_style() {
        let mut rc = RcParams::matplotlib();
        rc.text_usetex = true;
        assert_eq!(
            apply_xkcd(&mut rc, XkcdSketch::default()),
            Err(XkcdError::Usetex)
        );
        assert_eq!(rc.path_sketch, None, "nothing changes");
    }

    #[test]
    fn the_style_outlines_paths_in_white() {
        let mut rc = RcParams::matplotlib();
        let sketch = XkcdSketch {
            scale: 2.0,
            length: 50.0,
            randomness: 4.0,
        };
        apply_xkcd(&mut rc, sketch).unwrap();
        assert_eq!(rc.path_sketch, Some([2.0, 50.0, 4.0]));
        assert_eq!(rc.path_effects.len(), 1);
        assert!(rc.path_effects[0].then_normal);
        assert_eq!(rc.font_family[0], "xkcd");
        assert!(!rc.axes_unicode_minus);
    }
}
