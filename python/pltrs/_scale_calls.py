"""
How matplotlib calls the transform of a scale of the program's, for
pltrs (Rust).

The scale's transform maps the values of one coordinate as part of the
axes' ``transScale``, made once with the scale (``axis.get_transform()``).
Drawing and measuring map them through its non-affine part: its
``transform_non_affine``, handed a row of values (``values[:, 1]`` of the
points a blended transform maps). Autoscaling asks the axis for the
transform again (``axis.get_transform()``, which a scale of the program's
may make anew), maps the limits as a column through its ``transform`` and
back through its ``inverted()``. pltrs (Rust) hands a row where drawing
maps values and a column where matplotlib calls ``transform``
(`scale.ArrayLayout`), so the functions it calls tell the two apart by the
shape of what they are given.
"""

import numpy as np


class ScaleCalls:
    """The forward and inverse functions pltrs (Rust) maps through for the
    scale *scale* of the program's, whose ``transScale`` holds
    *transform*."""

    def __init__(self, scale, transform):
        #: The scale, or what pltrs holds for it (`._program_classes.held`).
        self._scale = scale
        self._transform = transform
        #: The inverse of *transform*, made when first needed.
        self._inverse = None
        #: The transform autoscaling asked the axis for last.
        self._autoscaling = transform

    def forward(self, values):
        """*values* through the scale: a row as drawing maps it, a column
        as autoscaling does, through the transform it asks for."""
        if np.ndim(values) == 1:
            return self._transform.transform_non_affine(values)
        from ._program_classes import unheld
        self._autoscaling = unheld(self._scale).get_transform()
        return self._autoscaling.transform(values)

    def inverse(self, values):
        """*values* back through the scale: a row through the inverse of
        the transform, a column through the inverse of the transform
        autoscaling asked for last."""
        if np.ndim(values) == 1:
            if self._inverse is None:
                self._inverse = self._transform.inverted()
            return self._inverse.transform_non_affine(values)
        return self._autoscaling.inverted().transform(values)
