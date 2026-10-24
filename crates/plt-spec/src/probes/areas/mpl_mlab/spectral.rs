//! `TestSpectral`'s operations: the spectra of the class's fixture signal,
//! recorded as summaries (the shape, sampled frequencies, times and
//! magnitude sums the tests check).

use std::collections::BTreeSet;
use std::f64::consts::PI;

use plt::mlab::{self, Complex, MlabError, SignalValue, SpecgramMode};

use super::super::stats::{spectral_options, spectrum_options};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

/// The fixture's sampling frequency.
const FS: f64 = 100.0;

/// The length of the fixture's time axis, `np.arange(0, 10, 1 / Fs)`.
const SAMPLES: u32 = 1000;

/// The fixture's signal, real or complex.
enum Stimulus {
    Real(Vec<f64>),
    Complex(Vec<Complex>),
}

/// Calls `$body` with `$x` bound to the samples of a [`Stimulus`].
macro_rules! on_stimulus {
    ($stimulus:expr, |$x:ident| $body:expr) => {
        match $stimulus {
            Stimulus::Real($x) => $body,
            Stimulus::Complex($x) => $body,
        }
    };
}

/// `y = sum(sin(fstim * x * pi * 2) * 10**i)` over the stimulus
/// frequencies of `stim`, at `x = np.arange(0, 10, 1 / Fs)[:len_x]`.
fn stimulus(args: Args<'_>) -> Result<Stimulus, ProbeError> {
    let stim = Args::new(args.json("stim")?);
    let fstims = stim.f64_vec("fstims")?;
    let len = match stim.opt_i64("len_x")? {
        None => SAMPLES,
        Some(n) => u32::try_from(n)
            .map_err(|_| ProbeError::BadArgs(format!("bad len_x {n}")))?
            .min(SAMPLES),
    };
    let step = 1.0 / FS;
    let y: Vec<f64> = (0..len)
        .map(|k| {
            let x = f64::from(k) * step;
            (0..).zip(&fstims).fold(0.0, |y, (i, f)| {
                y + (f * x * PI * 2.0).sin() * 10f64.powi(i)
            })
        })
        .collect();
    Ok(if stim.bool("iscomplex")? {
        Stimulus::Complex(y.into_iter().map(|v| Complex::new(v, 0.0)).collect())
    } else {
        Stimulus::Real(y)
    })
}

/// The summary of a spectrum: its shape, its frequencies at a few
/// indices, the times of a spectrogram, and the sum, sum of squares and
/// largest of the `magnitudes` of its values when given.
fn summary(
    shape: &[usize],
    freqs: &[f64],
    times: Option<&[f64]>,
    magnitudes: Option<Vec<f64>>,
) -> Json {
    let n = freqs.len();
    let picks: BTreeSet<usize> = [0, n / 4, n / 2, 3 * n / 4, n.saturating_sub(1)]
        .into_iter()
        .collect();
    let mut fields = vec![
        (
            "shape".to_owned(),
            Json::Array(shape.iter().map(|&d| Json::from(d as f64)).collect()),
        ),
        (
            "freqs".to_owned(),
            Json::from(
                picks
                    .into_iter()
                    .filter_map(|i| freqs.get(i).copied())
                    .collect::<Vec<_>>(),
            ),
        ),
    ];
    if let Some(times) = times {
        fields.push(("t".to_owned(), Json::from(times.to_vec())));
    }
    if let Some(magnitudes) = magnitudes {
        let sum = magnitudes.iter().sum::<f64>();
        let sumsq = magnitudes.iter().map(|m| m * m).sum::<f64>();
        let max = magnitudes.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        fields.extend([
            ("sum".to_owned(), Json::from(sum)),
            ("sumsq".to_owned(), Json::from(sumsq)),
            ("max".to_owned(), Json::from(max)),
        ]);
    }
    Json::Object(fields)
}

fn magnitudes(values: &[Complex]) -> Vec<f64> {
    values.iter().map(|z| z.abs()).collect()
}

pub(super) fn psd(args: Args<'_>) -> Outcome {
    let opts = mlab::SpectralOptions {
        fs: FS,
        ..spectral_options(args)?
    };
    let p = on_stimulus!(stimulus(args)?, |x| mlab::psd(&x, opts)).map_err(ProbeError::rejected)?;
    Ok(summary(
        &[p.values.len()],
        &p.freqs,
        None,
        Some(p.values.iter().map(|v| v.abs()).collect()),
    ))
}

/// `csd(x, x + offset)` for samples of any type.
fn cross<S: SignalValue>(
    x: &[S],
    offset: f64,
    opts: mlab::SpectralOptions,
) -> Result<mlab::Spectrum<Complex>, MlabError> {
    let y: Vec<Complex> = x
        .iter()
        .map(|&v| {
            let z: Complex = v.into();
            Complex::new(z.re + offset, z.im)
        })
        .collect();
    if S::COMPLEX {
        mlab::csd(x, &y, opts)
    } else {
        let real: Vec<f64> = y.iter().map(|z| z.re).collect();
        mlab::csd(x, &real, opts)
    }
}

