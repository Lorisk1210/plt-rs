//! How gridlines are stroked beyond their colour, width and style: their
//! antialiasing and the caps and joins of their solid and dashed lines
//! (the `grid_antialiased`, `grid_solid_capstyle`, ... tick parameters,
//! which matplotlib hands the gridlines' `Line2D`).

use pltrs_render::{CapStyle, JoinStyle};

use crate::rc::RcParams;

/// How gridlines are stroked beyond their colour, width and style.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridStroke {
    /// Anti-aliasing.
    pub antialiased: bool,
    /// Cap style of solid gridlines.
    pub solid_capstyle: CapStyle,
    /// Join style of solid gridlines.
    pub solid_joinstyle: JoinStyle,
    /// Cap style of dashed gridlines.
    pub dash_capstyle: CapStyle,
    /// Join style of dashed gridlines.
    pub dash_joinstyle: JoinStyle,
}

pltrs_core::archive_struct!(GridStroke {
    antialiased,
    solid_capstyle,
    solid_joinstyle,
    dash_capstyle,
    dash_joinstyle
});

impl GridStroke {
    /// The stroke of lines under `rc` (`lines.antialiased`,
    /// `lines.solid_capstyle`, ...), which gridlines start from.
    pub fn from_rc(rc: &RcParams) -> Self {
        Self {
            antialiased: rc.lines_antialiased,
            solid_capstyle: rc.lines_solid_capstyle,
            solid_joinstyle: rc.lines_solid_joinstyle,
            dash_capstyle: rc.lines_dash_capstyle,
            dash_joinstyle: rc.lines_dash_joinstyle,
        }
    }

    /// The cap and join style of solid or `dashed` gridlines.
    pub fn cap_join(&self, dashed: bool) -> (CapStyle, JoinStyle) {
        if dashed {
            (self.dash_capstyle, self.dash_joinstyle)
        } else {
            (self.solid_capstyle, self.solid_joinstyle)
        }
    }
}

/// Changes to a [`GridStroke`]: each field set replaces its own.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GridStrokeOptions {
    /// Anti-aliasing (`grid_antialiased`).
    pub antialiased: Option<bool>,
    /// Cap style of solid gridlines (`grid_solid_capstyle`).
    pub solid_capstyle: Option<CapStyle>,
    /// Join style of solid gridlines (`grid_solid_joinstyle`).
    pub solid_joinstyle: Option<JoinStyle>,
    /// Cap style of dashed gridlines (`grid_dash_capstyle`).
    pub dash_capstyle: Option<CapStyle>,
    /// Join style of dashed gridlines (`grid_dash_joinstyle`).
    pub dash_joinstyle: Option<JoinStyle>,
}

impl GridStrokeOptions {
    /// Applies the fields set to `stroke`.
    ///
    /// ```
    /// use pltrs::axes::{GridStroke, GridStrokeOptions};
    /// let mut stroke = GridStroke::from_rc(&pltrs::rc::params());
    /// GridStrokeOptions { antialiased: Some(false), ..Default::default() }.apply(&mut stroke);
    /// assert!(!stroke.antialiased);
    /// ```
    pub fn apply(&self, stroke: &mut GridStroke) {
        fn set<T: Copy>(slot: &mut T, value: Option<T>) {
            if let Some(value) = value {
                *slot = value;
            }
        }
        set(&mut stroke.antialiased, self.antialiased);
        set(&mut stroke.solid_capstyle, self.solid_capstyle);
        set(&mut stroke.solid_joinstyle, self.solid_joinstyle);
        set(&mut stroke.dash_capstyle, self.dash_capstyle);
        set(&mut stroke.dash_joinstyle, self.dash_joinstyle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gridlines_take_the_caps_and_joins_of_their_dashing() {
        let mut stroke = GridStroke::from_rc(&crate::rc::params());
        GridStrokeOptions {
            dash_capstyle: Some(CapStyle::Round),
            solid_joinstyle: Some(JoinStyle::Bevel),
            ..GridStrokeOptions::default()
        }
        .apply(&mut stroke);
        assert_eq!(stroke.cap_join(true).0, CapStyle::Round);
        assert_eq!(stroke.cap_join(false).1, JoinStyle::Bevel);
    }
}
