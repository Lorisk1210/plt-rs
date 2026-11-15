"""
matplotlib's `~.Axes.stackplot`, for an Axes class of the program's that
overrides `~.Axes.fill_between`, which matplotlib's draws each layer
with and pltrs (Rust) makes itself otherwise (matplotlib 3.11's code, as
it is, under the Matplotlib License).
"""

import numpy as np

from . import _api, _style_helpers, cbook, collections


def stackplot(axes, x, *args,
              labels=(), colors=None, baseline='zero',
              **kwargs):
    """`stackplot` as matplotlib makes it: each layer through
    *axes*' `~.Axes.fill_between`, which a class of the program's overrides."""

    y = np.vstack(args)

    labels = iter(labels)
    if colors is None:
        colors = [axes._get_lines.get_next_color() for _ in y]

    kwargs = cbook.normalize_kwargs(kwargs, collections.PolyCollection)
    kwargs.setdefault('facecolor', colors)

    kwargs, style_gen = _style_helpers.style_generator(kwargs)

    # Assume data passed has not been 'stacked', so stack it here.
    # We'll need a float buffer for the upcoming calculations.
    stack = np.cumsum(y, axis=0, dtype=np.promote_types(y.dtype, np.float32))

    _api.check_in_list(['zero', 'sym', 'wiggle', 'weighted_wiggle'],
                       baseline=baseline)
    if baseline == 'zero':
        first_line = 0.

    elif baseline == 'sym':
        first_line = -np.sum(y, 0) * 0.5
        stack += first_line[None, :]

    elif baseline == 'wiggle':
        m = y.shape[0]
        first_line = (y * (m - 0.5 - np.arange(m)[:, None])).sum(0)
        first_line /= -m
        stack += first_line

    elif baseline == 'weighted_wiggle':
        total = np.sum(y, 0)
        # multiply by 1/total (or zero) to avoid infinities in the division:
        inv_total = np.zeros_like(total)
        mask = total > 0
        inv_total[mask] = 1.0 / total[mask]
        increase = np.hstack((y[:, 0:1], np.diff(y)))
        below_size = total - stack
        below_size += 0.5 * y
        move_up = below_size * inv_total
        move_up[:, 0] = 0.5
        center = (move_up - 0.5) * increase
        center = np.cumsum(center.sum(0))
        first_line = center - 0.5 * total
        stack += first_line

    # Color between x = 0 and the first array.
    coll = axes.fill_between(x, first_line, stack[0, :],
                             label=next(labels, None),
                             **next(style_gen), **kwargs)
    coll.sticky_edges.y[:] = [0]
    r = [coll]

    # Color between array i-1 and array i
    for i in range(len(y) - 1):
        r.append(axes.fill_between(x, stack[i, :], stack[i + 1, :],
                                   label=next(labels, None),
                                   **next(style_gen), **kwargs))
    return r
