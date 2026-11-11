"""
Spectral plots: `~.Axes.psd`, `~.Axes.csd`, `~.Axes.cohere`,
`~.Axes.magnitude_spectrum`, `~.Axes.angle_spectrum`,
`~.Axes.phase_spectrum`, `~.Axes.specgram`, `~.Axes.acorr` and
`~.Axes.xcorr`.

The arguments default and check as matplotlib's (and its ``mlab``'s)
do; pltrs (Rust) computes the spectra and draws them. Arguments pltrs
(Rust) cannot take as they are, or refuses, are computed with as
matplotlib's ``mlab`` computes (`._mlab_reference`), which fails where and
as it fails (`_as_matplotlib`).
"""

import math
import numbers

import numpy as np

from ... import _docstring
from ... import _api, _lazy_imports, cbook, image as mimage, lines as mlines, mlab
from ... import collections as mcoll
from ..._artist_kwargs import check_known
from ..._native import plotting as _plotting
from ..._preprocess import _preprocess_data
from ._helpers import wrap
from .scatter import _mapping

_mlab_reference = _lazy_imports.module(globals(), "..._mlab_reference")


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


def _plain(signals, sizes, Fs, window):
    """Whether pltrs (Rust) takes the arguments of a spectrum as they are:
    *signals* of numbers along one dimension, *sizes* that are integers or
    None, a sampling frequency *Fs* that is a finite nonzero number or
    None, and a *window* that is None, a function or an array."""
    return (all(np.ndim(signal) == 1 and np.asarray(signal).dtype.kind in "biuf"
                for signal in signals)
            and all(size is None or isinstance(size, numbers.Integral) for size in sizes)
            and (Fs is None or (isinstance(Fs, numbers.Real) and math.isfinite(Fs)
                                and Fs != 0))
            and (window is None or callable(window) or isinstance(window, np.ndarray)))


def _as_matplotlib(compute, reference, plain):
    """
    What *compute()*, a spectrum pltrs (Rust) computes and plots, returns.
    With arguments that are not *plain*, or that it refuses,
    *reference()* first computes the spectrum as matplotlib's ``mlab``
    does, raising matplotlib's exception where that fails.
    """
    if not plain:
        reference()
    try:
        return compute()
    except (TypeError, ValueError, OverflowError, ZeroDivisionError) as error:
        failure = error
    reference()
    raise failure


def _shift_in_place(frequencies, Fc):
    """Fail as matplotlib's spectra fail to shift their frequencies in place
    by *Fc* (``freqs += Fc``) when it is no number, before they plot
    anything: *frequencies()* gives them as matplotlib's ``mlab`` does."""
    if not isinstance(Fc, numbers.Real):
        freqs = frequencies()
        freqs += Fc


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



def _plain_series(values):
    """Whether *values* are a non-empty 1-D array of finite numbers (not
    of booleans, which numpy sums as such), as pltrs (Rust) correlates
    them (numpy warns about what infinite ones make)."""
    array = np.asarray(values)
    return (array.ndim == 1 and array.size > 0 and array.dtype.kind in "iuf"
            and bool(np.isfinite(array).all()))

