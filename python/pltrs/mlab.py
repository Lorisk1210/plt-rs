"""
Numerical Python functions written for compatibility with MATLAB
commands with the same names (matplotlib's ``mlab``): spectral estimates,
detrending and kernel density estimation.

pltrs (Rust) computes them (``pltrs::mlab``); the functions here keep matplotlib's
signatures and defaults.

Spectral functions
------------------
`cohere`
    Coherence (normalized cross spectral density)
`csd`
    Cross spectral density using Welch's average periodogram
`detrend`
    Remove the mean or best fit line from an array
`psd`
    Power spectral density using Welch's average periodogram
`specgram`
    Spectrogram (spectrum over segments of time)
`complex_spectrum`
    Return the complex-valued frequency spectrum of a signal
`magnitude_spectrum`
    Return the magnitude of the frequency spectrum of a signal
`angle_spectrum`
    Return the angle (wrapped phase) of the frequency spectrum of a signal
`phase_spectrum`
    Return the phase (unwrapped angle) of the frequency spectrum of a signal
`detrend_mean`
    Remove the mean from a line.
`detrend_linear`
    Remove the best fit line from a line.
`detrend_none`
    Return the original line.
"""

import functools
from numbers import Number

import numpy as np

from . import _api, cbook
from ._native import mlab as _mlab



def window_hanning(x):
    """
    Return *x* times the Hanning (or Hann) window of len(*x*).

    See Also
    --------
    window_none : Another window algorithm.
    """
    return np.hanning(len(x))*x


def window_none(x):
    """
    No window function; simply return *x*.

    See Also
    --------
    window_hanning : Another window algorithm.
    """
    return x


def detrend(x, key=None, axis=None):
    """
    Return *x* with its trend removed.

    Parameters
    ----------
    x : array or sequence
        Array or sequence containing the data.

    key : {'default', 'constant', 'mean', 'linear', 'none'} or function
        The detrending algorithm to use. 'default', 'mean', and 'constant' are
        the same as `detrend_mean`. 'linear' is the same as `detrend_linear`.
        'none' is the same as `detrend_none`. The default is 'mean'. Can
        also be a function that carries out the detrend operation.

    axis : int
        The axis along which to do the detrending.
    """
    name = _detrend_name(key)
    if name == "none":
        return x
    if name is not None:
        return _detrend(x, name, axis)
    if callable(key):
        x = np.asarray(x)
        if axis is not None and axis + 1 > x.ndim:
            raise ValueError(f'axis(={axis}) out of bounds')
        if (axis is None and x.ndim == 0) or (not axis and x.ndim == 1):
            return key(x)
        # try to use the 'axis' argument if the function supports it,
        # otherwise use apply_along_axis to do it
        try:
            return key(x, axis=axis)
        except TypeError:
            return np.apply_along_axis(key, axis=axis, arr=x)
    raise ValueError(
        f"Unknown value for key: {key!r}, must be one of: 'default', "
        f"'constant', 'mean', 'linear', or a function")


def detrend_mean(x, axis=None):
    """
    Return *x* minus the mean(*x*).

    Parameters
    ----------
    x : array or sequence
        Array or sequence containing the data
        Can have any dimensionality

    axis : int
        The axis along which to take the mean.  See `numpy.mean` for a
        description of this argument.
    """
    return _detrend(x, "mean", axis)


def detrend_none(x, axis=None):
    """
    Return *x*: no detrending.

    Parameters
    ----------
    x : any object
        An object containing the data

    axis : int
        This parameter is ignored.
        It is included for compatibility with detrend_mean
    """
    return x


def detrend_linear(y):
    """
    Return *x* minus best fit line; 'linear' detrending.

    Parameters
    ----------
    y : 0-D or 1-D array or sequence
        Array or sequence containing the data
    """
    if np.ndim(y) > 1:
        raise ValueError('y cannot have ndim > 1')
    return _detrend(y, "linear", None)


#: The detrend functions pltrs (Rust) implements, by the name it knows them by.
_DETREND_FUNCTIONS = {detrend_none: "none", detrend_mean: "mean",
                      detrend_linear: "linear"}


def _detrend_name(key):
    """The pltrs (Rust) name of the detrend method *key*, or None for a function
    pltrs (Rust) does not know (or an invalid key)."""
    if key is None or cbook._str_equal(key, "default") or cbook._str_equal(
            key, "constant") or cbook._str_equal(key, "mean"):
        return "mean"
    if cbook._str_equal(key, "linear") or cbook._str_equal(key, "none"):
        return key
    if callable(key):
        return _DETREND_FUNCTIONS.get(key)
    return None


