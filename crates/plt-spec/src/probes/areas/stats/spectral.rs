//! `mlab.psd`, `csd`, `specgram`, `cohere` and the single spectra.

use plt::mlab::{
    self, Complex, Detrend, Sides, SignalValue, SpecgramMode, SpectralOptions, Spectrum,
    SpectrumOptions, Window,
};
use plt::numeric::Grid;

use super::{SignalArg, complex_json, floats, grid_json, object};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

fn rejected(err: mlab::MlabError) -> ProbeError {
    ProbeError::rejected(err)
}

fn parse_window(value: Option<&Json>) -> Result<Window, ProbeError> {
    match value {
        None => Ok(Window::Hanning),
        Some(Json::String(name)) => match name.as_str() {
            "hanning" => Ok(Window::Hanning),
            "none" => Ok(Window::Rectangular),
            _ => Err(ProbeError::BadArgs(format!("unknown window {name:?}"))),
        },
        Some(values) => floats(values).map(Window::Values),
    }
}

fn parse_sides(args: Args<'_>) -> Result<Sides, ProbeError> {
    match args.opt_str("sides")? {
        None | Some("default") => Ok(Sides::Default),
        Some("onesided") => Ok(Sides::OneSided),
        Some("twosided") => Ok(Sides::TwoSided),
        Some(other) => Err(ProbeError::rejected(format!("bad sides {other:?}"))),
    }
}

/// A non-negative integer argument (negative values are rejected like
/// matplotlib's range checks).
fn count(args: Args<'_>, key: &str) -> Result<Option<usize>, ProbeError> {
    args.opt_i64(key)?
        .map(|n| usize::try_from(n).map_err(|_| ProbeError::rejected(format!("{key} = {n}"))))
        .transpose()
}

fn spectral_options(args: Args<'_>) -> Result<SpectralOptions, ProbeError> {
    let detrend = match args.opt_str("detrend")? {
        None | Some("none") => Detrend::None,
        Some("mean" | "default" | "constant") => Detrend::Mean,
        Some("linear") => Detrend::Linear,
        Some(other) => return Err(ProbeError::rejected(format!("unknown detrend {other:?}"))),
    };
    let defaults = SpectralOptions::default();
    Ok(SpectralOptions {
        nfft: count(args, "NFFT")?.unwrap_or(defaults.nfft),
        fs: args.opt_f64("Fs")?.unwrap_or(defaults.fs),
        detrend,
        window: parse_window(args.get("window"))?,
        noverlap: count(args, "noverlap")?,
        pad_to: count(args, "pad_to")?,
        sides: parse_sides(args)?,
        scale_by_freq: args.opt_bool("scale_by_freq")?.unwrap_or(true),
    })
}

/// Calls `real` or `complex` depending on the signal's sample type.
macro_rules! on_signal {
    ($signal:expr, |$x:ident| $body:expr) => {
        match $signal {
            SignalArg::Real($x) => $body,
            SignalArg::Complex($x) => $body,
        }
    };
}

fn signal(args: Args<'_>, key: &str) -> Result<SignalArg, ProbeError> {
    SignalArg::parse(args.json(key)?)
}

fn real_spectrum(name: &str, spectrum: Spectrum) -> Json {
    object([
        (name, Json::from(spectrum.values)),
        ("freqs", Json::from(spectrum.freqs)),
    ])
}

fn complex_list(values: &[Complex]) -> Json {
    Json::Array(values.iter().map(|&z| complex_json(z)).collect())
}

pub(crate) fn psd(args: Args<'_>) -> Outcome {
    let opts = spectral_options(args)?;
    let p = on_signal!(signal(args, "x")?, |x| mlab::psd(&x, &opts)).map_err(rejected)?;
    Ok(real_spectrum("Pxx", p))
}

fn cross<S: SignalValue>(x: &[S], y: &SignalArg, opts: &SpectralOptions) -> Outcome {
    let p = on_signal!(y, |y| mlab::csd(x, y, opts)).map_err(rejected)?;
    Ok(object([
        ("Pxy", complex_list(&p.values)),
        ("freqs", Json::from(p.freqs)),
    ]))
}

