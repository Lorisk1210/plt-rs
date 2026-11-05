"""
Spectral plots: `~.Axes.psd`, `~.Axes.csd`, `~.Axes.cohere`,
`~.Axes.magnitude_spectrum`, `~.Axes.angle_spectrum`,
`~.Axes.phase_spectrum`, `~.Axes.specgram`, `~.Axes.acorr` and
`~.Axes.xcorr`.

The arguments default and check as matplotlib's (and its ``mlab``'s)
do; pltrs (Rust) computes the spectra and draws them.
"""

import numpy as np

from ... import _api, cbook, image as mimage, lines as mlines, mlab
from ... import collections as mcoll
from ..._native import plotting as _plotting
from ..._preprocess import _preprocess_data
from ._helpers import wrap
from .scatter import _mapping


def _welch(x, NFFT, Fs, detrend, window, noverlap, pad_to, sides, scale_by_freq,
           mode=None):
    """The options of a Welch estimate of *x*, defaulted and checked as
    ``mlab._spectral_helper`` does them (with `~.Axes.specgram`'s *mode*,
    None for the other spectra), in native form."""
    NFFT = 256 if NFFT is None else NFFT
    noverlap = 0 if noverlap is None else noverlap
    if not (0 <= noverlap < NFFT):
        raise ValueError('noverlap must be less than NFFT')
    if mode is not None:
        _api.check_in_list(['default', 'psd', 'complex', 'magnitude', 'angle', 'phase'],
                           mode=mode)
    sides = 'default' if sides is None else sides
    _api.check_in_list(['default', 'onesided', 'twosided'], sides=sides)
    return {
        "NFFT": int(NFFT), "Fs": 2.0 if Fs is None else float(Fs),
        "noverlap": int(noverlap),
        "pad_to": int(NFFT if pad_to is None else pad_to), "sides": sides,
        "scale_by_freq": True if scale_by_freq is None else bool(scale_by_freq),
        "detrend": mlab._detrend_spec(detrend),
        "window": mlab._window_spec(window, NFFT, np.asarray(x).dtype),
    }


def _line(kwargs):
    """`.Line2D` keyword arguments in native form."""
    return mlines._native_kwargs(cbook.normalize_kwargs(kwargs, mlines.Line2D))


def _integer_yticks(axes):
    """Type the y ticks `psd` and `csd` set as matplotlib's do: an integer
    ``np.arange`` from the floor of the bottom of the view."""
    from ... import ticker as mticker
    locator = axes.yaxis.get_major_locator()
    if (isinstance(locator, mticker.FixedLocator)
            and np.array_equal(locator.locs, np.round(locator.locs))):
        locator.locs = locator.locs.astype(np.int64)


def _last_line(axes):
    """The line a spectral plot just drew."""
    return axes.lines[-1]


def _floats(x):
    """A signal as a 1D float array."""
    return np.ravel(np.asarray(x, dtype=float))


