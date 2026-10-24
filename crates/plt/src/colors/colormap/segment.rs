//! The channel definitions of linearly segmented colormaps and matplotlib's
//! `_create_lookup_table`.

use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;

use plt_core::numeric::np_clip;

use super::{Channel, ColormapError, SegmentProblem};
use crate::colors::npmath::{np_less, np_power};
use crate::numeric::linspace;

/// A scalar function of `x ∈ [0, 1]` defining one colour channel, with a
/// name for display (matplotlib accepts any callable as segment data).
#[derive(Clone)]
pub struct SegmentFn {
    name: Cow<'static, str>,
    func: Arc<dyn Fn(f64) -> f64 + Send + Sync>,
}

impl SegmentFn {
    /// Wraps `func` under a descriptive `name`.
    pub fn new(
        name: impl Into<Cow<'static, str>>,
        func: impl Fn(f64) -> f64 + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            func: Arc::new(func),
        }
    }

    /// The function's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Evaluates the function.
    pub fn eval(&self, x: f64) -> f64 {
        (self.func)(x)
    }

    /// `x ↦ f(1 - x)`, named `reversed(<name>)` (matplotlib's `_reverser`).
    #[must_use]
    pub fn reversed(&self) -> Self {
        let func = Arc::clone(&self.func);
        Self::new(format!("reversed({})", self.name), move |x| func(1.0 - x))
    }
}

impl fmt::Debug for SegmentFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SegmentFn").field(&self.name).finish()
    }
}

/// How one channel varies along a [`LinearSegmented`](super::ColormapKind)
/// colormap.
#[derive(Clone, Debug)]
pub enum Segment {
    /// Anchor rows `[x, y_below, y_above]`: `x` runs from 0 to 1 without
    /// decreasing, and between anchors `i` and `i + 1` the channel
    /// interpolates linearly from `y_above[i]` to `y_below[i + 1]`, so a
    /// row with different `y` values makes a step.
    Anchors(Vec<[f64; 3]>),
    /// An arbitrary function of `x`.
    Func(SegmentFn),
}

impl Segment {
    /// The channel of the reversed colormap: anchors mirrored to `1 - x`
    /// (swapping the two `y` columns), functions evaluated at `1 - x`.
    #[must_use]
    pub fn reversed(&self) -> Self {
        match self {
            Self::Anchors(rows) => Self::Anchors(
                rows.iter()
                    .rev()
                    .map(|&[x, below, above]| [1.0 - x, above, below])
                    .collect(),
            ),
            Self::Func(func) => Self::Func(func.reversed()),
        }
    }
}

/// The per-channel definition of a linearly segmented colormap
/// (matplotlib's `segmentdata` dictionary).
#[derive(Clone, Debug)]
pub struct SegmentData {
    /// Red channel.
    pub red: Segment,
    /// Green channel.
    pub green: Segment,
    /// Blue channel.
    pub blue: Segment,
    /// Optional alpha channel (opaque when absent).
    pub alpha: Option<Segment>,
}

impl SegmentData {
    /// Opaque segment data from the three colour channels.
    pub fn new(red: Segment, green: Segment, blue: Segment) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: None,
        }
    }

    /// The same data with an alpha channel.
    #[must_use]
    pub fn with_alpha(self, alpha: Segment) -> Self {
        Self {
            alpha: Some(alpha),
            ..self
        }
    }

    /// The channel's segment, if defined.
    pub fn channel(&self, channel: Channel) -> Option<&Segment> {
        match channel {
            Channel::Red => Some(&self.red),
            Channel::Green => Some(&self.green),
            Channel::Blue => Some(&self.blue),
            Channel::Alpha => self.alpha.as_ref(),
        }
    }

    /// The data of the reversed colormap.
    #[must_use]
    pub fn reversed(&self) -> Self {
        Self {
            red: self.red.reversed(),
            green: self.green.reversed(),
            blue: self.blue.reversed(),
            alpha: self.alpha.as_ref().map(Segment::reversed),
        }
    }

    /// Dave Green's cubehelix scheme (matplotlib's `_cm.cubehelix`):
    /// brightness rises monotonically from black to white while the hue
    /// rotates `r` times starting at `s`, with saturation `h`; `gamma`
    /// emphasises low (`< 1`) or high (`> 1`) intensities.
    ///
    /// matplotlib's built-in `"cubehelix"` uses
    /// `cubehelix(1.0, 0.5, -1.5, 1.0)`.
    pub fn cubehelix(gamma: f64, s: f64, r: f64, h: f64) -> Self {
        let channel = |p0, p1| Segment::Func(cubehelix_channel([gamma, s, r, h, p0, p1]));
        Self::new(
            channel(-0.14861, 1.78277),
            channel(-0.29227, -0.90649),
            channel(1.97294, 0.0),
        )
    }
}

/// One cubehelix channel, `_ch_helper(gamma, s, r, h, p0, p1, x)`, named
/// like matplotlib's `functools.partial` of it.
pub(crate) fn cubehelix_channel(params: [f64; 6]) -> SegmentFn {
    let [gamma, s, r, h, p0, p1] = params;
    let name = format!("_ch_helper({gamma:?}, {s:?}, {r:?}, {h:?}, {p0:?}, {p1:?})");
    SegmentFn::new(name, move |x| {
        let xg = np_power(x, gamma);
        let a = h * xg * (1.0 - xg) / 2.0;
        let phi = 2.0 * std::f64::consts::PI * (s / 3.0 + r * x);
        xg + a * (p0 * phi.cos() + p1 * phi.sin())
    })
}