class SpectralMixin:
    """The spectral plots of `~.axes.Axes`."""

    @_api.make_keyword_only("3.10", "NFFT")
    @_preprocess_data(replace_names=["x"])
    @_docstring.interpd
    def psd(self, x, NFFT=None, Fs=None, Fc=None, detrend=None,
            window=None, noverlap=None, pad_to=None,
            sides=None, scale_by_freq=None, return_line=None, **kwargs):
        r"""
        Plot the power spectral density.

        The power spectral density :math:`P_{xx}` by Welch's average
        periodogram method.  The vector *x* is divided into *NFFT* length
        segments.  Each segment is detrended by function *detrend* and
        windowed by function *window*.  *noverlap* gives the length of
        the overlap between segments.  The :math:`|\mathrm{fft}(i)|^2`
        of each segment :math:`i` are averaged to compute :math:`P_{xx}`,
        with a scaling to correct for power loss due to windowing.

        If len(*x*) < *NFFT*, it will be zero padded to *NFFT*.

        Parameters
        ----------
        x : 1-D array or sequence
            Array or sequence containing the data

        %(Spectral)s

        %(PSD)s

        noverlap : int, default: 0 (no overlap)
            The number of points of overlap between segments.

        Fc : int, default: 0
            The center frequency of *x*, which offsets the x extents of the
            plot to reflect the frequency range used when a signal is acquired
            and then filtered and downsampled to baseband.

        return_line : bool, default: False
            Whether to include the line object plotted in the returned values.

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

        **kwargs
            Keyword arguments control the `.Line2D` properties:

            %(Line2D:kwdoc)s

        See Also
        --------
        specgram
            Differs in the default overlap; in not returning the mean of the
            segment periodograms; in returning the times of the segments; and
            in plotting a colormap instead of a line.
        magnitude_spectrum
            Plots the magnitude spectrum.
        csd
            Plots the spectral density between two signals.

        Notes
        -----
        For plotting, the power is plotted as
        :math:`10\log_{10}(P_{xx})` for decibels, though *Pxx* itself
        is returned.

        References
        ----------
        Bendat & Piersol -- Random Data: Analysis and Measurement Procedures,
        John Wiley & Sons (1986)
        """
        Fc = 0 if Fc is None else Fc
        _shift_in_place(lambda: _mlab_reference.csd(
            x, None, NFFT, Fs, detrend, window, noverlap, pad_to, sides, scale_by_freq)[1], Fc)
        pxx, freqs = _as_matplotlib(
            lambda: _plotting.psd(
                self._native, mlab._signal(np.asarray(x)),
                (_welch(x, NFFT, Fs, detrend, window, noverlap, pad_to, sides,
                        scale_by_freq), float(Fc)), _line(kwargs)),
            lambda: _mlab_reference.csd(x, None, NFFT, Fs, detrend, window,
                                        noverlap, pad_to, sides, scale_by_freq),
            _plain((x,), (NFFT, noverlap, pad_to), Fs, window))
        _integer_yticks(self)
        freqs = freqs + Fc
        if return_line is None or not return_line:
            return pxx, freqs
        # The list `plot` returns, as matplotlib returns it.
        return pxx, freqs, [_last_line(self)]

    @_api.make_keyword_only("3.10", "NFFT")
    @_preprocess_data(replace_names=["x", "y"], label_namer="y")
    @_docstring.interpd
    def csd(self, x, y, NFFT=None, Fs=None, Fc=None, detrend=None,
            window=None, noverlap=None, pad_to=None,
            sides=None, scale_by_freq=None, return_line=None, **kwargs):
        r"""
        Plot the cross-spectral density.

        The cross spectral density :math:`P_{xy}` by Welch's average
        periodogram method.  The vectors *x* and *y* are divided into
        *NFFT* length segments.  Each segment is detrended by function
        *detrend* and windowed by function *window*.  *noverlap* gives
        the length of the overlap between segments.  The product of
        the direct FFTs of *x* and *y* are averaged over each segment
        to compute :math:`P_{xy}`, with a scaling to correct for power
        loss due to windowing.

        If len(*x*) < *NFFT* or len(*y*) < *NFFT*, they will be zero
        padded to *NFFT*.

        Parameters
        ----------
        x, y : 1-D arrays or sequences
            Arrays or sequences containing the data.

        %(Spectral)s

        %(PSD)s

        noverlap : int, default: 0 (no overlap)
            The number of points of overlap between segments.

        Fc : int, default: 0
            The center frequency of *x*, which offsets the x extents of the
            plot to reflect the frequency range used when a signal is acquired
            and then filtered and downsampled to baseband.

        return_line : bool, default: False
            Whether to include the line object plotted in the returned values.

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

        **kwargs
            Keyword arguments control the `.Line2D` properties:

            %(Line2D:kwdoc)s

        See Also
        --------
        psd : is equivalent to setting ``y = x``.

        Notes
        -----
        For plotting, the power is plotted as
        :math:`10 \log_{10}(P_{xy})` for decibels, though :math:`P_{xy}` itself
        is returned.

        References
        ----------
        Bendat & Piersol -- Random Data: Analysis and Measurement Procedures,
        John Wiley & Sons (1986)
        """
        Fc = 0 if Fc is None else Fc
        _shift_in_place(lambda: _mlab_reference.csd(
            x, y, NFFT, Fs, detrend, window, noverlap, pad_to, sides, scale_by_freq)[1], Fc)
        (re, im), freqs = _as_matplotlib(
            # Without *y*, matplotlib's cross spectrum is the spectrum of *x*.
            lambda: _plotting.csd(
                self._native, (_floats(x), _floats(x if y is None else y)),
                (_welch(x, NFFT, Fs, detrend, window, noverlap, pad_to, sides,
                        scale_by_freq), float(Fc)), _line(kwargs)),
            lambda: _mlab_reference.csd(x, y, NFFT, Fs, detrend, window,
                                        noverlap, pad_to, sides, scale_by_freq),
            _plain((x, y), (NFFT, noverlap, pad_to), Fs, window))
        _integer_yticks(self)
        freqs = freqs + Fc
        pxy = re + 1j * im
        if return_line is None or not return_line:
            return pxy, freqs
        # The list `plot` returns, as matplotlib returns it.
        return pxy, freqs, [_last_line(self)]

    @_api.make_keyword_only("3.10", "NFFT")
    @_preprocess_data(replace_names=["x", "y"])
    @_docstring.interpd
    def cohere(self, x, y, NFFT=256, Fs=2, Fc=0, detrend=mlab.detrend_none,
               window=mlab.window_hanning, noverlap=0, pad_to=None,
               sides='default', scale_by_freq=None, **kwargs):
        r"""
        Plot the coherence between *x* and *y*.

        Coherence is the normalized cross spectral density:

        .. math::

          C_{xy} = \frac{|P_{xy}|^2}{P_{xx}P_{yy}}

        Parameters
        ----------
        %(Spectral)s

        %(PSD)s

        noverlap : int, default: 0 (no overlap)
            The number of points of overlap between blocks.

        Fc : int, default: 0
            The center frequency of *x*, which offsets the x extents of the
            plot to reflect the frequency range used when a signal is acquired
            and then filtered and downsampled to baseband.

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

        **kwargs
            Keyword arguments control the `.Line2D` properties:

            %(Line2D:kwdoc)s

        References
        ----------
        Bendat & Piersol -- Random Data: Analysis and Measurement Procedures,
        John Wiley & Sons (1986)
        """
        _shift_in_place(lambda: _mlab_reference.cohere(
            x, y, NFFT, Fs, detrend, window, noverlap, pad_to, sides, scale_by_freq)[1], Fc)
        cxy, freqs = _as_matplotlib(
            lambda: _plotting.cohere(
                self._native, (_floats(x), _floats(y)),
                (_welch(x, NFFT, Fs, detrend, window, noverlap, pad_to, sides,
                        scale_by_freq), float(Fc)), _line(kwargs)),
            lambda: _mlab_reference.cohere(x, y, NFFT, Fs, detrend, window,
                                           noverlap, pad_to, sides, scale_by_freq),
            _plain((x, y), (NFFT, noverlap, pad_to), Fs, window))
        return cxy, freqs + Fc

    def _single_spectrum(self, kind, x, Fs, Fc, window, pad_to, sides, scale,
                         kwargs):
        """A single-segment spectrum `kind` of *x*, plotted."""
        Fc = 0 if Fc is None else Fc
        _shift_in_place(lambda: _mlab_reference.single_spectrum_helper(
            kind, x, Fs, window, pad_to, sides)[1], Fc)

        def compute():
            signal = _floats(x)
            checked = 'default' if sides is None else sides
            _api.check_in_list(['default', 'onesided', 'twosided'], sides=checked)
            # matplotlib checks the scale once the spectrum is computed.
            named = _api.getitem_checked(
                {None: 'linear', 'default': 'linear', 'linear': 'linear', 'dB': 'dB'},
                scale=scale)
            return _plotting.single_spectrum(
                self._native, kind, signal,
                ((2.0 if Fs is None else float(Fs),
                  mlab._window_spec(window, len(signal), signal.dtype),
                  None if pad_to is None else int(pad_to), checked), float(Fc)),
                named, _line(kwargs))

        spectrum, freqs, line = _as_matplotlib(
            compute,
            lambda: _mlab_reference.single_spectrum_helper(kind, x, Fs, window,
                                                           pad_to, sides),
            _plain((x,), (pad_to,), Fs, window))
        return spectrum, freqs + Fc, wrap(self, line, mlines.Line2D)

    @_api.make_keyword_only("3.10", "Fs")
    @_preprocess_data(replace_names=["x"])
    @_docstring.interpd
    def magnitude_spectrum(self, x, Fs=None, Fc=None, window=None,
                           pad_to=None, sides=None, scale=None,
                           **kwargs):
        """
        Plot the magnitude spectrum.

        Compute the magnitude spectrum of *x*.  Data is padded to a
        length of *pad_to* and the windowing function *window* is applied to
        the signal.

        Parameters
        ----------
        x : 1-D array or sequence
            Array or sequence containing the data.

        %(Spectral)s

        %(Single_Spectrum)s

        scale : {'default', 'linear', 'dB'}
            The scaling of the values in the *spec*.  'linear' is no scaling.
            'dB' returns the values in dB scale, i.e., the dB amplitude
            (20 * log10). 'default' is 'linear'.

        Fc : int, default: 0
            The center frequency of *x*, which offsets the x extents of the
            plot to reflect the frequency range used when a signal is acquired
            and then filtered and downsampled to baseband.

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

        **kwargs
            Keyword arguments control the `.Line2D` properties:

            %(Line2D:kwdoc)s

        See Also
        --------
        psd
            Plots the power spectral density.
        angle_spectrum
            Plots the angles of the corresponding frequencies.
        phase_spectrum
            Plots the phase (unwrapped angle) of the corresponding frequencies.
        specgram
            Can plot the magnitude spectrum of segments within the signal in a
            colormap.
        """
        return self._single_spectrum("magnitude", x, Fs, Fc, window, pad_to, sides,
                                     scale, kwargs)

    @_api.make_keyword_only("3.10", "Fs")
    @_preprocess_data(replace_names=["x"])
    @_docstring.interpd
    def angle_spectrum(self, x, Fs=None, Fc=None, window=None,
                       pad_to=None, sides=None, **kwargs):
        """
        Plot the angle spectrum.

        Compute the angle spectrum (wrapped phase spectrum) of *x*.
        Data is padded to a length of *pad_to* and the windowing function
        *window* is applied to the signal.

        Parameters
        ----------
        x : 1-D array or sequence
            Array or sequence containing the data.

        %(Spectral)s

        %(Single_Spectrum)s

        Fc : int, default: 0
            The center frequency of *x*, which offsets the x extents of the
            plot to reflect the frequency range used when a signal is acquired
            and then filtered and downsampled to baseband.

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

        **kwargs
            Keyword arguments control the `.Line2D` properties:

            %(Line2D:kwdoc)s

        See Also
        --------
        magnitude_spectrum
            Plots the magnitudes of the corresponding frequencies.
        phase_spectrum
            Plots the unwrapped version of this function.
        specgram
            Can plot the angle spectrum of segments within the signal in a
            colormap.
        """
        return self._single_spectrum("angle", x, Fs, Fc, window, pad_to, sides,
                                     "linear", kwargs)

    @_api.make_keyword_only("3.10", "Fs")
    @_preprocess_data(replace_names=["x"])
    @_docstring.interpd
    def phase_spectrum(self, x, Fs=None, Fc=None, window=None,
                       pad_to=None, sides=None, **kwargs):
        """
        Plot the phase spectrum.

        Compute the phase spectrum (unwrapped angle spectrum) of *x*.
        Data is padded to a length of *pad_to* and the windowing function
        *window* is applied to the signal.

        Parameters
        ----------
        x : 1-D array or sequence
            Array or sequence containing the data

        %(Spectral)s

        %(Single_Spectrum)s

        Fc : int, default: 0
            The center frequency of *x*, which offsets the x extents of the
            plot to reflect the frequency range used when a signal is acquired
            and then filtered and downsampled to baseband.

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

        **kwargs
            Keyword arguments control the `.Line2D` properties:

            %(Line2D:kwdoc)s

        See Also
        --------
        magnitude_spectrum
            Plots the magnitudes of the corresponding frequencies.
        angle_spectrum
            Plots the wrapped version of this function.
        specgram
            Can plot the phase spectrum of segments within the signal in a
            colormap.
        """
        return self._single_spectrum("phase", x, Fs, Fc, window, pad_to, sides,
                                     "linear", kwargs)

    @_api.make_keyword_only("3.10", "NFFT")
    @_preprocess_data(replace_names=["x"])
    @_docstring.interpd
    def specgram(self, x, NFFT=None, Fs=None, Fc=None, detrend=None,
                 window=None, noverlap=None,
                 cmap=None, xextent=None, pad_to=None, sides=None,
                 scale_by_freq=None, mode=None, scale=None,
                 vmin=None, vmax=None, **kwargs):
        """
        Plot a spectrogram.

        Compute and plot a spectrogram of data in *x*.  Data are split into
        *NFFT* length segments and the spectrum of each section is
        computed.  The windowing function *window* is applied to each
        segment, and the amount of overlap of each segment is
        specified with *noverlap*. The spectrogram is plotted as a colormap
        (using imshow).

        Parameters
        ----------
        x : 1-D array or sequence
            Array or sequence containing the data.

        %(Spectral)s

        %(PSD)s

        mode : {'default', 'psd', 'magnitude', 'angle', 'phase'}
            What sort of spectrum to use.  Default is 'psd', which takes the
            power spectral density.  'magnitude' returns the magnitude
            spectrum.  'angle' returns the phase spectrum without unwrapping.
            'phase' returns the phase spectrum with unwrapping.

        noverlap : int, default: 128
            The number of points of overlap between blocks.

        scale : {'default', 'linear', 'dB'}
            The scaling of the values in the *spec*.  'linear' is no scaling.
            'dB' returns the values in dB scale.  When *mode* is 'psd',
            this is dB power (10 * log10).  Otherwise, this is dB amplitude
            (20 * log10). 'default' is 'dB' if *mode* is 'psd' or
            'magnitude' and 'linear' otherwise.  This must be 'linear'
            if *mode* is 'angle' or 'phase'.

        Fc : int, default: 0
            The center frequency of *x*, which offsets the x extents of the
            plot to reflect the frequency range used when a signal is acquired
            and then filtered and downsampled to baseband.

        cmap : `.Colormap`, default: :rc:`image.cmap`

        xextent : *None* or (xmin, xmax)
            The image extent along the x-axis. The default sets *xmin* to the
            left border of the first bin (*spectrum* column) and *xmax* to the
            right border of the last bin. Note that for *noverlap>0* the width
            of the bins is smaller than those of the segments.

        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        vmin, vmax : float, optional
            vmin and vmax define the data range that the colormap covers.
            By default, the colormap covers the complete value range of the
            data.

        **kwargs
            Additional keyword arguments are passed on to `~.axes.Axes.imshow`
            which makes the specgram image. The origin keyword argument
            is not supported.

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

        See Also
        --------
        psd
            Differs in the default overlap; in returning the mean of the
            segment periodograms; in not returning times; and in generating a
            line plot instead of colormap.
        magnitude_spectrum
            A single spectrum, similar to having a single segment when *mode*
            is 'magnitude'. Plots a line instead of a colormap.
        angle_spectrum
            A single spectrum, similar to having a single segment when *mode*
            is 'angle'. Plots a line instead of a colormap.
        phase_spectrum
            A single spectrum, similar to having a single segment when *mode*
            is 'phase'. Plots a line instead of a colormap.

        Notes
        -----
        The parameters *detrend* and *scale_by_freq* do only apply when *mode*
        is set to 'psd'.
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
        if len(x) <= NFFT:
            _api.warn_external("Only one segment is calculated since parameter "
                               f"NFFT (={NFFT}) >= signal length (={len(x)}).")
        reference_mode = mode
        welch = _as_matplotlib(
            lambda: _welch(x, NFFT, Fs, detrend, window, noverlap, pad_to, sides,
                           scale_by_freq, mode='default' if mode is None else mode),
            lambda: _mlab_reference.spectral_helper(
                x, None, NFFT, Fs, detrend, window, noverlap, pad_to, sides,
                scale_by_freq, reference_mode),
            _plain((x,), (NFFT, noverlap, pad_to), Fs, window))
        mode = 'psd' if mode in (None, 'default') else mode
        if scale not in ('linear', 'dB'):
            raise ValueError(f'Unknown scale {scale!r}')
        _shift_in_place(lambda: _mlab_reference.spectral_helper(
            x, None, NFFT, Fs, detrend, window, noverlap, pad_to, sides,
            scale_by_freq, reference_mode)[1], Fc)
        if 'origin' in kwargs:
            raise _api.kwarg_error("specgram", "origin")
        made, mapping = _mapping(cmap, kwargs.pop('norm', None), vmin, vmax,
                                 kwargs.pop('colorizer', None))
        spec, freqs, t, native = _as_matplotlib(
            lambda: _plotting.specgram(
                self._native, _floats(x), (welch, float(Fc)),
                (mode, scale), None if xextent is None else tuple(map(float, xextent)),
                mapping),
            lambda: _mlab_reference.spectral_helper(
                x, None, NFFT, Fs, detrend, window, noverlap, pad_to, sides,
                scale_by_freq, reference_mode),
            True)
        im = wrap(self, native, mimage.AxesImage)
        shown = spec
        if scale == 'dB':
            shown = (10 if mode == 'psd' else 20) * np.log10(spec)
        shown = np.flipud(shown)
        made._scale_norm(None, None, None, shown)
        im._set_mapped_data(made, shown)
        im._made_with(kwargs)
        return spec, freqs, t, im

    @_preprocess_data(replace_names=["x"], label_namer="x")
    def acorr(self, x, **kwargs):
        """
        Plot the autocorrelation of *x*.

        Parameters
        ----------
        x : array-like
            Not run through Matplotlib's unit conversion, so this should
            be a unit-less array.

        detrend : callable, default: `.mlab.detrend_none` (no detrending)
            A detrending function applied to *x*.  It must have the
            signature ::

                detrend(x: np.ndarray) -> np.ndarray

        normed : bool, default: True
            If ``True``, input vectors are normalised to unit length.

        usevlines : bool, default: True
            Determines the plot style.

            If ``True``, vertical lines are plotted from 0 to the acorr value
            using `.Axes.vlines`. Additionally, a horizontal line is plotted
            at y=0 using `.Axes.axhline`.

            If ``False``, markers are plotted at the acorr values using
            `.Axes.plot`.

        maxlags : int, default: 10
            Number of lags to show. If ``None``, will return all
            ``2 * len(x) - 1`` lags.

        Returns
        -------
        lags : array (length ``2*maxlags+1``)
            The lag vector.
        c : array  (length ``2*maxlags+1``)
            The auto correlation vector.
        line : `.LineCollection` or `.Line2D`
            `.Artist` added to the Axes of the correlation:

            - `.LineCollection` if *usevlines* is True.
            - `.Line2D` if *usevlines* is False.
        b : `~matplotlib.lines.Line2D` or None
            Horizontal line at 0 if *usevlines* is True
            None *usevlines* is False.

        Other Parameters
        ----------------
        linestyle : `~matplotlib.lines.Line2D` property, optional
            The linestyle for plotting the data points.
            Only used if *usevlines* is ``False``.

        marker : str, default: 'o'
            The marker for plotting the data points.
            Only used if *usevlines* is ``False``.

        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        **kwargs
            Additional parameters are passed to `.Axes.vlines` and
            `.Axes.axhline` if *usevlines* is ``True``; otherwise they are
            passed to `.Axes.plot`.

        Notes
        -----
        The cross correlation is performed with `numpy.correlate` with
        ``mode = "full"``.
        """
        return self.xcorr(x, x, **kwargs)

    @_api.make_keyword_only("3.10", "normed")
    @_preprocess_data(replace_names=["x", "y"], label_namer="y")
    def xcorr(self, x, y, normed=True, detrend=mlab.detrend_none,
              usevlines=True, maxlags=10, **kwargs):
        r"""
        Plot the cross correlation between *x* and *y*.

        The correlation with lag k is defined as
        :math:`\sum_n x[n+k] \cdot y^*[n]`, where :math:`y^*` is the complex
        conjugate of :math:`y`.

        Parameters
        ----------
        x, y : array-like of length n
            Neither *x* nor *y* are run through Matplotlib's unit conversion, so
            these should be unit-less arrays.

        detrend : callable, default: `.mlab.detrend_none` (no detrending)
            A detrending function applied to *x* and *y*.  It must have the
            signature ::

                detrend(x: np.ndarray) -> np.ndarray

        normed : bool, default: True
            If ``True``, input vectors are normalised to unit length.

        usevlines : bool, default: True
            Determines the plot style.

            If ``True``, vertical lines are plotted from 0 to the xcorr value
            using `.Axes.vlines`. Additionally, a horizontal line is plotted
            at y=0 using `.Axes.axhline`.

            If ``False``, markers are plotted at the xcorr values using
            `.Axes.plot`.

        maxlags : int, default: 10
            Number of lags to show. If None, will return all ``2 * len(x) - 1``
            lags.

        Returns
        -------
        lags : array (length ``2*maxlags+1``)
            The lag vector.
        c : array  (length ``2*maxlags+1``)
            The auto correlation vector.
        line : `.LineCollection` or `.Line2D`
            `.Artist` added to the Axes of the correlation:

            - `.LineCollection` if *usevlines* is True.
            - `.Line2D` if *usevlines* is False.
        b : `~matplotlib.lines.Line2D` or None
            Horizontal line at 0 if *usevlines* is True
            None *usevlines* is False.

        Other Parameters
        ----------------
        linestyle : `~matplotlib.lines.Line2D` property, optional
            The linestyle for plotting the data points.
            Only used if *usevlines* is ``False``.

        marker : str, default: 'o'
            The marker for plotting the data points.
            Only used if *usevlines* is ``False``.

        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        **kwargs
            Additional parameters are passed to `.Axes.vlines` and
            `.Axes.axhline` if *usevlines* is ``True``; otherwise they are
            passed to `.Axes.plot`.

        Notes
        -----
        The cross correlation is performed with `numpy.correlate` with
        ``mode = "full"``.
        """
        Nx = len(x)
        if Nx != len(y):
            raise ValueError('x and y must be equal length')
        if not callable(detrend):
            # matplotlib calls *detrend* on both series first.
            raise TypeError(f"'{type(detrend).__name__}' object is not callable")
        plain = _plain_series(x) and _plain_series(y)
        if not plain:
            # matplotlib correlates them with numpy before it looks at
            # *maxlags*, which fails on series of no numbers (and sums
            # booleans as numpy does).
            x_detrended = detrend(np.asarray(x))
            y_detrended = detrend(np.asarray(y))
            correls = np.correlate(x_detrended, y_detrended, mode="full")
            if normed:
                correls = correls / np.sqrt(np.dot(x_detrended, x_detrended)
                                            * np.dot(y_detrended, y_detrended))
        if maxlags is None:
            maxlags = Nx - 1
        if maxlags >= Nx or maxlags < 1:
            raise ValueError('maxlags must be None or strictly '
                             f'positive < {Nx:d}')
        if not plain:
            # matplotlib's plot of the correlations numpy computed.
            lags = np.arange(-maxlags, maxlags + 1)
            correls = correls[Nx - 1 - maxlags:Nx + maxlags]
            if usevlines:
                a = self.vlines(lags, [0], correls, **kwargs)
                kwargs.pop('label', '')
                b = self.axhline(**kwargs)
            else:
                kwargs.setdefault('marker', 'o')
                kwargs.setdefault('linestyle', 'None')
                a, = self.plot(lags, correls, **kwargs)
                b = None
            return lags, correls, a, b
        # matplotlib's lags, and the correlations at them, which a *maxlags*
        # that is no integer fails to make.
        _ = np.arange(-maxlags, maxlags + 1)
        _ = np.empty(0)[Nx - 1 - maxlags:Nx + maxlags]
        check_known(mcoll.LineCollection if usevlines else mlines.Line2D, kwargs)
        lags, c, (lines, zero, markers) = _plotting.xcorr(
            self._native, (_floats(x), _floats(y)),
            (bool(normed), mlab._detrend_spec(detrend), int(maxlags)),
            bool(usevlines), _line(kwargs))
        if markers is not None:
            return lags, c, wrap(self, markers, mlines.Line2D), None
        return (lags, c, wrap(self, lines, mcoll.LineCollection),
                wrap(self, zero, mlines.Line2D))
