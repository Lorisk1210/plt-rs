"""matplotlib's own ``tests/test_mlab.py``: windows, coherence, the one-sided
and oversampled power spectral densities, and the test classes ``TestDetrend``,
``TestSpectral``, ``TestGaussianKDE``, ``TestGaussianKDECustom`` and
``TestGaussianKDEEvaluate``.

Area ``mpl_mlab``. The ops and their arguments are those of the ``stats``
area (see ``test_stats.py``): ``window`` is ``'hanning'`` or ``'none'`` and
``detrend`` ``'none'``.

Ops
---
``mlab.window_hanning`` / ``mlab.window_none``  {x} -> the windowed values.
``mlab.psd``  {x, NFFT, Fs, window, detrend, noverlap, sides} -> ``{"Pxx",
    "freqs"}``.
``mlab.cohere``  {x, y, NFFT, Fs, noverlap} -> ``{"Cxy", "freqs"}``.
``mlab.detrend`` / ``mlab.detrend_mean``  {x, key?, axis?} -> the detrended
    values (the ``stats`` area's).
``mlab.GaussianKDE``  {dataset, points, bw_method?} -> ``{"dim", "num_dp",
    "factor", "covariance", "inv_cov", "norm_factor", "values"}`` (the
    ``stats`` area's).

``TestSpectral``
................
The class's fixture draws ``x = np.arange(0, 10, 1 / Fs)[:len_x]`` at ``Fs =
100`` and the signal ``y = sum(sin(fstim * x * pi * 2) * 10**i)`` over its
stimulus frequencies, as complex values when ``iscomplex``. A probe's
``stim`` is ``{"fstims", "len_x", "iscomplex"}`` and the other arguments
are those the tests pass (``null`` for matplotlib's default). Rather than
whole spectra, the ops record what the tests check: ``{"shape", "freqs",
"t"?, "sum", "sumsq", "max"}``, the shape of the result, its frequencies at
the indices ``0, n // 4, n // 2, 3 * n // 4, n - 1``, the times of a
spectrogram, and the sum, the sum of squares and the largest of the
magnitudes of its values. Angles and phases record no values: where the
spectrum vanishes they are rounding noise.

``mlab.psd.summary``  {stim, NFFT, noverlap, pad_to, sides, window?} -> the
    summary of ``psd(x=y, Fs=Fs, ...)``; ``window`` is ``"none"`` or a list
    of values (``window_hanning`` by default).
``mlab.csd.summary``  {stim, y_offset, NFFT, noverlap, pad_to, sides,
    window?} -> the summary of ``csd(x=y, y=y + y_offset, Fs=Fs, ...)``.
``mlab.specgram.summary``  {stim, NFFT, noverlap, pad_to, sides, mode?} ->
    the summary of ``specgram(x=y, Fs=Fs, ...)``.
``mlab.<kind>_spectrum.summary``  {stim, pad_to, sides} -> the summary of
    ``<kind>_spectrum(x=y, Fs=Fs, ...)`` for ``complex``, ``magnitude``,
    ``angle`` and ``phase``.

Cases
-----
Named after the upstream tests; random data comes from
``spec_support.rng`` (``test_window``: 1000 standard normal values plus
100; ``test_cohere``: 1024 values, ``y`` rolled by 20 and smoothed with a
20-point moving average). ``TestSpectral``'s tests run over all 66 fixtures
(eleven parameter sets, three ``sides``, real and complex);
``test_spectral_helper_raises`` checks its errors through ``psd`` and
``specgram``, on one fixture; ``test_spectral_helper_psd`` (the private
helper's frequencies, times and shapes in the ``psd`` and ``magnitude``
modes) is what the summaries of ``test_psd``, ``test_specgram`` and
``test_spectrum`` record. ``TestDetrend``'s signals have 100 points rather
than 1000, and its random base comes from ``spec_support.rng(0)``;
``TestGaussianKDECustom`` draws its data from ``spec_support.rng(8765678)``.

Already covered
---------------
By the ``stats`` area (same behaviour, its own signals):
``TestDetrend``'s detrending of 0-D, 1-D and 2-D signals along each axis,
its rejected keys and axes, the linear detrend errors;
``TestSpectral.test_psd_detrend``, ``test_psd_window_hanning``,
``test_psd_window_hanning_detrend_linear``, ``test_psd_window_flattop``,
``test_psd_windowarray``, ``test_psd_windowarray_scale_by_freq`` and
``test_psd_windowarray_equal`` (detrending and windows, window arrays and
``scale_by_freq``); ``TestGaussianKDE.test_kde_bandwidth_method`` and
``TestGaussianKDEEvaluate.test_evaluate_inv_dim``.

Left out
--------
``TestDetrend``'s callable keys and ``axis=-1`` (Python callables and
numpy's negative axes; ``axis=1`` is probed),
``test_single_spectrum_helper_unsupported_modes`` and the ``y is x`` half of
``test_spectral_helper_raises`` (a private helper and Python object
identity), the 32-bit ``sys.maxsize`` variant of ``test_csd``, the warning
of ``test_specgram_warn_only1seg`` (pltrs has no warnings; its values are
probed), ``TestGaussianKDECustom.test_callable_covariance_dataset`` (a
Python callable bandwidth).
"""

