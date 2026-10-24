r"""matplotlib's own non-image tests of marker styles
(``matplotlib/tests/test_marker.py``) that the ``markers`` area leaves out:
the mathtext markers it does not probe, and the invalid markers it does not
try.

Area ``mpl_marker``. Every op runs in the style matplotlib's test fixture
applies (``classic`` plus ``_classic_test_patch``). Markers and transform
op lists are encoded as in the ``markers`` area (a flat list of numbers is
a tuple marker, a list of lists a vertex list).

Ops
---
``MarkerStyle``  {marker, transform?, steps?} -> ``{filled, fillstyle,
    joinstyle, capstyle, user_transform}`` of ``MarkerStyle(marker,
    transform=T)`` after the ``rotated`` / ``scaled`` / ``transformed``
    steps of the ``markers`` area (``user_transform`` a 3 x 3 matrix or
    null), or the error for an invalid marker:

    * the mathtext markers of ``test_markers_valid`` (``$\frac{1}{2}$``
      and a beamed pair of quavers) and the ``$|||$`` marker of
      ``test_marker_transformed`` and ``test_marker_rotated``, given a
      translation and transformed or rotated;
    * the markers ``test_markers_invalid`` rejects that the ``markers``
      area does not try: an arbitrary name (``"square"``), a one-element
      tuple and a single row of five numbers.

``test_star_marker``, which only checks that star markers draw, is a case
of ``mpl_tests_marker``; the image tests are cases of ``mpl_small``. The
other tests are covered by the ``markers`` area: ``test_marker_fillstyle``
(every marker in every fill style), ``test_marker_init_transforms`` (user
transforms), ``test_marker_init_joinstyle`` and
``test_marker_init_captyle`` (the cap and join sweeps),
``test_marker_rotated_invalid``, ``test_marker_scaled``, the other cases
of ``test_marker_transformed`` and ``test_marker_rotated`` (derived
markers), ``test_alt_transform`` (alternate transforms with a user
transform) and the other markers of ``test_markers_valid`` and
``test_markers_invalid``.
"""

from matplotlib.markers import MarkerStyle
from matplotlib.transforms import Affine2D

from probes._mpl_tests import prober
from spec_support import ProbeSet

probes = ProbeSet("mpl_marker")
probe = prober(probes)


def _affine(ops):
    t = Affine2D()
    for name, *values in ops:
        getattr(t, name)(*values)
    return t


def _marker(value):
    if isinstance(value, list) and value and all(isinstance(v, (int, float)) for v in value):
        return tuple(value)
    return value


def _marker_style(marker, transform=None, steps=()):
    style = MarkerStyle(_marker(marker),
                        transform=None if transform is None else _affine(transform))
    for kind, *values in steps:
        if kind == "rotated":
            style = style.rotated(**values[0])
        elif kind == "scaled":
            style = style.scaled(*values)
        else:
            style = style.transformed(_affine(values[0]))
    user = style.get_user_transform()
    return {"filled": style.is_filled(), "fillstyle": style.get_fillstyle(),
            "joinstyle": style.get_joinstyle(), "capstyle": style.get_capstyle(),
            "user_transform": None if user is None else user.get_matrix()}


def test_markers_valid():
    for marker in [r"$\frac{1}{2}$", "$♫$"]:
        res = probe("MarkerStyle", _marker_style, marker=marker)
        assert res["user_transform"] is None


def test_markers_invalid():
    for marker in ["square", [1], [[-0.5, 0, 1, 2, 3]]]:
        assert isinstance(probe("MarkerStyle", _marker_style, marker=marker), ValueError)


def test_mathtext_marker_transformed_and_rotated():
    translate = [["translate", 1, 1]]
    res = probe("MarkerStyle", _marker_style, marker="$|||$", transform=translate,
                steps=[["transformed", translate]])
    assert res["user_transform"].tolist() == Affine2D().translate(2, 2).get_matrix().tolist()
    probe("MarkerStyle", _marker_style, marker="$|||$", transform=translate,
          steps=[["rotated", {"deg": 10}]])
