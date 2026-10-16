//! Draw styles: connecting points directly or with steps
//! (matplotlib's `drawstyle` and `cbook.pts_to_*step`).

use std::fmt;
use std::str::FromStr;

/// How consecutive data points are connected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DrawStyle {
    /// Straight segments between points.
    #[default]
    Default,
    /// Alias of [`StepsPre`](Self::StepsPre), reported as `"steps"`.
    Steps,
    /// The step happens at the start of each interval.
    StepsPre,
    /// The step happens halfway between points.
    StepsMid,
    /// The step happens at the end of each interval.
    StepsPost,
}

const NAMES: [(&str, DrawStyle); 5] = [
    ("default", DrawStyle::Default),
    ("steps-mid", DrawStyle::StepsMid),
    ("steps-pre", DrawStyle::StepsPre),
    ("steps-post", DrawStyle::StepsPost),
    ("steps", DrawStyle::Steps),
];

impl DrawStyle {
    /// Every draw style with its name, in matplotlib's `drawStyleKeys` order.
    pub fn all() -> impl Iterator<Item = (&'static str, Self)> {
        NAMES.into_iter()
    }

    /// matplotlib's name.
    pub fn name(self) -> &'static str {
        NAMES
            .iter()
            .find(|(_, style)| *style == self)
            .map_or("default", |(name, _)| name)
    }

    /// Converts point coordinates into the vertices actually drawn.
    /// `ys` holds one or more series sharing `x`.
    pub fn apply(self, x: &[f64], ys: &[&[f64]]) -> Result<Vec<Vec<f64>>, StepError> {
        match self {
            Self::Default => {
                let mut rows = vec![x.to_vec()];
                for y in ys {
                    rows.push(broadcast(y, x.len())?);
                }
                Ok(rows)
            }
            Self::Steps | Self::StepsPre => pts_to_prestep(x, ys),
            Self::StepsMid => pts_to_midstep(x, ys),
            Self::StepsPost => pts_to_poststep(x, ys),
        }
    }
}

impl FromStr for DrawStyle {
    type Err = StepError;

    fn from_str(name: &str) -> Result<Self, StepError> {
        NAMES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, style)| *style)
            .ok_or_else(|| StepError::UnknownStyle(name.to_owned()))
    }
}

impl fmt::Display for DrawStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The vertices a `Line2D` draws for data `(x, y)`: length-1 data is
/// broadcast against the other coordinate (as numpy does), then the draw
/// style is applied.
pub fn line_vertices(x: &[f64], y: &[f64], style: DrawStyle) -> Result<Vec<plt_core::Point>, StepError> {
    let n = x.len().max(y.len());
    let (x, y) = if x.len() == y.len() || x.is_empty() || y.is_empty() {
        (x.to_vec(), y.to_vec())
    } else {
        (broadcast(x, n).map_err(|_| mismatch(x, y))?, broadcast(y, n).map_err(|_| mismatch(x, y))?)
    };
    if x.len() != y.len() {
        return Err(mismatch(&x, &y));
    }
    let rows = style.apply(&x, &[&y])?;
    Ok(rows[0].iter().zip(&rows[1]).map(|(&x, &y)| plt_core::Point::new(x, y)).collect())
}

fn mismatch(x: &[f64], y: &[f64]) -> StepError {
    StepError::LengthMismatch { x: x.len(), y: y.len() }
}

/// Invalid draw style input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepError {
    /// Not one of matplotlib's draw style names.
    UnknownStyle(String),
    /// A y series cannot be broadcast against x.
    LengthMismatch {
        /// Number of x values.
        x: usize,
        /// Number of y values.
        y: usize,
    },
    /// At least one y series is required for non-empty x.
    NoSeries,
}

impl fmt::Display for StepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownStyle(name) => write!(f, "{name:?} is not a valid drawstyle"),
            Self::LengthMismatch { x, y } => write!(f, "y has {y} values but x has {x}"),
            Self::NoSeries => f.write_str("at least one y series is required"),
        }
    }
}

impl std::error::Error for StepError {}

fn broadcast(y: &[f64], n: usize) -> Result<Vec<f64>, StepError> {
    match y.len() {
        len if len == n => Ok(y.to_vec()),
        1 => Ok(vec![y[0]; n]),
        len => Err(StepError::LengthMismatch { x: n, y: len }),
    }
}

fn series(x: &[f64], ys: &[&[f64]]) -> Result<Vec<Vec<f64>>, StepError> {
    if ys.is_empty() && !x.is_empty() {
        return Err(StepError::NoSeries);
    }
    ys.iter().map(|y| broadcast(y, x.len())).collect()
}

