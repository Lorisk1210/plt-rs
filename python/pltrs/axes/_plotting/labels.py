"""
Labels of bar and pie charts: `~.Axes.bar_label` and `~.Axes.pie_label`.
"""

import numpy as np

from ... import _api
from ..._native import plotting as _plotting
from ._helpers import wrap


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
        label_type : {'edge', 'center'}, default: 'edge'
            The label type.
        padding : float or array-like, default: 0
            Distance of label from the end of the bar, in points.
        **kwargs
            Any remaining keyword arguments are passed through to
            `.Axes.annotate`.

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
            padding = [float(p) for p in np.asarray(padding)]
            if len(padding) != bars:
                raise ValueError(
                    f"padding must be of length {bars} when passed as a sequence")
        else:
            padding = [float(padding)]
        labels = [] if labels is None else [str(label) for label in labels]
        annotations = []
        for native in _plotting.bar_label(self._native, container._native, labels,
                                          fmt, label_type, padding):
            annotation = wrap(self, native, Annotation)
            annotation._init_annotation("data", "offset points", None, kwargs)
            annotations.append(annotation)
        return annotations

    def pie_label(self, container, /, labels, *, distance=0.6,
                  textprops=None, rotate=False, alignment='auto'):
        """
        Label a pie chart.

        Adds labels to wedges in the given `.PieContainer`.

        Parameters
        ----------
        container : `.PieContainer`
            Container with all the wedges, likely returned from `.pie`.
        labels : str or list of str
            A sequence of strings providing the labels for each wedge, or a
            format string with ``absval`` and/or ``frac`` placeholders.
        distance : float, default: 0.6
            The radial position of the labels, relative to the pie radius.
        textprops : dict, default: None
            Dict of arguments to pass to the `.Text` objects.
        rotate : bool, default: False
            Rotate each label to the angle of the corresponding slice if true.
        alignment : {'center', 'outer', 'auto'}, default: 'auto'
            Controls the alignment of the text objects relative to their
            nominal position.

        Returns
        -------
        list
            A list of the label `.Text` instances.
        """
        from ...text import Text
        _api.check_in_list(['center', 'outer', 'auto'], alignment=alignment)
        textprops = {} if textprops is None else dict(textprops)
        template = isinstance(labels, str)
        if template:
            usetex = textprops.get("usetex")
            from ... import rcParams
            if usetex is not None and usetex != rcParams["text.usetex"]:
                raise NotImplementedError("pltrs: pie_label(textprops={'usetex': ...})")
            labels = [labels]
        else:
            labels = [str(label) for label in labels]
        natives = _plotting.pie_label(
            self._native,
            ([wedge._native for wedge in container.wedges],
             [float(v) for v in container.values], [float(f) for f in container.fracs]),
            (labels, template), float(distance), bool(rotate), alignment)
        texts = [wrap(self, native, Text) for native in natives]
        for text in texts:
            text._internal_update(textprops)
        container.add_texts(texts)
        return texts
