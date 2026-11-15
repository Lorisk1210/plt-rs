"""
matplotlib's `~.Axes.pie`, for the arguments pltrs (Rust) does not take as
they are (see `.stats`): its wedges, shadows and labels made one by one
over the package's artists, failing where and as matplotlib's fails
(matplotlib 3.11's code, nearly as it is).
"""

import itertools
import math
import re
from numbers import Real

import numpy as np

from ... import _api, cbook, patches as mpatches, rcParams
from ...container import PieContainer
from ...text import Text


def pie(axes, x, explode, labels, colors, autopct, pctdistance, shadow, labeldistance,
        startangle, radius, counterclock, wedgeprops, textprops, center, frame,
        rotatelabels, normalize, hatch):
    """`~.Axes.pie` of *axes* as matplotlib makes it."""
    axes.set_aspect('equal')
    x = np.asarray(x)
    if x.ndim > 1:
        raise ValueError("x must be 1D")

    if np.any(x < 0):
        raise ValueError("Wedge sizes 'x' must be non negative values")

    if not np.all(np.isfinite(x)):
        raise ValueError('Wedge sizes must be finite numbers')

    sx = x.sum()

    if sx == 0:
        raise ValueError('All wedge sizes are zero')

    if normalize:
        fracs = x / sx
    elif sx > 1:
        raise ValueError('Cannot plot an unnormalized pie with sum(x) > 1')
    else:
        fracs = x
    if labels is None:
        labels = [''] * len(x)
    if explode is None:
        explode = [0] * len(x)
    if len(x) != len(labels):
        raise ValueError(f"'labels' must be of length 'x', not {len(labels)}")
    if len(x) != len(explode):
        raise ValueError(f"'explode' must be of length 'x', not {len(explode)}")
    if colors is None:
        get_next_color = axes._get_patches_for_fill.get_next_color
    else:
        color_cycle = itertools.cycle(colors)

        def get_next_color():
            return next(color_cycle)

    hatch_cycle = itertools.cycle(np.atleast_1d(hatch))

    _api.check_isinstance(Real, radius=radius, startangle=startangle)
    if radius <= 0:
        raise ValueError(f"'radius' must be a positive number, not {radius}")

    # Starting theta1 is the start fraction of the circle
    theta1 = startangle / 360

    if wedgeprops is None:
        wedgeprops = {}

    slices = []

    for frac, label, expl in zip(fracs, labels, explode):
        x_pos, y_pos = center
        theta2 = (theta1 + frac) if counterclock else (theta1 - frac)
        thetam = 2 * np.pi * 0.5 * (theta1 + theta2)
        x_pos += expl * math.cos(thetam)
        y_pos += expl * math.sin(thetam)

        w = mpatches.Wedge((x_pos, y_pos), radius, 360. * min(theta1, theta2),
                           360. * max(theta1, theta2),
                           facecolor=get_next_color(),
                           hatch=next(hatch_cycle),
                           clip_on=False,
                           label=label)
        w.set(**wedgeprops)
        slices.append(w)
        axes.add_patch(w)

        if shadow:
            # Make sure to add a shadow after the call to add_patch so the
            # figure and transform props will be set.
            shadow_dict = {'ox': -0.02, 'oy': -0.02, 'label': '_nolegend_'}
            if isinstance(shadow, dict):
                shadow_dict.update(shadow)
            axes.add_patch(mpatches.Shadow(w, **shadow_dict))

        theta1 = theta2

    pc = PieContainer(slices, x, normalize)

    if labeldistance is None:
        # Insert an empty list of texts for backwards compatibility of the
        # return value.
        pc.add_texts([])
    else:
        # Add labels to the wedges.
        labels_textprops = {
            'fontsize': rcParams['xtick.labelsize'],
            **cbook.normalize_kwargs(textprops or {}, Text)
        }
        axes.pie_label(pc, labels, distance=labeldistance,
                       alignment='outer', rotate=rotatelabels,
                       textprops=labels_textprops)

    if autopct is not None:
        # Add automatic percentage labels to wedges
        auto_labels = []
        for frac in fracs:
            if isinstance(autopct, str):
                s = autopct % (100. * frac)
            elif callable(autopct):
                s = autopct(100. * frac)
            else:
                raise TypeError(
                    'autopct must be callable or a format string')
            if textprops is not None and _val_or_rc(textprops.get("usetex"),
                                                        "text.usetex"):
                # escape % (i.e. \%) if it is not already escaped
                s = re.sub(r"([^\\])%", r"\1\\%", s)
            auto_labels.append(s)

        axes.pie_label(pc, auto_labels, distance=pctdistance,
                       alignment='center',
                       textprops=textprops)

    if frame:
        axes._request_autoscale_view()
    else:
        axes.set(frame_on=False, xticks=[], yticks=[],
                 xlim=(-1.25 + center[0], 1.25 + center[0]),
                 ylim=(-1.25 + center[1], 1.25 + center[1]))

    return pc


def pie_label(axes, container, labels, distance, rotate, alignment, textprops):
    """`~.Axes.pie_label` of *axes* as matplotlib makes it: each label
    written through its `~.Axes.text` (for a class of the program's
    overriding it), *alignment* checked."""
    if textprops is None:
        textprops = {}

    if isinstance(labels, str):
        # Assume we have a format string
        labels = [labels.format(absval=val, frac=frac) for val, frac in
                  zip(container.values, container.fracs)]
        if _val_or_rc(textprops.get("usetex"), "text.usetex"):
            # escape % (i.e. \%) if it is not already escaped
            labels = [re.sub(r"([^\\])%", r"\1\\%", s) for s in labels]
    elif (nw := len(container.wedges)) != (nl := len(labels)):
        raise ValueError(
            f'The number of labels ({nl}) must match the number of wedges ({nw})')

    texts = []

    for wedge, label in zip(container.wedges, labels):
        thetam = 2 * np.pi * 0.5 * (wedge.theta1 + wedge.theta2) / 360
        xt = wedge.center[0] + distance * wedge.r * math.cos(thetam)
        yt = wedge.center[1] + distance * wedge.r * math.sin(thetam)
        if alignment == 'outer':
            label_alignment_h = 'left' if xt > 0 else 'right'
        else:
            label_alignment_h = 'center'
        label_alignment_v = 'center'
        label_rotation = 'horizontal'
        if rotate:
            if alignment == 'outer':
                label_alignment_v = 'bottom' if yt > 0 else 'top'
            label_rotation = (np.rad2deg(thetam) + (0 if xt > 0 else 180))
        t = axes.text(xt, yt, label, clip_on=False, rotation=label_rotation,
                      horizontalalignment=label_alignment_h,
                      verticalalignment=label_alignment_v)
        t.set(**textprops)
        texts.append(t)

    container.add_texts(texts)

    return texts


def _val_or_rc(val, rc_name):
    """*val*, or the rcParam *rc_name* for None (matplotlib's
    ``_val_or_rc``)."""
    return val if val is not None else rcParams[rc_name]
