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
(``_native.mplot3d.set_program_depth``). So is a 3-D artist of a subclass
of the program's that overrides ``do_3d_projection`` (it reads its
``_segments3d``, say, and calls the method it overrides), and a 3-D patch
another class draws (qiskit's ``Arrow3D``).
"""

from pltrs import _python_drawing
from pltrs._native import mplot3d as _mplot3d


def follow(axes, artist):
    """Have the draws of *axes* call the ``do_3d_projection`` of *artist*,
    a patch or collection of a class of the program's just added. One
    that pltrs (Rust) projects itself (the class is a subclass of a class
    of `.art3d`) stays projected by it: the program's method is called
    before each draw as matplotlib's draw calls it, reads and changes
    what the artist keeps, and answers for its depth."""
    if vars(artist).get("_projects_itself"):
        return
    if _mplot3d.get_axlim_clip(axes._native, artist._native) is None:
        _mplot3d.project_by_program(axes._native, artist._native)
    artist._projects_itself = True
    if not _python_drawing._draws_from_program(type(artist)):
        _python_drawing.draw_through_python(artist, False)


def follow_patch(axes, patch):
    """Have the draws of *axes* call the ``do_3d_projection`` of *patch*,
    a 3-D patch another class of its own draws (`.art3d._draws_itself`),
    which keeps its outline in Python: matplotlib's draws project it and
    order it by its depth all the same."""
    follow(axes, patch)


def _project(artist):
    """Project *artist* for the draw coming, as matplotlib's
    ``Axes3D.draw`` does, if it projects itself and is visible."""
    if vars(artist).get("_projects_itself") and artist.get_visible():
        depth = artist.do_3d_projection()
        _mplot3d.set_program_depth(artist.axes._native, artist._native, float(depth))


_python_drawing.before_drawing(_project)
