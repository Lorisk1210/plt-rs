"""
Labels of bar and pie charts: `~.Axes.bar_label` and `~.Axes.pie_label`.
"""

import math
from numbers import Real

import numpy as np

from ... import _api
from ..._native import plotting as _plotting
from ._helpers import takes_properties, wrap


class LabelsMixin:
    """`bar_label` and `pie_label`."""

    def bar_label(self, container, labels=None, *, fmt="%g", label_type="edge",
                  padding=0, **kwargs):
        """
        Label a bar plot.

        Adds labels to bars in the given `.BarContainer`.
        You may need to adjust the axis limits to fit the labels.

        Parameters
        ----------
        container : `.BarContainer`
            Container with all the bars and optionally errorbars, likely
            returned from `.bar` or `.barh`.

        labels : array-like, optional
            A list of label texts, that should be displayed. If not given, the
            label texts will be the data values formatted with *fmt*.

        fmt : str or callable, default: '%g'
            An unnamed %-style or {}-style format string for the label or a
            function to call with the value as the first argument.
            When *fmt* is a string and can be interpreted in both formats,
            %-style takes precedence over {}-style.

            .. versionadded:: 3.7
               Support for {}-style format string and callables.

        label_type : {'edge', 'center'}, default: 'edge'
            The label type. Possible values:

            - 'edge': label placed at the end-point of the bar segment, and the
              value displayed will be the position of that end-point.
            - 'center': label placed in the center of the bar segment, and the
              value displayed will be the length of that segment.
              (useful for stacked bars, i.e.,
              :doc:`/gallery/lines_bars_and_markers/bar_label_demo`)

        padding : float or array-like, default: 0
            Distance of label from the end of the bar, in points.
            If an array-like is provided, the padding values are applied
            to each label individually. Must have the same length as container.
            .. versionadded:: 3.11

        **kwargs
            Any remaining keyword arguments are passed through to
            `.Axes.annotate`. The alignment parameters (
            *horizontalalignment* / *ha*, *verticalalignment* / *va*) are
            not supported because the labels are automatically aligned to
            the bars.

        Returns
        -------
        list of `.Annotation`
            A list of `.Annotation` instances for the labels.
        """
        from ...text import Annotation
        for key in ['horizontalalignment', 'ha', 'verticalalignment', 'va']:
            if key in kwargs:
                raise ValueError(
                    f"Passing {key!r} to bar_label() is not supported.")
        _api.check_in_list(['edge', 'center'], label_type=label_type)
        if not isinstance(fmt, str) and not callable(fmt):
            raise TypeError("fmt must be a str or callable")
        bars = len(container.patches)
        if np.iterable(padding):
            padding = np.asarray(padding)
            if len(padding) != bars:
                raise ValueError(
                    f"padding must be of length {bars} when passed as a sequence")
            padding = [float(p) for p in padding]
        else:
            if bars:
                # matplotlib offsets each label by its direction times it.
                _ = 1 * padding
            padding = [float(padding)]
        labels = [] if labels is None else [str(label) for label in labels]
        annotations = []
        for native in _plotting.bar_label(self._native, container._native, labels,
                                          fmt, label_type, padding):
            annotation = wrap(self, native, Annotation)
            annotation._init_annotation(annotation.xy, annotation.get_position(),
                                        "data", "offset points", None, kwargs)
            annotations.append(annotation)
        return annotations

    def pie_label(self, container, /, labels, *, distance=0.6,
                  textprops=None, rotate=False, alignment='auto'):
        """
        Label a pie chart.

        .. versionadded:: 3.11

        Adds labels to wedges in the given `.PieContainer`.

        Parameters
        ----------
        container : `.PieContainer`
            Container with all the wedges, likely returned from `.pie`.

        labels : str or list of str
            A sequence of strings providing the labels for each wedge, or a format
            string with ``absval`` and/or ``frac`` placeholders.  For example, to label
            each wedge with its value and the percentage in brackets::

                labels="{absval:d} ({frac:.0%})"

        distance : float, default: 0.6
            The radial position of the labels, relative to the pie radius. Values > 1
            are outside the wedge and values < 1 are inside the wedge.

        textprops : dict, default: None
            Dict of arguments to pass to the `.Text` objects.

        rotate : bool, default: False
            Rotate each label to the angle of the corresponding slice if true.

        alignment : {'center', 'outer', 'auto'}, default: 'auto'
            Controls the horizontal alignment of the text objects relative to their
            nominal position.

            - 'center': The labels are centered on their points.
            - 'outer': Labels are aligned away from the center of the pie, i.e., labels
              on the left side of the pie are right-aligned and labels on the right
              side are left-aligned.
            - 'auto': Translates to 'outer' if *distance* > 1 (so that the labels do not
              overlap the wedges) and 'center' if *distance* < 1.

            If *rotate* is True, the vertical alignment is also affected in an
            analogous way.

            - 'center': The labels are centered on their points.
            - 'outer': Labels are aligned away from the center of the pie, i.e., labels
              on the top half of the pie are bottom-aligned and labels on the bottom
              half are top-aligned.

        Returns
        -------
        list
            A list of the label `.Text` instances.
        """
        from ...text import Text
        _api.check_in_list(['center', 'outer', 'auto'], alignment=alignment)
        if alignment == 'auto':
            alignment = 'outer' if distance > 1 else 'center'
        if not isinstance(labels, str) and (
                (nw := len(container.wedges)) != (nl := len(labels))):
            raise ValueError(
                f'The number of labels ({nl}) must match the number of wedges ({nw})')
        if not isinstance(distance, Real):
            # matplotlib places each label at the distance, which fails for
            # one that is no number.
            for wedge in container.wedges:
                thetam = 2 * np.pi * 0.5 * (wedge.theta1 + wedge.theta2) / 360
                wedge.center[0] + distance * wedge.r * math.cos(thetam)
        textprops = {} if textprops is None else dict(textprops)
        template = isinstance(labels, str)
        usetex = textprops.get("usetex")
        if template:
            labels = [labels]
        else:
            labels = [str(label) for label in labels]
        natives = _plotting.pie_label(
            self._native,
            ([wedge._native for wedge in container.wedges],
             [float(v) for v in container.values], [float(f) for f in container.fracs]),
            (labels, template, None if usetex is None else bool(usetex)),
            float(distance), bool(rotate), alignment)
        texts = [wrap(self, native, Text) for native in natives]
        if texts and textprops and not takes_properties(Text(0, 0, ''), textprops):
            # matplotlib's first label fails to take them, before it makes
            # the others.
            for text in texts[1:]:
                text.remove()
            texts[0].set(**textprops)
        for text in texts:
            text._internal_update(textprops)
        container.add_texts(texts)
        return texts
