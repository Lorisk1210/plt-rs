"""
What matplotlib's patch classes compute with the arguments they are given,
for the patches to compute it with values pltrs (Rust) does not hold.

matplotlib's patch constructors keep most of what they are given and
compute with it as they are made or drawn, so that a value that is no
number fails with the error Python's or numpy's arithmetic raises there,
and one that is not finite warns as numpy does. The patches hand pltrs
(Rust) the numbers it holds (see `._kept_geometry`) and run this
arithmetic, ported from matplotlib 3.11.2's ``patches.py`` and
``path.py`` under the Matplotlib License, for the others.
"""

from numbers import Real

import numpy as np

from .path import Path


def regular_polygon(num_vertices):
    """
    The number of sides pltrs (Rust) makes a regular polygon of
    *num_vertices* vertices with, and matplotlib's path of it
    (``Path.unit_regular_polygon``) when that is no whole number of
    vertices (a number below one or with a fraction), else None; failing
    as matplotlib fails to make the path.
    """
    num_vertices <= 16  # noqa: B015  (matplotlib's cache is for 16 at most)
    if (isinstance(num_vertices, Real) and num_vertices >= 1
            and float(num_vertices).is_integer()):
        return int(num_vertices), None
    theta = ((2 * np.pi / num_vertices) * np.arange(num_vertices + 1)
             # This initial rotation is to make sure the polygon always
             # "points-up".
             + np.pi / 2)
    verts = np.column_stack((np.cos(theta), np.sin(theta)))
    return max(len(verts) - 1, 0), Path(verts, closed=True, readonly=True)


def place_point(patch, xy, s, axes=None):
    """The pixel position of the point *xy* of the `.ConnectionPatch`
    *patch* in the coordinates *s* (of *axes*, else the patch's), as
    matplotlib's ``ConnectionPatch._get_xy`` computes it."""
    from . import cbook, transforms
    s0 = s  # For the error message, if needed.
    if axes is None:
        axes = patch.axes

    # preserve mixed type input (such as str, int)
    x = np.array(xy[0])
    y = np.array(xy[1])

    fig = patch.get_figure(root=False)
    if s in ["figure points", "axes points"]:
        x = x * fig.dpi / 72
        y = y * fig.dpi / 72
        s = s.replace("points", "pixels")
    elif s == "figure fraction":
        s = fig.transFigure
    elif s == "subfigure fraction":
        s = fig.transSubfigure
    elif s == "axes fraction":
        s = axes.transAxes

    if s == 'data':
        trans = axes.transData
        x = cbook._to_unmasked_float_array(axes.xaxis.convert_units(x))
        y = cbook._to_unmasked_float_array(axes.yaxis.convert_units(y))
        return trans.transform((x, y))
    elif s == 'offset points':
        if patch.xycoords == 'offset points':  # prevent recursion
            return place_point(patch, patch.xy, 'data')
        return (
            place_point(patch, patch.xy, patch.xycoords)  # converted data point
            + xy * patch.get_figure(root=True).dpi / 72)  # converted offset
    elif s == 'polar':
        theta, r = x, y
        x = r * np.cos(theta)
        y = r * np.sin(theta)
        trans = axes.transData
        return trans.transform((x, y))
    elif s == 'figure pixels':
        # pixels from the lower left corner of the figure
        bb = patch.get_figure(root=False).figbbox
        x = bb.x0 + x if x >= 0 else bb.x1 + x
        y = bb.y0 + y if y >= 0 else bb.y1 + y
        return x, y
    elif s == 'subfigure pixels':
        # pixels from the lower left corner of the figure
        bb = patch.get_figure(root=False).bbox
        x = bb.x0 + x if x >= 0 else bb.x1 + x
        y = bb.y0 + y if y >= 0 else bb.y1 + y
        return x, y
    elif s == 'axes pixels':
        # pixels from the lower left corner of the Axes
        bb = axes.bbox
        x = bb.x0 + x if x >= 0 else bb.x1 + x
        y = bb.y0 + y if y >= 0 else bb.y1 + y
        return x, y
    elif isinstance(s, transforms.Transform):
        return s.transform(xy)
    else:
        raise ValueError(f"{s0} is not a valid coordinate transformation")


def plain_fancy_arrow(x, y, dx, dy, width, head_width, head_length, shape, overhang):
    """Whether pltrs (Rust) makes the `.FancyArrow` of these arguments as
    matplotlib does: finite numbers (or None for the head's size) and a
    shape it knows."""
    numbers = [x, y, dx, dy, width, overhang,
               *(size for size in (head_width, head_length) if size is not None)]
    return (all(isinstance(value, Real) and np.isfinite(value) for value in numbers)
            and isinstance(shape, str) and shape in ('full', 'left', 'right'))


