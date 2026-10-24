"""matplotlib's non-image tests from ``tests/test_marker.py`` that only
check that a figure draws: ``test_star_marker``, two five-pointed star
markers given as ``(5, 1)`` and ``(5, 1, 0)``, the second outside the x
range. The other tests of the file are probes of the ``markers`` and
``mpl_marker`` areas or cases of ``mpl_small``.
"""

from figures._mpl_tests import group_case

GROUP = "mpl_tests_marker"
mpl_test = group_case(GROUP)


@mpl_test("star_marker")
def _(fig):
    size = 20**2
    ax = fig.subplots()
    ax.scatter([0], [0], marker=(5, 1), s=size)
    ax.scatter([1], [1], marker=(5, 1, 0), s=size)
    ax.set(xlim=(-0.5, 0.5), ylim=(-0.5, 1.5))
