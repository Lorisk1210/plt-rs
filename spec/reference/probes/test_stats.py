"""Statistics: box/violin stats, KDE, histograms, windows/detrending, spectra, correlation.

Area ``stats`` (golden ``spec/goldens/probes/stats.json``).  Covers
``matplotlib.cbook.boxplot_stats`` / ``violin_stats``, ``matplotlib.mlab``
(``GaussianKDE``, windows, detrending, Welch spectra, single spectra,
coherence), numpy's histogram binning (which ``Axes.hist`` delegates to and
pltrs must reproduce), ``Axes.hist`` and ``Axes.acorr`` / ``Axes.xcorr``.

General conventions (apply to every op below)
=============================================

* An arg that is *absent* from a probe means "not passed": matplotlib's
  default applies.  An arg that is present with value ``null`` means
  ``None`` was passed explicitly (which is usually, but not always, the same).
* Numeric lists are converted with ``np.asarray``: a list made only of JSON
  integers becomes an ``int64`` array (numpy's integer rules apply, e.g. the
  histogram bin-width clamp to >= 1), anything containing a float becomes
  ``float64``; a list of JSON booleans becomes a ``bool`` array.  ``NaN`` /
  ``Infinity`` inside args are real non-finite floats.
* "Datasets" args (``X`` of boxplot/violin stats, ``x`` of ``Axes.hist``)
  follow ``cbook._reshape_2D``: a flat list is ONE dataset; a list of lists
  is one dataset per inner list (inner lists may have different lengths).
  When the companion flag ``X_ndarray`` / ``x_ndarray`` is ``true`` the
  value is first turned into a 2-D ndarray and its COLUMNS are the datasets.
* Complex arrays are written as ``[re, im]`` pairs, both in args (complex
  spectral input ``x``/``y`` is a list of ``[re, im]`` pairs) and in results.
  A 2-D complex array is a list of rows of ``[re, im]`` pairs.
* Results are recorded unrounded.  Many datasets are quantised to multiples
  of 1/64 so that sums and means are exact in binary64 whatever the
  summation order; such probes are recorded without ``tol`` and must match
  exactly.
* ``tol`` (relative) is used for KDE, FFT-based spectra, correlation,
  ``np.cov``-based linear detrending and ``cos``-based windows.  Several of
  these outputs contain values that are mathematically zero (e.g. the
  imaginary part of real-input DC/Nyquist bins, angle 0 at those bins,
  residuals of an exactly linear signal) and are stored as exact ``0.0``.
  The comparison is therefore meant to be scale-aware:
  ``|actual - golden| <= tol * max(|golden|, max_abs(golden array the value
  belongs to))`` (for scalars in a dict the scale is the scalar itself).
* numpy algorithms the outputs depend on:
  - ``np.percentile`` (method 'linear'): ``q = p/100``, ``h = (n-1)*q``,
    ``lo = floor(h)``, ``g = h - lo``, ``a = s[lo]``, ``b = s[min(lo+1, n-1)]``
    (``s`` sorted), result ``a + (b-a)*g`` if ``g < 0.5`` else
    ``b - (b-a)*(1-g)``.  Any NaN in the data makes the result NaN.
  - ``np.median``: mean of the two middle sorted values ``(a+b)/2`` (even n),
    the middle value (odd n); NaN if any NaN.
  - ``np.linspace(a, b, n)``: ``a + i*((b-a)/(n-1))`` with the last element
    set to ``b`` exactly.
  - ``np.hanning(M)``: ``0.5 + 0.5*cos(pi*k/(M-1))`` for
    ``k = 1-M, 3-M, ..., M-1``; ``[1.0]`` for M == 1, ``[]`` for M == 0.
  - ``np.std``: population std (ddof=0); ``np.cov``: sample cov (ddof=1)
    unless stated otherwise.

Ops
===

``cbook.boxplot_stats``
    args: ``X`` (datasets, see above), optional ``X_ndarray``, ``whis``
    (number, ``Infinity``, or ``[lo_pct, hi_pct]``; default 1.5; a string
    raises), ``labels`` (list of str), ``autorange`` (bool).  ``bootstrap`` is
    always None (bootstrapping uses ``np.random`` and is not specified).
    Returns a list (one per dataset) of dicts with keys ``mean, iqr, cilo,
    cihi, whishi, whislo, fliers, q1, med, q3`` plus ``label`` only when
    labels were given.  ``q1, med, q3`` = percentiles 25/50/75 of the
    non-masked data; ``iqr = q3 - q1``; if ``iqr == 0`` and autorange,
    ``whis = (0, 100)``; ``cilo/cihi = med -/+ 1.57*iqr/sqrt(N)``; numeric whis:
    ``loval = q1 - whis*iqr``, ``hival = q3 + whis*iqr``; pair whis:
    ``loval, hival = percentile(x, whis)``; ``whishi`` = max of data ``<=
    hival`` (q3 if none or if that max < q3); ``whislo`` symmetric with q1;
    ``fliers`` = data ``< whislo`` (in data order) followed by data
    ``> whishi`` (in data order).  Empty dataset: all NaN, fliers ``[]``.
    NaN data: everything NaN (comparisons with NaN are false so fliers ``[]``).
    Exact.

``cbook.violin_stats``
    args: ``X`` (datasets), optional ``X_ndarray``, ``bw_method`` ('scott',
    'silverman' or a number; with ``method_name`` absent the method is
    ``("GaussianKDE", bw_method)``; when both are absent matplotlib's default
    ``("GaussianKDE", "scott")`` is used), ``method_name`` (string; method =
    ``(method_name, bw_method or "scott")``; anything but "GaussianKDE"
    raises), ``points`` (int, default 100), ``quantiles`` (list or list of
    lists, fractions in [0, 1]).  Non-finite values are dropped first.
    Returns a list of dicts ``{coords, vals, mean, median, min, max,
    quantiles}``: ``coords = linspace(min, max, points)``; ``vals`` = the
    GaussianKDE (see below) evaluated at coords, except when all values are
    equal: then ``vals[i] = 1.0 if coords[i] == x[0] else 0.0``;
    ``quantiles = percentile(x, 100*q)``.  Empty dataset: empty
    ``vals/coords/quantiles``, NaN scalars.  ``tol``.

``mlab.GaussianKDE``
    args: ``dataset`` (1-D list, or ``[dim][n]`` nested list), ``bw_method``
    (absent/null = 'scott', 'silverman', number, else ValueError), ``points``
    (1-D list for dim 1, or ``[dim][m]``, or a scalar).  Returns ``{dim,
    num_dp, factor, covariance, inv_cov, norm_factor, values}`` where scott
    ``factor = n**(-1/(d+4))``, silverman ``(n*(d+2)/4)**(-1/(d+4))``, a
    number is used verbatim; ``covariance = cov(dataset, ddof=1) *
    factor**2`` (``[d][d]``), ``inv_cov = inv(cov) / factor**2``,
    ``norm_factor = sqrt(det(2*pi*covariance)) * n``, ``values[j] = sum_i
    exp(-(d_ij^T inv_cov d_ij)/2) / norm_factor``.  Errors: fewer than 2
    elements -> ValueError; singular covariance -> LinAlgError; points of
    the wrong dimension -> ValueError.  ``tol``.

``numpy.histogram``
    args: ``a``, ``bins`` (int, estimator name, or edge list; default 10),
    ``range`` (``[lo, hi]`` or null), ``density`` (bool), ``weights`` (list or
    null).  Returns ``{"hist": [...], "bin_edges": [...]}``.  ``hist`` holds
    JSON integers when not density and unweighted or integer-weighted
    (numpy keeps the weights' dtype), floats otherwise.
    Semantics are numpy's: data outside the range are dropped (so NaN is
    dropped when ``range`` is given); without ``range`` the outer edges are
    data min/max (``0, 1`` for empty data) and non-finite min/max raises
    ValueError; equal outer edges are expanded by -/+0.5; the last bin is
    closed; equal-width edges come from ``linspace`` (float64 even for
    integer data); boolean data are binned as uint8 0/1; ``density``:
    ``hist / diff(edges) / hist.sum()``.  ``n_bins`` such that edges are not
    strictly increasing raises ValueError ("Too many bins").
    Estimator names: see ``numpy.histogram_bin_width``; number of bins
    ``= ceil((hi - lo) / width)`` (1 when width is 0 or the trimmed data is
    empty); estimators reject weights (TypeError).  Exact.

``numpy.histogram_bin_edges``
    args: ``a``, ``bins``, ``range``, ``weights`` (as above).  Returns the
    edge list.  Exact.

``numpy.histogram_bin_width``
    args: ``a``, ``estimator``.  The raw bin width numpy's estimator returns
    for data ``a`` (no range: ``range = (min(a), max(a))``, expanded by
    -/+0.5 when equal; only 'stone' uses it).  Width formulas (n = size,
    ptp = max - min): sqrt ``ptp/sqrt(n)``; sturges ``ptp/(log2(n)+1)``;
    rice ``ptp/(2*n**(1/3))``; scott ``(24*sqrt(pi)/n)**(1/3)*std``; fd
    ``2*iqr*n**(-1/3)``; auto ``min(max(fd, sqrt/2), sturges)``; doane
    ``ptp/(1 + log2(n) + log2(1 + |g1|/sg1))`` with ``g1 = mean(((x -
    mean)/std)**3)``, ``sg1 = sqrt(6(n-2)/((n+1)(n+3)))`` (0 when n <= 2 or
    std == 0); stone: 0 when n <= 1 or ptp == 0, else ``ptp/k`` where k in
    ``1..max(100, int(sqrt(n)))`` minimises ``(2 - (n+1)*sum(p_k**2)) /
    (ptp/k)`` (``p_k`` = histogram(a, k, range)/n; first minimum wins).
    ``tol``.

``Axes.hist``
    args: ``x`` (datasets), optional ``x_ndarray``, and any of ``bins,
    range, density, weights, cumulative`` (bool or number; negative means
    reverse), ``bottom`` (number or list), ``histtype`` ('bar',
    'barstacked', 'step', 'stepfilled'), ``align`` ('left','mid','right'),
    ``orientation`` ('vertical','horizontal'), ``rwidth``, ``log``,
    ``stacked``.  Drawn on a fresh ``Figure().add_subplot()``.  Returns
    ``{"n", "bins", "patches", "xscale", "yscale"}``: ``n`` is a list (one
    dataset) or list of lists (several); ``bins`` the edges; ``patches`` is
    ALWAYS a list with one entry per dataset (matplotlib's single-dataset
    return is wrapped): for bar types a list of ``[x, y, width, height]``
    (``Rectangle.get_x/get_y/get_width/get_height``, i.e. lower-left corner;
    for horizontal bars x is the left/bottom value), for step types a list
    with one ``{"xy": Polygon.get_xy(), "closed": bool, "fill": bool}``.
    ``xscale``/``yscale`` are the axis scale names afterwards ('log' on the
    count axis when ``log`` is set).  Counts: when no ``range`` is given it
    defaults to (nanmin, nanmax) over all datasets (NaNs ignored, so all-NaN
    input reaches numpy without a range and raises); with several non-empty
    datasets the edges are ``histogram_bin_edges(concatenate(x), bins,
    range, concatenated weights)`` and every dataset is binned with them;
    ``density`` is passed to numpy only when not stacked.  ``stacked``
    (implied by 'barstacked'): tops are cumulated over datasets, and with
    density become ``tops / diff(bins) / tops[-1].sum()``.  ``cumulative``:
    cumsum along the bins (reversed for a negative number); with density
    ``cumsum(tops * diff(bins))``.  Bar geometry: ``totwidth = diff(bins)``;
    ``dr = clip(rwidth, 0, 1)`` if given, else 0.8 when several datasets and
    not stacked, else 1; unstacked 'bar': ``width = dr*totwidth/nx``,
    ``boffset = -0.5*dr*totwidth*(1 - 1/nx)``, advanced by ``width`` per
    dataset; stacked: ``width = dr*totwidth``, ``boffset = 0``; align 'mid'
    adds ``0.5*totwidth``, 'right' adds ``totwidth``.  Centre ``c =
    bins[:-1] + boffset``; vertical rect ``[c - width/2, bottom, width,
    height]``, horizontal ``[bottom, c - width/2, height, width]``; bottom
    defaults to 0; stacked: ``height = top - bottom`` and ``bottom = top``
    for the next dataset.  Step geometry follows matplotlib's construction
    in ``Axes.hist`` (perimeter of ``4*len(bins) - 3`` points: up the
    left edge, along the tops, back along the bottom/previous top; align
    'left'/'right' shifts x by -/+ ``0.5*(bins[1]-bins[0])``; horizontal swaps
    x and y); 'step' keeps the first ``2*len(bins)`` points (open, unfilled),
    'stepfilled' drops the last point and is closed (``get_xy()`` then
    repeats the first vertex at the end).

``mlab.window_hanning`` / ``mlab.window_none``
    args: ``x`` (1-D).  ``hanning(len(x)) * x`` / ``x``.  hanning has ``tol``.

``mlab.detrend``
    args: ``x`` (1-D or 2-D list or scalar), ``key`` (absent = None; None,
    'default', 'constant', 'mean' -> detrend_mean; 'linear' ->
    detrend_linear applied along ``axis`` (1-D slices); 'none' -> identity;
    anything else ValueError), ``axis`` (int or null).  Returns the array.
    Errors: axis out of bounds -> ValueError; 'linear' on 2-D with axis None
    -> TypeError.

``mlab.detrend_mean``
    args ``x``, ``axis``: ``x - mean(x, axis, keepdims)``; out-of-bounds axis
    -> ValueError.  Exact (inputs are 1/64-quantised).
``mlab.detrend_linear``
    args ``y`` (0-D or 1-D): ``y - (b*i + a)`` with ``i = 0..n-1``,
    ``b = C01/C00`` from ``np.cov(i, y, bias=1)``, ``a = mean(y) - b*mean(i)``;
    0-D input returns 0.0; 2-D raises ValueError.  ``tol``.
``mlab.detrend_none``
    args ``x``, ``axis``: returns ``x`` unchanged.
``mlab._stride_windows``
    (private helper) args ``x``, ``n``, ``noverlap``: 2-D array of shape
    ``(n, (len(x) - noverlap) // (n - noverlap))`` whose column j is
    ``x[j*step : j*step + n]`` with ``step = n - noverlap``.  Non-integer n or
    noverlap -> TypeError.

Spectral ops (``mlab.psd``, ``mlab.csd``, ``mlab.specgram``, ``mlab.cohere``)
    args: ``x`` (and ``y`` for csd/cohere), and any of ``NFFT`` (default 256),
    ``Fs`` (default 2), ``detrend`` (null/'none' -> none, 'mean'/'default'/
    'constant' -> mean, 'linear'; applied per segment), ``window``
    ('hanning' -> ``window_hanning``, 'none' -> ``window_none``, or an
    explicit list of length NFFT; default hanning), ``noverlap`` (default
    0; specgram default 128), ``pad_to`` (default NFFT), ``sides``
    ('default', 'onesided', 'twosided'; default is onesided for real and
    twosided for complex x), ``scale_by_freq`` (default True), ``mode``
    (specgram only: 'default', 'psd', 'magnitude', 'angle', 'phase',
    'complex').  ``cohere`` defaults: NFFT=256, Fs=2, detrend none, hanning,
    noverlap 0, and ``len(x) < 2*NFFT`` raises ValueError.  Algorithm
    (``mlab._spectral_helper``): inputs shorter than NFFT are zero padded to
    NFFT; segments of length NFFT start every ``NFFT - noverlap`` samples
    (count ``(len - NFFT)//step + 1``); each is detrended, multiplied by the
    window ``w`` and FFT'd with length ``pad_to`` (truncating when pad_to <
    NFFT); keep the first ``numFreqs`` bins (onesided: ``pad_to//2 + 1``
    (even) / ``(pad_to+1)//2`` (odd); twosided: pad_to).  psd/csd/cohere/
    specgram-psd use ``conj(X)*Y``, multiply bins ``[1:-1]`` (NFFT even) or
    ``[1:]`` (NFFT odd) by 2 when onesided (the parity test is on NFFT, not
    pad_to), then divide by ``Fs * sum(w**2)`` (scale_by_freq) or by
    ``sum(w)**2``; psd/csd average the segments; csd of inputs with
    different segment counts broadcasts when one count is 1 and raises
    ValueError otherwise.  magnitude ``|X|/sum(w)``, complex ``X/sum(w)``,
    angle ``angle(X)``, phase = angle then ``np.unwrap`` along frequency
    after the roll below.  ``freqs = fftfreq(pad_to, 1/Fs)[:numFreqs]``;
    twosided rolls freqs and values by ``-(pad_to//2)`` (even) or
    ``-((pad_to-1)//2 + 1)`` (odd) so they run from negative to positive;
    onesided with even pad_to flips the sign of the last (Nyquist) freq.
    ``t = arange(NFFT/2, len_padded - NFFT/2 + 1, NFFT - noverlap) / Fs``.
    Errors: ``not 0 <= noverlap < NFFT``, window length != NFFT, bad
    sides/mode -> ValueError.
    Returns: psd ``{"Pxx", "freqs"}`` (real); csd ``{"Pxy" (complex pairs),
    "freqs"}``; specgram ``{"spec" ([freq][segment], complex pairs for mode
    'complex'), "freqs", "t"}``; cohere ``{"Cxy", "freqs"}`` with ``Cxy =
    |Pxy|**2 / (Pxx*Pyy)``.  All ``tol``.  Test signals contain a positive
    offset and a ``(-1)**n`` component so real-input DC and Nyquist bins are
    positive real (angle 0, or rounding noise around 0 for non power-of-2
    FFT lengths, in the golden) rather than at the +/-pi branch cut.

Single spectra (``mlab.complex_spectrum``, ``mlab.magnitude_spectrum``,
``mlab.angle_spectrum``, ``mlab.phase_spectrum``)
    args: ``x``, and any of ``Fs``, ``window``, ``pad_to`` (default len(x)),
    ``sides``.  Same as the helper with ``NFFT = len(x)``, noverlap 0, detrend
    none, single segment.  Returns ``{"spectrum", "freqs"}`` (complex pairs
    for complex_spectrum).  Empty x -> ValueError (NFFT 0).  ``tol``.

``Axes.xcorr`` / ``Axes.acorr``
    args: ``x`` (and ``y`` for xcorr; acorr is ``xcorr(x, x)``), and any of
    ``normed`` (default True), ``detrend`` ('none' -> ``mlab.detrend_none``
    (default), 'mean' -> ``detrend_mean``, 'linear' -> ``detrend_linear``;
    applied to the whole input), ``usevlines`` (default True), ``maxlags``
    (default 10; null = ``len(x) - 1``).  ``c = np.correlate(x, y, 'full')``
    i.e. ``c[k] = sum_n x[n+k] * y[n]`` for lags ``k = -(N-1)..N-1``,
    normalised by ``sqrt(dot(x,x)*dot(y,y))`` when normed, then cut to
    ``-maxlags..maxlags``.  Returns ``{"lags", "c", "artist", "hline",
    "marker", "linestyle"}``: ``artist`` 'LineCollection' (usevlines;
    vertical segments from (lag, 0) to (lag, c); ``hline`` true = an axhline
    at 0 was added; marker/linestyle null) or 'Line2D' (``hline`` false,
    marker 'o', linestyle 'None').  Errors: different lengths, ``maxlags >=
    N`` or ``maxlags < 1`` -> ValueError.  ``tol``.
"""

