//! Arrow shapes drawn along a connection path (matplotlib's `ArrowStyle`).

mod curve;
mod quadratic;

use std::str::FromStr;

use plt_core::Path;

pub use curve::{Bracket, CurveArrow, CurveEnd};

use super::style_spec::StyleSpec;
use super::{PatchError, StyleError};

/// How an arrow is drawn along its path. Sizes are fractions of the
/// mutation size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ArrowStyle {
    /// The path itself with optional heads or brackets at its ends (`-`,
    /// `->`, `<-`, `<->`, `-|>`, `<|-`, `<|-|>`, `]-`, `-[`, `]-[`, `|-|`,
    /// `]->`, `<-[`); works with any path.
    Curve(CurveArrow),
    /// A constant-width tail and a head (`simple`); needs a quadratic curve.
    Simple {
        /// Head length.
        head_length: f64,
        /// Head width.
        head_width: f64,
        /// Tail width.
        tail_width: f64,
    },
    /// A tapering tail and a head (`fancy`); needs a quadratic curve.
    Fancy {
        /// Head length.
        head_length: f64,
        /// Head width.
        head_width: f64,
        /// Tail width.
        tail_width: f64,
    },
    /// A wedge narrowing to a point (`wedge`); needs a quadratic curve.
    Wedge {
        /// Width at the start.
        tail_width: f64,
        /// Width at the middle as a fraction of `tail_width`.
        shrink_factor: f64,
    },
}

/// The outcome of a transmutation: one fillable outline (the quadratic
/// styles) or several paths, each fillable or not (the curve styles).
pub(crate) enum StyledPath {
    One(Path),
    Many(Vec<(Path, bool)>),
}

impl ArrowStyle {
    /// matplotlib's default arrow of `FancyArrowPatch` (`simple`).
    pub const fn simple() -> Self {
        Self::Simple {
            head_length: 0.5,
            head_width: 0.5,
            tail_width: 0.2,
        }
    }

    /// A curve arrow with the given ends and default sizes.
    pub fn curve(begin: CurveEnd, end: CurveEnd) -> Self {
        Self::Curve(CurveArrow::new(begin, end))
    }

    /// The arrow along `path` (display pixels): each piece with whether it
    /// is filled. `mutation_size` scales the style and `linewidth` (pixels)
    /// keeps heads from overshooting; the path is squeezed vertically by
    /// `aspect` while the arrow is built (matplotlib's
    /// `ArrowStyle._Base.__call__`, which only stretches multi-piece arrows
    /// back).
    pub fn paths(
        &self,
        path: &Path,
        mutation_size: f64,
        linewidth: f64,
        aspect: f64,
    ) -> Result<Vec<(Path, bool)>, PatchError> {
        let mut squeezed = path.clone();
        for p in squeezed.vertices_mut() {
            p.y /= aspect;
        }
        Ok(match self.transmute(&squeezed, mutation_size, linewidth)? {
            StyledPath::One(path) => vec![(path, true)],
            StyledPath::Many(paths) => paths
                .into_iter()
                .map(|(mut path, fillable)| {
                    for p in path.vertices_mut() {
                        p.y *= aspect;
                    }
                    (path, fillable)
                })
                .collect(),
        })
    }

    fn transmute(
        &self,
        path: &Path,
        mutation_size: f64,
        linewidth: f64,
    ) -> Result<StyledPath, PatchError> {
        Ok(match *self {
            Self::Curve(curve) => curve.transmute(path, mutation_size, linewidth),
            Self::Simple {
                head_length,
                head_width,
                tail_width,
            } => StyledPath::One(quadratic::simple(
                quadratic::quadratic(path)?,
                mutation_size,
                (head_length, head_width, tail_width),
            )),
            Self::Fancy {
                head_length,
                head_width,
                tail_width,
            } => StyledPath::One(quadratic::fancy(
                quadratic::quadratic(path)?,
                mutation_size,
                (head_length, head_width, tail_width),
            )?),
            Self::Wedge {
                tail_width,
                shrink_factor,
            } => StyledPath::One(quadratic::wedge(
                quadratic::quadratic(path)?,
                mutation_size,
                (tail_width, shrink_factor),
            )),
        })
    }
}

/// The parameters each curve style accepts.
#[derive(Clone, Copy)]
enum CurveParameters {
    /// `-`: none.
    Nothing,
    /// Heads: every `_Curve` parameter.
    All,
    /// A bracket at the start only.
    BracketA,
    /// A bracket at the end only.
    BracketB,
    /// Brackets at both ends.
    BracketAB,
    /// Bars (zero-length brackets) at both ends.
    Bars,
}