/// matplotlib's `_create_lookup_table(N, data, gamma)`: the channel sampled
/// at `linspace(0, 1, n) ** gamma`, clipped to `[0, 1]` (NaN stays NaN).
pub(crate) fn lookup_table(
    n: usize,
    segment: &Segment,
    gamma: f64,
    channel: Channel,
) -> Result<Vec<f64>, ColormapError> {
    let invalid = |problem| ColormapError::Segment { channel, problem };
    let table = match segment {
        Segment::Func(func) => linspace(0.0, 1.0, n)
            .into_iter()
            .map(|x| func.eval(np_power(x, gamma)))
            .collect(),
        Segment::Anchors(rows) => anchors_table(n, rows, gamma).map_err(invalid)?,
    };
    Ok(table.into_iter().map(|v| np_clip(v, 0.0, 1.0)).collect())
}

/// The anchor-table branch of `_create_lookup_table`, before clipping.
fn anchors_table(n: usize, rows: &[[f64; 3]], gamma: f64) -> Result<Vec<f64>, SegmentProblem> {
    let (Some(first), Some(last)) = (rows.first(), rows.last()) else {
        return Err(SegmentProblem::Empty);
    };
    if first[0] != 0.0 || last[0] != 1.0 {
        return Err(SegmentProblem::Endpoints);
    }
    if rows.windows(2).any(|pair| pair[1][0] - pair[0][0] < 0.0) {
        return Err(SegmentProblem::Decreasing);
    }
    if n == 1 {
        // matplotlib's convention: a one-entry table holds f(1).
        return Ok(vec![last[1]]);
    }
    let span = (n - 1) as f64;
    let x: Vec<f64> = rows.iter().map(|row| row[0] * span).collect();
    let samples = linspace(0.0, 1.0, n);
    let mut table = Vec::with_capacity(n);
    table.push(first[2]);
    for &t in &samples[1..n - 1] {
        let xind = span * np_power(t, gamma);
        // numpy `searchsorted(x, xind)` (left side); index -1 wraps.
        let ind = x.partition_point(|&xi| np_less(xi, xind));
        if ind == x.len() {
            return Err(SegmentProblem::Gamma);
        }
        let prev = (ind + x.len() - 1) % x.len();
        let distance = (xind - x[prev]) / (x[ind] - x[prev]);
        table.push(distance * (rows[ind][1] - rows[prev][2]) + rows[prev][2]);
    }
    table.push(last[1]);
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchors(rows: &[[f64; 3]]) -> Segment {
        Segment::Anchors(rows.to_vec())
    }

    #[test]
    fn linear_ramp_and_gamma() {
        let ramp = anchors(&[[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]]);
        assert_eq!(
            lookup_table(5, &ramp, 1.0, Channel::Red).unwrap(),
            [0.0, 0.25, 0.5, 0.75, 1.0]
        );
        assert_eq!(
            lookup_table(3, &ramp, 2.0, Channel::Red).unwrap(),
            [0.0, 0.25, 1.0]
        );
        assert_eq!(lookup_table(1, &ramp, 1.0, Channel::Red).unwrap(), [1.0]);
    }

    #[test]
    fn steps_overshoot_and_functions() {
        let step = anchors(&[[0.0, 0.0, 0.0], [0.5, 0.0, 1.0], [1.0, 1.0, 1.0]]);
        assert_eq!(
            lookup_table(4, &step, 1.0, Channel::Green).unwrap(),
            [0.0, 0.0, 1.0, 1.0]
        );
        let overshoot = anchors(&[[0.0, -0.5, -0.5], [1.0, 1.5, 1.5]]);
        assert_eq!(
            lookup_table(3, &overshoot, 1.0, Channel::Red).unwrap(),
            [0.0, 0.5, 1.0]
        );
        let func = Segment::Func(SegmentFn::new("double", |x| 2.0 * x));
        assert_eq!(
            lookup_table(3, &func, 1.0, Channel::Blue).unwrap(),
            [0.0, 1.0, 1.0]
        );
    }

    #[test]
    fn invalid_anchors() {
        let check = |rows: &[[f64; 3]], problem| {
            assert_eq!(
                lookup_table(4, &anchors(rows), 1.0, Channel::Alpha),
                Err(ColormapError::Segment {
                    channel: Channel::Alpha,
                    problem
                })
            );
        };
        check(&[], SegmentProblem::Empty);
        check(
            &[[0.1, 0.0, 0.0], [1.0, 1.0, 1.0]],
            SegmentProblem::Endpoints,
        );
        check(
            &[
                [0.0, 0.0, 0.0],
                [0.7, 1.0, 1.0],
                [0.3, 0.0, 0.0],
                [1.0, 1.0, 1.0],
            ],
            SegmentProblem::Decreasing,
        );
        assert_eq!(
            lookup_table(
                4,
                &anchors(&[[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]]),
                -1.0,
                Channel::Red
            ),
            Err(ColormapError::Segment {
                channel: Channel::Red,
                problem: SegmentProblem::Gamma
            })
        );
    }

    #[test]
    fn reversal() {
        let step = anchors(&[[0.0, 0.0, 0.2], [0.25, 0.4, 0.6], [1.0, 1.0, 1.0]]);
        let Segment::Anchors(rows) = step.reversed() else {
            panic!("anchors stay anchors");
        };
        assert_eq!(rows, [[0.0, 1.0, 1.0], [0.75, 0.6, 0.4], [1.0, 0.2, 0.0]]);
        let f = SegmentFn::new("f", |x| x * x).reversed();
        assert_eq!(f.name(), "reversed(f)");
        assert_eq!(f.eval(0.25), 0.5625);
    }
}