from __future__ import annotations

import math
import warnings

import numpy as np
import pytest
from matplotlib import cbook, mlab
from matplotlib.figure import Figure
from numpy.lib import _histograms_impl as _np_hist

from spec_support import ProbeSet, rng

probes = ProbeSet("stats")

INF = float("inf")
NAN = float("nan")

# Tolerances.
TOL_KDE = 1e-9
TOL_FFT = 1e-9
TOL_CORR = 1e-11
TOL_COS = 1e-12
TOL_LIN = 1e-9


# --------------------------------------------------------------------------
# Deterministic data
# --------------------------------------------------------------------------


def _q64(values, scale=64):
    """Quantise to multiples of 1/scale (exact sums in binary64)."""
    return [round(float(v) * scale) / scale for v in values]


def _r(values, nd=4):
    return [round(float(v), nd) for v in values]


UNIF40 = _q64(rng(101).uniform(40, 0.0, 10.0))
NORM100 = _q64(rng(102).normal(100, 5.0, 2.0))
SKEW60 = _q64([-3.0 * math.log(1.0 - u) for u in rng(103).uniform(60)])
OUTL = _q64(rng(104).normal(30, 0.0, 1.0)) + [8.5, -7.25, 15.0, 6.0]
INTS50 = rng(105).integers(50, 0, 12).tolist()
SMALL5 = [1.5, -0.5, 4.0, 2.25, 9.0]
SMALL3 = [2.0, 7.5, 3.25]
CONST8 = [2.5] * 8
NORM30 = NORM100[:30]
NORM20B = _q64(rng(106).normal(20, 6.0, 1.5))
UNIF25 = UNIF40[:25]