def _detrend(x, name, axis):
    """*x* (an array-like of any dtype) detrended by pltrs (Rust)'s *name*."""
    x = np.asarray(x)
    return _mlab.detrend(np.asarray(x, dtype=float), name, axis).astype(
        np.result_type(x.dtype, float), copy=False)


def _signal(x):
    """The ``(re, im)`` form of the signal *x* for the native functions."""
    if np.iscomplexobj(x):
        return np.ascontiguousarray(x.real, float), np.ascontiguousarray(x.imag, float)
    return np.asarray(x, float), None


def _window_spec(window, nfft, dtype):
    """The native form of the window *window* of segments of *nfft*."""
    if window is None or window is window_hanning:
        return "hanning"
    if window is window_none:
        return "none"
    if not np.iterable(window):
        window = window(np.ones(nfft, dtype))
    return np.real(np.asarray(window))


def _detrend_spec(detrend_func):
    """The native form of the detrend method *detrend_func*."""
    if detrend_func is None:
        return "none"
    name = _detrend_name(detrend_func)
    if name is not None:
        return name
    if callable(detrend_func):
        return detrend_func
    raise ValueError(
        f"Unknown value for key: {detrend_func!r}, must be one of: 'default', "
        f"'constant', 'mean', 'linear', or a function")


def _spectral_helper(x, y=None, NFFT=None, Fs=None, detrend_func=None,
                     window=None, noverlap=None, pad_to=None,
                     sides=None, scale_by_freq=None, mode=None):
    """
    Private helper implementing the common parts between the psd, csd,
    spectrogram and complex, magnitude, angle, and phase spectrums.
    """
    # The checks for if y is x are so that we can use the same function to
    # implement the core of psd(), csd(), and spectrogram() without doing
    # extra calculations.  We return the unaveraged Pxy, freqs, and t.
    same_data = y is None or y is x
    if Fs is None:
        Fs = 2
    if noverlap is None:
        noverlap = 0
    if NFFT is None:
        NFFT = 256
    if not (0 <= noverlap < NFFT):
        raise ValueError('noverlap must be less than NFFT')
    if mode is None or mode == 'default':
        mode = 'psd'
    _api.check_in_list(
        ['default', 'psd', 'complex', 'magnitude', 'angle', 'phase'],
        mode=mode)
    if not same_data and mode != 'psd':
        raise ValueError("x and y must be equal if mode is not 'psd'")
    x = np.asarray(x)
    if sides is None:
        sides = 'default'
    _api.check_in_list(['default', 'onesided', 'twosided'], sides=sides)
    if pad_to is None:
        pad_to = NFFT
    if mode != 'psd':
        scale_by_freq = False
    elif scale_by_freq is None:
        scale_by_freq = True
    options = {
        "NFFT": NFFT, "Fs": Fs, "noverlap": noverlap, "pad_to": pad_to,
        "sides": sides, "scale_by_freq": scale_by_freq,
        "detrend": _detrend_spec(detrend_func),
        "window": _window_spec(window, NFFT, x.dtype),
    }
    (re, im), freqs, t = _mlab.spectral_helper(
        _signal(x), None if same_data else _signal(np.asarray(y)), options, mode)
    result = re + 1j * im if mode in ('psd', 'complex') else re
    # matplotlib's spectra are laid out frequency first (its transform keeps
    # the Fortran order of the segment view), which decides how numpy sums
    # them, as in the segment mean of `csd`.
    return np.asfortranarray(result), freqs, t


def _single_spectrum_helper(
        mode, x, Fs=None, window=None, pad_to=None, sides=None):
    """
    Private helper implementing the commonality between the complex, magnitude,
    angle, and phase spectrums.
    """
    _api.check_in_list(['complex', 'magnitude', 'angle', 'phase'], mode=mode)

    if pad_to is None:
        pad_to = len(x)

    spec, freqs, _ = _spectral_helper(x=x, y=None, NFFT=len(x), Fs=Fs,
                                      detrend_func=detrend_none, window=window,
                                      noverlap=0, pad_to=pad_to,
                                      sides=sides,
                                      scale_by_freq=False,
                                      mode=mode)
    if mode != 'complex':
        spec = spec.real

    if spec.ndim == 2 and spec.shape[1] == 1:
        spec = spec[:, 0]

    return spec, freqs