class SpectralMixin:
    """The spectral plots of `~.axes.Axes`."""

    @_preprocess_data(replace_names=["x"])
    def psd(self, x, NFFT=None, Fs=None, Fc=None, detrend=None,
            window=None, noverlap=None, pad_to=None,
            sides=None, scale_by_freq=None, return_line=None, **kwargs):
        r"""
        Plot the power spectral density.

        Returns
        -------
        Pxx : 1-D array
            The values for the power spectrum :math:`P_{xx}` before scaling
            (real valued).
        freqs : 1-D array
            The frequencies corresponding to the elements in *Pxx*.
        line : `~matplotlib.lines.Line2D`
            The line created by this function.
            Only returned if *return_line* is True.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        Fc = 0 if Fc is None else Fc
        pxx, freqs = _plotting.psd(
            self._native, mlab._signal(np.asarray(x)),
            (_welch(x, NFFT, Fs, detrend, window, noverlap, pad_to, sides,
                    scale_by_freq), float(Fc)), _line(kwargs))
        _integer_yticks(self)
        freqs = freqs + Fc
        if return_line is None or not return_line:
            return pxx, freqs
        # The list `plot` returns, as matplotlib returns it.
        return pxx, freqs, [_last_line(self)]

    @_preprocess_data(replace_names=["x", "y"], label_namer="y")
    def csd(self, x, y, NFFT=None, Fs=None, Fc=None, detrend=None,
            window=None, noverlap=None, pad_to=None,
            sides=None, scale_by_freq=None, return_line=None, **kwargs):
        r"""
        Plot the cross-spectral density.

        Returns
        -------
        Pxy : 1-D array
            The values for the cross spectrum :math:`P_{xy}` before scaling
            (complex valued).
        freqs : 1-D array
            The frequencies corresponding to the elements in *Pxy*.
        line : `~matplotlib.lines.Line2D`
            The line created by this function.
            Only returned if *return_line* is True.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        Fc = 0 if Fc is None else Fc
        (re, im), freqs = _plotting.csd(
            self._native, (_floats(x), _floats(y)),
            (_welch(x, NFFT, Fs, detrend, window, noverlap, pad_to, sides,
                    scale_by_freq), float(Fc)), _line(kwargs))
        _integer_yticks(self)
        freqs = freqs + Fc
        pxy = re + 1j * im
        if return_line is None or not return_line:
            return pxy, freqs
        # The list `plot` returns, as matplotlib returns it.
        return pxy, freqs, [_last_line(self)]

    @_preprocess_data(replace_names=["x", "y"])
    def cohere(self, x, y, NFFT=256, Fs=2, Fc=0, detrend=mlab.detrend_none,
               window=mlab.window_hanning, noverlap=0, pad_to=None,
               sides='default', scale_by_freq=None, **kwargs):
        r"""
        Plot the coherence between *x* and *y*.

        Returns
        -------
        Cxy : 1-D array
            The coherence vector.
        freqs : 1-D array
            The frequencies for the elements in *Cxy*.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        cxy, freqs = _plotting.cohere(
            self._native, (_floats(x), _floats(y)),
            (_welch(x, NFFT, Fs, detrend, window, noverlap, pad_to, sides,
                    scale_by_freq), float(Fc)), _line(kwargs))
        return cxy, freqs + Fc

    def _single_spectrum(self, kind, x, Fs, Fc, window, pad_to, sides, scale,
                         kwargs):
        """A single-segment spectrum `kind` of *x*, plotted."""
        x = _floats(x)
        sides = 'default' if sides is None else sides
        _api.check_in_list(['default', 'onesided', 'twosided'], sides=sides)
        # matplotlib checks the scale once the spectrum is computed.
        scale = _api.getitem_checked(
            {None: 'linear', 'default': 'linear', 'linear': 'linear', 'dB': 'dB'},
            scale=scale)
        Fc = 0 if Fc is None else Fc
        spectrum, freqs, line = _plotting.single_spectrum(
            self._native, kind, x,
            ((2.0 if Fs is None else float(Fs), mlab._window_spec(window, len(x), x.dtype),
              None if pad_to is None else int(pad_to), sides), float(Fc)),
            scale, _line(kwargs))
        return spectrum, freqs + Fc, wrap(self, line, mlines.Line2D)

    @_preprocess_data(replace_names=["x"])
    def magnitude_spectrum(self, x, Fs=None, Fc=None, window=None,
                           pad_to=None, sides=None, scale=None,
                           **kwargs):
        """
        Plot the magnitude spectrum.

        Returns
        -------
        spectrum : 1-D array
            The values for the magnitude spectrum before scaling (real valued).
        freqs : 1-D array
            The frequencies corresponding to the elements in *spectrum*.
        line : `~matplotlib.lines.Line2D`
            The line created by this function.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        return self._single_spectrum("magnitude", x, Fs, Fc, window, pad_to, sides,
                                     scale, kwargs)

    @_preprocess_data(replace_names=["x"])
    def angle_spectrum(self, x, Fs=None, Fc=None, window=None,
                       pad_to=None, sides=None, **kwargs):
        """
        Plot the angle spectrum.

        Returns
        -------
        spectrum : 1-D array
            The values for the angle spectrum in radians (real valued).
        freqs : 1-D array
            The frequencies corresponding to the elements in *spectrum*.
        line : `~matplotlib.lines.Line2D`
            The line created by this function.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        return self._single_spectrum("angle", x, Fs, Fc, window, pad_to, sides,
                                     "linear", kwargs)

    @_preprocess_data(replace_names=["x"])
    def phase_spectrum(self, x, Fs=None, Fc=None, window=None,
                       pad_to=None, sides=None, **kwargs):
        """
        Plot the phase spectrum.

        Returns
        -------
        spectrum : 1-D array
            The values for the phase spectrum in radians (real valued).
        freqs : 1-D array
            The frequencies corresponding to the elements in *spectrum*.
        line : `~matplotlib.lines.Line2D`
            The line created by this function.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        return self._single_spectrum("phase", x, Fs, Fc, window, pad_to, sides,
                                     "linear", kwargs)

    @_preprocess_data(replace_names=["x"])
    def specgram(self, x, NFFT=None, Fs=None, Fc=None, detrend=None,
                 window=None, noverlap=None,
                 cmap=None, xextent=None, pad_to=None, sides=None,
                 scale_by_freq=None, mode=None, scale=None,
                 vmin=None, vmax=None, **kwargs):
        """
        Plot a spectrogram.

        Returns
        -------
        spectrum : 2D array
            Columns are the periodograms of successive segments.
        freqs : 1-D array
            The frequencies corresponding to the rows in *spectrum*.
        t : 1-D array
            The times corresponding to midpoints of segments (i.e., the columns
            in *spectrum*).
        im : `.AxesImage`
            The image created by imshow containing the spectrogram.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        if NFFT is None:
            NFFT = 256  # same default as in mlab.specgram()
        if Fc is None:
            Fc = 0  # same default as in mlab._spectral_helper()
        if noverlap is None:
            noverlap = 128  # same default as in mlab.specgram()
        if Fs is None:
            Fs = 2  # same default as in mlab._spectral_helper()
        if mode == 'complex':
            raise ValueError('Cannot plot a complex specgram')
        if scale is None or scale == 'default':
            scale = 'linear' if mode in ['angle', 'phase'] else 'dB'
        elif mode in ['angle', 'phase'] and scale == 'dB':
            raise ValueError('Cannot use dB scale with angle or phase mode')
        welch = _welch(x, NFFT, Fs, detrend, window, noverlap, pad_to, sides,
                       scale_by_freq, mode='default' if mode is None else mode)
        mode = 'psd' if mode in (None, 'default') else mode
        if scale not in ('linear', 'dB'):
            raise ValueError(f'Unknown scale {scale!r}')
        if 'origin' in kwargs:
            raise _api.kwarg_error("specgram", "origin")
        made, mapping = _mapping(cmap, kwargs.pop('norm', None), vmin, vmax,
                                 kwargs.pop('colorizer', None))
        spec, freqs, t, native = _plotting.specgram(
            self._native, _floats(x), (welch, float(Fc)),
            (mode, scale), None if xextent is None else tuple(map(float, xextent)),
            mapping)
        im = wrap(self, native, mimage.AxesImage)
        shown = spec
        if scale == 'dB':
            shown = (10 if mode == 'psd' else 20) * np.log10(spec)
        shown = np.flipud(shown)
        made._scale_norm(None, None, None, shown)
        im._set_mapped_data(made, shown)
        im._internal_update(kwargs)
        return spec, freqs, t, im

    @_preprocess_data(replace_names=["x"], label_namer="x")
    def acorr(self, x, **kwargs):
        """
        Plot the autocorrelation of *x*.

        Returns
        -------
        lags : array (length ``2*maxlags+1``)
            The lag vector.
        c : array  (length ``2*maxlags+1``)
            The auto correlation vector.
        line : `.LineCollection` or `.Line2D`
            `.Artist` added to the Axes of the correlation.
        b : `~matplotlib.lines.Line2D` or None
            Horizontal line at 0 if *usevlines* is True
            None *usevlines* is False.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        return self.xcorr(x, x, **kwargs)

    @_preprocess_data(replace_names=["x", "y"], label_namer="y")
    def xcorr(self, x, y, normed=True, detrend=mlab.detrend_none,
              usevlines=True, maxlags=10, **kwargs):
        r"""
        Plot the cross correlation between *x* and *y*.

        Returns
        -------
        lags : array (length ``2*maxlags+1``)
            The lag vector.
        c : array  (length ``2*maxlags+1``)
            The auto correlation vector.
        line : `.LineCollection` or `.Line2D`
            `.Artist` added to the Axes of the correlation.
        b : `~matplotlib.lines.Line2D` or None
            Horizontal line at 0 if *usevlines* is True
            None *usevlines* is False.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        Nx = len(x)
        if Nx != len(y):
            raise ValueError('x and y must be equal length')
        if not callable(detrend):
            # matplotlib calls *detrend* on both series first.
            raise TypeError(f"'{type(detrend).__name__}' object is not callable")
        if maxlags is None:
            maxlags = Nx - 1
        if maxlags >= Nx or maxlags < 1:
            raise ValueError('maxlags must be None or strictly '
                             f'positive < {Nx:d}')
        lags, c, (lines, zero, markers) = _plotting.xcorr(
            self._native, (_floats(x), _floats(y)),
            (bool(normed), mlab._detrend_spec(detrend), int(maxlags)),
            bool(usevlines), _line(kwargs))
        if markers is not None:
            return lags, c, wrap(self, markers, mlines.Line2D), None
        return (lags, c, wrap(self, lines, mcoll.LineCollection),
                wrap(self, zero, mlines.Line2D))
