"""
matplotlib's statistics of `~.Axes.boxplot` (``cbook.boxplot_stats``)
for arguments pltrs (Rust) does not take as they are: data that are no
numbers, whiskers that are no number or pair of percentiles, bootstrap
counts that are no positive integer. It computes as matplotlib does,
raising what numpy raises there; plain arguments go to pltrs (Rust)
(`.cbook.boxplot_stats`).
"""

import itertools
from numbers import Integral, Real

import numpy as np

from ... import cbook


def is_plain(X, whis, bootstrap):
    """Whether pltrs (Rust) takes the data *X*, *whis* and *bootstrap* as
    they are: columns of numbers, a number or two percentiles, and no or a
    positive number of bootstrap samples."""
    def numbers(values):
        return np.asarray(values).dtype.kind in "iuf"
    try:
        columns = cbook._reshape_2D(X, "X")
    except (TypeError, ValueError):
        return False
    whiskers = (isinstance(whis, Real) or np.iterable(whis) and not isinstance(whis, str)
                and len(whis) == 2 and all(isinstance(w, Real) for w in whis))
    return (whiskers and all(map(numbers, columns))
            and (bootstrap is None or isinstance(bootstrap, Integral) and bootstrap > 0))


def boxplot_stats(X, whis=1.5, bootstrap=None, labels=None, autorange=False):
    """matplotlib's ``boxplot_stats`` up to its statistics of each column
    (the fliers are left out: they fail nowhere)."""
    def _bootstrap_median(data, N=5000):
        # determine 95% confidence intervals of the median
        M = len(data)
        percentiles = [2.5, 97.5]

        bs_index = np.random.randint(M, size=(N, M))
        bsData = data[bs_index]
        estimate = np.median(bsData, axis=1, overwrite_input=True)

        CI = np.percentile(estimate, percentiles)
        return CI

    def _compute_conf_interval(data, med, iqr, bootstrap):
        if bootstrap is not None:
            # Do a bootstrap estimate of notch locations.
            # get conf. intervals around median
            CI = _bootstrap_median(data, N=bootstrap)
            notch_min = CI[0]
            notch_max = CI[1]
        else:

            N = len(data)
            notch_min = med - 1.57 * iqr / np.sqrt(N)
            notch_max = med + 1.57 * iqr / np.sqrt(N)

        return notch_min, notch_max

    # convert X to a list of lists
    X = cbook._reshape_2D(X, "X")

    ncols = len(X)
    if labels is None:
        labels = itertools.repeat(None)
    elif len(labels) != ncols:
        raise ValueError(f"The number of labels ({len(labels)}) must match the"
                         f" number of columns ({ncols}).")

    input_whis = whis
    for x in X:
        # restore whis to the input values in case it got changed in the loop
        whis = input_whis
        if len(x) == 0:
            continue

        # up-convert to an array, just to be safe
        x = np.ma.asarray(x)
        x = x.data[~x.mask].ravel()

        # arithmetic mean
        np.mean(x)

        # medians and quartiles
        q1, med, q3 = np.percentile(x, [25, 50, 75])

        # interquartile range
        iqr = q3 - q1
        if iqr == 0 and autorange:
            whis = (0, 100)

        # conf. interval around median
        _compute_conf_interval(x, med, iqr, bootstrap)

        # lowest/highest non-outliers
        if np.iterable(whis) and not isinstance(whis, str):
            _, _ = np.percentile(x, whis)
        elif np.isreal(whis):
            _ = q1 - whis * iqr, q3 + whis * iqr
        else:
            raise ValueError('whis must be a float or list of percentiles')
