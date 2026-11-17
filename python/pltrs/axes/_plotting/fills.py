"""
Filled areas and steps: `~.Axes.fill`, `~.Axes.fill_between`,
`~.Axes.fill_betweenx`, `~.Axes.stairs`, `~.Axes.stackplot` and
`~.Axes.step`.
"""

import numpy as np

from ... import _docstring, _transform_trees
from ... import (_api, _live_transforms, _style_helpers, _values, cbook,
                 collections as mcoll, colors as mcolors)
from ... import lines as mlines, patches as mpatches, stackplot as mstack
from ..._native import axes_base as _axes_base, plotting as _plotting
from ..._preprocess import _preprocess_data
from ._helpers import (_make_axes_method, converted_line_data, floats, without_defaulted,
                       wrap, wrap_all)
from .lines import _plot_datasets



def _prepared_stack(axes, x, y, colors, baseline, kwargs):
    """The checked positions, layers, fill, keywords and style generator of
    a `stackplot`, raising where matplotlib's fails before it adds a
    layer."""
    _api.check_in_list(['zero', 'sym', 'wiggle', 'weighted_wiggle'],
                       baseline=baseline)
    # matplotlib passes these on to the `fill_between` of every layer.
    where, interpolate, step = (kwargs.pop('where', None),
                                kwargs.pop('interpolate', False), kwargs.pop('step', None))
    kwargs = cbook.normalize_kwargs(kwargs, mcoll.PolyCollection)
    if colors is not None:
        kwargs.setdefault('facecolor', colors)
    kwargs, style_gen = _style_helpers.style_generator(kwargs)
    x, = axes._process_unit_info([("x", x)])
    # matplotlib fills each layer with `fill_between`, whose checks of the
    # data fail first between the baseline and the first layer.
    first = 0. if baseline == 'zero' else np.zeros(np.shape(y)[1:])
    mcoll.FillBetweenPolyCollection._checked_data(
        "x", *map(np.ma.masked_invalid, (x, first, y[0])), where)
    # The layers broadcast with the positions, as `fill_between`'s data do.
    x, y = np.broadcast_arrays(np.atleast_1d(floats(x)), np.asarray(y, dtype=float))
    x = x[0] if len(x) else np.atleast_1d(floats(x))
    fill = (_fill_mask(where, np.size(x), "x"), bool(interpolate), step)
    mcoll._check_step(step, [floats(x), *(floats(row) for row in y)], fill[0])
    return x, y, fill, kwargs, style_gen

def _fill_mask(where, size, ind_dir):
    """The flags of `fill_between`'s *where* for *size* points along
    *ind_dir* (``"x"`` or ``"y"``), as a list; None for None."""
    if where is None:
        return None
    where = np.asarray(where, dtype=bool)
    if where.size != size:
        raise ValueError(f"where size ({where.size}) does not match "
                         f"{ind_dir} size ({size})")
    return where.tolist()


def _rcParams():
    """The global rcParams (imported when first needed)."""
    from ... import rcParams
    return rcParams


#: The patch properties a property cycle can give a filled polygon: those
#: given to `~.Axes.fill` are not taken from the cycle.
_CYCLED_PATCH_PROPS = ("color", "facecolor", "edgecolor", "linestyle",
                       "linewidth", "hatch", "alpha")


def _in_given_order(given, updates):
    """
    The property *updates* of a fill between curves made with the
    properties *given* (in the order given): with the colour given after
    an edge or face colour too, which matplotlib sets in that order, so
    that it overrides them.
    """
    names = list(given)
    if "color" not in names or "color" in updates:
        return updates
    if not any(name in names[:names.index("color")] for name in ("edgecolor", "facecolor")):
        return updates
    return {name: given["color"] if name == "color" else updates[name]
            for name in names if name == "color" or name in updates}


def _first_color(color):
    """*color*, or the first of the colours *color* lists ('none' for none
    listed): the colour pltrs (Rust) fills with until the collection takes
    them all."""
    if color is None or mcolors.is_color_like(color):
        return color
    return color[0] if len(color) else "none"


