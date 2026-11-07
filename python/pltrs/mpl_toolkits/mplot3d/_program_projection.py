"""
Patches and collections of the program's classes that project themselves
onto a 3-D axes: ``do_3d_projection`` defined by a subclass, such as the
``Arrow3D`` of many programs (a `.FancyArrowPatch` placing its ends where
its 3-D points project through ``axes.M``).

matplotlib's ``Axes3D.draw`` calls ``do_3d_projection`` on every visible
collection and patch of the axes before drawing them, and orders them by
the depths returned. pltrs (Rust) projects its own 3-D artists itself; an
artist of the program's is drawn through Python (`._python_drawing`), its
projection runs once the figure is laid out, before it is drawn, and the
depth it returns orders it among the others
(``_native.mplot3d.set_program_depth``).
"""

from pltrs import _python_drawing
from pltrs._native import mplot3d as _mplot3d


def follow(axes, artist):
    """Have the draws of *axes* call the ``do_3d_projection`` of *artist*,
    a patch or collection of a class of the program's just added."""
    _mplot3d.project_by_program(axes._native, artist._native)
    artist._projects_itself = True
    _python_drawing.draw_through_python(artist, False)


def _project(artist):
    """Project *artist* for the draw coming, as matplotlib's
    ``Axes3D.draw`` does, if it projects itself and is visible."""
    if vars(artist).get("_projects_itself") and artist.get_visible():
        depth = artist.do_3d_projection()
        _mplot3d.set_program_depth(artist.axes._native, artist._native, float(depth))


_python_drawing.before_drawing(_project)