# Spectral signals: offset + sines + (-1)^n component + noise.
def _real_signal(n, seed, f1=5.0 / 64, f2=0.19):
    k = np.arange(n)
    noise = rng(seed).normal(n)
    sig = (0.7 + np.sin(2 * np.pi * f1 * k) + 0.5 * np.cos(2 * np.pi * f2 * k + 0.3)
           + 0.4 * (-1.0) ** k + 0.3 * noise)
    return _r(sig)


def _complex_signal(n, seed):
    k = np.arange(n)
    nr = rng(seed).normal(n)
    ni = rng(seed + 1).normal(n)
    sig = (np.exp(2j * np.pi * 7 / 64 * k) + 0.4 * np.exp(-2j * np.pi * 3.2 / 64 * k)
           + 0.2 * (nr + 1j * ni) + (0.3 + 0.1j))
    return [[round(float(v.real), 4), round(float(v.imag), 4)] for v in sig]


S64 = _real_signal(64, 201)
S63 = S64[:63]
S128 = _real_signal(128, 202)
S256 = _real_signal(256, 203)
Y128 = _r(0.8 * np.roll(np.array(S128), 3) + 0.5 * rng(204).normal(128) - 0.2)
Y64 = Y128[:64]
C64 = _complex_signal(64, 205)
C64B = _complex_signal(64, 207)
HAMMING32 = _r(np.hamming(32), 8)
TRI16 = [float(min(i + 1, 16 - i)) for i in range(16)]

CORR20 = rng(301).integers(20, -5, 6).tolist()
CORR20B = rng(302).integers(20, -4, 7).tolist()
CORR32 = _r(rng(303).normal(32), 4)
CORR32B = _r(np.roll(np.array(CORR32), 2) * 0.7 + 0.3 * rng(304).normal(32), 4)


# --------------------------------------------------------------------------
# Adapter helpers
# --------------------------------------------------------------------------


def _quiet(fn):
    """Run ``fn`` with warnings suppressed (warnings are not part of the spec)."""

    def wrapper(**kw):
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            return fn(**kw)

    wrapper.__name__ = fn.__name__
    return wrapper


def _cplx(a):
    """Complex ndarray -> nested [re, im] pairs."""
    a = np.asarray(a)
    return np.stack([a.real, a.imag], axis=-1).tolist()


def _in_signal(x):
    """Spectral input: list of numbers (real) or of [re, im] pairs (complex)."""
    if len(x) and isinstance(x[0], (list, tuple)):
        a = np.asarray(x, float)
        return a[:, 0] + 1j * a[:, 1]
    return np.asarray(x, float)


def _datasets(X, as_array):
    return np.asarray(X) if as_array else X


def _spectral_kw(kw):
    out = dict(kw)
    if "window" in out:
        w = out["window"]
        if w == "hanning":
            out["window"] = mlab.window_hanning
        elif w == "none":
            out["window"] = mlab.window_none
        elif w is not None:
            out["window"] = np.asarray(w, float)
    for k in ("x", "y"):
        if k in out:
            out[k] = _in_signal(out[k])
    return out


# --------------------------------------------------------------------------
# boxplot_stats
# --------------------------------------------------------------------------


@_quiet
def _boxplot_stats(X, X_ndarray=False, **kw):
    stats = cbook.boxplot_stats(_datasets(X, X_ndarray), **kw)
    return [dict(s) for s in stats]


def bxp(**args):
    return probes.probe("cbook.boxplot_stats", _boxplot_stats, **args)


def test_boxplot_stats_basic():
    s = bxp(X=NORM100)[0]
    assert s["q1"] <= s["med"] <= s["q3"]
    assert s["whislo"] >= s["q1"] - 1.5 * s["iqr"]
    assert s["whishi"] <= s["q3"] + 1.5 * s["iqr"]
    s = bxp(X=OUTL)[0]
    assert set([8.5, -7.25, 15.0, 6.0]) <= set(s["fliers"].tolist())
    bxp(X=UNIF40)
    bxp(X=SKEW60)
    bxp(X=INTS50)
    bxp(X=SMALL5)
    bxp(X=SMALL3)
    s = bxp(X=[4.0])[0]
    assert s["q1"] == s["med"] == s["q3"] == s["whislo"] == s["whishi"] == 4.0
    bxp(X=[1.0, 2.0])
    bxp(X=[7, 3])
    bxp(X=[-1.5, 3.0, 3.0, 3.0, 12.0])


@pytest.mark.parametrize("whis", [0, 0.5, 1.5, 3.0, INF, [5, 95], [0, 100], [10.0, 90.0], [25, 75]])
def test_boxplot_stats_whis(whis):
    bxp(X=OUTL, whis=whis)
    bxp(X=SKEW60, whis=whis)


def test_boxplot_stats_whis_inf_covers_range():
    s = bxp(X=SMALL5, whis=INF)[0]
    assert s["whislo"] == -0.5 and s["whishi"] == 9.0 and len(s["fliers"]) == 0


def test_boxplot_stats_bad_whis():
    assert isinstance(bxp(X=SMALL5, whis="range"), ValueError)
    assert isinstance(bxp(X=SMALL5, whis="1.5"), ValueError)


def test_boxplot_stats_autorange():
    data = [1.0, 2.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 4.0, 9.0]
    off = bxp(X=data, autorange=False)[0]
    on = bxp(X=data, autorange=True)[0]
    assert off["iqr"] == 0 and off["whislo"] == 3.0 and off["whishi"] == 3.0
    assert on["whislo"] == 1.0 and on["whishi"] == 9.0
    bxp(X=CONST8, autorange=True)
    bxp(X=CONST8, autorange=False)
    bxp(X=CONST8, autorange=True, whis=[10, 90])
    bxp(X=NORM100, autorange=True)