pub(super) fn csd(args: Args<'_>) -> Outcome {
    let opts = mlab::SpectralOptions {
        fs: FS,
        ..spectral_options(args)?
    };
    let offset = args.f64("y_offset")?;
    let p =
        on_stimulus!(stimulus(args)?, |x| cross(&x, offset, opts)).map_err(ProbeError::rejected)?;
    Ok(summary(
        &[p.values.len()],
        &p.freqs,
        None,
        Some(magnitudes(&p.values)),
    ))
}

pub(super) fn specgram(args: Args<'_>) -> Outcome {
    let opts = mlab::SpectralOptions {
        fs: FS,
        ..spectral_options(args)?
    };
    let x = stimulus(args)?;
    let mode = match args.opt_str("mode")? {
        None | Some("default" | "psd") => SpecgramMode::Psd,
        Some("magnitude") => SpecgramMode::Magnitude,
        Some("angle") => SpecgramMode::Angle,
        Some("phase") => SpecgramMode::Phase,
        Some("complex") => {
            let s = on_stimulus!(x, |x| mlab::complex_specgram(&x, opts))
                .map_err(ProbeError::rejected)?;
            return Ok(summary(
                &[s.values.rows(), s.values.cols()],
                &s.freqs,
                Some(&s.times),
                Some(magnitudes(s.values.as_slice())),
            ));
        }
        Some(other) => return Err(ProbeError::rejected(format!("bad mode {other:?}"))),
    };
    let s = on_stimulus!(x, |x| mlab::specgram(&x, opts, mode)).map_err(ProbeError::rejected)?;
    let values = matches!(mode, SpecgramMode::Psd | SpecgramMode::Magnitude)
        .then(|| s.values.as_slice().iter().map(|v| v.abs()).collect());
    Ok(summary(
        &[s.values.rows(), s.values.cols()],
        &s.freqs,
        Some(&s.times),
        values,
    ))
}

/// The spectrum options of a probe, at the fixture's sampling frequency.
fn single_options(args: Args<'_>) -> Result<mlab::SpectrumOptions, ProbeError> {
    Ok(mlab::SpectrumOptions {
        fs: FS,
        ..spectrum_options(args)?
    })
}

pub(super) fn complex_spectrum(args: Args<'_>) -> Outcome {
    let opts = single_options(args)?;
    let s = on_stimulus!(stimulus(args)?, |x| mlab::complex_spectrum(&x, opts))
        .map_err(ProbeError::rejected)?;
    Ok(summary(
        &[s.values.len()],
        &s.freqs,
        None,
        Some(magnitudes(&s.values)),
    ))
}

/// A real single spectrum, with its values summarised when `values`.
fn real_spectrum<F, G>(args: Args<'_>, real: F, complex: G, values: bool) -> Outcome
where
    F: Fn(&[f64], mlab::SpectrumOptions) -> Result<mlab::Spectrum, MlabError>,
    G: Fn(&[Complex], mlab::SpectrumOptions) -> Result<mlab::Spectrum, MlabError>,
{
    let opts = single_options(args)?;
    let s = match stimulus(args)? {
        Stimulus::Real(x) => real(&x, opts),
        Stimulus::Complex(x) => complex(&x, opts),
    }
    .map_err(ProbeError::rejected)?;
    let magnitudes = values.then(|| s.values.iter().map(|v| v.abs()).collect());
    Ok(summary(&[s.values.len()], &s.freqs, None, magnitudes))
}

pub(super) fn magnitude_spectrum(args: Args<'_>) -> Outcome {
    real_spectrum(
        args,
        mlab::magnitude_spectrum,
        mlab::magnitude_spectrum,
        true,
    )
}

/// Angles are rounding noise where the spectrum vanishes: no values.
pub(super) fn angle_spectrum(args: Args<'_>) -> Outcome {
    real_spectrum(args, mlab::angle_spectrum, mlab::angle_spectrum, false)
}

/// Phases are rounding noise where the spectrum vanishes: no values.
pub(super) fn phase_spectrum(args: Args<'_>) -> Outcome {
    real_spectrum(args, mlab::phase_spectrum, mlab::phase_spectrum, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summaries_sample_five_frequencies() {
        let freqs: Vec<f64> = (0..9).map(f64::from).collect();
        let json = summary(&[9], &freqs, None, Some(vec![3.0, -4.0]));
        assert_eq!(
            json.get("freqs"),
            Some(&Json::from(vec![0.0, 2.0, 4.0, 6.0, 8.0]))
        );
        assert_eq!(json.get("sumsq").and_then(Json::as_f64), Some(25.0));
        assert_eq!(json.get("max").and_then(Json::as_f64), Some(3.0));
    }
}