/// Steps where each interval takes the *next* point's y value at its start
/// (`2N - 1` vertices).
pub fn pts_to_prestep(x: &[f64], ys: &[&[f64]]) -> Result<Vec<Vec<f64>>, StepError> {
    let ys = series(x, ys)?;
    let mut rows = vec![interleave(x, |i| (x[i], x[i]))];
    rows.extend(ys.iter().map(|y| interleave(y, |i| (y[i], y[i + 1]))));
    Ok(rows)
}

/// Steps where each interval keeps the current y until its end
/// (`2N - 1` vertices).
pub fn pts_to_poststep(x: &[f64], ys: &[&[f64]]) -> Result<Vec<Vec<f64>>, StepError> {
    let ys = series(x, ys)?;
    let mut rows = vec![interleave(x, |i| (x[i], x[i + 1]))];
    rows.extend(ys.iter().map(|y| interleave(y, |i| (y[i], y[i]))));
    Ok(rows)
}

/// Steps halfway between points (`2N` vertices).
pub fn pts_to_midstep(x: &[f64], ys: &[&[f64]]) -> Result<Vec<Vec<f64>>, StepError> {
    let ys = series(x, ys)?;
    let n = x.len();
    let mut xs = Vec::with_capacity(2 * n);
    if let (Some(&first), Some(&last)) = (x.first(), x.last()) {
        xs.push(first);
        for pair in x.windows(2) {
            let mid = f64::midpoint(pair[0], pair[1]);
            xs.extend([mid, mid]);
        }
        xs.push(last);
    }
    let mut rows = vec![xs];
    rows.extend(ys.iter().map(|y| y.iter().flat_map(|&v| [v, v]).collect()));
    Ok(rows)
}

/// `[v0, f(0).1, v1, f(1).1, ..., v(n-1)]` where `f(i).0 == v(i)`: the
/// `2N - 1`-long interleaving used by the pre/post step functions.
fn interleave(values: &[f64], pair: impl Fn(usize) -> (f64, f64)) -> Vec<f64> {
    let n = values.len();
    let mut out = Vec::with_capacity((2 * n).saturating_sub(1));
    for i in 0..n.saturating_sub(1) {
        let (at, between) = pair(i);
        out.extend([at, between]);
    }
    out.extend(values.last());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const X: [f64; 4] = [0.0, 1.0, 2.0, 3.0];
    const Y: [f64; 4] = [0.0, 1.0, 4.0, 9.0];

    #[test]
    fn prestep_matches_matplotlib() {
        let rows = pts_to_prestep(&X, &[&Y]).expect("valid");
        assert_eq!(rows[0], [0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 3.0]);
        assert_eq!(rows[1], [0.0, 1.0, 1.0, 4.0, 4.0, 9.0, 9.0]);
    }

    #[test]
    fn poststep_matches_matplotlib() {
        let rows = pts_to_poststep(&X, &[&Y]).expect("valid");
        assert_eq!(rows[0], [0.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0]);
        assert_eq!(rows[1], [0.0, 0.0, 1.0, 1.0, 4.0, 4.0, 9.0]);
    }

    #[test]
    fn midstep_matches_matplotlib() {
        let rows = pts_to_midstep(&X, &[&Y]).expect("valid");
        assert_eq!(rows[0], [0.0, 0.5, 0.5, 1.5, 1.5, 2.5, 2.5, 3.0]);
        assert_eq!(rows[1], [0.0, 0.0, 1.0, 1.0, 4.0, 4.0, 9.0, 9.0]);
    }

    #[test]
    fn broadcasting_and_errors() {
        let rows = pts_to_prestep(&X, &[&[5.0]]).expect("broadcasts");
        assert_eq!(rows[1], [5.0; 7]);
        assert!(pts_to_prestep(&X, &[&[1.0, 2.0]]).is_err());
        assert_eq!(pts_to_prestep(&X, &[]), Err(StepError::NoSeries));
        assert_eq!(pts_to_prestep(&[], &[]), Ok(vec![Vec::new()]));
    }

    #[test]
    fn line_vertices_broadcast_single_values() {
        let vertical = line_vertices(&[0.0], &[1.0, 2.0, 3.0], DrawStyle::Default).expect("broadcasts");
        assert_eq!(vertical.len(), 3);
        assert!(line_vertices(&[0.0, 1.0], &[1.0, 2.0, 3.0], DrawStyle::Default).is_err());
        assert!(line_vertices(&[], &[], DrawStyle::StepsMid).expect("empty").is_empty());
    }

    #[test]
    fn names() {
        assert_eq!("steps".parse(), Ok(DrawStyle::Steps));
        assert!("Steps-pre".parse::<DrawStyle>().is_err());
        assert_eq!(DrawStyle::StepsMid.to_string(), "steps-mid");
    }
}