/// The ends and accepted parameters of a curve style name.
fn curve_kind(name: &str) -> Option<(CurveEnd, CurveEnd, CurveParameters)> {
    use CurveEnd::{Bracket, FilledHead, Head, Plain};
    Some(match name {
        "-" => (Plain, Plain, CurveParameters::Nothing),
        "<-" => (Head, Plain, CurveParameters::All),
        "->" => (Plain, Head, CurveParameters::All),
        "<->" => (Head, Head, CurveParameters::All),
        "<|-" => (FilledHead, Plain, CurveParameters::All),
        "-|>" => (Plain, FilledHead, CurveParameters::All),
        "<|-|>" => (FilledHead, FilledHead, CurveParameters::All),
        "]-" => (Bracket, Plain, CurveParameters::BracketA),
        "-[" => (Plain, Bracket, CurveParameters::BracketB),
        "]-[" => (Bracket, Bracket, CurveParameters::BracketAB),
        "|-|" => (Bracket, Bracket, CurveParameters::Bars),
        "]->" => (Bracket, Head, CurveParameters::BracketA),
        "<-[" => (Head, Bracket, CurveParameters::BracketB),
        _ => return None,
    })
}

/// Overwrites `field` with the attribute `key` of `spec`, if given.
fn take_into(spec: &mut StyleSpec, key: &str, field: &mut f64) {
    if let Some(value) = spec.take(key) {
        *field = value;
    }
}

/// Reads the curve parameters `accepted` allows from `spec`.
fn curve_style(
    spec: &mut StyleSpec,
    begin: CurveEnd,
    end: CurveEnd,
    accepted: CurveParameters,
) -> CurveArrow {
    let mut curve = CurveArrow::new(begin, end);
    let (a, b, heads) = match accepted {
        CurveParameters::Nothing => {
            curve.head_length = 0.2;
            curve.head_width = 0.1;
            (false, false, false)
        }
        CurveParameters::All => (true, true, true),
        CurveParameters::BracketA => (true, false, false),
        CurveParameters::BracketB => (false, true, false),
        CurveParameters::BracketAB | CurveParameters::Bars => (true, true, false),
    };
    if heads {
        take_into(spec, "head_length", &mut curve.head_length);
        take_into(spec, "head_width", &mut curve.head_width);
    }
    let bars = matches!(accepted, CurveParameters::Bars);
    for (enabled, bracket, suffix) in [
        (a, &mut curve.bracket_a, "A"),
        (b, &mut curve.bracket_b, "B"),
    ] {
        if bars {
            bracket.length = 0.0;
        }
        if !enabled {
            continue;
        }
        take_into(spec, &format!("width{suffix}"), &mut bracket.width);
        take_into(spec, &format!("angle{suffix}"), &mut bracket.angle);
        if !bars {
            take_into(spec, &format!("length{suffix}"), &mut bracket.length);
        }
        if heads {
            bracket.scale = spec.take(&format!("scale{suffix}"));
        }
    }
    curve
}

impl FromStr for ArrowStyle {
    type Err = StyleError;

    /// Parses matplotlib's description, e.g. `"->"`, `"-[,widthB=1.5"` or
    /// `"fancy,head_length=0.6"`.
    fn from_str(text: &str) -> Result<Self, StyleError> {
        let mut spec = StyleSpec::parse(text)?;
        let take =
            |spec: &mut StyleSpec, key: &str, default: f64| spec.take(key).unwrap_or(default);
        let name = spec.name.clone();
        let style = match name.as_str() {
            "simple" => Self::Simple {
                head_length: take(&mut spec, "head_length", 0.5),
                head_width: take(&mut spec, "head_width", 0.5),
                tail_width: take(&mut spec, "tail_width", 0.2),
            },
            "fancy" => Self::Fancy {
                head_length: take(&mut spec, "head_length", 0.4),
                head_width: take(&mut spec, "head_width", 0.4),
                tail_width: take(&mut spec, "tail_width", 0.4),
            },
            "wedge" => Self::Wedge {
                tail_width: take(&mut spec, "tail_width", 0.3),
                shrink_factor: take(&mut spec, "shrink_factor", 0.5),
            },
            name => {
                let (begin, end, accepted) = curve_kind(name).ok_or_else(|| spec.unknown())?;
                Self::Curve(curve_style(&mut spec, begin, end, accepted))
            }
        };
        spec.finish()?;
        Ok(style)
    }
}

#[cfg(test)]
mod tests;
