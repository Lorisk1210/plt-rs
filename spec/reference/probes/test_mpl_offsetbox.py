"""matplotlib's own non-image tests of offset boxes
(``matplotlib/tests/test_offsetbox.py``) that the ``offsetbox_behaviour``
area leaves out.

Area ``mpl_offsetbox``. Every op runs in the style matplotlib's test
fixture applies (``classic`` plus ``_classic_test_patch``).

Ops
---
``AnnotationBbox.xyann``  {xybox?, boxcoords?} -> ``[xyann, anncoords]`` of
    an ``AnnotationBbox`` of a 20 x 20 ``DrawingArea`` at ``(0.5, 0.5)``
    in ``"data"`` coordinates, given ``xybox`` and ``boxcoords`` when
    present: the box goes to the annotated point and takes its coordinates
    unless told otherwise (``test_annotationbbox_properties``).
``Annotation.draggable``  {scene} -> what the draggable annotation tests
    observe:

    * ``remove``  a draggable annotation removed before a button release
      at ``(1, 1)``: the number of texts left on the axes
      (``test_remove_draggable``, which checks that the release is
      harmless);
    * ``subfigure``  a draggable ``annotate("foo", (0, 0))`` on an axes
      filling the subfigure of ``fig.subfigures()``, drawn: its text
      position (``xyann``) at the start, after a press on it at ``(1, 1)``,
      a move to ``(11, 6)`` and a release there (it follows the pointer),
      after a scroll on it and a move (a scroll picks nothing up), and
      after a press 2 pixels above and right of its extent and a move
      (nothing there to pick up) (``test_draggable_in_subfigure``, which
      reads the private dragging flag instead).

The tests that only draw are cases of ``mpl_tests_offsetbox``
(``test_offsetbox_loc_codes``, ``test_expand_with_tight_layout``).

Left out, besides the tests the ``offsetbox_behaviour`` area lists as
covered: ``test_offsetbox_clip_children`` (plt-rs offset boxes are values
put together before they join a figure, without a handle to toggle a
drawing area's clipping afterwards), ``test_paddedbox_default_values`` (a
``PaddedBox`` around an ``AnchoredText``: plt-rs anchored boxes are roots
of box trees, not children), ``test_zorder``, ``test_arrowprops_copied``
and ``test_textarea_properties`` (Python keyword and attribute handling;
plt-rs text areas are plain values) and the child offsets of
``test_packers`` (internal to the packer's layout; the
``offsetbox_behaviour`` area checks the alignment it computes).
"""

import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent
from matplotlib.offsetbox import AnnotationBbox, DrawingArea

from probes._mpl_tests import prober
from spec_support import ProbeSet

probes = ProbeSet("mpl_offsetbox")
probe = prober(probes)


def _xyann(**options):
    ab = AnnotationBbox(DrawingArea(20, 20, 0, 0, clip=True), (0.5, 0.5), xycoords="data",
                        **{k: tuple(v) if isinstance(v, list) else v for k, v in options.items()})
    return [list(ab.xyann), ab.anncoords]


def test_annotationbbox_properties():
    assert probe("AnnotationBbox.xyann", _xyann) == [[0.5, 0.5], "data"]
    assert probe("AnnotationBbox.xyann", _xyann, xybox=[-0.2, 0.4],
                 boxcoords="axes fraction") == [[-0.2, 0.4], "axes fraction"]


def _mouse(fig, name, x, y, **kwargs):
    MouseEvent(name, fig.canvas, x, y, **kwargs)._process()


def _draggable(scene):
    if scene == "remove":
        fig, ax = plt.subplots()
        an = ax.annotate("foo", (.5, .5))
        an.draggable(True)
        an.remove()
        _mouse(fig, "button_release_event", 1, 1)
        return len(ax.texts)
    fig = plt.figure()
    ann = fig.subfigures().add_axes((0, 0, 1, 1)).annotate("foo", (0, 0))
    ann.draggable(True)
    fig.canvas.draw()
    out = [list(ann.xyann)]
    _mouse(fig, "button_press_event", 1, 1, button=1)
    _mouse(fig, "motion_notify_event", 11, 6)
    _mouse(fig, "button_release_event", 11, 6, button=1)
    out.append(list(ann.xyann))
    _mouse(fig, "scroll_event", 12, 7, step=1)
    _mouse(fig, "motion_notify_event", 30, 30)
    out.append(list(ann.xyann))
    bbox = ann.get_window_extent()
    _mouse(fig, "button_press_event", bbox.x1 + 2, bbox.y1 + 2, button=1)
    _mouse(fig, "motion_notify_event", bbox.x1 + 40, bbox.y1 + 40)
    _mouse(fig, "button_release_event", bbox.x1 + 40, bbox.y1 + 40, button=1)
    out.append(list(ann.xyann))
    return out


def test_remove_draggable():
    assert probe("Annotation.draggable", _draggable, scene="remove") == 0


def test_draggable_in_subfigure():
    start, dragged, scrolled, outside = probe("Annotation.draggable", _draggable,
                                              scene="subfigure")
    assert start != dragged and dragged == scrolled == outside
