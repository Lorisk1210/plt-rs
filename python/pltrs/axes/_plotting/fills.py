"""
Filled areas and steps: `~.Axes.fill`, `~.Axes.fill_between`,
`~.Axes.fill_betweenx`, `~.Axes.stairs`, `~.Axes.stackplot` and
`~.Axes.step`.
"""

import numpy as np

from ... import _api, _live_transforms, _style_helpers, cbook, collections as mcoll
from ... import lines as mlines, patches as mpatches
from ..._native import plotting as _plotting
from ..._preprocess import _preprocess_data
from ._helpers import axis_values, floats, wrap, wrap_all
from .lines import _plot_datasets


def _rcParams():
    """The global rcParams (imported when first needed)."""
    from ... import rcParams
    return rcParams


class FillsMixin:
    """`fill`, `fill_between`, `fill_betweenx`, `stairs`, `stackplot`, `step`."""

    def fill(self, *args, data=None, **kwargs):
        """
        Plot filled polygons.

        Returns
        -------
        list of `~matplotlib.patches.Polygon`
        """
        kwargs = cbook.normalize_kwargs(kwargs, mlines.Line2D)
        closed = kwargs.pop('closed', True)
        patches = []
        for x, y, fmt, label in list(_plot_datasets(list(args), data, kwargs)):
            color = kwargs.get('color', kwargs.get('facecolor'))
            if fmt is not None and color is None:
                color = _plotting.format_parts(fmt)[2]
            native = _plotting.fill(self._native, axis_values(x), axis_values(y),
                                    color, bool(closed))
            polygon = wrap(self, native, mpatches.Polygon)
            props = {k: v for k, v in kwargs.items()
                     if k not in ('color', 'label') and v is not None}
            polygon._internal_update(cbook.normalize_kwargs(props, mpatches.Patch))
            if label is not None:
                polygon.set_label(label)
            patches.append(polygon)
        return patches

    def _fill_between_x_or_y(self, ind_dir, ind, dep1, dep2=0, *,
                             where=None, interpolate=False, step=None,
                             **kwargs):
        """`fill_between` (*ind_dir* ``"x"``) or `fill_betweenx` (``"y"``)."""
        kwargs = cbook.normalize_kwargs(kwargs, mcoll.Collection)
        color = kwargs.pop("color", None)
        facecolor = kwargs.pop("facecolor", None)
        if step is not None:
            _api.check_in_list(('pre', 'post', 'mid'), step=step)
        ind, dep1, dep2 = np.broadcast_arrays(
            *(np.ma.masked_invalid(np.ma.asarray(v, dtype=float)
                                   if not isinstance(v, np.ndarray) or v.dtype != object
                                   else v)
              for v in (ind, dep1, dep2)), subok=True)
        for name, array in zip([ind_dir, f"{'y' if ind_dir == 'x' else 'x'}1",
                                f"{'y' if ind_dir == 'x' else 'x'}2"],
                               [ind, dep1, dep2]):
            if array.ndim > 1:
                raise ValueError(f"{name!r} is not 1-dimensional")
        if where is None:
            mask = None
        else:
            where = np.asarray(where, dtype=bool)
            if where.size != ind.size:
                raise ValueError(f"where size ({where.size}) does not match "
                                 f"{ind_dir} size ({ind.size})")
            mask = where.tolist()
        native = _plotting.fill_between(
            self._native, ind_dir == "y",
            (floats(ind), floats(dep1), floats(dep2)), (mask, bool(interpolate), step),
            (color, facecolor),
            _live_transforms.given_to_native(kwargs.get("transform")))
        collection = wrap(self, native, mcoll.FillBetweenPolyCollection)
        collection.t_direction = ind_dir
        collection._internal_update(kwargs)
        return collection

    @_preprocess_data(replace_names=["x", "y1", "y2", "where"])
    def fill_between(self, x, y1, y2=0, where=None, interpolate=False,
                     step=None, **kwargs):
        """
        Fill the area between two horizontal curves.

        Returns
        -------
        `.FillBetweenPolyCollection`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        return self._fill_between_x_or_y(
            "x", x, y1, y2,
            where=where, interpolate=interpolate, step=step, **kwargs)

    @_preprocess_data(replace_names=["y", "x1", "x2", "where"])
    def fill_betweenx(self, y, x1, x2=0, where=None,
                      step=None, interpolate=False, **kwargs):
        """
        Fill the area between two vertical curves.

        Returns
        -------
        `.FillBetweenPolyCollection`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        return self._fill_between_x_or_y(
            "y", y, x1, x2,
            where=where, interpolate=interpolate, step=step, **kwargs)

    @_preprocess_data()
    def stairs(self, values, edges=None, *,
               orientation='vertical', baseline=0, fill=False, **kwargs):
        """
        Draw a stepwise constant function as a line or a filled plot.

        Returns
        -------
        `~matplotlib.patches.StepPatch`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        _api.check_in_list(('vertical', 'horizontal'), orientation=orientation)
        kwargs = cbook.normalize_kwargs(kwargs, mpatches.StepPatch)
        color = kwargs.pop('color', None)
        if baseline is None and fill:
            _api.warn_external(
                f"Both {baseline=} and {fill=} have been passed. "
                "baseline=None is only intended for unfilled stair plots. "
                "Because baseline is None, the Path used to draw the stairs will "
                "not be closed, thus because fill is True the polygon will be closed "
                "by drawing an (unstroked) edge from the first to last point.  It is "
                "very likely that the resulting fill patterns is not the desired "
                "result."
            )
        values = floats(values)
        native = _plotting.stairs(
            self._native, values, None if edges is None else floats(edges),
            None if baseline is None else np.asarray(baseline, dtype=float),
            (orientation, bool(fill)), color)
        patch = wrap(self, native, mpatches.StepPatch)
        patch._internal_update(kwargs)
        return patch

    @_preprocess_data()
    def stackplot(self, x, *args, labels=(), colors=None, hatch=None,
                  baseline='zero', **kwargs):
        """
        Draw a stacked area plot or a streamgraph.

        Returns
        -------
        list of `.PolyCollection`
            A list of `.PolyCollection` instances, one for each element in the
            stacked area plot.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        y = np.vstack(args) if args else np.empty((0, len(np.atleast_1d(x))))
        _api.check_in_list(['zero', 'sym', 'wiggle', 'weighted_wiggle'],
                           baseline=baseline)
        kwargs = cbook.normalize_kwargs(kwargs, mcoll.PolyCollection)
        if colors is not None:
            kwargs.setdefault('facecolor', colors)
        if hatch is not None:
            kwargs['hatch'] = hatch
        kwargs, style_gen = _style_helpers.style_generator(kwargs)
        natives = _plotting.stackplot(self._native, axis_values(x),
                                      [list(floats(row)) for row in y], baseline, [])
        collections = wrap_all(self, natives, mcoll.FillBetweenPolyCollection)
        labels = iter(labels)
        for collection in collections:
            style = next(style_gen)
            collection._internal_update({**kwargs, **style})
            label = next(labels, None)
            if label is not None:
                collection.set_label(label)
        return collections

    def step(self, x, y, *args, where='pre', data=None, **kwargs):
        """
        Make a step plot.

        Returns
        -------
        list of `.Line2D`
            Objects representing the plotted data.
        """
        _api.check_in_list(('pre', 'post', 'mid'), where=where)
        kwargs['drawstyle'] = 'steps-' + where
        return self.plot(x, y, *args, data=data, **kwargs)
