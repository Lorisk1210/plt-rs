"""matplotlib's own tests of ``matplotlib.pyplot``
(``matplotlib/tests/test_pyplot.py``): the frame toggle, interactive mode
and its context managers, choosing and reusing subplots and axes by
projection, figure numbering and registration, subfigures, minor ticks,
backend switching, ``matshow`` and making figures under a number.

Area ``mpl_pyplot``. Every op starts and ends with ``plt.close("all")``;
identity checks are recorded as booleans.

Ops
---
``pyplot.box``  {} -> ``ax.get_frame_on()`` after ``box(False)``,
    ``box(True)``, ``box()`` and ``box()`` (``test_pyplot_box``).
``pyplot.stackplot``  {x, y} -> ``[collections, xlim, ylim]`` after
    ``stackplot(x, y)`` (``test_stackplot_smoke``).
``pyplot.isinteractive``  {steps} -> ``isinteractive()`` at every
    ``"check"`` of ``steps``: ``"ion"``, ``"ioff"``, ``"check"``, or
    ``["with_ion", steps]`` / ``["with_ioff", steps]`` running the inner
    steps in a ``with plt.ion():`` / ``with plt.ioff():`` block
    (``test_ioff``, ``test_ion``, ``test_nested_ion_ioff``).
``pyplot.subplot``  {case} -> the assertions of a scenario of
    ``plt.subplot`` calls (see ``SUBPLOT_CASES``).
``pyplot.axes``  {} -> the assertions of ``test_axes_kwargs``: whether a
    second ``axes()`` (then ``axes(projection="polar")``) made a new axes,
    and the name and novelty of a plain ``axes()`` after a polar one.
``pyplot.gca``  {} -> whether a second ``gca()`` returns the first, and
    the number of axes (``test_gca``).
``pyplot.polar``  {} -> whether a second ``polar`` call drew in the axes
    of the first, and its name (``test_polar_second_call``).
``pyplot.figure``  {case} -> the assertions of a scenario of
    ``plt.figure`` calls (see ``FIGURE_CASES``).
``pyplot.sca``  {} -> whether ``gca()`` differs from an axes of the first
    of two subfigures before ``sca(ax)`` and is it after
    (``test_set_current_axes_on_subfigure``).
``pyplot.xticks``  {} -> ``xticks(minor=True)`` after ``plot(arange(1,
    10))``: the minor tick locations and label texts (``test_minor_ticks``).
``pyplot.yticks``  {ticks, labels} -> ``get_yticks(minor=True)`` and the
    minor label texts after ``plot(arange(1, 10))`` and
    ``yticks(ticks, labels, minor=True)`` (``test_minor_ticks``).
``pyplot.switch_backend``  {backends} -> ``len(get_fignums())`` after two
    ``figure()`` calls and after switching to each backend in turn
    (``test_switch_backend_no_close``).
``pyplot.matshow``  {existing} -> ``[size_inches, axes, fignums]`` of the
    figure ``matshow([[0, 1], [1, 2]], fignum=...)`` draws in: that of a
    figure made first when ``existing``, else a new one (``test_matshow``).
``pyplot.subplots``  {existing, clear} -> ``subplots(num=1, clear=clear)``
    with or without a figure 1 made first: whether it returned that figure
    and its number of axes, or the error
    (``test_subplots_reuse_existing_figure_error``).
``pyplot.subplot_mosaic``  {existing, clear} -> the same for
    ``subplot_mosaic([["A", "B"]], num=2, clear=clear)``
    (``test_subplot_mosaic_reuse_existing_figure_error``).

Left out:

* tests of Python machinery: ``test_pyplot_up_to_date``,
  ``test_copy_docstring_and_deprecators``, ``test_pylab_integration``,
  ``test_doc_pyplot_summary``, ``test_setloglevel_signature`` and
  ``test_figure_hook`` (``figure.hooks`` imports Python callables);
* ``TypeError`` checks of Python calling conventions: ``test_nrows_error``
  and ``test_close``; likewise the ``FigureBase`` passed as ``num`` to
  ``subplots`` and ``subplot_mosaic``, and the ``polar=True`` with
  ``projection="3d"`` conflict of ``test_subplot_polar_normalization``,
  which Rust's types rule out;
* ``test_fallback_position``: the ``position`` keyword standing in for the
  rectangle is a Python calling convention;
* ``test_subplot_kwarg_collision``: plt-rs's projection requests carry no
  projection keywords (``theta_offset`` is set on the axes afterwards);
* the ``"3d"`` projection of ``test_subplot_change_projection``: pyplot's
  axes functions make 2-D axes, 3-D axes have a type of their own;
* ``test_multiple_same_figure_calls`` and ``test_close_all_warning``, which
  check warnings about arguments plt-rs's ``figure_num`` and
  ``figure_label`` do not take;
* of ``test_register_existing_figure_with_pyplot``, the ``AttributeError``
  for the number of an unregistered figure: plt-rs figures have no number
  attribute.
"""

import matplotlib as mpl
import matplotlib.pyplot as plt
import numpy as np
from matplotlib.figure import Figure