import numpy as np
from matplotlib import mlab

from probes.test_stats import TOL_KDE, TOL_LIN, _detrend, _detrend_mean, _kde
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


def _detrend_signals(n=100):
    """``TestDetrend``'s signals (on 100 points rather than 1000), its
    random base drawn from ``spec_support.rng``."""
    x = np.linspace(0., 100, n)
    base = rng(0).normal(n) + np.sin(x * 2 * np.pi / (n / 100))
    return {"zeros": np.zeros(n), "off": np.zeros(n) + 100., "slope": np.linspace(-10., 90., n),
            "base": base - base.mean()}


def detrend(x, tol=TOL_LIN, **kw):
    return probes.probe("mlab.detrend", _detrend, tol=tol, x=np.asarray(x).tolist(), **kw)


def detrend_mean(x, tol=TOL_LIN, **kw):
    return probes.probe("mlab.detrend_mean", _detrend_mean, tol=tol, x=np.asarray(x).tolist(), **kw)


class TestDetrend:
    SIG_SLOPE = np.linspace(-10., 90., 1000).tolist()

    def test_detrend_mean_1d_base_slope_off_list_andor_axis0(self):
        sig = _detrend_signals()
        signal = sig["base"] + sig["slope"] + sig["off"]
        detrend_mean(signal, axis=0)
        detrend_mean(signal)

    def test_detrend_mean_2d(self):
        sig = _detrend_signals()
        two = np.vstack([sig["off"], sig["base"] + sig["off"]])
        for x, kw in [(two, {}), (two, {"axis": None}), (two.T, {"axis": None})]:
            detrend_mean(x, **kw)
        for x, kw in [(two, {}), (two, {"axis": None}), (two.T, {"key": "constant", "axis": None})]:
            detrend(x, **kw)
        four = np.vstack([sig["base"], sig["base"] + sig["off"], sig["base"] + sig["slope"],
                          sig["base"] + sig["off"] + sig["slope"]])
        detrend_mean(four.T, axis=0)
        detrend_mean(four, axis=1)
        detrend(four, key="default", axis=1)
        detrend(four.T, key="mean", axis=0)

    def test_detrend_str_linear_1d(self):
        sig = _detrend_signals()
        detrend(sig["slope"] + sig["off"], key="linear")

    def test_detrend_linear_2d(self):
        sig = _detrend_signals()
        three = np.vstack([sig["off"], sig["slope"], sig["slope"] + sig["off"]])
        detrend(three.T, key="linear", axis=0)
        detrend(three, key="linear", axis=1)

    def test_detrend_ValueError(self):
        r = probes.probe("mlab.detrend", _detrend, x=5.5, axis=0)
        assert isinstance(r, ValueError)

    def test_detrend_mean_ValueError(self):
        for x, axis in [(5.5, 0), ([self.SIG_SLOPE], 2)]:
            r = probes.probe("mlab.detrend_mean", _detrend_mean, x=x, axis=axis)
            assert isinstance(r, ValueError)


