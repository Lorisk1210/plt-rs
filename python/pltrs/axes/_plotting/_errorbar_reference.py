"""
matplotlib's `~.Axes.errorbar`, for an Axes class of the program's that
overrides a plotting method matplotlib's draws the bars with
(`~.Axes.vlines`, `~.Axes.hlines`: ultraplot adds sticky edges there),
which pltrs (Rust) makes itself otherwise (matplotlib 3.11's code, nearly
as it is, under the Matplotlib License).
"""

import numpy as np

from ... import _val_or_rc, cbook, lines as mlines, markers as mmarkers
from ... import transforms as mtransforms
from ...transforms import _ScaledRotation
from ...container import ErrorbarContainer
from .errorbar import _errorevery_to_mask


def errorbar(axes, x, y, yerr=None, xerr=None,
             fmt='', ecolor=None, elinewidth=None, capsize=None,
             barsabove=False, lolims=False, uplims=False,
             xlolims=False, xuplims=False, errorevery=1,
             capthick=None, elinestyle=None,
             **kwargs):
    """`~.Axes.errorbar` of *axes* as matplotlib makes it: the bars made
    through its `~.Axes.vlines` and `~.Axes.hlines`."""
    kwargs = cbook.normalize_kwargs(kwargs, mlines.Line2D)
    # Drop anything that comes in as None to use the default instead.
    kwargs = {k: v for k, v in kwargs.items() if v is not None}
    kwargs.setdefault('zorder', 2)

    # Casting to object arrays preserves units.
    if not isinstance(x, np.ndarray):
        x = np.asarray(x, dtype=object)
    if not isinstance(y, np.ndarray):
        y = np.asarray(y, dtype=object)

    def _upcast_err(err):
        """
        Safely handle tuple of containers that carry units.

        This function covers the case where the input to the xerr/yerr is a
        length 2 tuple of equal length ndarray-subclasses that carry the
        unit information in the container.

        If we have a tuple of nested numpy array (subclasses), we defer
        coercing the units to be consistent to the underlying unit
        library (and implicitly the broadcasting).

        Otherwise, fallback to casting to an object array.
        """

        if (
                # make sure it is not a scalar
                np.iterable(err) and
                # and it is not empty
                len(err) > 0 and
                # and the first element is an array sub-class use
                # safe_first_element because getitem is index-first not
                # location first on pandas objects so err[0] almost always
                # fails.
                isinstance(cbook._safe_first_finite(err), np.ndarray)
        ):
            # Get the type of the first element
            atype = type(cbook._safe_first_finite(err))
            # Promote the outer container to match the inner container
            if atype is np.ndarray:
                # Converts using np.asarray, because data cannot
                # be directly passed to init of np.ndarray
                return np.asarray(err, dtype=object)
            # If atype is not np.ndarray, directly pass data to init.
            # This works for types such as unyts and astropy units
            return atype(err)
        # Otherwise wrap it in an object array
        return np.asarray(err, dtype=object)

    if xerr is not None and not isinstance(xerr, np.ndarray):
        xerr = _upcast_err(xerr)
    if yerr is not None and not isinstance(yerr, np.ndarray):
        yerr = _upcast_err(yerr)
    x, y = np.atleast_1d(x, y)  # Make sure all the args are iterable.
    if len(x) != len(y):
        raise ValueError("'x' and 'y' must have the same size")

    everymask = _errorevery_to_mask(x, errorevery)

    label = kwargs.pop("label", None)
    kwargs['label'] = '_nolegend_'

    # Create the main line and determine overall kwargs for child artists.
    # We avoid calling axes.plot() directly, or axes._get_lines(), because
    # that would call axes._process_unit_info again, and do other indirect
    # data processing.
    (data_line, base_style), = axes._get_lines._plot_args(
        axes, (x, y) if fmt == '' else (x, y, fmt), kwargs, return_kwargs=True)

    # Do this after creating `data_line` to avoid modifying `base_style`.
    if barsabove:
        data_line.set_zorder(kwargs['zorder'] - .1)
    else:
        data_line.set_zorder(kwargs['zorder'] + .1)

    # Add line to plot, or throw it away and use it to determine kwargs.
    if fmt.lower() != 'none':
        axes.add_line(data_line)
    else:
        data_line = None
        # Remove alpha=0 color that _get_lines._plot_args returns for
        # 'none' format, and replace it with user-specified color, if
        # supplied.
        base_style.pop('color')
        if 'color' in kwargs:
            base_style['color'] = kwargs.pop('color')

    if 'color' not in base_style:
        base_style['color'] = 'C0'
    if ecolor is None:
        ecolor = base_style['color']

    # Eject any line-specific information from format string, as it's not
    # needed for bars or caps.
    for key in ['marker', 'markersize', 'markerfacecolor',
                'markerfacecoloralt',
                'markeredgewidth', 'markeredgecolor', 'markevery',
                'linestyle', 'fillstyle', 'drawstyle', 'dash_capstyle',
                'dash_joinstyle', 'solid_capstyle', 'solid_joinstyle',
                'dashes']:
        base_style.pop(key, None)

    # Make the style dict for the line collections (the bars).
    eb_lines_style = {**base_style, 'color': ecolor}
    elinewidth = _val_or_rc(elinewidth, "errorbar.elinewidth")

    if elinewidth is not None:
        eb_lines_style['linewidth'] = elinewidth
    elif 'linewidth' in kwargs:
        eb_lines_style['linewidth'] = kwargs['linewidth']

    for key in ('transform', 'alpha', 'zorder', 'rasterized'):
        if key in kwargs:
            eb_lines_style[key] = kwargs[key]

    if elinestyle is not None:
        eb_lines_style['linestyle'] = elinestyle

    # Make the style dict for caps (the "hats").
    eb_cap_style = {**base_style, 'linestyle': 'none'}
    capsize = _val_or_rc(capsize, "errorbar.capsize")
    capthick = _val_or_rc(capthick, "errorbar.capthick")

    if capsize > 0:
        eb_cap_style['markersize'] = 2. * capsize
    if capthick is not None:
        eb_cap_style['markeredgewidth'] = capthick

    # For backwards-compat, allow explicit setting of
    # 'markeredgewidth' to over-ride capthick.
    for key in ('markeredgewidth', 'transform', 'alpha',
                'zorder', 'rasterized'):
        if key in kwargs:
            eb_cap_style[key] = kwargs[key]
    eb_cap_style["markeredgecolor"] = ecolor

    barcols = []
    caplines = {'x': [], 'y': []}

    # Vectorized fancy-indexer.
    def apply_mask(arrays, mask):
        return [array[mask] for array in arrays]

    # dep: dependent dataset, indep: independent dataset
    for (dep_axis, dep, err, lolims, uplims, indep, lines_func,
         marker, lomarker, himarker) in [
            ("x", x, xerr, xlolims, xuplims, y, axes.hlines,
             "|", mlines.CARETRIGHTBASE, mlines.CARETLEFTBASE),
            ("y", y, yerr, lolims, uplims, x, axes.vlines,
             "_", mlines.CARETUPBASE, mlines.CARETDOWNBASE),
    ]:
        if err is None:
            continue
        lolims = np.broadcast_to(lolims, len(dep)).astype(bool)
        uplims = np.broadcast_to(uplims, len(dep)).astype(bool)
        try:
            np.broadcast_to(err, (2, len(dep)))
        except ValueError:
            raise ValueError(
                f"'{dep_axis}err' (shape: {np.shape(err)}) must be a "
                f"scalar or a 1D or (2, n) array-like whose shape matches "
                f"'{dep_axis}' (shape: {np.shape(dep)})") from None
        if err.dtype is np.dtype(object) and np.any(err == None):  # noqa: E711
            raise ValueError(
                f"'{dep_axis}err' must not contain None. "
                "Use NaN if you want to skip a value.")

        # Raise if any errors are negative, but not if they are nan.
        # To avoid nan comparisons (which lead to warnings on some
        # platforms), we select with `err==err` (which is False for nan).
        # Also, since datetime.timedelta cannot be compared with 0,
        # we compare with the negative error instead.
        if np.any((check := err[err == err]) < -check):
            raise ValueError(
                f"'{dep_axis}err' must not contain negative values")
        # This is like
        #     elow, ehigh = np.broadcast_to(...)
        #     return dep - elow * ~lolims, dep + ehigh * ~uplims
        # except that broadcast_to would strip units.
        low, high = dep + np.vstack([-(1 - lolims), 1 - uplims]) * err
        barcols.append(lines_func(
            *apply_mask([indep, low, high], everymask), **eb_lines_style))
        if axes.name == "polar" and dep_axis == "x":
            for b in barcols:
                for p in b.get_paths():
                    p._interpolation_steps = 2
        # Normal errorbars for points without upper/lower limits.
        nolims = ~(lolims | uplims)
        if nolims.any() and capsize > 0:
            indep_masked, lo_masked, hi_masked = apply_mask(
                [indep, low, high], nolims & everymask)
            for lh_masked in [lo_masked, hi_masked]:
                # Since this has to work for x and y as dependent data, we
                # first set both x and y to the independent variable and
                # overwrite the respective dependent data in a second step.
                line = mlines.Line2D(indep_masked, indep_masked,
                                     marker=marker, **eb_cap_style)
                line.set(**{f"{dep_axis}data": lh_masked})
                caplines[dep_axis].append(line)
        for idx, (lims, hl) in enumerate([(lolims, high), (uplims, low)]):
            if not lims.any():
                continue
            hlmarker = (
                himarker
                if axes._axis_map[dep_axis].get_inverted() ^ idx
                else lomarker)
            x_masked, y_masked, hl_masked = apply_mask(
                [x, y, hl], lims & everymask)
            # As above, we set the dependent data in a second step.
            line = mlines.Line2D(x_masked, y_masked,
                                 marker=hlmarker, **eb_cap_style)
            line.set(**{f"{dep_axis}data": hl_masked})
            caplines[dep_axis].append(line)
            if capsize > 0:
                caplines[dep_axis].append(mlines.Line2D(
                    x_masked, y_masked, marker=marker, **eb_cap_style))
    if axes.name == 'polar':
        trans_shift = axes.transShift
        for axis in caplines:
            for l in caplines[axis]:
                # Rotate caps to be perpendicular to the error bars
                for theta, r in zip(l.get_xdata(), l.get_ydata()):
                    rotation = _ScaledRotation(theta=theta, trans_shift=trans_shift)
                    if axis == 'y':
                        rotation += mtransforms.Affine2D().rotate(np.pi / 2)
                    ms = mmarkers.MarkerStyle(marker=marker,
                                              transform=rotation)
                    axes.add_line(mlines.Line2D([theta], [r], marker=ms,
                                                **eb_cap_style))
    else:
        for axis in caplines:
            for l in caplines[axis]:
                axes.add_line(l)

    axes._request_autoscale_view()
    caplines = caplines['x'] + caplines['y']
    errorbar_container = ErrorbarContainer(
        (data_line, tuple(caplines), tuple(barcols)),
        has_xerr=(xerr is not None), has_yerr=(yerr is not None),
        label=label)
    axes.add_container(errorbar_container)

    return errorbar_container  # (l0, caplines, barcols)