def psd(x, NFFT=None, Fs=None, detrend=None, window=None,
        noverlap=None, pad_to=None, sides=None, scale_by_freq=None):
    r"""
    Compute the power spectral density.

    The power spectral density :math:`P_{xx}` by Welch's average
    periodogram method.  The vector *x* is divided into *NFFT* length
    segments.  Each segment is detrended by function *detrend* and
    windowed by function *window*.  *noverlap* gives the length of
    the overlap between segments.  The :math:`|\mathrm{fft}(i)|^2`
    of each segment :math:`i` are averaged to compute :math:`P_{xx}`.

    If len(*x*) < *NFFT*, it will be zero padded to *NFFT*.

    Returns
    -------
    Pxx : 1-D array
        The values for the power spectrum :math:`P_{xx}` (real valued)

    freqs : 1-D array
        The frequencies corresponding to the elements in *Pxx*
    """
    Pxx, freqs = csd(x=x, y=None, NFFT=NFFT, Fs=Fs, detrend=detrend,
                     window=window, noverlap=noverlap, pad_to=pad_to,
                     sides=sides, scale_by_freq=scale_by_freq)
    return Pxx.real, freqs


def csd(x, y, NFFT=None, Fs=None, detrend=None, window=None,
        noverlap=None, pad_to=None, sides=None, scale_by_freq=None):
    """
    Compute the cross-spectral density.

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

    Returns
    -------
    Pxy : 1-D array
        The values for the cross spectrum :math:`P_{xy}` before scaling (real
        valued)

    freqs : 1-D array
        The frequencies corresponding to the elements in *Pxy*
    """
    if NFFT is None:
        NFFT = 256
    Pxy, freqs, _ = _spectral_helper(x=x, y=y, NFFT=NFFT, Fs=Fs,
                                     detrend_func=detrend, window=window,
                                     noverlap=noverlap, pad_to=pad_to,
                                     sides=sides, scale_by_freq=scale_by_freq,
                                     mode='psd')

    if Pxy.ndim == 2:
        if Pxy.shape[1] > 1:
            Pxy = Pxy.mean(axis=1)
        else:
            Pxy = Pxy[:, 0]
    return Pxy, freqs


complex_spectrum = functools.partial(_single_spectrum_helper, "complex")
complex_spectrum.__doc__ = "Compute the complex-valued frequency spectrum of *x*."
magnitude_spectrum = functools.partial(_single_spectrum_helper, "magnitude")
magnitude_spectrum.__doc__ = (
    "Compute the magnitude (absolute value) of the frequency spectrum of *x*.")
angle_spectrum = functools.partial(_single_spectrum_helper, "angle")
angle_spectrum.__doc__ = (
    "Compute the angle of the frequency spectrum (wrapped phase spectrum) of *x*.")
phase_spectrum = functools.partial(_single_spectrum_helper, "phase")
phase_spectrum.__doc__ = (
    "Compute the phase of the frequency spectrum (unwrapped phase spectrum) of *x*.")


def specgram(x, NFFT=None, Fs=None, detrend=None, window=None,
             noverlap=None, pad_to=None, sides=None, scale_by_freq=None,
             mode=None):
    """
    Compute a spectrogram.

    Compute and plot a spectrogram of data in *x*.  Data are split into
    *NFFT* length segments and the spectrum of each section is
    computed.  The windowing function *window* is applied to each
    segment, and the amount of overlap of each segment is
    specified with *noverlap*.

    Returns
    -------
    spectrum : array-like
        2D array, columns are the periodograms of successive segments.

    freqs : array-like
        1-D array, frequencies corresponding to the rows in *spectrum*.

    t : array-like
        1-D array, the times corresponding to midpoints of segments
        (i.e the columns in *spectrum*).
    """
    if noverlap is None:
        noverlap = 128  # default in _spectral_helper() is noverlap = 0
    if NFFT is None:
        NFFT = 256  # same default as in _spectral_helper()
    if len(x) <= NFFT:
        _api.warn_external("Only one segment is calculated since parameter "
                           f"NFFT (={NFFT}) >= signal length (={len(x)}).")

    spec, freqs, t = _spectral_helper(x=x, y=None, NFFT=NFFT, Fs=Fs,
                                      detrend_func=detrend, window=window,
                                      noverlap=noverlap, pad_to=pad_to,
                                      sides=sides,
                                      scale_by_freq=scale_by_freq,
                                      mode=mode)

    if mode != 'complex':
        spec = spec.real  # Needed since helper implements generically

    return spec, freqs, t