def test_boxplot_stats_multiple_and_labels():
    res = bxp(X=[NORM30, SMALL5, [3.0], [], UNIF25], labels=["a", "b", "c", "d", "e"])
    assert [r["label"] for r in res] == ["a", "b", "c", "d", "e"]
    assert math.isnan(res[3]["med"])
    bxp(X=[NORM30, SMALL5])
    bxp(X=[[1, 2, 3, 4, 100], [5, 6, 7]], whis=1.0, labels=["x", "y"])
    assert isinstance(bxp(X=[NORM30, SMALL5], labels=["only one"]), ValueError)
    bxp(X=SMALL5, labels=["single"])
    assert isinstance(bxp(X=SMALL5, labels=["a", "b"]), ValueError)


def test_boxplot_stats_2d_and_edge():
    grid = [[float(r * 3 + c) * 0.5 + (r % 3) * c for c in range(3)] for r in range(9)]
    res = bxp(X=grid, X_ndarray=True)
    assert len(res) == 3  # columns
    res = bxp(X=grid)
    assert len(res) == 9  # rows as lists
    bxp(X=[[1.0], [2.0], [3.0]], X_ndarray=True)
    res = bxp(X=[], whis=1.5)
    assert len(res) == 1 and math.isnan(res[0]["mean"])
    res = bxp(X=[1.0, 2.0, NAN, 4.0])
    assert math.isnan(res[0]["med"]) and len(res[0]["fliers"]) == 0
    bxp(X=[1.0, INF, 2.0, 3.0])
    bxp(X=[-INF, 1.0, 2.0, 3.0, 4.0])
    bxp(X=[[[1.0, 2.0]], [3.0]])  # 3-D nested -> error


# --------------------------------------------------------------------------
# violin_stats / GaussianKDE
# --------------------------------------------------------------------------


def _violin_adapter(X, X_ndarray=False, **kw):
    if "method_name" in kw:
        kw["method"] = (kw.pop("method_name"), kw.pop("bw_method", "scott"))
    elif "bw_method" in kw:
        kw["method"] = ("GaussianKDE", kw.pop("bw_method"))
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        stats = cbook.violin_stats(_datasets(X, X_ndarray), **kw)
    return [dict(s) for s in stats]


def vio(**args):
    return probes.probe("cbook.violin_stats", _violin_adapter, tol=TOL_KDE, **args)


def test_violin_stats_basic():
    s = vio(X=NORM30, points=20)[0]
    assert len(s["coords"]) == 20 and s["coords"][0] == min(NORM30) and s["coords"][-1] == max(NORM30)
    vio(X=NORM100)  # default 100 points
    vio(X=SKEW60, points=25, quantiles=[0.1, 0.5, 0.9])
    vio(X=SMALL5, points=9)
    vio(X=[1.0, 2.0], points=5)
    vio(X=INTS50, points=12, quantiles=[0.25, 0.75])


@pytest.mark.parametrize("bw", ["scott", "silverman", 0.3, 1.0, 2, -0.5])
def test_violin_stats_bw(bw):
    vio(X=SKEW60, points=15, bw_method=bw)


def test_violin_stats_method_name():
    vio(X=SMALL5, points=5, method_name="GaussianKDE", bw_method="silverman")
    assert isinstance(vio(X=SMALL5, points=5, method_name="tophat"), ValueError)
    assert isinstance(vio(X=SMALL5, points=5, bw_method="bogus"), ValueError)


def test_violin_stats_edge():
    s = vio(X=CONST8, points=4)[0]
    assert s["vals"].tolist() == [1.0, 1.0, 1.0, 1.0]
    vio(X=[3.0], points=3)
    s = vio(X=[1.0, NAN, 2.0, INF, 4.0, -INF], points=6)[0]
    assert s["min"] == 1.0 and s["max"] == 4.0
    vio(X=[], points=5)
    vio(X=SMALL5, points=1)
    vio(X=SMALL5, points=0)
    vio(X=SMALL5, points=2, quantiles=[])
    vio(X=SMALL5, points=3, quantiles=[0.0, 1.0])
    assert isinstance(vio(X=SMALL5, points=3, quantiles=[1.5]), ValueError)


def test_violin_stats_multiple():
    res = vio(X=[NORM30, SKEW60[:20], [], CONST8], points=10,
              quantiles=[[0.25, 0.75], [0.5], [], [0.1]])
    assert len(res) == 4
    vio(X=[NORM30, SMALL5], points=8)
    grid = [[float(r) + 0.25 * c * c - (r % 4) * 0.5 for c in range(3)] for r in range(10)]
    vio(X=grid, X_ndarray=True, points=7, quantiles=[[0.5], [0.2, 0.8], [0.9]])
    assert isinstance(vio(X=[NORM30, SMALL5], points=8, quantiles=[[0.5]]), ValueError)


@_quiet
def _kde(dataset, points, **kw):
    kde = mlab.GaussianKDE(np.asarray(dataset, float), **kw)
    return {
        "dim": kde.dim,
        "num_dp": kde.num_dp,
        "factor": kde.factor,
        "covariance": kde.covariance,
        "inv_cov": kde.inv_cov,
        "norm_factor": kde.norm_factor,
        "values": kde.evaluate(np.asarray(points, float)),
    }


def kde(**args):
    return probes.probe("mlab.GaussianKDE", _kde, tol=TOL_KDE, **args)


def test_gaussian_kde_1d():
    pts = _r(np.linspace(-1.0, 12.0, 14), 6)
    r = kde(dataset=NORM30, points=pts)
    assert r["factor"] == pytest.approx(30 ** (-1 / 5))
    kde(dataset=NORM30, points=pts, bw_method="scott")
    r = kde(dataset=NORM30, points=pts, bw_method="silverman")
    assert r["factor"] == pytest.approx((30 * 3 / 4) ** (-1 / 5))
    kde(dataset=NORM30, points=pts, bw_method=0.25)
    kde(dataset=NORM30, points=pts, bw_method=None)
    kde(dataset=SKEW60, points=pts)
    kde(dataset=SMALL5, points=[0.0, 2.0, 4.5, 10.0, -3.0, 1.5, 2.25])  # more points than data
    kde(dataset=SMALL5, points=[2.0, 3.0])  # fewer points than data
    kde(dataset=SMALL5, points=2.0)  # scalar point
    kde(dataset=SMALL5, points=[])
    kde(dataset=[1.0, 2.0], points=[1.5])
    kde(dataset=INTS50, points=[0.0, 5.5, 11.0])


def test_gaussian_kde_2d():
    d2 = [NORM20B, _q64((np.array(NORM20B) * 0.5 + np.array(_q64(rng(107).normal(20))))[:20])]
    grid = [[-1.0, 0.0, 1.0, 2.0, 3.0] * 5, [float(v) for v in np.repeat([-2.0, 0.0, 1.0, 3.0, 5.0], 5)]]
    r = kde(dataset=d2, points=grid)  # 25 points >= 20 data
    assert r["dim"] == 2 and r["num_dp"] == 20
    kde(dataset=d2, points=[[0.0, 1.0], [1.0, 2.0]])  # 2 points < data
    kde(dataset=d2, points=[0.5, 1.0])  # flat list -> shape (1, 2) -> dimension mismatch
    kde(dataset=d2, points=[[0.5], [1.0]])
    kde(dataset=d2, points=grid, bw_method="silverman")
    kde(dataset=d2, points=grid, bw_method=0.6)
    kde(dataset=[[0.0, 1.0, 2.0, 0.5], [1.0, 0.0, 2.5, 1.0], [0.0, 0.5, 1.0, 3.0]],
        points=[[0.5], [1.0], [1.0]])  # 3-D data


def test_gaussian_kde_errors():
    assert type(kde(dataset=[1.0], points=[1.0])).__name__ == "ValueError"
    assert type(kde(dataset=[], points=[1.0])).__name__ == "ValueError"
    assert type(kde(dataset=CONST8, points=[1.0])).__name__ == "LinAlgError"
    assert type(kde(dataset=[[1.0, 2.0, 3.0], [2.0, 4.0, 6.0]], points=[[1.0], [1.0]])).__name__ == "LinAlgError"
    assert type(kde(dataset=SMALL5, points=[1.0], bw_method="bogus")).__name__ == "ValueError"
    assert type(kde(dataset=SMALL5, points=[[1.0], [2.0]])).__name__ == "ValueError"


# --------------------------------------------------------------------------
# numpy histogram binning
# --------------------------------------------------------------------------

ESTIMATORS = ["auto", "fd", "doane", "scott", "stone", "rice", "sturges", "sqrt"]
HIST_DATA = {
    "uniform40": UNIF40,
    "normal100": NORM100,
    "skewed60": SKEW60,
    "outliers34": OUTL,
    "small5": SMALL5,
    "ints50": INTS50,
    "const8": CONST8,
}


@_quiet
def _histogram(a, **kw):
    if kw.get("weights") is not None:
        kw["weights"] = np.asarray(kw["weights"])
    hist, edges = np.histogram(np.asarray(a), **kw)
    return {"hist": hist, "bin_edges": edges}


@_quiet
def _histogram_bin_edges(a, **kw):
    if kw.get("weights") is not None:
        kw["weights"] = np.asarray(kw["weights"])
    return np.histogram_bin_edges(np.asarray(a), **kw)


@_quiet
def _histogram_bin_width(a, estimator):
    a = np.asarray(a)
    lo, hi = _np_hist._get_outer_edges(a, None)
    return _np_hist._hist_bin_selectors[estimator](a, (lo, hi))