FS = 100.0

#: The fixture's parameter sets: ``fstims, len_x, NFFT_density,
#: nover_density, pad_to_density, pad_to_spectrum``.
FIXTURES = {
    "nosig": ([], None, -1, -1, -1, -1),
    "Fs4": ([4], None, -1, -1, -1, -1),
    "FsAll": ([4, 5, 10], None, -1, -1, -1, -1),
    "nosig_noNFFT": ([], None, None, -1, -1, None),
    "nosig_nopad_to": ([], None, -1, -1, None, None),
    "nosig_noNFFT_no_pad_to": ([], None, None, -1, None, None),
    "nosig_trim": ([], 1024, 512, -1, -1, 128),
    "nosig_odd": ([], 256, -1, -1, 33, 257),
    "nosig_oddlen": ([], 255, 33, -1, -1, None),
    "nosig_stretch": ([], 256, 128, -1, 256, 256),
    "nosig_overlap": ([], None, -1, 32, -1, -1),
}


def _stimulus(stim):
    """The fixture's signal ``y``."""
    x = np.arange(0, 10, 1 / FS)
    if stim["len_x"] is not None:
        x = x[:stim["len_x"]]
    y = np.zeros_like(x)
    for i, fstim in enumerate(stim["fstims"]):
        y += np.sin(fstim * x * np.pi * 2) * 10**i
    return y.astype("complex") if stim["iscomplex"] else y


def _fixture(name, iscomplex):
    """The fixture's stimulus and the arguments its tests pass to the
    density estimates (``psd``, ``csd``, ``specgram``) and to the single
    spectra."""
    fstims, len_x, nfft, noverlap, pad_to, pad_to_spectrum = FIXTURES[name]
    x_len = len(np.arange(0, 10, 1 / FS)[:len_x])
    nfft_real = 256 if nfft is None else (100 if nfft < 0 else nfft)
    if nfft is not None and nfft < 0:
        nfft = 100
    if noverlap is not None and noverlap < 0:
        noverlap = nfft_real // 2
    if pad_to is not None and pad_to < 0:
        pad_to = int(2**np.ceil(np.log2(nfft_real)))
    if pad_to_spectrum is not None and pad_to_spectrum < 0:
        pad_to_spectrum = x_len
    stim = {"fstims": [FS / f for f in fstims], "len_x": len_x, "iscomplex": iscomplex}
    return stim, {"NFFT": nfft, "noverlap": noverlap, "pad_to": pad_to}, pad_to_spectrum


def _fixtures():
    for name in FIXTURES:
        for sides in ["onesided", "twosided", "default"]:
            for iscomplex in [False, True]:
                stim, density, pad_to_spectrum = _fixture(name, iscomplex)
                yield stim, sides, density, pad_to_spectrum


