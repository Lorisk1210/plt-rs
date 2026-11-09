"""
Module for creating Sankey diagrams using Matplotlib.

The checks of the arguments are matplotlib's (from matplotlib 3.11.2, under
the Matplotlib License); pltrs (Rust) lays out and draws the diagrams
(``_native.sankey``).
"""

import logging
import math
from numbers import Real
import operator
from types import SimpleNamespace

import numpy as np

from . import _docstring
from . import _handles, cbook
from . import patches  # noqa: F401  (whose Patch the docstrings list)
from ._native import sankey as _sankey

_log = logging.getLogger(__name__)

__author__ = "Kevin L. Davies"
__credits__ = ["Yannick Copin"]
__license__ = "BSD"
__version__ = "2011/09/16"

# Angles [deg/90]
RIGHT = 0
UP = 1
# LEFT = 2
DOWN = 3


def _labelling_failure(sankey, flows, labels):
    """The exception matplotlib's labelling of the flows of a diagram of
    *sankey* raises on *labels* that are no strings (numbers it adds the
    quantity to), or None."""
    if sankey.unit is None or not isinstance(sankey.format, str):
        return None
    for flow, label in zip(flows, labels):
        if label is None or abs(flow) < sankey.tolerance:
            continue
        try:
            if label != '':
                label + "\n"
        except TypeError as err:
            return err
    return None


def _extent_failure(n):
    """The exception matplotlib's extent of a diagram of *n* flows raises:
    its labels' locations are empty without flows."""
    if n:
        return None
    try:
        np.min(np.zeros(0))
    except ValueError as err:
        return err
    return None


def _place_first_label(offset, flows, orientations, tolerance):
    """
    Place the label of the first flow matplotlib's `Sankey.add` places
    *offset* away from the flow's tip, as its arithmetic does, for an
    *offset* that is no number (which pltrs (Rust) cannot hold): there it
    fails with numpy's error. matplotlib places the top-side flows first,
    then the bottom-side ones from the middle outwards, the left-side
    inputs from the bottom up and the right-side outputs from the top down.
    """
    sides = []
    for flow, orient in zip(flows, orientations):
        is_input = True if flow >= tolerance else False if flow <= -tolerance else None
        if is_input is None:
            sides.append((None, None))
        elif orient == 0:
            sides.append((RIGHT, is_input))
        else:
            sides.append(({1: DOWN, -1: UP}[orient] if is_input
                          else {1: UP, -1: DOWN}[orient], is_input))
    order = [side for side in sides if side in ((DOWN, True), (UP, False))]
    order += [side for side in reversed(sides) if side in ((UP, True), (DOWN, False))]
    order += [side for side in reversed(sides) if side == (RIGHT, True)]
    order += [side for side in sides if side == (RIGHT, False)]
    if not order:
        return
    angle, is_input = order[0]
    tip = np.float64(0.0)  # A coordinate of the tip, a numpy float there.
    sign = 1 if angle == UP else -1
    if angle == RIGHT:
        location = [tip - offset if is_input else tip + offset, tip]
    else:
        location = [tip, tip - sign * offset if is_input else tip + sign * offset]
    np.zeros((1, 2))[0, :] = location