def hist_np(**args):
    return probes.probe("numpy.histogram", _histogram, **args)


def edges_np(**args):
    return probes.probe("numpy.histogram_bin_edges", _histogram_bin_edges, **args)


@pytest.mark.parametrize("name", list(HIST_DATA))
@pytest.mark.parametrize("estimator", ESTIMATORS)
def test_histogram_estimators(name, estimator):
    data = HIST_DATA[name]
    edges = edges_np(a=data, bins=estimator)
    assert edges[0] == min(data) - (0.5 if name == "const8" else 0)
    if name in ("normal100", "skewed60", "ints50", "small5", "outliers34"):
        probes.probe("numpy.histogram_bin_width", _histogram_bin_width, tol=1e-12,
                     a=data, estimator=estimator)
    if name in ("normal100", "ints50", "skewed60"):
        r = hist_np(a=data, bins=estimator)
        assert r["hist"].sum() == len(data)


@pytest.mark.parametrize("estimator", ESTIMATORS)
def test_histogram_estimators_with_range(estimator):
    edges_np(a=NORM100, bins=estimator, range=[3.0, 7.0])
    hist_np(a=SKEW60, bins=estimator, range=[0.0, 5.0])
    edges_np(a=[], bins=estimator)
    edges_np(a=[4.0], bins=estimator)
    edges_np(a=[5], bins=estimator)
    edges_np(a=[1.0, 2.0], bins=estimator)
    edges_np(a=[1.0, 1.0, 1.0, 9.0], bins=estimator)
    hist_np(a=NORM100, bins=estimator, range=[20.0, 30.0])  # no data in range


def test_histogram_stone_large_n():
    # n large enough that sqrt(n) bound matters is impractical; probe the
    # 100-bin ceiling with very concentrated data instead.
    data = _q64(np.concatenate([np.linspace(0, 1, 60), [50.0, 100.0]]), 256)
    edges_np(a=data, bins="stone")
    probes.probe("numpy.histogram_bin_width", _histogram_bin_width, tol=1e-12, a=data, estimator="stone")


def test_histogram_int_and_sequence_bins():
    for data in (UNIF40, INTS50, SMALL5):
        for bins in (1, 3, 7, 10, 20):
            hist_np(a=data, bins=bins)
    r = hist_np(a=[0, 1, 2, 3, 4, 5], bins=5)
    assert r["hist"].tolist() == [1, 1, 1, 1, 2]  # last bin closed
    hist_np(a=[0.0, 1.0, 2.0, 3.0, 4.0, 5.0], bins=[0, 1, 2, 3, 4, 5])
    hist_np(a=[0.0, 1.0, 2.0, 3.0, 4.0, 5.0], bins=[0.0, 2.5, 5.0])
    hist_np(a=UNIF40, bins=[0.0, 0.5, 2.0, 2.25, 6.0, 10.0])
    hist_np(a=UNIF40, bins=[2.0, 4.0, 6.0])  # data outside edges dropped
    hist_np(a=UNIF40, bins=[1.0, 1.0, 3.0])  # zero-width bin allowed
    hist_np(a=INTS50, bins=[0, 3, 6, 12])
    hist_np(a=UNIF40, bins=[5.0])  # single edge -> no bins
    hist_np(a=UNIF40, bins=4, range=[2.0, 6.0])
    hist_np(a=UNIF40, bins=4, range=[-5.0, 15.0])
    hist_np(a=UNIF40, bins=4, range=[3.0, 3.0])  # expanded by 0.5
    hist_np(a=INTS50, bins=6, range=[0, 12])
    hist_np(a=[], bins=4)
    hist_np(a=[], bins=[0.0, 1.0, 2.0])
    hist_np(a=[7.0], bins=3)
    hist_np(a=[7], bins=3)
    hist_np(a=[True, False, True, True], bins=2)
    hist_np(a=[-3.0, -1.0, 0.0, 2.0], bins=2)
    hist_np(a=[1.0, 2.0, NAN, 3.0], bins=3, range=[1.0, 3.0])
    hist_np(a=[1.0, 2.0, INF, 3.0], bins=3, range=[1.0, 3.0])
    hist_np(a=[1.0, 2.0, NAN, 3.0], bins=[0.0, 2.0, 4.0])
    edges_np(a=UNIF40, bins=5)
    edges_np(a=UNIF40, bins=5, range=[0.0, 1.0])
    edges_np(a=UNIF40, bins=[0.0, 3.0, 4.0])
    edges_np(a=[0.1, 0.7], bins=3)
    edges_np(a=[-1e-3, 1e-3], bins=4)
    edges_np(a=[1e15, 1e15 + 64.0], bins=4)


def test_histogram_density_and_weights():
    w = _q64(rng(110).uniform(40, 0.0, 3.0), 4)
    hist_np(a=UNIF40, bins=8, density=True)
    hist_np(a=UNIF40, bins=[0.0, 1.0, 5.0, 10.0], density=True)
    hist_np(a=UNIF40, bins=8, weights=w)
    hist_np(a=UNIF40, bins=[0.0, 1.0, 5.0, 10.0], weights=w)
    hist_np(a=UNIF40, bins=8, weights=w, density=True)
    hist_np(a=UNIF40, bins=8, weights=[-x for x in w])
    hist_np(a=INTS50, bins=5, weights=[1] * 25 + [2] * 25)
    hist_np(a=SMALL5, bins=3, weights=[0.25, 0.5, 1.0, 2.0, 4.0], range=[0.0, 5.0])
    hist_np(a=[], bins=3, density=True)
    hist_np(a=SMALL5, bins=3, weights=[0.0] * 5, density=True)
    edges_np(a=SMALL5, bins=3, weights=[1.0] * 5)


def test_histogram_errors():
    def err(r):
        return type(r).__name__

    assert err(hist_np(a=[1.0, NAN, 3.0], bins=3)) == "ValueError"
    assert err(hist_np(a=[1.0, INF, 3.0], bins=3)) == "ValueError"
    assert err(hist_np(a=[1.0, -INF], bins="auto")) == "ValueError"
    assert err(hist_np(a=SMALL5, bins=0)) == "ValueError"
    assert err(hist_np(a=SMALL5, bins=-2)) == "ValueError"
    assert err(hist_np(a=SMALL5, bins=2.5)) == "TypeError"
    assert err(hist_np(a=SMALL5, bins="bogus")) == "ValueError"
    assert err(hist_np(a=SMALL5, bins=[3.0, 1.0, 2.0])) == "ValueError"
    assert err(hist_np(a=SMALL5, bins=[[0.0, 1.0], [2.0, 3.0]])) == "ValueError"
    assert err(hist_np(a=SMALL5, bins=3, range=[5.0, 1.0])) == "ValueError"
    assert err(hist_np(a=SMALL5, bins=3, range=[0.0, INF])) == "ValueError"
    assert err(hist_np(a=SMALL5, bins=3, range=[NAN, 1.0])) == "ValueError"
    assert err(hist_np(a=SMALL5, bins=3, weights=[1.0, 2.0])) == "ValueError"
    assert err(hist_np(a=SMALL5, bins="auto", weights=[1.0] * 5)) == "TypeError"
    assert err(hist_np(a=[1.0, 1.0 + 2.0 ** -52], bins=10)) == "ValueError"  # too many bins
    assert err(edges_np(a=SMALL5, bins="sturges", weights=[1.0] * 5)) == "TypeError"


# --------------------------------------------------------------------------
# Axes.hist
# --------------------------------------------------------------------------


def _patch_desc(p):
    from matplotlib.patches import Polygon, Rectangle

    if isinstance(p, Rectangle):
        return [p.get_x(), p.get_y(), p.get_width(), p.get_height()]
    assert isinstance(p, Polygon)
    return {"xy": p.get_xy(), "closed": p.get_closed(), "fill": p.get_fill()}


@_quiet
def _axes_hist(x, x_ndarray=False, **kw):
    ax = Figure().add_subplot()
    n, bins, patches = ax.hist(_datasets(x, x_ndarray), **kw)
    n = np.asarray(n)
    if n.ndim == 1:
        patches = [patches]
    return {
        "n": n,
        "bins": bins,
        "patches": [[_patch_desc(p) for p in group] for group in patches],
        "xscale": ax.get_xscale(),
        "yscale": ax.get_yscale(),
    }


def ahist(**args):
    return probes.probe("Axes.hist", _axes_hist, **args)


def test_hist_single_dataset():
    r = ahist(x=NORM30)
    assert len(r["n"]) == 10 and r["n"].sum() == 30
    assert r["patches"][0][0][2] == pytest.approx(r["bins"][1] - r["bins"][0])
    ahist(x=NORM30, bins=5)
    ahist(x=NORM30, bins="auto")
    ahist(x=NORM30, bins="fd")
    ahist(x=NORM30, bins="sturges")
    ahist(x=NORM30, bins="stone")
    ahist(x=NORM30, bins=[2.0, 4.0, 5.0, 6.0, 9.0])
    ahist(x=NORM30, bins=6, range=[3.0, 7.0])
    ahist(x=NORM30, bins=6, density=True)
    ahist(x=NORM30, bins=6, weights=_q64(rng(111).uniform(30, 0.0, 2.0), 8))
    ahist(x=INTS50, bins=6)
    ahist(x=INTS50, bins="auto")
    ahist(x=[3.0], bins=4)
    ahist(x=3.0, bins=2)  # scalar input
    ahist(x=[], bins=4)
    ahist(x=[])
    ahist(x=[1.0, NAN, 2.0, 2.5, NAN, 4.0], bins=3)
    assert isinstance(ahist(x=[1.0, INF, 2.0], bins=3), ValueError)
    ahist(x=[NAN, NAN], bins=3)