class FillsMixin:
    """`fill`, `fill_between`, `fill_betweenx`, `stairs`, `stackplot`, `step`."""

    def fill(self, *args, data=None, **kwargs):
        """
        Plot filled polygons.

        Parameters
        ----------
        *args : sequence of x, y, [color]
            Each polygon is defined by the lists of *x* and *y* positions of
            its nodes, optionally followed by a *color* specifier. See
            :mod:`matplotlib.colors` for supported color specifiers. The
            standard color cycle is used for polygons without a color
            specifier.

            You can plot multiple polygons by providing multiple *x*, *y*,
            *[color]* groups.

            For example, each of the following is legal::

                ax.fill(x, y)                    # a polygon with default color
                ax.fill(x, y, "b")               # a blue polygon
                ax.fill(x, y, x2, y2)            # two polygons
                ax.fill(x, y, "b", x2, y2, "r")  # a blue and a red polygon

        data : indexable object, optional
            An object with labelled data. If given, provide the label names to
            plot in *x* and *y*, e.g.::

                ax.fill("time", "signal",
                        data={"time": [0, 1, 2], "signal": [0, 1, 0]})

        Returns
        -------
        list of `~matplotlib.patches.Polygon`

        Other Parameters
        ----------------
        **kwargs : `~matplotlib.patches.Polygon` properties

        Notes
        -----
        Use :meth:`fill_between` if you would like to fill the region between
        two curves.
        """
        kwargs = cbook.normalize_kwargs(kwargs, mlines.Line2D)
        closed = kwargs.pop('closed', True)
        self._process_unit_info(kwargs=kwargs)
        patches = []
        for x, y, fmt, label in list(_plot_datasets(list(args), data, kwargs, self)):
            fmt_color = None if fmt is None else _plotting.format_parts(fmt)[2]
            # (matplotlib's polygon takes ``fill=None`` as no fill, as
            # `hist`'s steps give it.)
            props = cbook.normalize_kwargs(
                without_defaulted({k: v for k, v in kwargs.items() if k != 'label'},
                                  mpatches.Polygon, kept=('fill',)),
                mpatches.Patch)
            native = _plotting.fill(
                self._native, *converted_line_data(self, x, y), (fmt_color, bool(closed)),
                {k: v for k, v in props.items() if k in _CYCLED_PATCH_PROPS})
            polygon = wrap(self, native, mpatches.Polygon)
            if ("linestyle" not in props
                    and "linestyle" in _values.prop_cycle(self).keys):
                # The style the cycle gave, as the cycle spells it.
                polygon._linestyle = polygon._get("linestyle")
            polygon._made_with(props)
            # None too, as matplotlib's ``_makefill`` gives the polygon the
            # label of its dataset.
            polygon.set_label(label)
            patches.append(polygon)
        _transform_trees.patches_added(self, patches)
        return patches

    def _fill_between_x_or_y(self, ind_dir, ind, dep1, dep2=0, *,
                             where=None, interpolate=False, step=None,
                             **kwargs):
        """
        Fill the area between two {dir} curves.

        The curves are defined by the points (*{ind}*, *{dep}1*) and (*{ind}*,
        *{dep}2*).  This creates one or multiple polygons describing the filled
        area.

        You may exclude some {dir} sections from filling using *where*.

        By default, the edges connect the given points directly.  Use *step*
        if the filling should be a step function, i.e. constant in between
        *{ind}*.

        Parameters
        ----------
        {ind} : array-like
            The {ind} coordinates of the nodes defining the curves.

        {dep}1 : array-like or float
            The {dep} coordinates of the nodes defining the first curve.

        {dep}2 : array-like or float, default: 0
            The {dep} coordinates of the nodes defining the second curve.

        where : array-like of bool, optional
            Define *where* to exclude some {dir} regions from being filled.
            The filled regions are defined by the coordinates ``{ind}[where]``.
            More precisely, fill between ``{ind}[i]`` and ``{ind}[i+1]`` if
            ``where[i] and where[i+1]``.  Note that this definition implies
            that an isolated *True* value between two *False* values in *where*
            will not result in filling.  Both sides of the *True* position
            remain unfilled due to the adjacent *False* values.

        interpolate : bool, default: False
            This option is only relevant if *where* is used and the two curves
            are crossing each other.

            Semantically, *where* is often used for *{dep}1* > *{dep}2* or
            similar.  By default, the nodes of the polygon defining the filled
            region will only be placed at the positions in the *{ind}* array.
            Such a polygon cannot describe the above semantics close to the
            intersection.  The {ind}-sections containing the intersection are
            simply clipped.

            Setting *interpolate* to *True* will calculate the actual
            intersection point and extend the filled region up to this point.

        step : {{'pre', 'post', 'mid'}}, optional
            Define *step* if the filling should be a step function,
            i.e. constant in between *{ind}*.  The value determines where the
            step will occur:

            - 'pre': The {dep} value is continued constantly to the left from
              every *{ind}* position, i.e. the interval ``({ind}[i-1], {ind}[i]]``
              has the value ``{dep}[i]``.
            - 'post': The y value is continued constantly to the right from
              every *{ind}* position, i.e. the interval ``[{ind}[i], {ind}[i+1])``
              has the value ``{dep}[i]``.
            - 'mid': Steps occur half-way between the *{ind}* positions.

        Returns
        -------
        `.FillBetweenPolyCollection`
            A `.FillBetweenPolyCollection` containing the plotted polygons.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        **kwargs
            All other keyword arguments are passed on to
            `.FillBetweenPolyCollection`. They control the `.Polygon` properties:

            %(FillBetweenPolyCollection:kwdoc)s

        See Also
        --------
        fill_between : Fill between two sets of y-values.
        fill_betweenx : Fill between two sets of x-values.
        """
        # `fill_between` (*ind_dir* "x") or `fill_betweenx` ("y").
        kwargs = cbook.normalize_kwargs(kwargs, mcoll.Collection)
        # Colours given per polygon (plotnine's ribbons) are the
        # collection's once it is made, in the order given; pltrs (Rust)
        # takes their first, which keeps the property cycle from
        # advancing, as matplotlib's given colours do.
        per_polygon = {name: value for name, value in kwargs.items()
                       if name in ("color", "facecolor") and value is not None
                       and not mcolors.is_color_like(value)}
        cycled = not (_rcParams()["_internal.classic_mode"]
                      or any(name in kwargs for name in ("color", "facecolor")))
        given = dict(kwargs)
        color, facecolor = (_first_color(kwargs.pop(name, None))
                            for name in ("color", "facecolor"))
        dep_dir = "y" if ind_dir == "x" else "x"
        try:
            curves, mask = mcoll.FillBetweenPolyCollection._checked_data(
                ind_dir, *map(np.ma.masked_invalid, self._process_unit_info(
                    [(ind_dir, ind), (dep_dir, dep1), (dep_dir, dep2)], kwargs)), where)
            mcoll._check_step(step, curves, mask)
        except Exception:
            if cycled:
                # matplotlib takes the colour from the cycle before it
                # checks the data.
                self._get_patches_for_fill.get_next_color()
            raise
        native = _plotting.fill_between(
            self._native, ind_dir == "y",
            tuple(map(floats, curves)), (mask, bool(interpolate), step),
            (color, facecolor),
            _live_transforms.given_to_native(kwargs.get("transform"), self))
        collection = wrap(self, native, mcoll.FillBetweenPolyCollection)
        collection.t_direction = ind_dir
        if color is not None:
            # A colour given sets the hatches' too (`Collection.set_color`).
            collection.set_hatchcolor(color)
        collection._made_with(_in_given_order(given, {**per_polygon, **kwargs}))
        return collection

    def fill_between(self, x, y1, y2=0, where=None, interpolate=False,
                     step=None, **kwargs):
        return self._fill_between_x_or_y(
            "x", x, y1, y2,
            where=where, interpolate=interpolate, step=step, **kwargs)

    if _fill_between_x_or_y.__doc__:
        fill_between.__doc__ = _fill_between_x_or_y.__doc__.format(
            dir="horizontal", ind="x", dep="y"
        )
    fill_between = _preprocess_data(
        _docstring.interpd(fill_between),
        replace_names=["x", "y1", "y2", "where"])

    def fill_betweenx(self, y, x1, x2=0, where=None,
                      step=None, interpolate=False, **kwargs):
        return self._fill_between_x_or_y(
            "y", y, x1, x2,
            where=where, interpolate=interpolate, step=step, **kwargs)

    if _fill_between_x_or_y.__doc__:
        fill_betweenx.__doc__ = _fill_between_x_or_y.__doc__.format(
            dir="vertical", ind="y", dep="x"
        )
    fill_betweenx = _preprocess_data(
        _docstring.interpd(fill_betweenx),
        replace_names=["y", "x1", "x2", "where"])

    @_preprocess_data()
    def stairs(self, values, edges=None, *,
               orientation='vertical', baseline=0, fill=False, **kwargs):
        """
        Draw a stepwise constant function as a line or a filled plot.

        *edges* define the x-axis positions of the steps. *values* the function values
        between these steps. Depending on *fill*, the function is drawn either as a
        continuous line with vertical segments at the edges, or as a filled area.

        Parameters
        ----------
        values : array-like
            The step heights.

        edges : array-like
            The step positions, with ``len(edges) == len(vals) + 1``,
            between which the curve takes on vals values.

        orientation : {'vertical', 'horizontal'}, default: 'vertical'
            The direction of the steps. Vertical means that *values* are along
            the y-axis, and edges are along the x-axis.

        baseline : float, array-like or None, default: 0
            The bottom value of the bounding edges or when
            ``fill=True``, position of lower edge. If *fill* is
            True or an array is passed to *baseline*, a closed
            path is drawn.

            If None, then drawn as an unclosed Path.

        fill : bool, default: False
            Whether the area under the step curve should be filled.

            Passing both ``fill=True` and ``baseline=None`` will likely result in
            undesired filling: the first and last points will be connected
            with a straight line and the fill will be between this line and the stairs.


        Returns
        -------
        StepPatch : `~matplotlib.patches.StepPatch`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        **kwargs
            `~matplotlib.patches.StepPatch` properties

        """
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
        try:
            if edges is None:
                edges = np.arange(len(values) + 1)
            edge_axis, value_axis = ("x", "y") if orientation == "vertical" else ("y", "x")
            edges, values, baseline = self._process_unit_info(
                [(edge_axis, edges), (value_axis, values), (value_axis, baseline)],
                kwargs)
            # The checks of matplotlib's `StepPatch`, which takes any
            # orientation other than 'vertical' for 'horizontal'.
            mpatches.StepPatch._check_data(edges, values, baseline)
            # `StepPatch` takes masked values and edges as their data.
            native = _plotting.stairs(
                self._native, *(np.asarray(data, dtype=float).ravel() for data in (values, edges)),
                None if baseline is None else np.asarray(baseline, dtype=float),
                ('vertical' if orientation == 'vertical' else 'horizontal', bool(fill)), color)
        except Exception:
            if color is None:
                # matplotlib takes the colour from the cycle first.
                self._get_lines.get_next_color()
            raise
        patch = wrap(self, native, mpatches.StepPatch)
        patch._given_orientation = orientation
        patch._keep_data(values, edges, baseline)
        if color is None:
            patch._take_cycle_color("facecolor" if fill else "edgecolor")
        if not fill:
            # The face keeps the default colour, transparent.
            _values.remember(patch, "facecolor", None)
        patch._made_with(kwargs)
        _transform_trees.patches_added(self, [patch])
        return patch

    stackplot = _preprocess_data()(_make_axes_method(mstack.stackplot))

    def _stackplot(axes, x, *args, labels=(), colors=None, baseline='zero',
                   **kwargs):
        """`.stackplot.stackplot`, which pltrs (Rust) stacks."""
        y = np.vstack(args)
        labels = iter(labels)
        try:
            prepared = _prepared_stack(axes, x, y, colors, baseline, kwargs)
        except Exception:
            if colors is None:
                # matplotlib takes the layers' colours from the cycle first.
                for _ in y:
                    axes._get_lines.get_next_color()
            raise
        x, y, fill, kwargs, style_gen = prepared
        # pltrs (Rust) colours the layers from the cycle; given colours,
        # matplotlib's take none from it.
        position = _axes_base.cycle_position(axes._native, "lines")
        natives = _plotting.stackplot(axes._native, floats(x),
                                      [floats(row) for row in y], baseline, [], fill)
        if colors is not None:
            _axes_base.set_cycle_position(axes._native, "lines", position)
        collections = wrap_all(axes, natives, mcoll.FillBetweenPolyCollection)
        for collection in collections:
            style = next(style_gen)
            collection._made_with({**kwargs, **style})
            label = next(labels, None)
            if label is not None:
                collection.set_label(label)
        return collections

    def step(self, x, y, *args, where='pre', data=None, **kwargs):
        """
        Make a step plot.

        Call signatures::

            step(x, y, [fmt], *, data=None, where='pre', **kwargs)
            step(x, y, [fmt], x2, y2, [fmt2], ..., *, where='pre', **kwargs)

        This is just a thin wrapper around `.plot` which changes some
        formatting options. Most of the concepts and parameters of plot can be
        used here as well.

        .. note::

            This method uses a standard plot with a step drawstyle: The *x*
            values are the reference positions and steps extend left/right/both
            directions depending on *where*.

            For the common case where you know the values and edges of the
            steps, use `~.Axes.stairs` instead.

        Parameters
        ----------
        x : array-like
            1D sequence of x positions. It is assumed, but not checked, that
            it is uniformly increasing.

        y : array-like
            1D sequence of y levels.

        fmt : str, optional
            A format string, e.g. 'g' for a green line. See `.plot` for a more
            detailed description.

            Note: While full format strings are accepted, it is recommended to
            only specify the color. Line styles are currently ignored (use
            the keyword argument *linestyle* instead). Markers are accepted
            and plotted on the given positions, however, this is a rarely
            needed feature for step plots.

        where : {'pre', 'post', 'mid'}, default: 'pre'
            Define where the steps should be placed:

            - 'pre': The y value is continued constantly to the left from
              every *x* position, i.e. the interval ``(x[i-1], x[i]]`` has the
              value ``y[i]``.
            - 'post': The y value is continued constantly to the right from
              every *x* position, i.e. the interval ``[x[i], x[i+1])`` has the
              value ``y[i]``.
            - 'mid': Steps occur half-way between the *x* positions.

        data : indexable object, optional
            An object with labelled data. If given, provide the label names to
            plot in *x* and *y*.

        **kwargs
            Additional parameters are the same as those for `.plot`.

        Returns
        -------
        list of `.Line2D`
            Objects representing the plotted data.
        """
        _api.check_in_list(('pre', 'post', 'mid'), where=where)
        kwargs['drawstyle'] = 'steps-' + where
        return self.plot(x, y, *args, data=data, **kwargs)
