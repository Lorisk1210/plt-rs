"""
matplotlib's spectral computations as its ``mlab`` writes them (from
matplotlib 3.11.2, under the Matplotlib License): ``_spectral_helper``,
``_single_spectrum_helper`` and the averaging of ``csd`` and ``cohere``.

pltrs (Rust) computes the spectra the package plots and returns. These
run where pltrs (Rust) cannot take the arguments as they are given
(numbers given as strings, sizes that are no integers, windows that are
no arrays) or refuses them: computing with them as matplotlib does
raises matplotlib's exception at the same step (see
`.axes._plotting.spectral`).
"""

import numpy as np

from . import _api
from .mlab import detrend, detrend_none, window_hanning


def spectral_helper(x, y=None, NFFT=None, Fs=None, detrend_func=None,
                    window=None, noverlap=None, pad_to=None,
                    sides=None, scale_by_freq=None, mode=None):
    """matplotlib's ``mlab._spectral_helper``: the unaveraged spectrum of
    *x* (with *y*, their cross spectrum), its frequencies and times."""
    same_data = True if y is None else y is x
    if Fs is None:
        Fs = 2
    if noverlap is None:
        noverlap = 0
    if detrend_func is None:
        detrend_func = detrend_none
    if window is None:
        window = window_hanning
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
    if not same_data:
        y = np.asarray(y)
    if sides is None or sides == 'default':
        sides = 'twosided' if np.iscomplexobj(x) else 'onesided'
    _api.check_in_list(['default', 'onesided', 'twosided'], sides=sides)
    if len(x) < NFFT:
        n = len(x)
        x = np.resize(x, NFFT)
        x[n:] = 0
    if not same_data and len(y) < NFFT:
        n = len(y)
        y = np.resize(y, NFFT)
        y[n:] = 0
    if pad_to is None:
        pad_to = NFFT
    if mode != 'psd':
        scale_by_freq = False
    elif scale_by_freq is None:
        scale_by_freq = True
    if sides == 'twosided':
        numFreqs = pad_to
        if pad_to % 2:
            freqcenter = (pad_to - 1)//2 + 1
        else:
            freqcenter = pad_to//2
        scaling_factor = 1.
    elif sides == 'onesided':
        if pad_to % 2:
            numFreqs = (pad_to + 1)//2
        else:
            numFreqs = pad_to//2 + 1
        scaling_factor = 2.
    if not np.iterable(window):
        window = window(np.ones(NFFT, x.dtype))
    if len(window) != NFFT:
        raise ValueError(
            "The window length must match the data's first dimension")
    result = _segments(x, NFFT, noverlap, detrend_func, window, pad_to, numFreqs)
    freqs = np.fft.fftfreq(pad_to, 1/Fs)[:numFreqs]
    if not same_data:
        resultY = _segments(y, NFFT, noverlap, detrend_func, window, pad_to, numFreqs)
        result = np.conj(result) * resultY
    elif mode == 'psd':
        result = np.conj(result) * result
    elif mode == 'magnitude':
        result = np.abs(result) / window.sum()
    elif mode == 'angle' or mode == 'phase':
        result = np.angle(result)
    elif mode == 'complex':
        result /= window.sum()
    if mode == 'psd':
        # The first and the last (for an even NFFT) frequencies have no
        # negative counterpart.
        slc = slice(1, -1, None) if not NFFT % 2 else slice(1, None, None)
        result[slc] *= scaling_factor
        if scale_by_freq:
            result /= Fs
            result /= (window**2).sum()
        else:
            result /= window.sum()**2
    t = np.arange(NFFT/2, len(x) - NFFT/2 + 1, NFFT - noverlap)/Fs
    if sides == 'twosided':
        freqs = np.roll(freqs, -freqcenter, axis=0)
        result = np.roll(result, -freqcenter, axis=0)
    elif not pad_to % 2:
        freqs[-1] *= -1
    if mode == 'phase':
        result = np.unwrap(result, axis=0)
    return result, freqs, t


def _segments(x, NFFT, noverlap, detrend_func, window, pad_to, numFreqs):
    """The transforms of the windowed, detrended segments of *x*, as
    ``_spectral_helper`` makes them (on a 64-bit host)."""
    result = np.lib.stride_tricks.sliding_window_view(
        x, NFFT, axis=0)[::NFFT - noverlap].T
    result = detrend(result, detrend_func, axis=0)
    result = result * window.reshape((-1, 1))
    return np.fft.fft(result, n=pad_to, axis=0)[:numFreqs, :]


def single_spectrum_helper(mode, x, Fs=None, window=None, pad_to=None, sides=None):
    """matplotlib's ``mlab._single_spectrum_helper``: the spectrum *mode*
    of *x* as one segment, and its frequencies."""
    _api.check_in_list(['complex', 'magnitude', 'angle', 'phase'], mode=mode)
    if pad_to is None:
        pad_to = len(x)
    spec, freqs, _ = spectral_helper(x=x, y=None, NFFT=len(x), Fs=Fs,
                                     detrend_func=detrend_none, window=window,
                                     noverlap=0, pad_to=pad_to, sides=sides,
                                     scale_by_freq=False, mode=mode)
    if mode != 'complex':
        spec = spec.real
    if spec.ndim == 2 and spec.shape[1] == 1:
        spec = spec[:, 0]
    return spec, freqs


def csd(x, y, NFFT=None, Fs=None, detrend=None, window=None,
        noverlap=None, pad_to=None, sides=None, scale_by_freq=None):
    """matplotlib's ``mlab.csd`` (``mlab.psd`` with *y* None)."""
    if NFFT is None:
        NFFT = 256
    Pxy, freqs, _ = spectral_helper(x=x, y=y, NFFT=NFFT, Fs=Fs,
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


def cohere(x, y, NFFT=256, Fs=2, detrend=detrend_none, window=window_hanning,
           noverlap=0, pad_to=None, sides='default', scale_by_freq=None):
    """matplotlib's ``mlab.cohere``."""
    if len(x) < 2 * NFFT:
        raise ValueError(
            "Coherence is calculated by averaging over *NFFT* length "
            "segments.  Your signal is too short for your choice of *NFFT*.")
    Pxx, f = csd(x, None, NFFT, Fs, detrend, window, noverlap, pad_to, sides,
                 scale_by_freq)
    Pyy, f = csd(y, None, NFFT, Fs, detrend, window, noverlap, pad_to, sides,
                 scale_by_freq)
    Pxy, f = csd(x, y, NFFT, Fs, detrend, window, noverlap, pad_to, sides,
                 scale_by_freq)
    return np.abs(Pxy) ** 2 / (Pxx * Pyy), f
