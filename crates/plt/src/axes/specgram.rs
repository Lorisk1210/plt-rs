//! Spectrograms (matplotlib's `Axes.specgram`): an image of the spectrum
//! of each segment over time.

use std::fmt;

use super::{Axes, AxisMode, ImshowOptions, SpectrumScale};
use crate::image::{AxesImage, Extent, ImageError, Origin};
use crate::mlab::{self, MlabError, Specgram, SpecgramMode, SpectralOptions};
use crate::numeric::Grid;

/// Options of [`Axes::specgram`].
#[derive(Debug, Default)]
pub struct SpecgramOptions {
    /// The segments and their spectra (`noverlap` defaults to 128).
    pub spectral: SpectralOptions,
    /// Centre frequency added to the frequencies (`Fc`).
    pub fc: f64,
    /// What each segment's spectrum reports.
    pub mode: SpecgramMode,
    /// Decibels or the values themselves (`None` = decibels, except
    /// linear for angle and phase).
    pub scale: Option<SpectrumScale>,
    /// The time span the image covers (`None` = the segment midpoints
    /// widened by half a segment step).
    pub xextent: Option<(f64, f64)>,
    /// Colormap, limits and other image options; the origin is fixed.
    pub image: ImshowOptions,
}

/// What [`Axes::specgram`] computed and drew.
#[derive(Clone, Debug)]
pub struct SpecgramPlot {
    /// The spectra (unscaled), with the frequencies shifted by `fc`.
    pub spectrogram: Specgram,
    /// The image.
    pub image: AxesImage,
}

/// Why a spectrogram cannot be drawn.
#[derive(Clone, Debug, PartialEq)]
pub enum SpecgramError {
    /// The spectral options are invalid for the signal.
    Mlab(MlabError),
    /// The image cannot be made.
    Image(ImageError),
    /// Decibels of angles or phases.
    DecibelPhase,
    /// An image origin was given; spectrograms put high frequencies on top.
    Origin,
}

impl fmt::Display for SpecgramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mlab(err) => err.fmt(f),
            Self::Image(err) => err.fmt(f),
            Self::DecibelPhase => f.write_str("Cannot use dB scale with angle or phase mode"),
            Self::Origin => f.write_str("specgram() got an unexpected keyword argument 'origin'"),
        }
    }
}

impl std::error::Error for SpecgramError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Mlab(err) => Some(err),
            Self::Image(err) => Some(err),
            Self::DecibelPhase | Self::Origin => None,
        }
    }
}

impl Axes {
    /// Draws the spectrogram of `x` as an image with time along x and
    /// frequency along y, and resets the aspect to automatic (matplotlib's
    /// `specgram`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::SpecgramOptions;
    /// use plt::mlab::SpectralOptions;
    /// let ax = Figure::new().subplot();
    /// let x: Vec<f64> = (0..512).map(|i| (f64::from(i) * 0.4).sin()).collect();
    /// let options = SpecgramOptions {
    ///     spectral: SpectralOptions { nfft: 64, noverlap: Some(32), ..SpectralOptions::default() },
    ///     ..SpecgramOptions::default()
    /// };
    /// let plot = ax.specgram(&x, options)?;
    /// assert_eq!(plot.spectrogram.freqs.len(), 33);
    /// # Ok::<(), plt::axes::SpecgramError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the spectral options are invalid for `x`, decibels are asked
    /// of angles or phases, an image origin is given, or the image options
    /// are invalid.
    pub fn specgram(
        &self,
        x: &[f64],
        options: SpecgramOptions,
    ) -> Result<SpecgramPlot, SpecgramError> {
        let phase = matches!(options.mode, SpecgramMode::Angle | SpecgramMode::Phase);
        let scale = match options.scale {
            None if phase => SpectrumScale::Linear,
            None => SpectrumScale::Decibels,
            Some(SpectrumScale::Decibels) if phase => return Err(SpecgramError::DecibelPhase),
            Some(scale) => scale,
        };
        if options.image.origin.is_some() {
            return Err(SpecgramError::Origin);
        }
        // The time extent is padded by half a segment step on each side.
        let pad = {
            let spectral = &options.spectral;
            let noverlap = spectral.noverlap.unwrap_or(128);
            (spectral.nfft as f64 - noverlap as f64) / spectral.fs / 2.0
        };
        let mut spectrogram =
            mlab::specgram(x, options.spectral, options.mode).map_err(SpecgramError::Mlab)?;
        let decibels = |factor: f64| move |v: &f64| factor * v.log10();
        let z = match (scale, options.mode) {
            (SpectrumScale::Linear, _) => spectrogram.values.clone(),
            (SpectrumScale::Decibels, SpecgramMode::Psd) => spectrogram.values.map(decibels(10.0)),
            (SpectrumScale::Decibels, _) => spectrogram.values.map(decibels(20.0)),
        };
        let (rows, cols) = z.shape();
        let flipped = Grid::from_fn(rows, cols, |r, c| z[(rows - 1 - r, c)]);
        let (xmin, xmax) = options.xextent.unwrap_or_else(|| {
            let times = &spectrogram.times;
            let first = times.iter().copied().fold(f64::INFINITY, f64::min);
            let last = times.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            (first - pad, last + pad)
        });
        for freq in &mut spectrogram.freqs {
            *freq += options.fc;
        }
        let (&low, &high) = spectrogram
            .freqs
            .first()
            .zip(spectrogram.freqs.last())
            .expect("a spectrum has at least one frequency");
        let image = ImshowOptions {
            extent: Some(Extent::new(xmin, xmax, low, high)),
            origin: Some(Origin::Upper),
            ..options.image
        };
        let image = self.imshow(flipped, image).map_err(SpecgramError::Image)?;
        self.axis_mode(AxisMode::Auto)
            .expect("every axes takes the automatic mode");
        Ok(SpecgramPlot { spectrogram, image })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    fn signal() -> Vec<f64> {
        (0..300).map(|i| (f64::from(i) * 0.7).sin()).collect()
    }

    fn options(nfft: usize, noverlap: usize) -> SpecgramOptions {
        SpecgramOptions {
            spectral: SpectralOptions {
                nfft,
                noverlap: Some(noverlap),
                fs: 10.0,
                ..SpectralOptions::default()
            },
            ..SpecgramOptions::default()
        }
    }

    #[test]
    fn the_image_spans_time_and_frequency() {
        let ax = Figure::new().subplot();
        let plot = ax.specgram(&signal(), options(32, 16)).unwrap();
        let times = &plot.spectrogram.times;
        // Half a segment step (16 samples at 10 Hz) beyond the midpoints.
        assert_eq!(ax.xlim(), (times[0] - 0.8, times[times.len() - 1] + 0.8));
        assert_eq!(ax.ylim(), (0.0, 5.0));
    }

    #[test]
    fn invalid_combinations_are_rejected() {
        let ax = Figure::new().subplot();
        let phase = SpecgramOptions {
            mode: SpecgramMode::Phase,
            scale: Some(SpectrumScale::Decibels),
            ..options(32, 16)
        };
        assert_eq!(
            ax.specgram(&signal(), phase).err(),
            Some(SpecgramError::DecibelPhase)
        );
        let lower = SpecgramOptions {
            image: ImshowOptions {
                origin: Some(Origin::Lower),
                ..ImshowOptions::default()
            },
            ..options(32, 16)
        };
        assert_eq!(
            ax.specgram(&signal(), lower).err(),
            Some(SpecgramError::Origin)
        );
        assert!(matches!(
            ax.specgram(&signal(), options(32, 32)),
            Err(SpecgramError::Mlab(_))
        ));
    }
}