@pytest.mark.parametrize("cumulative", [True, -1, False, 1, -0.5])
@pytest.mark.parametrize("density", [False, True])
def test_hist_cumulative(cumulative, density):
    ahist(x=UNIF25, bins=5, cumulative=cumulative, density=density)


@pytest.mark.parametrize("histtype", ["bar", "barstacked", "step", "stepfilled"])
@pytest.mark.parametrize("orientation", ["vertical", "horizontal"])
def test_hist_histtype_orientation(histtype, orientation):
    ahist(x=UNIF25, bins=4, histtype=histtype, orientation=orientation)
    ahist(x=[UNIF25, NORM20B], bins=4, histtype=histtype, orientation=orientation)
    ahist(x=[UNIF25, NORM20B], bins=4, histtype=histtype, orientation=orientation, stacked=True)


@pytest.mark.parametrize("histtype", ["bar", "step", "stepfilled"])
def test_hist_bottom_log_align(histtype):
    ahist(x=UNIF25, bins=4, histtype=histtype, bottom=2.0)
    ahist(x=UNIF25, bins=4, histtype=histtype, bottom=[1.0, 0.0, 3.0, 0.5])
    ahist(x=UNIF25, bins=4, histtype=histtype, log=True)
    ahist(x=UNIF25, bins=4, histtype=histtype, log=True, orientation="horizontal")
    ahist(x=UNIF25, bins=4, histtype=histtype, align="left")
    ahist(x=UNIF25, bins=4, histtype=histtype, align="right")
    ahist(x=UNIF25, bins=[0.0, 1.0, 5.0, 10.0], histtype=histtype, align="left")
    ahist(x=[UNIF25, NORM20B], bins=4, histtype=histtype, stacked=True, bottom=1.0)


def test_hist_multiple():
    r = ahist(x=[UNIF25, NORM20B, SMALL5], bins=5)
    assert len(r["n"]) == 3 and len(r["patches"]) == 3
    ahist(x=[UNIF25, NORM20B, SMALL5], bins=5, rwidth=0.5)
    ahist(x=[UNIF25, NORM20B, SMALL5], bins=5, histtype="barstacked")
    ahist(x=[UNIF25, NORM20B, SMALL5], bins=5, histtype="barstacked", rwidth=0.6)
    ahist(x=[UNIF25, NORM20B], bins=5, stacked=True, density=True)
    ahist(x=[UNIF25, NORM20B], bins=5, stacked=True, density=True, histtype="step")
    ahist(x=[UNIF25, NORM20B], bins=5, stacked=True, cumulative=True)
    ahist(x=[UNIF25, NORM20B], bins=5, stacked=True, cumulative=-1, density=True)
    ahist(x=[UNIF25, NORM20B], bins=5, density=True)
    ahist(x=[UNIF25, NORM20B], bins="auto")
    ahist(x=[UNIF25, NORM20B], bins="doane")
    ahist(x=[UNIF25, NORM20B], bins=[0.0, 4.0, 6.0, 12.0])
    ahist(x=[UNIF25, NORM20B], bins=5, range=[2.0, 8.0])
    ahist(x=[UNIF25, []], bins=3)
    ahist(x=[[], []], bins=3)
    ahist(x=[UNIF25[:5], [NAN, 4.0, 5.0]], bins=3)
    w = [_q64(rng(112).uniform(25, 0.0, 2.0), 4), [0.5] * 20]
    ahist(x=[UNIF25, NORM20B], bins=5, weights=w)
    ahist(x=[UNIF25, NORM20B], bins=5, weights=w, stacked=True, density=True)
    assert isinstance(ahist(x=[UNIF25, NORM20B], bins="auto", weights=w), TypeError)
    grid = [[float(r) * 0.75 + c * 1.5 - (r % 3) for c in range(3)] for r in range(12)]
    r = ahist(x=grid, x_ndarray=True, bins=4)
    assert len(r["n"]) == 3
    ahist(x=grid, x_ndarray=True, bins=4, histtype="step", stacked=True)


def test_hist_errors():
    assert isinstance(ahist(x=SMALL5, histtype="bogus"), ValueError)
    assert isinstance(ahist(x=SMALL5, orientation="diagonal"), ValueError)
    assert isinstance(ahist(x=SMALL5, align="top"), ValueError)
    assert isinstance(ahist(x=SMALL5, weights=[1.0, 2.0]), ValueError)
    assert isinstance(ahist(x=[SMALL5, SMALL3], weights=[[1.0] * 5]), ValueError)
    assert isinstance(ahist(x=SMALL5, bins=3, range=[4.0, 1.0]), ValueError)
    assert isinstance(ahist(x=SMALL5, bins=0), ValueError)


# --------------------------------------------------------------------------
# Windows and detrending
# --------------------------------------------------------------------------


def test_windows():
    for x in ([1.0] * 8, [1.0] * 7, [2.0, -1.0, 0.5, 3.0, 4.0], [5.0], [], [1.0, 1.0], _r(np.arange(16) * 0.25 - 1.0)):
        probes.probe("mlab.window_hanning", lambda x: mlab.window_hanning(np.asarray(x, float)), tol=TOL_COS, x=x)
        r = probes.probe("mlab.window_none", lambda x: mlab.window_none(np.asarray(x, float)), x=x)
        assert list(r) == x
    h = mlab.window_hanning(np.ones(9))
    assert h[0] == 0.0 and h[4] == 1.0


DET1 = _q64([3.0, 1.5, 4.0, 1.0, 5.5, 9.0, 2.5, 6.0, 5.0, 3.5, 5.0, 8.0])
DET2 = [[float((r * 7 + c * 3) % 11) * 0.5 + r * 0.25 for c in range(4)] for r in range(6)]
LINEAR = [2.0 + 0.5 * i for i in range(10)]


@_quiet
def _detrend(x, **kw):
    return mlab.detrend(np.asarray(x, float), **kw)


@_quiet
def _detrend_mean(x, **kw):
    return mlab.detrend_mean(np.asarray(x, float), **kw)


@_quiet
def _detrend_none(x, **kw):
    return mlab.detrend_none(np.asarray(x, float), **kw)


@_quiet
def _detrend_linear(y):
    return mlab.detrend_linear(np.asarray(y, float))


def test_detrend_dispatch():
    for key in (None, "default", "constant", "mean"):
        r = probes.probe("mlab.detrend", _detrend, x=DET1, key=key)
        assert abs(float(np.sum(r))) < 1e-12
    probes.probe("mlab.detrend", _detrend, x=DET1)
    probes.probe("mlab.detrend", _detrend, tol=TOL_LIN, x=DET1, key="linear")
    probes.probe("mlab.detrend", _detrend, x=DET1, key="none")
    assert isinstance(probes.probe("mlab.detrend", _detrend, x=DET1, key="bogus"), ValueError)
    for axis in (None, 0, 1):
        probes.probe("mlab.detrend", _detrend, x=DET2, key="mean", axis=axis)
        probes.probe("mlab.detrend", _detrend, x=DET2, key="none", axis=axis)
    probes.probe("mlab.detrend", _detrend, tol=TOL_LIN, x=DET2, key="linear", axis=0)
    probes.probe("mlab.detrend", _detrend, tol=TOL_LIN, x=DET2, key="linear", axis=1)
    assert isinstance(probes.probe("mlab.detrend", _detrend, tol=TOL_LIN, x=DET2, key="linear"), TypeError)
    assert isinstance(probes.probe("mlab.detrend", _detrend, x=DET2, key="mean", axis=2), ValueError)
    assert isinstance(probes.probe("mlab.detrend", _detrend, x=DET1, key="mean", axis=1), ValueError)
    probes.probe("mlab.detrend", _detrend, x=DET1, key="mean", axis=0)
    probes.probe("mlab.detrend", _detrend, tol=TOL_LIN, x=DET1, key="linear", axis=0)
    probes.probe("mlab.detrend", _detrend, x=4.5, key="mean")
    probes.probe("mlab.detrend", _detrend, tol=TOL_LIN, x=4.5, key="linear")
    probes.probe("mlab.detrend", _detrend, x=[], key="mean")