from spec_support import ProbeSet

probes = ProbeSet("mpl_pyplot")


def _fresh(scenario):
    """Runs ``scenario`` between two ``close("all")``."""
    def run(**args):
        plt.close("all")
        try:
            return scenario(**args)
        finally:
            plt.close("all")
    return run


@_fresh
def _box():
    fig, ax = plt.subplots()
    seen = []
    for on in [False, True, None, None]:
        plt.box(on)
        seen.append(ax.get_frame_on())
    return seen


@_fresh
def _stackplot(x, y):
    plt.stackplot(x, y)
    ax = plt.gca()
    return [len(ax.collections), list(ax.get_xlim()), list(ax.get_ylim())]


def _interactive(steps):
    seen = []

    def run(steps):
        for step in steps:
            if step == "check":
                seen.append(mpl.is_interactive())
            elif step == "ion":
                plt.ion()
            elif step == "ioff":
                plt.ioff()
            else:
                kind, inner = step
                with plt.ion() if kind == "with_ion" else plt.ioff():
                    run(inner)

    try:
        run(steps)
    finally:
        plt.ioff()
    return seen


def _subplot_reuse():
    ax1 = plt.subplot(121)
    first = ax1 is plt.gca()
    ax2 = plt.subplot(122)
    second = ax2 is plt.gca()
    ax3 = plt.subplot(121)
    return [first, second, ax1 is plt.gca(), ax1 is ax3]


def _subplot_replace_projection():
    fig = plt.figure()
    ax = plt.subplot(1, 2, 1)
    ax1 = plt.subplot(1, 2, 1)
    ax2 = plt.subplot(1, 2, 2)
    ax3 = plt.subplot(1, 2, 1, projection="polar")
    ax4 = plt.subplot(1, 2, 1, projection="polar")
    return [ax1 is ax, ax2 is not ax, ax3 is not ax, ax3 is ax4, ax in fig.axes,
            ax2 in fig.axes, ax3 in fig.axes, ax.name, ax2.name, ax3.name]


def _subplot_projection_reuse():
    ax1 = plt.subplot(111)
    seen = [ax1 is plt.gca(), ax1 is plt.subplot(111)]
    ax1.remove()
    ax2 = plt.subplot(111, projection="polar")
    seen += [ax2 is plt.gca(), ax1 not in plt.gcf().axes, ax2 is plt.subplot(111)]
    ax2.remove()
    ax3 = plt.subplot(111, projection="rectilinear")
    return seen + [ax3 is plt.gca(), ax3 is not ax2, ax2 not in plt.gcf().axes]


def _subplot_polar_normalization():
    ax1 = plt.subplot(111, projection="polar")
    ax2 = plt.subplot(111, polar=True)
    ax3 = plt.subplot(111, polar=True, projection="polar")
    return [ax1 is ax2, ax1 is ax3]


def _subplot_change_projection():
    created = set()
    ax = plt.subplot()
    created.add(ax)
    seen = []
    for projection in ["aitoff", "hammer", "lambert", "mollweide", "polar", "rectilinear"]:
        ax.remove()
        ax = plt.subplot(projection=projection)
        seen.append([ax is plt.subplot(), ax.name])
        created.add(ax)
    return [seen, len(created)]


#: ``pyplot.subplot`` scenarios, after the tests they come from.
SUBPLOT_CASES = {
    "reuse": _subplot_reuse,
    "replace_projection": _subplot_replace_projection,
    "projection_reuse": _subplot_projection_reuse,
    "polar_normalization": _subplot_polar_normalization,
    "change_projection": _subplot_change_projection,
}


@_fresh
def _subplot(case):
    return SUBPLOT_CASES[case]()


@_fresh
def _axes():
    seen = []
    for first, second in [({}, {}), ({"projection": "polar"}, {"projection": "polar"}),
                          ({"projection": "polar"}, {})]:
        plt.figure()
        ax = plt.axes(**first)
        ax1 = plt.axes(**second)
        seen += [ax1 is not ax, ax1.name]
        plt.close()
    return seen


@_fresh
def _gca():
    plt.figure()
    ax = plt.gca()
    return [plt.gca() is ax, len(plt.gcf().axes)]


@_fresh
def _polar():
    ln1, = plt.polar(0., 1., "ro")
    ln2, = plt.polar(1.57, .5, "bo")
    return [ln1.axes is ln2.axes, ln1.axes.name]


def _figure_subfigure():
    fig1 = plt.figure()
    subfigs = fig1.subfigures(2)
    plt.figure()
    other = plt.gcf() != fig1
    current = plt.figure(subfigs[1])
    return [other, plt.gcf() == fig1, current == fig1]


def _figure_register_existing():
    fig = Figure()
    plt.figure(fig)
    number = fig.number
    plt.suptitle("my title")
    fig2 = plt.figure()
    return [number, fig.get_suptitle(), fig2.number, plt.figure(1) is fig, plt.gcf() is fig]


