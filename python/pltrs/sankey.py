"""
Module for creating Sankey diagrams using Matplotlib.

The checks of the arguments are matplotlib's (from matplotlib 3.11.2, under
the Matplotlib License); pltrs (Rust) lays out and draws the diagrams
(``_native.sankey``).
"""

import logging
from types import SimpleNamespace

import numpy as np

from . import _handles, cbook
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
            Scaling factor for the flows.
        unit : str
            The physical unit associated with the flow quantities.  If *unit*
            is None, then none of the quantities are labeled.
        format : str or callable
            A Python number formatting string or callable used to label the
            flows with their quantities.
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
        self._native = _sankey.new(
            ax._native, (float(scale), None if unit is None else str(unit), format),
            tuple(map(float, (gap, radius, shoulder, offset, head_angle, margin,
                              tolerance))))

        # If there are any kwargs, create the first subdiagram.
        if len(kwargs):
            self.add(**kwargs)

    def add(self, patchlabel='', flows=None, orientations=None, labels='',
            trunklength=1.0, pathlengths=0.25, prior=None, connect=(0, 0),
            rotation=0, **kwargs):
        """
        Add a simple Sankey diagram with flows at the same hierarchical level.

        Parameters
        ----------
        patchlabel : str
            Label to be placed at the center of the diagram.
        flows : list of float
            Array of flow values.  By convention, inputs are positive and
            outputs are negative.
        orientations : list of {-1, 0, 1}
            List of orientations of the flows (or a single orientation to be
            used for all flows).
        labels : list of (str or None)
            List of labels for the flows (or a single label to be used for all
            flows).
        trunklength : float
            Length between the bases of the input and output groups (in
            data-space units).
        pathlengths : list of float
            List of lengths of the vertical arrows before break-in or after
            break-away.
        prior : int
            Index of the prior diagram to which this diagram should be
            connected.
        connect : (int, int)
            A (prior, this) tuple indexing the flow of the prior diagram and
            the flow of this diagram which should be connected.
        rotation : float
            Angle of rotation of the diagram in degrees.

        Returns
        -------
        Sankey
            The current `.Sankey` instance.

        Other Parameters
        ----------------
        **kwargs
           Additional keyword arguments set `matplotlib.patches.PathPatch`
           properties.
        """
        from .patches import PathPatch
        from .text import Text
        # Check and preprocess the arguments.
        flows = np.array([1.0, -1.0]) if flows is None else np.array(flows)
        n = flows.shape[0]  # Number of flows
        rotation = 0 if rotation is None else rotation
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
        else:
            pathlengths = [float(pathlengths)]

        # The outline is made with the face colour and the line width, as
        # matplotlib's: without a face colour it takes the next of the
        # property cycle.
        fc = kwargs.pop('fc', kwargs.pop('facecolor', None))
        lw = kwargs.pop('lw', kwargs.pop('linewidth', None))
        style = {key: value for key, value in (("facecolor", fc), ("linewidth", lw))
                 if value is not None}
        patch, flows, angles, tips, text, texts = _sankey.add(
            self._native, str(patchlabel),
            ([float(f) for f in flows], [int(o) for o in orientations],
             [None if label is None else str(label) for label in labels]),
            (float(trunklength), pathlengths, prior,
             (int(connect[0]), int(connect[1])), float(rotation)),
            style)
        figure = self.ax.figure
        patch = _handles.wrap(patch, PathPatch, figure)
        patch._internal_update(cbook.normalize_kwargs(kwargs, PathPatch))
        self.diagrams.append(SimpleNamespace(
            patch=patch, flows=np.asarray(flows),
            angles=[None if angle is None else int(angle) for angle in angles],
            tips=tips, text=_handles.wrap(text, Text, figure),
            texts=_handles.wrap_all(texts, Text, figure)))
        return self

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
