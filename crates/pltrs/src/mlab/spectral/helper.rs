//! The engine shared by every spectral estimate (`mlab._spectral_helper`).

use pltrs_core::numeric::remainder;

use super::{SignalValue, SpectralOptions, SpectrumSides};
use crate::mlab::fft::{FftPlan, fftfreq};
use crate::mlab::{Complex, MAX_FFT_LENGTH, MlabError};
use crate::numeric::{Grid, arange};
use crate::stats::pairwise;

/// What the helper computes per frequency and segment (matplotlib's
/// `mode` of `_spectral_helper`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// (Cross) power spectral density `conj(X)·Y`, scaled.
    Psd,
    /// `X / sum(w)`.
    Complex,
    /// `|X| / sum(w)`.
    Magnitude,
    /// `angle(X)`.
    Angle,
    /// `angle(X)`, unwrapped along frequency.
    Phase,
}

/// A signal converted to complex samples, remembering whether it was real.
#[derive(Clone, Debug)]
pub(super) struct Signal {
    samples: Vec<Complex>,
    complex: bool,
}

impl Signal {
    pub(super) fn new<S: SignalValue>(x: &[S]) -> Self {
        Self {
            samples: x.iter().map(|&v| v.into()).collect(),
            complex: S::COMPLEX,
        }
    }
}

/// The unaveraged result: `values` is `frequencies × segments`.
#[derive(Clone, Debug)]
pub(super) struct Raw {
    pub(super) values: Grid<Complex>,
    pub(super) freqs: Vec<f64>,
    pub(super) times: Vec<f64>,
}

/// matplotlib's `_spectral_helper` with `noverlap` already defaulted.
pub(super) fn spectral_helper(
    x: &Signal,
    y: Option<&Signal>,
    opts: &SpectralOptions,
    noverlap: usize,
    mode: Mode,
) -> Result<Raw, MlabError> {
    let nfft = opts.nfft;
    if noverlap >= nfft {
        return Err(MlabError::InvalidOverlap { noverlap, nfft });
    }
    let twosided = match opts.sides {
        SpectrumSides::Default => x.complex,
        SpectrumSides::OneSided => false,
        SpectrumSides::TwoSided => true,
    };
    let pad_to = opts.pad_to.unwrap_or(nfft);
    if pad_to == 0 {
        return Err(MlabError::ZeroLengthFft);
    }
    if let Some(&len) = [nfft, pad_to].iter().find(|&&len| len > MAX_FFT_LENGTH) {
        return Err(MlabError::FftTooLong(len));
    }
    let num_freqs = match (twosided, !pad_to.is_multiple_of(2)) {
        (true, _) => pad_to,
        (false, true) => pad_to.div_ceil(2),
        (false, false) => pad_to / 2 + 1,
    };
    let window = opts.window.values(nfft)?;
    let segment = SegmentTransform {
        nfft,
        step: nfft - noverlap,
        window: &window,
        plan: FftPlan::new(pad_to),
        num_freqs,
        opts,
    };
    let x_padded = zero_padded(&x.samples, nfft);
    let mut values = segment.transform(&x_padded, x.complex)?;
    match (y, mode) {
        (Some(y), _) => {
            let y_spec = segment.transform(&zero_padded(&y.samples, nfft), y.complex)?;
            values = cross(&values, &y_spec)?;
        }
        (None, Mode::Psd) => values = values.map(|&z| z.conj() * z),
        (None, Mode::Magnitude) => {
            let sum = pairwise(&window, 0.0);
            values = values.map(|z| Complex::real(z.abs() / sum));
        }
        (None, Mode::Angle | Mode::Phase) => values = values.map(|z| Complex::real(z.arg())),
        (None, Mode::Complex) => {
            let sum = Complex::real(pairwise(&window, 0.0));
            values = values.map(|&z| z / sum);
        }
    }
    if mode == Mode::Psd {
        scale_density(&mut values, &window, opts, twosided);
    }
    let mut freqs: Vec<f64> = fftfreq(pad_to, 1.0 / opts.fs)
        .into_iter()
        .take(num_freqs)
        .collect();
    let half = nfft as f64 / 2.0;
    let times = arange(
        half,
        x_padded.len() as f64 - half + 1.0,
        segment.step as f64,
    )
    .into_iter()
    .map(|t| t / opts.fs)
    .collect();
    if twosided {
        let centre = if pad_to.is_multiple_of(2) {
            pad_to / 2
        } else {
            (pad_to - 1) / 2 + 1
        };
        let len = freqs.len().max(1);
        freqs.rotate_left(centre % len);
        values = roll_rows(&values, centre);
    } else if pad_to.is_multiple_of(2)
        && let Some(last) = freqs.last_mut()
    {
        *last = -*last;
    }
    if mode == Mode::Phase {
        values = unwrap_columns(&values);
    }
    Ok(Raw {
        values,
        freqs,
        times,
    })
}

/// Segmenting, detrending, windowing and transforming one signal.
struct SegmentTransform<'a> {
    nfft: usize,
    step: usize,
    window: &'a [f64],
    plan: FftPlan,
    num_freqs: usize,
    opts: &'a SpectralOptions,
}

