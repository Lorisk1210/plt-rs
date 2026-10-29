"""matplotlib's own tests of ``matplotlib.patches`` (``tests/test_patches.py``)
that the ``patch_behaviour`` area does not already cover: what the drawing
tests assert about the patches they draw (angle, dash pattern, annulus
radii, hatch line width) and the box style strings that are refused.

Area ``mpl_patches``. Every op runs in the test suite's style (see
``_mpl_tests.py``).

Ops
---
``Rectangle.get_angle``  {} -> the angle of ``Rectangle((0, 0), 1, 1,
    angle=30)``, then of a ``Rectangle((0, 0), 1, 1)`` added to an axes,
    then of the latter after ``set_angle(30)`` (``test_rotate_rect_draw``).
``Patch.get_linestyle``  {linestyle} -> ``get_linestyle()`` of a rectangle
    made with ``linestyle=(offset, dashes)`` given as ``[offset, dashes]``
    (``test_dash_offset_patch_draw``).
``Annulus.set_semimajor``  {r, width, semimajor, semiminor} -> the radii
    of ``Annulus((0, 0), r, width)`` (``r`` a radius or ``[a, b]``) after
    ``set_semimajor(semimajor)`` and ``set_semiminor(semiminor)``
    (``test_annulus_setters2``).
``Patch.get_hatch_linewidth``  {setter} -> ``get_hatch_linewidth()`` of a
    hatched rectangle made under ``hatch.linewidth: 2``, or made in the
    test style and given ``set_hatch_linewidth(2)`` when ``setter``
    (``test_set_and_get_hatch_linewidth``).
``patches.BoxStyle``  {style, rect, mutation_size} -> the ``fancy``
    area's op (``test_boxstyle_errors``: ``foo`` and ``Round,foo`` are
    refused).

Already covered
---------------
By the ``patch_behaviour`` area: ``test_Polygon_close``,
``test_corner_center``, ``test_ellipse_vertices``, ``test_rotate_rect``,
``test_negative_rect``, ``test_patch_color_none``,
``test_wedge_movement``, ``test_contains_point``, ``test_contains_points``,
``test_fancyarrow_setdata``, ``test_degenerate_polygon``,
``test_empty_verts``, ``test_default_capstyle``,
``test_default_joinstyle``, ``test_arrow_set_data``,
``test_patch_hatchcolor_inherit_logic``,
``test_patch_hatchcolor_fallback_logic``,
``test_facecolor_none_force_edgecolor_false``,
``test_facecolor_none_force_edgecolor_true``,
``test_facecolor_none_edgecolor_force_edgecolor``,
``test_patch_edgegapcolor_getter_setter``,
``test_patch_edgegapcolor_init`` and ``test_patch_has_dashed_edge``. By the
``path`` area: ``test_clip_to_bbox``'s clipping. By the ``mpl_collections``
figure group: every image test (``test_rotate_rect_draw``,
``test_dash_offset_patch_draw``, ``test_clip_to_bbox``,
``test_patch_alpha_coloring``, ``test_patch_alpha_override``,
``test_patch_custom_linestyle``, ``test_patch_linestyle_none``,
``test_wedge_range``, ``test_multi_color_hatch``, ``test_connection_patch``,
``test_connection_patch_fig``, ``test_connection_patch_pixel_points``,
``test_shadow``, ``test_large_arc``, ``test_rotated_arcs``,
``test_annulus``, ``test_annulus_setters``, ``test_annulus_setters2``,
``test_autoscale_arc``, ``test_arc_in_collection``, ``test_modifying_arc``,
``test_set_and_get_hatch_linewidth`` and
``test_patch_edgegapcolor_visual``) but ``test_units_rectangle``. By the
``mpl_tests_patches`` figure group: ``test_patch_linestyle_accents`` and
``test_patch_zero_linewidth_dashed_uses_solid_gc_dashes``, which only
draw.

Left out
--------
``test_patch_str`` (Python ``repr``), ``test_units_rectangle``,
``test_datetime_rectangle``, ``test_datetime_datetime_fails`` and
``test_fancyarrow_units`` (Python unit conversion of patch coordinates),
``test_fancyarrow_shape_error`` (the arrow shape is an enum in pltrs),
``test_color_override_warning`` (a warning), ``test_default_antialiased``
and ``test_default_linestyle`` (``None`` restoring a default: pltrs
setters take values), ``test_empty_fancyarrow`` (empty arrays for
``ax.arrow``'s numbers) and ``test_patch_edgegapcolor_update_from``
(``update_from`` copies Python attributes).
"""

import matplotlib.pyplot as plt
from matplotlib.patches import Annulus, Rectangle

from probes._mpl_tests import prober
from probes.test_fancy import _box
from spec_support import ProbeSet

probes = ProbeSet("mpl_patches")
probe = prober(probes)


def _angles():
    fig = plt.figure()
    ref = Rectangle((0, 0), 1, 1, angle=30)
    fig.add_subplot().add_patch(ref)
    test = Rectangle((0, 0), 1, 1)
    before = test.get_angle()
    fig.add_subplot().add_patch(test)
    test.set_angle(30)
    return [ref.get_angle(), before, test.get_angle()]


def test_rotate_rect_draw():
    probe("Rectangle.get_angle", _angles)


def _linestyle(linestyle):
    offset, dashes = linestyle
    return Rectangle((0.1, 0.1), 0.8, 0.8, linewidth=3, edgecolor="b",
                     linestyle=(offset, dashes)).get_linestyle()


def test_dash_offset_patch_draw():
    for linestyle in ([0, [6, 6]], [0, [0, 6, 6, 0]], [6, [6, 6]]):
        probe("Patch.get_linestyle", _linestyle, linestyle=linestyle)


def _radii(r, width, semimajor, semiminor):
    ring = Annulus((0., 0.), tuple(r) if isinstance(r, list) else r, width)
    ring.set_semimajor(semimajor)
    ring.set_semiminor(semiminor)
    return list(ring.radii)


def test_annulus_setters2():
    probe("Annulus.set_semimajor", _radii, r=0.2, width=0.01, semimajor=0.2, semiminor=0.2)
    probe("Annulus.set_semimajor", _radii, r=[1, 2], width=0.1, semimajor=0.5, semiminor=0.3)


def _hatch_linewidth(setter):
    ax = plt.figure().add_subplot()
    if setter:
        ax.add_patch(Rectangle((0, 0), 1, 1, hatch="x"))
        ax.patches[0].set_hatch_linewidth(2.0)
    else:
        with plt.rc_context({"hatch.linewidth": 2.0}):
            ax.add_patch(Rectangle((0, 0), 1, 1, hatch="x"))
    return ax.patches[0].get_hatch_linewidth()


def test_set_and_get_hatch_linewidth():
    for setter in (False, True):
        probe("Patch.get_hatch_linewidth", _hatch_linewidth, setter=setter)


def test_boxstyle_errors():
    for style in ("foo", "Round,foo"):
        probes.probe("patches.BoxStyle", _box, style=style, rect=[0.0, 0.0, 1.0, 1.0],
                     mutation_size=1.0)
