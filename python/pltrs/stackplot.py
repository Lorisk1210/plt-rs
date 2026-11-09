"""
Stacked area plot for 1D arrays inspired by Douglas Y'barbo's stackoverflow
answer:
https://stackoverflow.com/q/2225995/

(https://stackoverflow.com/users/66549/doug)

pltrs keeps matplotlib 3.11.2's module (under the Matplotlib License): its
function is `.Axes.stackplot`, which pltrs (Rust) stacks.
"""

__all__ = ['stackplot']


def stackplot(axes, x, *args,
              labels=(), colors=None, baseline='zero',
              **kwargs):
    """
    Draw a stacked area plot or a streamgraph.

    Parameters
    ----------
    x : (N,) array-like

    y : (M, N) array-like
        The data can be either stacked or unstacked. Each of the following
        calls is legal::

            stackplot(x, y)  # where y has shape (M, N) e.g. y = [y1, y2, y3, y4]
            stackplot(x, y1, y2, y3, y4)  # where y1, y2, y3, y4 have length N

    baseline : {'zero', 'sym', 'wiggle', 'weighted_wiggle'}
        Method used to calculate the baseline:

        - ``'zero'``: Constant zero baseline, i.e. a simple stacked plot.
        - ``'sym'``:  Symmetric around zero and is sometimes called
          'ThemeRiver'.
        - ``'wiggle'``: Minimizes the sum of the squared slopes.
        - ``'weighted_wiggle'``: Does the same but weights to account for
          size of each layer. It is also called 'Streamgraph'-layout. More
          details can be found at http://leebyron.com/streamgraph/.

    labels : list of str, optional
        A sequence of labels to assign to each data series. If unspecified,
        then no labels will be applied to artists.

    colors : list of :mpltype:`color`, optional
        A sequence of colors to be cycled through and used to color the stacked
        areas. The sequence need not be exactly the same length as the number
        of provided *y*, in which case the colors will repeat from the
        beginning.

        If not specified, the colors from the Axes property cycle will be used.

    data : indexable object, optional
        DATA_PARAMETER_PLACEHOLDER

    **kwargs
        All other keyword arguments are passed to `.Axes.fill_between`.  The
            following parameters additionally accept a sequence of values
            corresponding to the *y* datasets:

            - *hatch*
            - *edgecolor*
            - *facecolor*
            - *linewidth*
            - *linestyle*

            .. versionadded:: 3.9
               Allowing a sequence of strings for *hatch*.

            .. versionadded:: 3.11
               Allowing sequences of values in above listed `.Axes.fill_between`
               parameters.

    Returns
    -------
    list of `.PolyCollection`
        A list of `.PolyCollection` instances, one for each element in the
        stacked area plot.
    """
    if not hasattr(axes, "_stackplot"):
        # matplotlib's function stacks the data, then asks *axes* for the
        # colours of the layers (else for its `fill_between`).
        y = np.vstack(args)
        if colors is None:
            [axes._get_lines.get_next_color() for _ in y]
        axes.fill_between  # noqa: B018
    return axes._stackplot(x, *args, labels=labels, colors=colors,
                           baseline=baseline, **kwargs)


# What matplotlib's module also has from its imports, which programs
# import from it.
import numpy as np  # noqa: E402, F401
from . import cbook, collections  # noqa: E402, F401