def test_detrend_functions():
    probes.probe("mlab.detrend_mean", _detrend_mean, x=DET1)
    probes.probe("mlab.detrend_mean", _detrend_mean, x=CONST8)
    probes.probe("mlab.detrend_mean", _detrend_mean, x=[7.0])
    probes.probe("mlab.detrend_mean", _detrend_mean, x=DET1, axis=0)
    for axis in (None, 0, 1):
        probes.probe("mlab.detrend_mean", _detrend_mean, x=DET2, axis=axis)
    assert isinstance(probes.probe("mlab.detrend_mean", _detrend_mean, x=DET1, axis=1), ValueError)
    probes.probe("mlab.detrend_mean", _detrend_mean, x=[1.0, NAN, 3.0])
    probes.probe("mlab.detrend_none", _detrend_none, x=DET1)
    probes.probe("mlab.detrend_none", _detrend_none, x=DET2, axis=1)
    probes.probe("mlab.detrend_linear", _detrend_linear, tol=TOL_LIN, y=DET1)
    probes.probe("mlab.detrend_linear", _detrend_linear, tol=TOL_LIN, y=_q64(NORM30))
    probes.probe("mlab.detrend_linear", _detrend_linear, tol=TOL_LIN, y=[1.0, 4.0])  # 2 points -> zeros
    probes.probe("mlab.detrend_linear", _detrend_linear, tol=TOL_LIN, y=[1.0, 5.0, 2.0])
    probes.probe("mlab.detrend_linear", _detrend_linear, tol=TOL_LIN, y=[3.0, 3.0, 3.0, 3.0])
    probes.probe("mlab.detrend_linear", _detrend_linear, tol=TOL_LIN, y=3.5)
    probes.probe("mlab.detrend_linear", _detrend_linear, tol=TOL_LIN,
                 y=[v + (1.0 if i % 2 else -1.0) for i, v in enumerate(LINEAR)])
    assert isinstance(probes.probe("mlab.detrend_linear", _detrend_linear, tol=TOL_LIN, y=DET2), ValueError)
    # An exactly linear signal detrends to (numerically) zero; not recorded
    # because the golden would only hold rounding noise.
    assert np.allclose(mlab.detrend_linear(np.asarray(LINEAR)), 0.0, atol=1e-12)
    probes.probe("mlab.detrend_linear", _detrend_linear, tol=TOL_LIN, y=[1.0])


@_quiet
def _stride(x, n, noverlap=0):
    return mlab._stride_windows(np.asarray(x, float), n, noverlap)


def test_stride_windows():
    x = [float(i) for i in range(10)]
    r = probes.probe("mlab._stride_windows", _stride, x=x, n=4)
    assert r.shape == (4, 2)
    r = probes.probe("mlab._stride_windows", _stride, x=x, n=4, noverlap=2)
    assert r.shape == (4, 4) and r[:, 1].tolist() == [2.0, 3.0, 4.0, 5.0]
    probes.probe("mlab._stride_windows", _stride, x=x, n=3, noverlap=1)
    probes.probe("mlab._stride_windows", _stride, x=x, n=10)
    probes.probe("mlab._stride_windows", _stride, x=x, n=5, noverlap=4)
    assert isinstance(probes.probe("mlab._stride_windows", _stride, x=x, n=4.0), TypeError)
    assert isinstance(probes.probe("mlab._stride_windows", _stride, x=x, n=4, noverlap=1.5), TypeError)


# --------------------------------------------------------------------------
# Spectral estimates
# --------------------------------------------------------------------------


@_quiet
def _psd(**kw):
    pxx, freqs = mlab.psd(**_spectral_kw(kw))
    return {"Pxx": pxx, "freqs": freqs}


@_quiet
def _csd(**kw):
    pxy, freqs = mlab.csd(**_spectral_kw(kw))
    return {"Pxy": _cplx(pxy), "freqs": freqs}


@_quiet
def _specgram(**kw):
    spec, freqs, t = mlab.specgram(**_spectral_kw(kw))
    spec = _cplx(spec) if np.iscomplexobj(spec) else spec
    return {"spec": spec, "freqs": freqs, "t": t}


@_quiet
def _cohere(**kw):
    cxy, freqs = mlab.cohere(**_spectral_kw(kw))
    return {"Cxy": cxy, "freqs": freqs}


def _single(name):
    fn = getattr(mlab, name)

    @_quiet
    def adapter(**kw):
        spec, freqs = fn(**_spectral_kw(kw))
        spec = _cplx(spec) if np.iscomplexobj(spec) else spec
        return {"spectrum": spec, "freqs": freqs}

    adapter.__name__ = f"_{name}"
    return adapter


def psd(**args):
    return probes.probe("mlab.psd", _psd, tol=TOL_FFT, **args)


def csd(**args):
    return probes.probe("mlab.csd", _csd, tol=TOL_FFT, **args)


def specgram(**args):
    return probes.probe("mlab.specgram", _specgram, tol=TOL_FFT, **args)


def cohere(**args):
    return probes.probe("mlab.cohere", _cohere, tol=TOL_FFT, **args)


def test_psd():
    r = psd(x=S128, NFFT=32)
    assert len(r["Pxx"]) == 17 and r["freqs"][-1] == 1.0
    assert np.all(r["Pxx"] > 0)
    psd(x=S128, NFFT=32, noverlap=16)
    psd(x=S128, NFFT=32, noverlap=31)
    psd(x=S128, NFFT=32, noverlap=8, Fs=100.0)
    psd(x=S128, NFFT=32, window="none")
    psd(x=S128, NFFT=32, window="hanning")
    psd(x=S128, NFFT=32, window=HAMMING32)
    psd(x=S128, NFFT=32, window=None)
    for det in (None, "none", "mean", "default", "constant", "linear"):
        psd(x=S128, NFFT=32, detrend=det)
    psd(x=S128, NFFT=32, pad_to=64)
    psd(x=S128, NFFT=32, pad_to=33)  # odd pad_to, even NFFT
    psd(x=S128, NFFT=31, pad_to=32)  # even pad_to, odd NFFT
    psd(x=S128, NFFT=31)
    psd(x=S128, NFFT=32, pad_to=16)  # pad_to < NFFT truncates
    for sides in ("default", "onesided", "twosided"):
        psd(x=S128, NFFT=32, sides=sides)
    psd(x=S128, NFFT=31, sides="twosided")
    psd(x=S128, NFFT=32, scale_by_freq=False)
    psd(x=S128, NFFT=32, scale_by_freq=True)
    psd(x=S128, NFFT=32, scale_by_freq=False, sides="twosided", Fs=8.0)
    psd(x=S64)  # NFFT 256 default -> zero padded
    psd(x=S256)
    psd(x=S64, NFFT=64)
    psd(x=S64, NFFT=16, noverlap=8, detrend="linear", window="none", pad_to=20, Fs=3.0)
    psd(x=[], NFFT=8)
    psd(x=[1.0, 2.0, 3.0], NFFT=8)


def test_psd_complex():
    r = psd(x=C64, NFFT=16)
    assert len(r["Pxx"]) == 16 and r["freqs"][0] < 0
    psd(x=C64, NFFT=16, noverlap=8)
    psd(x=C64, NFFT=15)
    psd(x=C64, NFFT=16, sides="onesided")
    psd(x=C64, NFFT=16, sides="twosided", detrend="mean", window="none")
    psd(x=C64, NFFT=16, pad_to=32, scale_by_freq=False)


def test_psd_errors():
    assert isinstance(psd(x=S128, NFFT=32, noverlap=32), ValueError)
    assert isinstance(psd(x=S128, NFFT=32, noverlap=40), ValueError)
    assert isinstance(psd(x=S128, NFFT=32, noverlap=-1), ValueError)
    assert isinstance(psd(x=S128, NFFT=32, window=TRI16), ValueError)
    assert isinstance(psd(x=S128, NFFT=32, sides="bogus"), ValueError)
    assert isinstance(psd(x=S128, NFFT=32, detrend="bogus"), ValueError)


def test_csd():
    r = csd(x=S128, y=Y128, NFFT=32)
    assert len(r["Pxy"]) == 17
    csd(x=S128, y=Y128, NFFT=32, noverlap=16)
    csd(x=S128, y=Y128, NFFT=32, detrend="mean")
    csd(x=S128, y=Y128, NFFT=32, detrend="linear", window="none")
    csd(x=S128, y=Y128, NFFT=32, sides="twosided")
    csd(x=S128, y=Y128, NFFT=31, pad_to=40, Fs=10.0)
    csd(x=S128, y=Y128, NFFT=32, scale_by_freq=False)
    csd(x=S128, y=Y128, NFFT=32, window=HAMMING32)
    csd(x=S128, y=S128, NFFT=32)  # separate copies -> cross path, imag 0
    csd(x=Y128, y=S128, NFFT=32)  # conjugate of (S128, Y128)
    csd(x=C64, y=C64B, NFFT=16)
    csd(x=C64, y=C64B, NFFT=16, sides="onesided")
    csd(x=S64, y=C64, NFFT=16)
    csd(x=S64, y=Y64)  # default NFFT 256, both zero-padded
    csd(x=S64, y=Y128[:40], NFFT=32)  # 2 vs 1 segments: broadcasts
    assert isinstance(csd(x=S64, y=Y128[:96], NFFT=32), ValueError)  # 2 vs 3
    assert isinstance(csd(x=S128, y=Y128, NFFT=32, noverlap=32), ValueError)


SPEC_MODES = ["default", "psd", "magnitude", "angle", "phase", "complex"]


