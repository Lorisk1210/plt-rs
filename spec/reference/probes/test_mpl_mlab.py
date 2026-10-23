"""The four module-level tests of matplotlib's own ``tests/test_mlab.py``:
windows, coherence and the one-sided and oversampled power spectral
densities (its test classes are covered by the ``stats`` area).

Area ``mpl_mlab``. The ops and their arguments are those of the ``stats``
area (see ``test_stats.py``): ``window`` is ``'hanning'`` or ``'none'`` and
``detrend`` ``'none'``.

Ops
---
``mlab.window_hanning`` / ``mlab.window_none``  {x} -> the windowed values.
``mlab.psd``  {x, NFFT, Fs, window, detrend, noverlap, sides} -> ``{"Pxx",
    "freqs"}``.
``mlab.cohere``  {x, y, NFFT, Fs, noverlap} -> ``{"Cxy", "freqs"}``.

Cases
-----
Named after the upstream tests; random data comes from
``spec_support.rng`` (``test_window``: 1000 standard normal values plus
100; ``test_cohere``: 1024 values, ``y`` rolled by 20 and smoothed with a
20-point moving average).
"""

import numpy as np
from matplotlib import mlab

from spec_support import ProbeSet, rng

probes = ProbeSet("mpl_mlab")

TOL_COS = 1e-12
TOL_FFT = 1e-9
WINDOWS = {"hanning": mlab.window_hanning, "none": mlab.window_none}
DETRENDS = {"none": mlab.detrend_none}


def _psd(x, window, detrend, **kw):
    pxx, freqs = mlab.psd(np.asarray(x, float), window=WINDOWS[window],
                          detrend=DETRENDS[detrend], **kw)
    return {"Pxx": pxx, "freqs": freqs}


def _cohere(x, y, **kw):
    cxy, freqs = mlab.cohere(np.asarray(x, float), np.asarray(y, float), **kw)
    return {"Cxy": cxy, "freqs": freqs}


def test_window():
    rand = (rng(0).normal(1000) + 100).tolist()
    ones = [1.0] * 1000
    for x in (ones, rand):
        probes.probe("mlab.window_none", lambda x: mlab.window_none(np.asarray(x)), x=x)
        probes.probe("mlab.window_hanning", lambda x: mlab.window_hanning(np.asarray(x)),
                     tol=TOL_COS, x=x)


def test_cohere():
    x = rng(19680801).normal(1024)
    y = np.convolve(np.roll(x, 20), np.ones(20) / 20., mode="same")
    probes.probe("mlab.cohere", _cohere, tol=TOL_FFT, x=x.tolist(), y=y.tolist(), NFFT=256,
                 Fs=2, noverlap=128)


def test_psd_onesided_norm():
    u = [0, 1, 2, 3, 1, 2, 1]
    probes.probe("mlab.psd", _psd, tol=TOL_FFT, x=u, NFFT=len(u), Fs=1.0, window="none",
                 detrend="none", noverlap=0, sides="onesided")


def test_psd_oversampling():
    u = [0, 1, 2, 3, 1, 2, 1]
    probes.probe("mlab.psd", _psd, tol=TOL_FFT, x=u, NFFT=2 * len(u), Fs=1.0, window="none",
                 detrend="none", noverlap=0, sides="onesided")
