"""
matplotlib's trees of the transforms pltrs (Rust) resolves (``ax.transData``,
``ax.transAxes``, ``fig.transFigure``, ``fig.dpi_scale_trans``), built of
matplotlib's transform classes around the values the figure has now: what
``str`` shows of a live transform (`._live_transforms.LiveTransform`), as
matplotlib shows its own trees.

An axes class with transforms of its own gives their trees through its
``_transform_tree(name)`` (polar axes for ``transData``), None leaving the
name to this module.
"""

from .transforms import (
    Affine2D, Bbox, BboxTransformFrom, BboxTransformTo, TransformedBbox,
    TransformWrapper, blended_transform_factory)


def tree(owner, name):
    """
    matplotlib's tree of the live transform *name* of *owner* (an axes or a
    figure), or None for a transform matplotlib has no tree of here.
    """
    special = getattr(owner, "_transform_tree", None)
    made = None if special is None else special(name)
    if made is not None:
        return made
    builder = _BUILDERS.get(name)
    return None if builder is None else builder(owner)


def dpi_scale_trans(figure):
    """``fig.dpi_scale_trans``: inches to display pixels."""
    return Affine2D().scale(figure.get_figure(root=True).dpi)


def trans_subfigure(figure):
    """``fig.transSubfigure`` of a root figure (its ``transFigure``):
    fractions of the figure to display pixels; None for a subfigure."""
    if figure is not figure.get_figure(root=True):
        return None
    return BboxTransformTo(TransformedBbox(figure.bbox_inches, dpi_scale_trans(figure)))


def trans_scale(ax):
    """``ax.transScale``: the scales of the x and y axes."""
    return TransformWrapper(blended_transform_factory(
        ax.xaxis.get_transform(), ax.yaxis.get_transform()))


def trans_axes(ax):
    """``ax.transAxes``: fractions of the Axes to display pixels (None in
    a subfigure)."""
    figure = trans_subfigure(ax.get_figure(root=False))
    if figure is None:
        return None
    return BboxTransformTo(TransformedBbox(ax.get_position(original=True), figure))


def trans_data(ax):
    """``ax.transData`` of rectilinear Axes: data to display pixels."""
    axes = trans_axes(ax)
    if axes is None:
        return None
    scale = trans_scale(ax)
    limits = BboxTransformFrom(TransformedBbox(Bbox(ax.viewLim.get_points()), scale))
    return scale + (limits + axes)


_BUILDERS = {
    "dpi_scale_trans": dpi_scale_trans,
    "transSubfigure": trans_subfigure,
    "transAxes": trans_axes,
    "transData": trans_data,
}