def cohere(x, y, NFFT=256, Fs=2, detrend=detrend_none, window=window_hanning,
           noverlap=0, pad_to=None, sides='default', scale_by_freq=None):
    r"""
    The coherence between *x* and *y*.  Coherence is the normalized
    cross spectral density:

    .. math::

        C_{xy} = \frac{|P_{xy}|^2}{P_{xx}P_{yy}}

    Returns
    -------
    Cxy : 1-D array
        The coherence vector.
    freqs : 1-D array
            The frequencies for the elements in *Cxy*.
    """
    if len(x) < 2 * NFFT:
        raise ValueError(
            "Coherence is calculated by averaging over *NFFT* length "
            "segments.  Your signal is too short for your choice of *NFFT*.")
    Pxx, f = psd(x, NFFT, Fs, detrend, window, noverlap, pad_to, sides,
                 scale_by_freq)
    Pyy, f = psd(y, NFFT, Fs, detrend, window, noverlap, pad_to, sides,
                 scale_by_freq)
    Pxy, f = csd(x, y, NFFT, Fs, detrend, window, noverlap, pad_to, sides,
                 scale_by_freq)
    Cxy = np.abs(Pxy) ** 2 / (Pxx * Pyy)
    return Cxy, f


class GaussianKDE:
    """
    Representation of a kernel-density estimate using Gaussian kernels.

    Parameters
    ----------
    dataset : array-like
        Datapoints to estimate from. In case of univariate data this is a 1-D
        array, otherwise a 2D array with shape (# of dims, # of data).
    bw_method : {'scott', 'silverman'} or float or callable, optional
        The method used to calculate the estimator bandwidth.  If a
        float, this will be used directly as `!kde.factor`.  If a
        callable, it should take a `GaussianKDE` instance as only
        parameter and return a float. If None (default), 'scott' is used.

    Attributes
    ----------
    dataset : ndarray
        The dataset passed to the constructor.
    dim : int
        Number of dimensions.
    num_dp : int
        Number of datapoints.
    factor : float
        The bandwidth factor, obtained from `~GaussianKDE.covariance_factor`,
        with which the covariance matrix is multiplied.
    covariance : ndarray
        The covariance matrix of *dataset*, scaled by the calculated bandwidth
        (`!kde.factor`).
    inv_cov : ndarray
        The inverse of *covariance*.
    """

    def __init__(self, dataset, bw_method=None):
        self.dataset = np.atleast_2d(dataset)
        if not np.array(self.dataset).size > 1:
            raise ValueError("`dataset` input should have multiple elements.")

        self.dim, self.num_dp = np.array(self.dataset).shape

        if bw_method is None:
            pass
        elif cbook._str_equal(bw_method, 'scott'):
            self.covariance_factor = self.scotts_factor
        elif cbook._str_equal(bw_method, 'silverman'):
            self.covariance_factor = self.silverman_factor
        elif isinstance(bw_method, Number):
            self._bw_method = 'use constant'
            self.covariance_factor = lambda: bw_method
        elif callable(bw_method):
            self._bw_method = bw_method
            self.covariance_factor = lambda: self._bw_method(self)
        else:
            raise ValueError("`bw_method` should be 'scott', 'silverman', a "
                             "scalar or a callable")

        self.factor = self.covariance_factor()
        self._native = _mlab.gaussian_kde(
            np.asarray(self.dataset, dtype=float), float(self.factor))
        self.covariance, self.inv_cov, self.norm_factor = _mlab.kde_covariance(
            self._native)

    def scotts_factor(self):
        """Scott's rule: ``num_dp ** (-1 / (dim + 4))``."""
        return _mlab.bandwidth_factor("scott", self.dim, self.num_dp)

    def silverman_factor(self):
        """Silverman's rule: ``(num_dp * (dim + 2) / 4) ** (-1 / (dim + 4))``."""
        return _mlab.bandwidth_factor("silverman", self.dim, self.num_dp)

    #  Default method to calculate bandwidth, can be overwritten by subclass
    covariance_factor = scotts_factor

    def evaluate(self, points):
        """
        Evaluate the estimated pdf on a set of points.

        Parameters
        ----------
        points : (# of dimensions, # of points)-array
            Alternatively, a (# of dimensions,) vector can be passed in and
            treated as a single point.

        Returns
        -------
        (# of points,)-array
            The values at each point.

        Raises
        ------
        ValueError : if the dimensionality of the input points is different
                     than the dimensionality of the KDE.
        """
        points = np.atleast_2d(points)
        return _mlab.kde_evaluate(self._native, np.asarray(points, dtype=float))

    __call__ = evaluate
