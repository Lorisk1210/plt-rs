"""
matplotlib's `~.Axes.grouped_bar`, for the positions and colours pltrs
(Rust) does not take as they are (see `.bars`): each dataset's bars made
by a call of `~.Axes.bar` (`~.Axes.barh`), which takes the dataset's
colour, cycled, as it takes any colour, failing where and as matplotlib's
fail (matplotlib 3.11's code, nearly as it is).
"""

import itertools

import numpy as np

from ... import _api, _style_helpers


def grouped_bar(axes, heights, positions, spacing, tick_labels, labels, orientation,
                colors, kwargs):
    """The bar containers of `~.Axes.grouped_bar` of *axes*, as matplotlib
    makes them, for the datasets *heights* (as it reads them from a frame,
    mapping or array) and the group and bar *spacing*."""
    group_spacing, bar_spacing = spacing
    num_datasets = len(heights)
    num_groups = len(next(iter(heights)))  # inferred from first dataset

    # validate that all datasets have the same length, i.e. num_groups
    # - can be skipped if heights is an array
    if not hasattr(heights, 'shape'):
        for i, dataset in enumerate(heights):
            if len(dataset) != num_groups:
                raise ValueError(
                    "'heights' contains datasets with different number of "
                    f"elements. dataset 0 has {num_groups} elements but "
                    f"dataset {i} has {len(dataset)} elements."
                )

    if positions is None:
        group_centers = np.arange(num_groups)
        group_distance = 1
    else:
        group_centers = np.asanyarray(positions)
        if len(group_centers) > 1:
            d = np.diff(group_centers)
            if not np.allclose(d, d.mean()):
                raise ValueError("'positions' must be equidistant")
            group_distance = d[0]
        else:
            group_distance = 1

    _api.check_in_list(["vertical", "horizontal"], orientation=orientation)

    if colors is None:
        colors = itertools.cycle([None])
    else:
        # Note: This is equivalent to the behavior in stackplot
        colors = itertools.cycle(colors)

    kwargs, style_gen = _style_helpers.style_generator(kwargs)

    bar_width = (group_distance /
                 (num_datasets + (num_datasets - 1) * bar_spacing + group_spacing))
    bar_spacing_abs = bar_spacing * bar_width
    margin_abs = 0.5 * group_spacing * bar_width

    if labels is None:
        labels = [None] * num_datasets
    else:
        assert len(labels) == num_datasets

    # place the bars, but only use numerical positions, categorical tick labels
    # are handled separately below
    bar_containers = []
    for i, (hs, label, color, styles) in enumerate(zip(heights, labels, colors,
                                                       style_gen)):
        lefts = (group_centers - 0.5 * group_distance + margin_abs
                 + i * (bar_width + bar_spacing_abs))
        if orientation == "vertical":
            bc = axes.bar(lefts, hs, width=bar_width, align="edge",
                          label=label, color=color, **styles, **kwargs)
        else:
            bc = axes.barh(lefts, hs, height=bar_width, align="edge",
                           label=label, color=color, **styles, **kwargs)
        bar_containers.append(bc)

    if tick_labels is not None:
        if orientation == "vertical":
            axes.xaxis.set_ticks(group_centers, labels=tick_labels)
        else:
            axes.yaxis.set_ticks(group_centers, labels=tick_labels)

    return bar_containers
