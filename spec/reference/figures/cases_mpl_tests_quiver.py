"""matplotlib's non-image tests from ``tests/test_quiver.py`` that draw a
figure and check what it holds:

* ``quiverkey_angles``: a quiver of four unit vectors given an array of
  angles and its key, which draws a single arrow (``test_quiverkey_angles``
  counts the key's paths).
* ``quiverkey_angles_scale_units_cartesian_<kwargs>``: three vectors drawn
  with ``angles="xy"``, ``scale_units="xy"`` or both, and two keys, each a
  single arrow (``test_quiverkey_angles_scale_units_cartesian``, one case
  per keyword set).
* ``quiverkey_angles_xy_aitoff_<kwargs>``: eleven upward vectors along
  latitude 30° on an Aitoff map with the same keyword sets and a key
  (``test_quiverkey_angles_xy_aitoff``).
* ``quiver_setuvc_numbers``: a quiver of four unit vectors given the
  components ``0`` and ``1`` for every arrow with ``set_UVC``
  (``test_quiver_setuvc_numbers``), drawn after the change.

The other tests of the file are probes of the ``quiver_behaviour`` area or
cases of ``mpl_rendering``.
"""

import numpy as np

from figures._mpl_tests import group_case

GROUP = "mpl_tests_quiver"
mpl_test = group_case(GROUP)

#: The keyword sets of the key tests, named after their keywords.
KEY_KWARGS = {
    "angles_xy": {"angles": "xy"},
    "angles_xy_scale_units_xy": {"angles": "xy", "scale_units": "xy"},
    "scale_units_xy": {"scale_units": "xy"},
}


@mpl_test("quiverkey_angles")
def _(fig):
    ax = fig.subplots()
    X, Y = np.meshgrid(np.arange(2), np.arange(2))
    U = V = angles = np.ones_like(X)
    q = ax.quiver(X, Y, U, V, angles=angles)
    ax.quiverkey(q, 1, 1, 2, "Label")


def _cartesian(kwargs):
    def build(fig):
        ax = fig.subplots()
        q = ax.quiver([0, -1, 0], [0, -1, 0], [1, -1, 1], [1, -1, 0], **kwargs)
        ax.quiverkey(q, X=0.3, Y=1.1, U=1, label="Quiver key, length = 1", labelpos="E")
        ax.quiverkey(q, 0, 0, 1, "1 units")

    return build


def _aitoff(kwargs):
    def build(fig):
        x = np.linspace(-np.pi, np.pi, 11)
        y = np.ones_like(x) * np.pi / 6
        ax = fig.add_subplot(projection="aitoff")
        q = ax.quiver(x, y, np.zeros_like(x), np.ones_like(x), **kwargs)
        ax.quiverkey(q, 0, 0, 1, "1 units")

    return build


for _name, _kwargs in KEY_KWARGS.items():
    mpl_test(f"quiverkey_angles_scale_units_cartesian_{_name}")(_cartesian(_kwargs))
    mpl_test(f"quiverkey_angles_xy_aitoff_{_name}")(_aitoff(_kwargs))


@mpl_test("quiver_setuvc_numbers")
def _(fig):
    ax = fig.subplots()
    X, Y = np.meshgrid(np.arange(2), np.arange(2))
    U = V = np.ones_like(X)
    q = ax.quiver(X, Y, U, V)
    q.set_UVC(0, 1)