pub(super) fn csd(args: Args<'_>) -> Outcome {
    let opts = spectral_options(args)?;
    let y = signal(args, "y")?;
    on_signal!(signal(args, "x")?, |x| cross(&x, &y, &opts))
}

fn coherence<S: SignalValue>(x: &[S], y: &SignalArg, opts: &SpectralOptions) -> Outcome {
    let c = on_signal!(y, |y| mlab::cohere(x, y, opts)).map_err(rejected)?;
    Ok(real_spectrum("Cxy", c))
}

pub(crate) fn cohere(args: Args<'_>) -> Outcome {
    let opts = spectral_options(args)?;
    let y = signal(args, "y")?;
    on_signal!(signal(args, "x")?, |x| coherence(&x, &y, &opts))
}

pub(super) fn specgram(args: Args<'_>) -> Outcome {
    let opts = spectral_options(args)?;
    let x = signal(args, "x")?;
    let mode = match args.opt_str("mode")? {
        None | Some("default" | "psd") => SpecgramMode::Psd,
        Some("magnitude") => SpecgramMode::Magnitude,
        Some("angle") => SpecgramMode::Angle,
        Some("phase") => SpecgramMode::Phase,
        Some("complex") => {
            let s = on_signal!(x, |x| mlab::complex_specgram(&x, &opts)).map_err(rejected)?;
            return Ok(specgram_json(complex_grid(&s.values), s.freqs, s.times));
        }
        Some(other) => return Err(ProbeError::rejected(format!("bad mode {other:?}"))),
    };
    let s = on_signal!(x, |x| mlab::specgram(&x, &opts, mode)).map_err(rejected)?;
    Ok(specgram_json(grid_json(&s.values), s.freqs, s.times))
}

fn complex_grid(values: &Grid<Complex>) -> Json {
    Json::Array(values.iter_rows().map(complex_list).collect())
}

fn specgram_json(spec: Json, freqs: Vec<f64>, times: Vec<f64>) -> Json {
    object([
        ("spec", spec),
        ("freqs", Json::from(freqs)),
        ("t", Json::from(times)),
    ])
}

fn spectrum_options(args: Args<'_>) -> Result<SpectrumOptions, ProbeError> {
    Ok(SpectrumOptions {
        fs: args.opt_f64("Fs")?.unwrap_or(SpectrumOptions::default().fs),
        window: parse_window(args.get("window"))?,
        pad_to: count(args, "pad_to")?,
        sides: parse_sides(args)?,
    })
}

pub(super) fn complex_spectrum(args: Args<'_>) -> Outcome {
    let opts = spectrum_options(args)?;
    let s =
        on_signal!(signal(args, "x")?, |x| mlab::complex_spectrum(&x, &opts)).map_err(rejected)?;
    Ok(object([
        ("spectrum", complex_list(&s.values)),
        ("freqs", Json::from(s.freqs)),
    ]))
}

/// The real-valued single spectra share one adapter shape.
fn single_real<F, G>(args: Args<'_>, real: F, complex: G) -> Outcome
where
    F: Fn(&[f64], &SpectrumOptions) -> Result<Spectrum, mlab::MlabError>,
    G: Fn(&[Complex], &SpectrumOptions) -> Result<Spectrum, mlab::MlabError>,
{
    let opts = spectrum_options(args)?;
    let s = match signal(args, "x")? {
        SignalArg::Real(x) => real(&x, &opts),
        SignalArg::Complex(x) => complex(&x, &opts),
    }
    .map_err(rejected)?;
    Ok(real_spectrum("spectrum", s))
}

pub(super) fn magnitude_spectrum(args: Args<'_>) -> Outcome {
    single_real(args, mlab::magnitude_spectrum, mlab::magnitude_spectrum)
}

pub(super) fn angle_spectrum(args: Args<'_>) -> Outcome {
    single_real(args, mlab::angle_spectrum, mlab::angle_spectrum)
}

pub(super) fn phase_spectrum(args: Args<'_>) -> Outcome {
    single_real(args, mlab::phase_spectrum, mlab::phase_spectrum)
}
