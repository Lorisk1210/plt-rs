"""
Routines to adjust subplot params so that subplots are nicely fit in the
figure (matplotlib's ``_tight_layout``): the helper that works on Python
objects, `get_subplotspec_list`, and `get_tight_layout_figure`, which
`.layout_engine` re-exports. The layout itself is pltrs (Rust)'s
(``pltrs::layout``, through `.TightLayoutEngine`, `.Figure.tight_layout`
and `get_tight_layout_figure`).

From matplotlib 3.11.2, under the Matplotlib License.
"""

from . import _api
from ._native import figure_layout as _figure_layout


def get_subplotspec_list(axes_list, grid_spec=None):
    """
    Return a list of subplotspec from the given list of Axes.

    For an instance of Axes that does not support subplotspec, None is inserted
    in the list.

    If grid_spec is given, None is inserted for those not from the given
    grid_spec.
    """
    subplotspec_list = []
    for ax in axes_list:
        axes_or_locator = ax.get_axes_locator()
        if axes_or_locator is None:
            axes_or_locator = ax

        if hasattr(axes_or_locator, "get_subplotspec"):
            subplotspec = axes_or_locator.get_subplotspec()
            if subplotspec is not None:
                subplotspec = subplotspec.get_topmost_subplotspec()
                gs = subplotspec.get_gridspec()
                if grid_spec is not None:
                    if gs != grid_spec:
                        subplotspec = None
                elif gs.locally_modified_subplot_params():
                    subplotspec = None
        else:
            subplotspec = None

        subplotspec_list.append(subplotspec)

    return subplotspec_list


def get_tight_layout_figure(fig, axes_list, subplotspec_list, renderer,
                            pad=1.08, h_pad=None, w_pad=None, rect=None):
    """
    Return subplot parameters for tight-layouted-figure with specified padding.

    Parameters
    ----------
    fig : Figure
    axes_list : list of Axes
    subplotspec_list : list of `.SubplotSpec`
        The subplotspecs of each Axes.
    renderer : renderer
    pad : float
        Padding between the figure edge and the edges of subplots, as a
        fraction of the font size.
    h_pad, w_pad : float
        Padding (height/width) between edges of adjacent subplots.  Defaults to
        *pad*.
    rect : tuple (left, bottom, right, top), default: None.
        rectangle in normalized figure coordinates
        that the whole subplots area (including labels) will fit into.
        Defaults to using the entire figure.

    Returns
    -------
    subplotspec or None
        subplotspec kwargs to be passed to `.Figure.subplots_adjust` or
        None if tight_layout could not be accomplished.
    """
    # pltrs (Rust) groups the Axes by their topmost subplot specifications,
    # measures them with the figure's renderer and fits the parameters, as
    # matplotlib's does with *subplotspec_list*, leaving the figure as it is.
    if None in subplotspec_list:
        _api.warn_external(
            "This figure includes Axes that are not compatible with "
            "tight_layout, so results might be incorrect.")
    axes = [ax for ax, ss in zip(axes_list, subplotspec_list) if ss is not None]
    if not axes:
        return {}
    kwargs, not_applied = _figure_layout.tight_layout_params(
        fig._native, [ax._native for ax in axes], float(pad), h_pad, w_pad, rect)
    if not_applied is not None:
        _api.warn_external(not_applied)
    return None if kwargs is None else dict(kwargs)