impl SegmentTransform<'_> {
    /// `frequencies × segments` spectra of the segments of `signal`.
    fn transform(&self, signal: &[Complex], complex: bool) -> Result<Grid<Complex>, MlabError> {
        let count = (signal.len() - self.nfft) / self.step + 1;
        let mut columns = Vec::with_capacity(count);
        for start in (0..count).map(|j| j * self.step) {
            let segment = &signal[start..start + self.nfft];
            let detrended = self.opts.detrend.apply_complex(segment, complex)?;
            let windowed: Vec<Complex> = detrended
                .iter()
                .zip(self.window)
                .map(|(&z, &w)| z * w)
                .collect();
            let mut spectrum = self.plan.forward(&windowed);
            spectrum.truncate(self.num_freqs);
            columns.push(spectrum);
        }
        Ok(Grid::from_fn(self.num_freqs, count, |f, s| columns[s][f]))
    }
}

/// `x` extended with zeros to at least `len` samples.
fn zero_padded(x: &[Complex], len: usize) -> Vec<Complex> {
    let mut padded = x.to_vec();
    if padded.len() < len {
        padded.resize(len, Complex::ZERO);
    }
    padded
}

/// `conj(X)·Y`, broadcasting a single segment against several.
fn cross(x: &Grid<Complex>, y: &Grid<Complex>) -> Result<Grid<Complex>, MlabError> {
    let (nx, ny) = (x.cols(), y.cols());
    if nx != ny && nx != 1 && ny != 1 {
        return Err(MlabError::SegmentCountMismatch { x: nx, y: ny });
    }
    let pick = |n: usize, s: usize| if n == 1 { 0 } else { s };
    Ok(Grid::from_fn(x.rows(), nx.max(ny), |f, s| {
        x[(f, pick(nx, s))].conj() * y[(f, pick(ny, s))]
    }))
}

/// One-sided doubling and density (or power) normalisation.
fn scale_density(
    values: &mut Grid<Complex>,
    window: &[f64],
    opts: &SpectralOptions,
    twosided: bool,
) {
    let rows = values.rows();
    let doubled = if opts.nfft.is_multiple_of(2) {
        1..rows.saturating_sub(1).max(1)
    } else {
        1..rows.max(1)
    };
    let factor = if twosided { 1.0 } else { 2.0 };
    let cols = values.cols();
    for f in doubled {
        for s in 0..cols {
            values[(f, s)] = values[(f, s)] * factor;
        }
    }
    let divisors: Vec<f64> = if opts.scale_by_freq {
        let squares: Vec<f64> = window.iter().map(|w| w * w).collect();
        vec![opts.fs, pairwise(&squares, 0.0)]
    } else {
        let sum = pairwise(window, 0.0);
        vec![sum * sum]
    };
    for divisor in divisors {
        let divisor = Complex::real(divisor);
        for z in values.as_mut_slice() {
            *z = *z / divisor;
        }
    }
}

/// `np.roll(values, -shift, axis=0)`.
fn roll_rows(values: &Grid<Complex>, shift: usize) -> Grid<Complex> {
    let rows = values.rows();
    if rows == 0 {
        return values.clone();
    }
    Grid::from_fn(rows, values.cols(), |r, c| values[((r + shift) % rows, c)])
}

/// `np.unwrap(values, axis=0)` of real angles stored as complex numbers.
fn unwrap_columns(values: &Grid<Complex>) -> Grid<Complex> {
    let mut out = values.clone();
    for c in 0..values.cols() {
        let phases = unwrap(&values.column(c).iter().map(|z| z.re).collect::<Vec<_>>());
        for (r, p) in phases.into_iter().enumerate() {
            out[(r, c)] = Complex::real(p);
        }
    }
    out
}

/// numpy `unwrap` with the default period `2π`: jumps larger than `π`
/// between neighbours are replaced by their `2π` complement.
fn unwrap(p: &[f64]) -> Vec<f64> {
    let period = 2.0 * std::f64::consts::PI;
    let high = period / 2.0;
    let mut out = p.to_vec();
    let mut correction = 0.0;
    for i in 1..p.len() {
        let dd = p[i] - p[i - 1];
        let mut ddmod = remainder(dd + high, period) - high;
        if ddmod == -high && dd > 0.0 {
            ddmod = high;
        }
        let ph_correct = if dd.abs() < high { 0.0 } else { ddmod - dd };
        correction += ph_correct;
        out[i] = p[i] + correction;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unwrap_matches_numpy() {
        let p = [0.0, 3.0, -3.0, -0.5, 3.1];
        let u = unwrap(&p);
        let tau = 2.0 * std::f64::consts::PI;
        assert_eq!(u[..2], [0.0, 3.0]);
        assert!((u[2] - (tau - 3.0)).abs() < 1e-14);
        assert!((u[3] - (tau - 0.5)).abs() < 1e-14);
        assert!((u[4] - 3.1).abs() < 1e-14);
        assert_eq!(unwrap(&[]), Vec::<f64>::new());
    }

    #[test]
    fn segment_broadcasting() {
        let a = Grid::filled(2, 2, Complex::new(1.0, 1.0));
        let b = Grid::filled(2, 1, Complex::new(2.0, 0.0));
        assert_eq!(cross(&a, &b).unwrap().shape(), (2, 2));
        let c = Grid::filled(2, 3, Complex::ONE);
        assert_eq!(
            cross(&a, &c).unwrap_err(),
            MlabError::SegmentCountMismatch { x: 2, y: 3 }
        );
    }
}