def fancy_arrow_verts(arrow):
    """The vertices of the polygon of the `.FancyArrow` *arrow* as
    matplotlib's ``_make_verts`` computes them from what the arrow keeps
    (``_x``, ``_dx``, ...), failing and warning as it does."""
    if arrow._head_width is None:
        head_width = 3 * arrow._width
    else:
        head_width = arrow._head_width
    if arrow._head_length is None:
        head_length = 1.5 * head_width
    else:
        head_length = arrow._head_length

    distance = np.hypot(arrow._dx, arrow._dy)

    if arrow._length_includes_head:
        length = distance
    else:
        length = distance + head_length
    if np.size(length) == 0:
        return np.empty([0, 2])  # display nothing if empty
    # start by drawing horizontal arrow, point at (0, 0)
    hw, hl = head_width, head_length
    hs, lw = arrow._overhang, arrow._width
    left_half_arrow = np.array([
        [0.0, 0.0],                 # tip
        [-hl, -hw / 2],             # leftmost
        [-hl * (1 - hs), -lw / 2],  # meets stem
        [-length, -lw / 2],         # bottom left
        [-length, 0],
    ])
    # if we're not including the head, shift up by head length
    if not arrow._length_includes_head:
        left_half_arrow += [head_length, 0]
    # if the head starts at 0, shift up by another head length
    if arrow._head_starts_at_zero:
        left_half_arrow += [head_length / 2, 0]
    # figure out the shape, and complete accordingly
    if arrow._shape == 'left':
        coords = left_half_arrow
    else:
        right_half_arrow = left_half_arrow * [1, -1]
        if arrow._shape == 'right':
            coords = right_half_arrow
        elif arrow._shape == 'full':
            # The half-arrows contain the midpoint of the stem,
            # which we can omit from the full arrow. Including it
            # twice caused a problem with xpdf.
            coords = np.concatenate([left_half_arrow[:-1],
                                     right_half_arrow[-2::-1]])
        else:
            raise ValueError(f"Got unknown shape: {arrow._shape!r}")
    if distance != 0:
        cx = arrow._dx / distance
        sx = arrow._dy / distance
    else:
        # Account for division by zero
        cx, sx = 0, 1
    M = [[cx, sx], [-sx, cx]]
    return np.dot(coords, M) + [
        arrow._x + arrow._dx,
        arrow._y + arrow._dy,
    ]


def fancy_box_path(patch):
    """
    What matplotlib's ``FancyBboxPatch.get_path`` computes with the box
    the `.FancyBboxPatch` *patch* keeps, failing as it does: the box
    squeezed by the mutation aspect, then padded by its box style (the
    arithmetic matplotlib's styles start with), or the program's style
    called.
    """
    from ._patch_styles import BoxStyle
    boxstyle = patch.get_boxstyle()
    m_aspect = patch.get_mutation_aspect()
    # Call boxstyle with y, height squeezed by aspect_ratio.
    x0, y0 = patch._x, patch._y / m_aspect
    width, height = patch._width, patch._height / m_aspect
    mutation_size = patch.get_mutation_scale()
    if not isinstance(boxstyle, BoxStyle._Base):
        boxstyle(x0, y0, width, height, mutation_size)
        return
    pad = mutation_size * getattr(boxstyle, "pad", 0.)
    # width and height with padding added.
    width, height = width + 2 * pad, height + 2 * pad
    # boundary of the padded box
    x0, y0 = x0 - pad, y0 - pad
    x0 + width, y0 + height  # noqa: B018


def arc_angles(width, height, theta1, theta2):
    """What matplotlib's `.Arc` computes with its angles *theta1* and
    *theta2* (degrees) as it is made, failing and warning as it does: the
    angles stretched by the arc's *width* and *height* (``_theta_stretch``),
    then the number of segments of its path (``Path.arc``)."""
    def theta_stretch(theta, scale):
        theta = np.deg2rad(theta)
        x = np.cos(theta)
        y = np.sin(theta)
        stheta = np.rad2deg(np.arctan2(scale * y, x))
        # arctan2 has the range [-pi, pi], we expect [0, 2*pi]
        return (stheta + 360) % 360

    if (
        # if we need to stretch the angles because we are distorted
        width != height
        # and we are not doing a full circle.
        and not (theta1 != theta2 and theta1 % 360 == theta2 % 360)
    ):
        theta1 = theta_stretch(theta1, width / height)
        theta2 = theta_stretch(theta2, width / height)
    halfpi = np.pi * 0.5
    eta1 = theta1
    n_turns = (theta2 - theta1) / 360
    nearest_turn = np.rint(n_turns)
    is_full_circle = nearest_turn != 0 and abs(n_turns - nearest_turn) <= 1e-12
    if is_full_circle:
        eta2 = theta1 + 360
    else:
        eta2 = theta2 - 360 * np.floor(n_turns)
    eta1, eta2 = np.deg2rad([eta1, eta2])
    # number of curve segments to make
    int(2 ** np.ceil((eta2 - eta1) / halfpi))


def arrow_transform(arrow):
    """The patch transform of the `.Arrow` *arrow* as matplotlib's
    ``set_data`` builds it from what the arrow keeps (``_x``, ``_dx``,
    ...), failing and warning as it does."""
    from .transforms import Affine2D
    return (Affine2D()
            .scale(np.hypot(arrow._dx, arrow._dy), arrow._width)
            .rotate(np.arctan2(arrow._dy, arrow._dx))
            .translate(arrow._x, arrow._y)
            .frozen())