@pytest.mark.parametrize("mode", SPEC_MODES)
def test_specgram_modes(mode):
    r = specgram(x=S128, NFFT=32, noverlap=16, mode=mode)
    assert len(r["t"]) == 7 and r["t"][0] == 8.0
    specgram(x=S128, NFFT=32, noverlap=0, Fs=10.0, mode=mode, sides="twosided")
    specgram(x=C64, NFFT=16, noverlap=8, mode=mode)
    specgram(x=S64, NFFT=15, noverlap=5, pad_to=20, mode=mode, window="none")


def test_specgram_options():
    r = specgram(x=S128, NFFT=32, noverlap=16)
    assert np.all(r["spec"] > 0)
    specgram(x=S128, NFFT=32, noverlap=16, detrend="mean")
    specgram(x=S128, NFFT=32, noverlap=16, detrend="linear")
    specgram(x=S128, NFFT=32, noverlap=16, scale_by_freq=False)
    specgram(x=S128, NFFT=32, noverlap=16, window=HAMMING32, Fs=1000.0)
    specgram(x=S128, NFFT=32, noverlap=16, pad_to=64)
    specgram(x=S128, NFFT=32, noverlap=24, sides="onesided")
    specgram(x=S128, NFFT=32, noverlap=16, mode="magnitude", detrend="mean", scale_by_freq=True)
    specgram(x=S256)  # all defaults: NFFT 256, noverlap 128 -> 1 segment
    specgram(x=S64, noverlap=0)  # NFFT 256 > len -> zero padded, 1 segment
    specgram(x=S64, NFFT=64, noverlap=0, mode="complex")
    assert isinstance(specgram(x=S128, NFFT=32), ValueError)  # default noverlap 128 >= NFFT
    assert isinstance(specgram(x=S128, NFFT=32, noverlap=16, mode="bogus"), ValueError)
    assert isinstance(specgram(x=S128, NFFT=32, noverlap=16, sides="bogus"), ValueError)


def test_cohere():
    r = cohere(x=S128, y=Y128, NFFT=32)
    assert np.all((r["Cxy"] >= 0) & (r["Cxy"] <= 1 + 1e-12))
    cohere(x=S128, y=Y128, NFFT=32, noverlap=16)
    cohere(x=S128, y=Y128, NFFT=16, noverlap=8, Fs=10.0)
    cohere(x=S128, y=Y128, NFFT=32, detrend="mean")
    cohere(x=S128, y=Y128, NFFT=32, detrend="linear", window="none")
    cohere(x=S128, y=Y128, NFFT=32, window=HAMMING32)
    cohere(x=S128, y=Y128, NFFT=32, pad_to=48)
    cohere(x=S128, y=Y128, NFFT=32, sides="twosided")
    cohere(x=S128, y=Y128, NFFT=32, scale_by_freq=False)
    r = cohere(x=S128, y=S128, NFFT=32)
    assert np.allclose(r["Cxy"], 1.0)
    cohere(x=S128, y=Y128, NFFT=64)  # exactly 2 segments
    cohere(x=C64, y=C64B, NFFT=16)
    assert isinstance(cohere(x=S128, y=Y128), ValueError)  # default NFFT 256: too short
    assert isinstance(cohere(x=S128, y=Y128, NFFT=65), ValueError)


SINGLE = ["complex_spectrum", "magnitude_spectrum", "angle_spectrum", "phase_spectrum"]


@pytest.mark.parametrize("name", SINGLE)
def test_single_spectra(name):
    op = f"mlab.{name}"
    fn = _single(name)

    def p(**args):
        return probes.probe(op, fn, tol=TOL_FFT, **args)

    r = p(x=S64)
    assert len(r["freqs"]) == 33
    p(x=S63)
    p(x=S64, Fs=10.0)
    p(x=S64, window="none")
    p(x=S64, window="hanning")
    p(x=S64, window=_r(np.hamming(64), 8))
    p(x=S64, pad_to=128)
    p(x=S64, pad_to=81)
    p(x=S64, pad_to=40)  # truncation
    p(x=S64, sides="twosided")
    p(x=S63, sides="twosided")
    p(x=S64, sides="onesided")
    p(x=C64)
    p(x=C64, sides="onesided")
    p(x=C64, pad_to=65, Fs=4.0)
    p(x=TRI16)
    p(x=[2.0])
    assert isinstance(p(x=[]), ValueError)
    assert isinstance(p(x=S64, sides="bogus"), ValueError)
    assert isinstance(p(x=S64, window=TRI16), ValueError)


def test_single_spectra_consistency():
    c = mlab.complex_spectrum(np.asarray(S64))[0]
    m = mlab.magnitude_spectrum(np.asarray(S64))[0]
    a = mlab.angle_spectrum(np.asarray(S64))[0]
    assert np.allclose(np.abs(c), m) and np.allclose(np.angle(c), a)
    assert a[0] == 0.0 and a[-1] == 0.0  # DC & Nyquist positive real


# --------------------------------------------------------------------------
# acorr / xcorr
# --------------------------------------------------------------------------

_DETRENDS = {"none": mlab.detrend_none, "mean": mlab.detrend_mean, "linear": mlab.detrend_linear}


def _corr_result(ret):
    lags, c, a, b = ret
    from matplotlib.collections import LineCollection

    if isinstance(a, LineCollection):
        if np.all(np.isfinite(c)):  # sanity check of the drawn segments
            segs = a.get_segments()
            assert all(s[0][0] == s[1][0] == lag and s[0][1] == 0 and s[1][1] == v
                       for s, lag, v in zip(segs, lags, c))
        return {"lags": lags, "c": c, "artist": "LineCollection", "hline": b is not None,
                "marker": None, "linestyle": None}
    return {"lags": lags, "c": c, "artist": "Line2D", "hline": b is not None,
            "marker": a.get_marker(), "linestyle": a.get_linestyle()}


@_quiet
def _xcorr(x, y, **kw):
    if "detrend" in kw:
        kw["detrend"] = _DETRENDS[kw["detrend"]]
    ax = Figure().add_subplot()
    return _corr_result(ax.xcorr(np.asarray(x, float), np.asarray(y, float), **kw))


@_quiet
def _acorr(x, **kw):
    if "detrend" in kw:
        kw["detrend"] = _DETRENDS[kw["detrend"]]
    ax = Figure().add_subplot()
    return _corr_result(ax.acorr(np.asarray(x, float), **kw))


def acorr(**args):
    return probes.probe("Axes.acorr", _acorr, tol=TOL_CORR, **args)


def xcorr(**args):
    return probes.probe("Axes.xcorr", _xcorr, tol=TOL_CORR, **args)


def test_acorr():
    r = acorr(x=CORR20)
    assert r["lags"].tolist() == list(range(-10, 11)) and r["c"][10] == pytest.approx(1.0)
    acorr(x=CORR20, normed=False)
    acorr(x=CORR20, maxlags=None)
    acorr(x=CORR20, maxlags=3)
    acorr(x=CORR20, maxlags=19)
    acorr(x=CORR20, maxlags=1)
    acorr(x=CORR20, usevlines=False)
    acorr(x=CORR20, usevlines=False, maxlags=4, normed=False)
    for det in ("none", "mean", "linear"):
        acorr(x=CORR20, detrend=det)
        acorr(x=CORR32, detrend=det, maxlags=8)
    acorr(x=CORR32)
    acorr(x=[0.0] * 12, normed=True)  # 0/0 -> NaN
    acorr(x=[0.0] * 12, normed=False)
    acorr(x=[1.0, 2.0, 3.0], maxlags=2)
    assert isinstance(acorr(x=CORR20, maxlags=20), ValueError)
    assert isinstance(acorr(x=CORR20, maxlags=0), ValueError)
    assert isinstance(acorr(x=CORR20, maxlags=-1), ValueError)
    assert isinstance(acorr(x=SMALL5), ValueError)  # default maxlags 10 >= 5
    assert isinstance(acorr(x=[1.0], maxlags=None), ValueError)


def test_xcorr():
    r = xcorr(x=CORR20, y=CORR20B)
    assert len(r["c"]) == 21
    xcorr(x=CORR20, y=CORR20B, normed=False)
    xcorr(x=CORR20B, y=CORR20, normed=False)  # reversed lags
    xcorr(x=CORR20, y=CORR20B, maxlags=None)
    xcorr(x=CORR20, y=CORR20B, maxlags=5, usevlines=False)
    for det in ("none", "mean", "linear"):
        xcorr(x=CORR32, y=CORR32B, detrend=det)
        xcorr(x=CORR32, y=CORR32B, detrend=det, normed=False, maxlags=None)
    xcorr(x=[1.0, 0.0, 0.0, 0.0], y=[0.0, 0.0, 1.0, 0.0], maxlags=3, normed=False)
    xcorr(x=[1.0, 2.0, 3.0, 4.0], y=[0.0] * 4, maxlags=3)  # normed by 0 -> NaN
    assert isinstance(xcorr(x=CORR20, y=CORR20B[:15]), ValueError)
    assert isinstance(xcorr(x=CORR20, y=CORR20B, maxlags=25), ValueError)
