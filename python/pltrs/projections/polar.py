"""
Polar axes (matplotlib's ``projections.polar``): plt-rs axes in the polar
projection, made by ``add_subplot(projection='polar')``.
"""

from ..axes import Axes


class PolarAxes(Axes):
    """
    A polar graph projection, where the input dimensions are *theta*, *r*.

    Theta starts pointing east and goes anti-clockwise.
    """
    name = 'polar'