def _summary(spec, freqs, t=None, values=True):
    spec = np.asarray(spec)
    freqs = np.asarray(freqs)
    n = len(freqs)
    out = {"shape": list(spec.shape), "freqs": freqs[sorted({0, n // 4, n // 2, 3 * n // 4, n - 1})]}
    if t is not None:
        out["t"] = np.asarray(t)
    if values:
        magnitude = np.abs(spec)
        out.update(sum=magnitude.sum(), sumsq=(magnitude ** 2).sum(), max=magnitude.max())
    return out


def _window(window):
    if window is None:
        return mlab.window_hanning
    return mlab.window_none if window == "none" else np.asarray(window, float)


def _psd_summary(stim, window=None, **kw):
    spec, freqs = mlab.psd(x=_stimulus(stim), Fs=FS, window=_window(window), **kw)
    return _summary(spec, freqs)


def _csd_summary(stim, y_offset, window=None, **kw):
    y = _stimulus(stim)
    spec, freqs = mlab.csd(x=y, y=y + y_offset, Fs=FS, window=_window(window), **kw)
    return _summary(spec, freqs)


def _specgram_summary(stim, **kw):
    spec, freqs, t = mlab.specgram(x=_stimulus(stim), Fs=FS, **kw)
    return _summary(spec, freqs, t, values=kw.get("mode") not in ("angle", "phase"))


def _spectrum_summary(kind):
    def summary(stim, **kw):
        spec, freqs = getattr(mlab, f"{kind}_spectrum")(x=_stimulus(stim), Fs=FS, **kw)
        return _summary(spec, freqs, values=kind not in ("angle", "phase"))

    return summary


class TestSpectral:
    def test_spectral_helper_raises(self):
        stim, _, _ = _fixture("Fs4", False)
        for op, fn, kw in [
                ("mlab.specgram.summary", _specgram_summary, {"mode": "spam"}),
                ("mlab.psd.summary", _psd_summary, {"sides": "eggs"}),
                ("mlab.psd.summary", _psd_summary, {"NFFT": 10, "noverlap": 20}),
                ("mlab.psd.summary", _psd_summary, {"NFFT": 10, "noverlap": 10}),
                ("mlab.psd.summary", _psd_summary, {"NFFT": 10, "window": [1.0] * 9})]:
            assert isinstance(probes.probe(op, fn, stim=stim, **kw), ValueError)

    def test_csd(self):
        for stim, sides, density, _ in _fixtures():
            probes.probe("mlab.csd.summary", _csd_summary, tol=TOL_FFT, stim=stim, y_offset=1,
                         sides=sides, **density)

    def test_csd_padding(self):
        for stim, sides, density, _ in _fixtures():
            if density["NFFT"] is None:
                continue
            for nfft in (density["NFFT"], 2 * density["NFFT"]):
                probes.probe("mlab.csd.summary", _csd_summary, tol=TOL_FFT, stim=stim,
                             y_offset=1, window="none", sides=sides, NFFT=nfft)

    def test_psd(self):
        for stim, sides, density, _ in _fixtures():
            probes.probe("mlab.psd.summary", _psd_summary, tol=TOL_FFT, stim=stim, sides=sides,
                         **density)

    def test_spectrum(self):
        for stim, sides, _, pad_to in _fixtures():
            for kind in ["complex", "magnitude", "angle", "phase"]:
                probes.probe(f"mlab.{kind}_spectrum.summary", _spectrum_summary(kind),
                             tol=TOL_FFT, stim=stim, sides=sides, pad_to=pad_to)

    # test_specgram_auto_default_psd_equal and test_specgram_complex_equivalent
    # compare modes: each mode's summary is recorded.
    def test_specgram(self):
        for stim, sides, density, _ in _fixtures():
            for mode in [None, "default", "psd", "magnitude", "complex", "angle", "phase"]:
                kw = {} if mode is None else {"mode": mode}
                probes.probe("mlab.specgram.summary", _specgram_summary, tol=TOL_FFT, stim=stim,
                             sides=sides, **density, **kw)

    def test_specgram_warn_only1seg(self):
        for stim, _, _, _ in _fixtures():
            n = len(_stimulus(stim))
            probes.probe("mlab.specgram.summary", _specgram_summary, tol=TOL_FFT, stim=stim,
                         NFFT=n)

    def test_psd_csd_equal(self):
        for stim, sides, density, _ in _fixtures():
            probes.probe("mlab.csd.summary", _csd_summary, tol=TOL_FFT, stim=stim, y_offset=0,
                         sides=sides, **density)


def _kde_probe(**args):
    return probes.probe("mlab.GaussianKDE", _kde, tol=TOL_KDE, **args)


class TestGaussianKDE:
    def test_kde_integer_input(self):
        r = _kde_probe(dataset=list(range(5)), points=list(range(5)))
        np.testing.assert_array_almost_equal(
            r["values"], [0.13480721, 0.18222869, 0.19514935, 0.18222869, 0.13480721],
            decimal=6)

    def test_gaussian_kde_covariance_caching(self):
        r = _kde_probe(dataset=[-7, -5, 1, 4, 5], points=np.linspace(-10, 10, num=5).tolist(),
                       bw_method="scott")
        np.testing.assert_array_almost_equal(
            [0.02463386, 0.04689208, 0.05395444, 0.05337754, 0.01664475], r["values"],
            decimal=7)


class TestGaussianKDECustom:
    def test_single_dataset_element(self):
        assert isinstance(_kde_probe(dataset=[42], points=[0]), ValueError)

    def test_scalar_empty_dataset(self):
        assert isinstance(_kde_probe(dataset=[], points=[0], bw_method=5), ValueError)

    def test_scalar_covariance_dataset(self):
        data = rng(8765678).normal(5 * 50).reshape(5, 50).tolist()
        r = _kde_probe(dataset=data, points=[[0]] * 5, bw_method=0.5)
        assert r["factor"] == 0.5

    def test_callable_singledim_dataset(self):
        _kde_probe(dataset=rng(8765678).normal(50).tolist(), points=[0], bw_method="silverman")

    def test_wrong_bw_method(self):
        r = _kde_probe(dataset=rng(8765678).normal(50).tolist(), points=[0], bw_method="invalid")
        assert isinstance(r, ValueError)

    def test_silverman_multidim_dataset(self):
        r = _kde_probe(dataset=[[1, 2, 3], [4, 5, 6], [7, 8, 9]], points=[[1], [2], [3]],
                       bw_method="silverman")
        assert type(r).__name__ == "LinAlgError"

    def test_silverman_singledim_dataset(self):
        r = _kde_probe(dataset=[-7, -5, 1, 4, 5], points=[0], bw_method="silverman")
        np.testing.assert_almost_equal(r["factor"], 0.76770389927475502, 7)

    def test_scott_multidim_dataset(self):
        r = _kde_probe(dataset=[[1, 2, 3], [4, 5, 6], [7, 8, 9]], points=[[1], [2], [3]],
                       bw_method="scott")
        assert type(r).__name__ == "LinAlgError"

    def test_scott_singledim_dataset(self):
        r = _kde_probe(dataset=[-7, -5, 1, 4, 5], points=[0], bw_method="scott")
        np.testing.assert_almost_equal(r["factor"], 0.72477966367769553, 7)


class TestGaussianKDEEvaluate:
    X1 = list(range(3, 10, 2))

    def test_evaluate_diff_dim(self):
        r = _kde_probe(dataset=self.X1, points=list(range(3, 12, 2)))
        np.testing.assert_array_almost_equal(
            r["values"], [0.08797252, 0.11774109, 0.11774109, 0.08797252, 0.0370153], 7)

    def test_evaluate_dim_and_num(self):
        r = _kde_probe(dataset=self.X1, points=[3])
        np.testing.assert_array_almost_equal(r["values"], [0.08797252], 7)

    def test_evaluate_point_dim_not_one(self):
        r = _kde_probe(dataset=self.X1, points=[self.X1, self.X1])
        assert isinstance(r, ValueError)

    def test_evaluate_equal_dim_and_num_lt(self):
        r = _kde_probe(dataset=self.X1, points=list(range(3, 8, 2)))
        np.testing.assert_array_almost_equal(r["values"], [0.08797252, 0.11774109, 0.11774109], 7)
