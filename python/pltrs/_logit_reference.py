"""
matplotlib's `.LogitFormatter` (from matplotlib 3.11.2, under the
Matplotlib License), for tick locations pltrs (Rust) does not label (NaN,
infinities, values that are no numbers): matplotlib's keeps them as they
are and labels each value only when it is asked, failing then on what it
cannot compute with.
"""

import math

import numpy as np


def _is_decade(x, *, base=10, rtol=None):
    """Return True if *x* is an integer power of *base*."""
    if not np.isfinite(x):
        return False
    if x == 0.0:
        return True
    lx = np.log(abs(x)) / np.log(base)
    if rtol is None:
        return np.isclose(lx, np.round(lx))
    else:
        return np.isclose(lx, np.round(lx), rtol=rtol)


def _is_close_to_int(x):
    return math.isclose(x, round(x))


def set_locs(formatter, locs):
    """Keep the locations *locs* and choose the minor ticks labelled
    among them (matplotlib's ``LogitFormatter.set_locs``)."""
    formatter._locs = np.array(locs)
    formatter._labelled.clear()

    if not formatter._minor:
        return None
    if all(
        _is_decade(x, rtol=1e-7)
        or _is_decade(1 - x, rtol=1e-7)
        or (_is_close_to_int(2 * x) and
            int(np.round(2 * x)) == 1)
        for x in locs
    ):
        # minor ticks are subsample from ideal, so no label
        return None
    if len(locs) < formatter._minor_threshold:
        if len(locs) < formatter._minor_number:
            formatter._labelled.update(locs)
        else:
            # we do not have a lot of minor ticks, so only few decades are
            # displayed, then we choose some (spaced) minor ticks to label.
            # Only minor ticks are known, we assume it is sufficient to
            # choice which ticks are displayed.
            # For each ticks we compute the distance between the ticks and
            # the previous, and between the ticks and the next one. Ticks
            # with smallest minimum are chosen. As tiebreak, the ticks
            # with smallest sum is chosen.
            diff = np.diff(-np.log(1 / formatter._locs - 1))
            space_pessimistic = np.minimum(
                np.concatenate(((np.inf,), diff)),
                np.concatenate((diff, (np.inf,))),
            )
            space_sum = (
                np.concatenate(((0,), diff))
                + np.concatenate((diff, (0,)))
            )
            good_minor = sorted(
                range(len(formatter._locs)),
                key=lambda i: (space_pessimistic[i], space_sum[i]),
            )[-formatter._minor_number:]
            formatter._labelled.update(locs[i] for i in good_minor)


def _format_value(x, locs, sci_notation=True):
    if sci_notation:
        exponent = math.floor(np.log10(x))
        min_precision = 0
    else:
        exponent = 0
        min_precision = 1
    value = x * 10 ** (-exponent)
    if len(locs) < 2:
        precision = min_precision
    else:
        diff = np.sort(np.abs(locs - x))[1]
        precision = -np.log10(diff) + exponent
        precision = (
            int(np.round(precision))
            if _is_close_to_int(precision)
            else math.ceil(precision)
        )
        if precision < min_precision:
            precision = min_precision
    mantissa = r"%.*f" % (precision, value)
    if not sci_notation:
        return mantissa
    s = r"%s\cdot10^{%d}" % (mantissa, exponent)
    return s


def _one_minus(formatter, s):
    if formatter._use_overline:
        return r"\overline{%s}" % s
    else:
        return f"1-{s}"


def call(formatter, x, pos=None):
    """The label of the value *x* (matplotlib's ``LogitFormatter.__call__``)."""
    if formatter._minor and x not in formatter._labelled:
        return ""
    if x <= 0 or x >= 1:
        return ""
    if _is_close_to_int(2 * x) and round(2 * x) == 1:
        s = formatter._one_half
    elif x < 0.5 and _is_decade(x, rtol=1e-7):
        exponent = round(math.log10(x))
        s = "10^{%d}" % exponent
    elif x > 0.5 and _is_decade(1 - x, rtol=1e-7):
        exponent = round(math.log10(1 - x))
        s = _one_minus(formatter, "10^{%d}" % exponent)
    elif x < 0.1:
        s = _format_value(x, formatter._locs)
    elif x > 0.9:
        s = _one_minus(formatter, _format_value(1-x, 1-formatter._locs))
    else:
        s = _format_value(x, formatter._locs, sci_notation=False)
    return r"$\mathdefault{%s}$" % s