def _figure_numbering():
    plt.figure()
    plt.figure()
    plt.close(1)
    after_close = plt.figure().number
    fignums = plt.get_fignums()
    plt.close("all")
    plt.figure(5)
    after_five = plt.figure().number
    plt.close("all")
    plt.figure("a")
    plt.figure(7)
    labelled = plt.figure("b").number
    return [after_close, fignums, after_five, labelled, plt.get_figlabels()]


#: ``pyplot.figure`` scenarios: ``test_set_current_figure_via_subfigure``,
#: ``test_register_existing_figure_with_pyplot`` and the numbers new
#: figures get.
FIGURE_CASES = {
    "subfigure": _figure_subfigure,
    "register_existing": _figure_register_existing,
    "numbering": _figure_numbering,
}


@_fresh
def _figure(case):
    return FIGURE_CASES[case]()


@_fresh
def _sca():
    fig = plt.figure()
    subfigs = fig.subfigures(2)
    ax = subfigs[0].subplots(1, squeeze=True)
    subfigs[1].subplots(1, squeeze=True)
    before = plt.gca() != ax
    plt.sca(ax)
    return [before, plt.gca() == ax]


@_fresh
def _xticks():
    plt.figure()
    plt.plot(np.arange(1, 10))
    tick_pos, tick_labels = plt.xticks(minor=True)
    return [list(tick_pos), [label.get_text() for label in tick_labels]]


@_fresh
def _yticks(ticks, labels):
    plt.figure()
    plt.plot(np.arange(1, 10))
    plt.yticks(ticks=ticks, labels=labels, minor=True)
    ax = plt.gca()
    return [list(ax.get_yticks(minor=True)),
            [label.get_text() for label in ax.get_yticklabels(minor=True)]]


@_fresh
def _switch_backend(backends):
    previous = plt.get_backend()
    try:
        plt.figure()
        plt.figure()
        seen = [len(plt.get_fignums())]
        for backend in backends:
            plt.switch_backend(backend)
            seen.append(len(plt.get_fignums()))
        return seen
    finally:
        plt.switch_backend(previous)


@_fresh
def _matshow(existing):
    fignum = plt.figure().number if existing else None
    image = plt.matshow([[0, 1], [1, 2]], fignum=fignum)
    fig = image.axes.figure
    return [list(fig.get_size_inches()), len(fig.axes), plt.get_fignums()]


@_fresh
def _subplots(existing, clear):
    fig = plt.figure(1) if existing else None
    fig_new, axs = plt.subplots(num=1, clear=clear)
    return [fig_new is fig, len(fig_new.axes)]


@_fresh
def _subplot_mosaic(existing, clear):
    fig = plt.figure(2) if existing else None
    fig_new, axd = plt.subplot_mosaic([["A", "B"]], num=2, clear=clear)
    return [fig_new is fig, len(fig_new.axes)]


def test_pyplot_box():
    probes.probe("pyplot.box", _box)


def test_stackplot_smoke():
    probes.probe("pyplot.stackplot", _stackplot, x=[1, 2, 3], y=[1, 2, 3])


def test_interactive():
    # test_ioff, test_ion and test_nested_ion_ioff, in that order.
    scenarios = [
        ["ion", "check", ["with_ioff", ["check"]], "check",
         "ioff", "check", ["with_ioff", ["check"]], "check"],
        ["ioff", "check", ["with_ion", ["check"]], "check",
         "ion", "check", ["with_ion", ["check"]], "check"],
        ["ion",
         ["with_ioff", ["check", ["with_ion", ["check"]], "check"]], "check",
         ["with_ioff", [["with_ioff", ["check"]]]], "check",
         ["with_ion", ["ioff"]], "check",
         "ioff",
         ["with_ion", ["check", ["with_ioff", ["check"]], "check"]], "check",
         ["with_ion", [["with_ion", ["check"]]]], "check",
         ["with_ioff", ["ion"]], "check"],
    ]
    for steps in scenarios:
        probes.probe("pyplot.isinteractive", _interactive, steps=steps)


def test_subplots_and_axes():
    for case in SUBPLOT_CASES:
        probes.probe("pyplot.subplot", _subplot, case=case)
    probes.probe("pyplot.axes", _axes)
    probes.probe("pyplot.gca", _gca)
    probes.probe("pyplot.polar", _polar)
    probes.probe("pyplot.sca", _sca)


def test_figures():
    for case in FIGURE_CASES:
        probes.probe("pyplot.figure", _figure, case=case)
    probes.probe("pyplot.switch_backend", _switch_backend, backends=["agg", "agg", "svg"])
    for existing in [True, False]:
        probes.probe("pyplot.matshow", _matshow, existing=existing)
    for existing, clear in [(True, False), (True, True), (False, False)]:
        probes.probe("pyplot.subplots", _subplots, existing=existing, clear=clear)
        probes.probe("pyplot.subplot_mosaic", _subplot_mosaic, existing=existing, clear=clear)


def test_minor_ticks():
    probes.probe("pyplot.xticks", _xticks)
    probes.probe("pyplot.yticks", _yticks, ticks=[3.5, 6.5], labels=["a", "b"])
