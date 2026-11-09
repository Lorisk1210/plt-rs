"""
The statistics of one box of `.cbook.boxplot_stats` for what pltrs (Rust)
does not take: data that are no real numbers (booleans, dates, strings,
objects), whiskers that are no numbers, and bootstraps, which resample
with numpy's global random state. They are computed with matplotlib's
numpy arithmetic, in its order, so that they fail with its errors, and
its numpy warnings, at the same step. Ported from matplotlib 3.11.2's ``cbook.boxplot_stats``
under the Matplotlib License.
"""

from numbers import Real

import numpy as np


def plain(data, whis, bootstrap):
    """Whether pltrs (Rust) computes the statistics of *data* (an array of
    the box's values) for *whis* and *bootstrap* as matplotlib does."""
    if data.dtype.kind not in 'iu' and data.dtype != np.float64:
        return False
    if np.iterable(whis) and not isinstance(whis, str):
        if not all(isinstance(w, Real) for w in whis):
            return False
    elif not isinstance(whis, Real):
        return False
    # A bootstrap resamples with numpy's global random state, as
    # matplotlib's does, so that a program seeding it gets its notches.
    return bootstrap is None


def compute(stats, x, whis, bootstrap, autorange):
    """Fill *stats* with the statistics of the non-empty values *x* as
    matplotlib's loop computes them."""
    stats['mean'] = np.mean(x)
    q1, med, q3 = np.percentile(x, [25, 50, 75])
    stats['iqr'] = q3 - q1
    if stats['iqr'] == 0 and autorange:
        whis = (0, 100)
    stats['cilo'], stats['cihi'] = _confidence_interval(
        x, med, stats['iqr'], bootstrap)
    if np.iterable(whis) and not isinstance(whis, str):
        loval, hival = np.percentile(x, whis)
    elif np.isreal(whis):
        loval = q1 - whis * stats['iqr']
        hival = q3 + whis * stats['iqr']
    else:
        raise ValueError('whis must be a float or list of percentiles')
    wiskhi = x[x <= hival]
    if len(wiskhi) == 0 or np.max(wiskhi) < q3:
        stats['whishi'] = q3
    else:
        stats['whishi'] = np.max(wiskhi)
    wisklo = x[x >= loval]
    if len(wisklo) == 0 or np.min(wisklo) > q1:
        stats['whislo'] = q1
    else:
        stats['whislo'] = np.min(wisklo)
    stats['fliers'] = np.concatenate([
        x[x < stats['whislo']],
        x[x > stats['whishi']],
    ])
    stats['q1'], stats['med'], stats['q3'] = q1, med, q3


def _confidence_interval(data, med, iqr, bootstrap):
    """The notch around the median *med*: bootstrapped *bootstrap* times,
    else Gaussian."""
    if bootstrap is not None:
        M = len(data)
        bs_index = np.random.randint(M, size=(bootstrap, M))
        estimate = np.median(data[bs_index], axis=1, overwrite_input=True)
        return tuple(np.percentile(estimate, [2.5, 97.5]))
    N = len(data)
    return med - 1.57 * iqr / np.sqrt(N), med + 1.57 * iqr / np.sqrt(N)