class Sankey:
    """
    Sankey diagram.

      Sankey diagrams are a specific type of flow diagram, in which
      the width of the arrows is shown proportionally to the flow
      quantity.  They are typically used to visualize energy or
      material or cost transfers between processes.
      `Wikipedia (6/1/2011) <https://en.wikipedia.org/wiki/Sankey_diagram>`_

    """

    def __init__(self, ax=None, scale=1.0, unit='', format='%G', gap=0.25,
                 radius=0.1, shoulder=0.03, offset=0.15, head_angle=100,
                 margin=0.4, tolerance=1e-6, **kwargs):
        """
        Create a new Sankey instance.

        The optional arguments listed below are applied to all subdiagrams so
        that there is consistent alignment and formatting.

        In order to draw a complex Sankey diagram, create an instance of
        `Sankey` by calling it without any kwargs::

            sankey = Sankey()

        Then add simple Sankey sub-diagrams::

            sankey.add() # 1
            sankey.add() # 2
            #...
            sankey.add() # n

        Finally, create the full diagram::

            sankey.finish()

        Or, instead, simply daisy-chain those calls::

            Sankey().add().add...  .add().finish()

        Other Parameters
        ----------------
        ax : `~matplotlib.axes.Axes`
            Axes onto which the data should be plotted.  If *ax* isn't
            provided, new Axes will be created.
        scale : float
            Scaling factor for the flows.  *scale* sizes the width of the paths
            in order to maintain proper layout.  The same scale is applied to
            all subdiagrams.  The value should be chosen such that the product
            of the scale and the sum of the inputs is approximately 1.0 (and
            the product of the scale and the sum of the outputs is
            approximately -1.0).
        unit : str
            The physical unit associated with the flow quantities.  If *unit*
            is None, then none of the quantities are labeled.
        format : str or callable
            A Python number formatting string or callable used to label the
            flows with their quantities (i.e., a number times a unit, where the
            unit is given). If a format string is given, the label will be
            ``format % quantity``. If a callable is given, it will be called
            with ``quantity`` as an argument.
        gap : float
            Space between paths that break in/break away to/from the top or
            bottom.
        radius : float
            Inner radius of the vertical paths.
        shoulder : float
            Size of the shoulders of output arrows.
        offset : float
            Text offset (from the dip or tip of the arrow).
        head_angle : float
            Angle, in degrees, of the arrow heads (and negative of the angle of
            the tails).
        margin : float
            Minimum space between Sankey outlines and the edge of the plot
            area.
        tolerance : float
            Acceptable maximum of the magnitude of the sum of flows.  The
            magnitude of the sum of connected flows cannot be greater than
            *tolerance*.
        **kwargs
            Any additional keyword arguments will be passed to `add`, which
            will create the first subdiagram.

        See Also
        --------
        Sankey.add
        Sankey.finish

        Examples
        --------
        .. plot:: gallery/specialty_plots/sankey_basics.py
        """
        # Check the arguments.
        if gap < 0:
            raise ValueError(
                "'gap' is negative, which is not allowed because it would "
                "cause the paths to overlap")
        if radius > gap:
            raise ValueError(
                "'radius' is greater than 'gap', which is not allowed because "
                "it would cause the paths to overlap")
        if head_angle < 0:
            raise ValueError(
                "'head_angle' is negative, which is not allowed because it "
                "would cause inputs to look like outputs and vice versa")
        if tolerance < 0:
            raise ValueError(
                "'tolerance' is negative, but it must be a magnitude")

        # Create Axes if necessary.
        if ax is None:
            from . import pyplot as plt
            fig = plt.figure()
            ax = fig.add_subplot(1, 1, 1, xticks=[], yticks=[])

        self.diagrams = []

        # Store the inputs.
        self.ax = ax
        self.unit = unit
        self.format = format
        self.scale = scale
        self.gap = gap
        self.radius = radius
        self.shoulder = shoulder
        self.offset = offset
        self.margin = margin
        self.pitch = np.tan(np.pi * (1 - head_angle / 180.0) / 2.0)
        self.tolerance = tolerance
        # matplotlib keeps an offset that is no number, and fails to place
        # the labels with it (see `add`).
        usable_offset = offset if isinstance(offset, Real) else 0.0
        self._native = _sankey.new(
            ax._native, (float(scale), None if unit is None else str(unit), format),
            tuple(map(float, (gap, radius, shoulder, usable_offset, head_angle,
                              margin, tolerance))))

        # If there are any kwargs, create the first subdiagram.
        if len(kwargs):
            self.add(**kwargs)

    @_docstring.interpd
    def add(self, patchlabel='', flows=None, orientations=None, labels='',
            trunklength=1.0, pathlengths=0.25, prior=None, connect=(0, 0),
            rotation=0, **kwargs):
        """
        Add a simple Sankey diagram with flows at the same hierarchical level.

        Parameters
        ----------
        patchlabel : str
            Label to be placed at the center of the diagram.
            Note that *label* (not *patchlabel*) can be passed as keyword
            argument to create an entry in the legend.

        flows : list of float
            Array of flow values.  By convention, inputs are positive and
            outputs are negative.

            Flows are placed along the top of the diagram from the inside out
            in order of their index within *flows*.  They are placed along the
            sides of the diagram from the top down and along the bottom from
            the outside in.

            If the sum of the inputs and outputs is
            nonzero, the discrepancy will appear as a cubic Bézier curve along
            the top and bottom edges of the trunk.

        orientations : list of {-1, 0, 1}
            List of orientations of the flows (or a single orientation to be
            used for all flows).  Valid values are 0 (inputs from
            the left, outputs to the right), 1 (from and to the top) or -1
            (from and to the bottom).

        labels : list of (str or None)
            List of labels for the flows (or a single label to be used for all
            flows).  Each label may be *None* (no label), or a labeling string.
            If an entry is a (possibly empty) string, then the quantity for the
            corresponding flow will be shown below the string.  However, if
            the *unit* of the main diagram is None, then quantities are never
            shown, regardless of the value of this argument.

        trunklength : float
            Length between the bases of the input and output groups (in
            data-space units).

        pathlengths : list of float
            List of lengths of the vertical arrows before break-in or after
            break-away.  If a single value is given, then it will be applied to
            the first (inside) paths on the top and bottom, and the length of
            all other arrows will be justified accordingly.  The *pathlengths*
            are not applied to the horizontal inputs and outputs.

        prior : int
            Index of the prior diagram to which this diagram should be
            connected.

        connect : (int, int)
            A (prior, this) tuple indexing the flow of the prior diagram and
            the flow of this diagram which should be connected.  If this is the
            first diagram or *prior* is *None*, *connect* will be ignored.

        rotation : float
            Angle of rotation of the diagram in degrees.  The interpretation of
            the *orientations* argument will be rotated accordingly (e.g., if
            *rotation* == 90, an *orientations* entry of 1 means to/from the
            left).  *rotation* is ignored if this diagram is connected to an
            existing one (using *prior* and *connect*).

        Returns
        -------
        Sankey
            The current `.Sankey` instance.

        Other Parameters
        ----------------
        **kwargs
           Additional keyword arguments set `matplotlib.patches.PathPatch`
           properties, listed below.  For example, one may want to use
           ``fill=False`` or ``label="A legend entry"``.

        %(Patch:kwdoc)s

        See Also
        --------
        Sankey.finish
        """
        from .patches import PathPatch
        from .text import Text
        # Check and preprocess the arguments.
        flows = np.array([1.0, -1.0]) if flows is None else np.array(flows)
        n = flows.shape[0]  # Number of flows
        if rotation is None:
            rotation = 0
        else:
            # matplotlib turns the rotation into quarter turns at once
            # (``rotation /= 90.0``), which refuses what is no number.
            operator.itruediv(rotation, 90.0)
        if orientations is None:
            orientations = 0
        try:
            orientations = np.broadcast_to(orientations, n)
        except ValueError:
            raise ValueError(
                f"The shapes of 'flows' {np.shape(flows)} and 'orientations' "
                f"{np.shape(orientations)} are incompatible"
            ) from None
        try:
            labels = np.broadcast_to(labels, n)
        except ValueError:
            raise ValueError(
                f"The shapes of 'flows' {np.shape(flows)} and 'labels' "
                f"{np.shape(labels)} are incompatible"
            ) from None
        if trunklength < 0:
            raise ValueError(
                "'trunklength' is negative, which is not allowed because it "
                "would cause poor layout")
        if abs(np.sum(flows)) > self.tolerance:
            _log.info("The sum of the flows is nonzero (%f; patchlabel=%r); "
                      "is the system not at steady state?",
                      np.sum(flows), patchlabel)
        if prior is not None:
            if prior < 0:
                raise ValueError("The index of the prior diagram is negative")
            if min(connect) < 0:
                raise ValueError(
                    "At least one of the connection indices is negative")
            if prior >= len(self.diagrams):
                raise ValueError(
                    f"The index of the prior diagram is {prior}, but there "
                    f"are only {len(self.diagrams)} other diagrams")
            if connect[0] >= len(self.diagrams[prior].flows):
                raise ValueError(
                    "The connection index to the source diagram is {}, but "
                    "that diagram has only {} flows".format(
                        connect[0], len(self.diagrams[prior].flows)))
            if connect[1] >= n:
                raise ValueError(
                    f"The connection index to this diagram is {connect[1]}, "
                    f"but this diagram has only {n} flows")
            if self.diagrams[prior].angles[connect[0]] is None:
                raise ValueError(
                    f"The connection cannot be made, which may occur if the "
                    f"magnitude of flow {connect[0]} of diagram {prior} is "
                    f"less than the specified tolerance")
            flow_error = (self.diagrams[prior].flows[connect[0]] +
                          flows[connect[1]])
            if abs(flow_error) >= self.tolerance:
                raise ValueError(
                    f"The scaled sum of the connected flows is {flow_error}, "
                    f"which is not within the tolerance ({self.tolerance})")
        for i, orient in enumerate(orientations):
            if orient not in (-1, 0, 1):
                raise ValueError(
                    f"The value of orientations[{i}] is {orient}, "
                    f"but it must be -1, 0, or 1")
        if np.iterable(pathlengths):
            if len(pathlengths) != n:
                raise ValueError(
                    f"The lengths of 'flows' ({n}) and 'pathlengths' "
                    f"({len(pathlengths)}) are incompatible")
            pathlengths = [float(length) for length in pathlengths]
        elif pathlengths is None and not np.any(orientations):
            # Only the flows on the top and bottom take a path length.
            pathlengths = [0.25]
        else:
            pathlengths = [float(pathlengths)]
        if not isinstance(self.offset, Real):
            _place_first_label(self.offset, flows, orientations, self.tolerance)

        # The outline is made with the face colour and the line width, as
        # matplotlib's: without a face colour it takes the next of the
        # property cycle.
        fc = kwargs.pop('fc', kwargs.pop('facecolor', None))
        lw = kwargs.pop('lw', kwargs.pop('linewidth', None))
        style = {key: value for key, value in (("facecolor", fc), ("linewidth", lw))
                 if value is not None}
        if prior is None and rotation is not None and rotation / 90.0 != 0:
            # matplotlib turns the diagram by the angle in radians, which
            # `math.cos` refuses when infinite.
            math.cos(math.radians(rotation / 90.0 * 90))
        # matplotlib labels the flows after it drew the patch, and measures
        # the labels' extent then: what fails there leaves the patch.
        failure = _labelling_failure(self, flows, labels) or _extent_failure(n)
        patch, flows, angles, tips, text, texts = _sankey.add(
            self._native, str(patchlabel),
            ([float(f) for f in flows], [int(o) for o in orientations],
             [None if label is None or failure else str(label) for label in labels]),
            (float(trunklength), pathlengths, prior,
             (0, 0) if prior is None else (int(connect[0]), int(connect[1])),
             float(rotation)),
            style)
        figure = self.ax.figure
        patch = _handles.wrap(patch, PathPatch, figure)
        patch._internal_update(cbook.normalize_kwargs(kwargs, PathPatch))
        if failure is not None:
            raise failure
        self.diagrams.append(SimpleNamespace(
            patch=patch, flows=np.asarray(flows),
            angles=self._kept_angles(angles, rotation, prior, connect),
            tips=tips, text=_handles.wrap(text, Text, figure),
            texts=_handles.wrap_all(texts, Text, figure)))
        return self

    def _kept_angles(self, angles, rotation, prior, connect):
        """The *angles* of the flows pltrs (Rust) placed (quarter turns), as
        matplotlib keeps them: whole numbers, advanced by the turn of the
        diagram and of its type when it turns it (a rotation given, or the
        angle of the flow of the prior diagram it connects to)."""
        if prior is None:
            turn = 0 if rotation is None else rotation / 90.0
            turned = turn != 0
        else:
            turn = self.diagrams[prior].angles[connect[0]] - 0
            turned = True
        kind = type(turn) if turned else int
        return [None if angle is None else kind(angle) for angle in angles]

    def finish(self):
        """
        Adjust the Axes and return a list of information about the Sankey
        subdiagram(s).

        Returns a list of subdiagrams with the following fields:

        ========  =============================================================
        Field     Description
        ========  =============================================================
        *patch*   Sankey outline (a `~matplotlib.patches.PathPatch`).
        *flows*   Flow values (positive for input, negative for output).
        *angles*  List of angles of the arrows [deg/90].
                  For example, if the diagram has not been rotated,
                  an input to the top side has an angle of 3 (DOWN),
                  and an output from the top side has an angle of 1 (UP).
                  If a flow has been skipped (because its magnitude is less
                  than *tolerance*), then its angle will be *None*.
        *tips*    (N, 2)-array of the (x, y) positions of the tips (or "dips")
                  of the flow paths.
                  If the magnitude of a flow is less the *tolerance* of this
                  `Sankey` instance, the flow is skipped and its tip will be at
                  the center of the diagram.
        *text*    `.Text` instance for the diagram label.
        *texts*   List of `.Text` instances for the flow labels.
        ========  =============================================================

        See Also
        --------
        Sankey.add
        """
        _sankey.finish(self._native)
        return self.diagrams


# What matplotlib's module also has from its imports, which programs
# import from it.
from .patches import PathPatch  # noqa: E402, F401
from .path import Path  # noqa: E402, F401
from .transforms import Affine2D  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
