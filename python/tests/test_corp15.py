"""
What the corpora still showed on the merged tree (task CORP15) of pltrs
doing otherwise than matplotlib 3.11, reduced to matplotlib's API. Every
expected value was recorded from matplotlib.
"""

import io
import logging

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import font_manager
from pltrs.collections import PolyCollection
from pltrs.colors import FuncNorm
from pltrs.mpl_toolkits.mplot3d.art3d import Poly3DCollection


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _number(value):
    return value if value is None or isinstance(value, str) else float(value)


@pytest.mark.parametrize("positions, depths, zorders", [
    (None, [-1.072832, -1.060344], [3.5, 2.5]),
    ((0.5, 0.5), [-1.016743, -1.016743], [2.5, 3.5]),
    ((0.2, 0.8), [-1.003623, -1.03021], [2.5, 3.5])])
def test_an_assigned_sort_position_orders_3d_bars(positions, depths, zorders):
    # qutip's matrix histograms order their bars by the camera,
    # `bars._sort_zpos = z_order[i]`.
    fig = plt.figure(figsize=(3, 3))
    ax = fig.add_subplot(projection="3d")
    bars = [ax.bar3d([0], [0], [0], [1], [1], [1]),
            ax.bar3d([0.5], [0.5], [0], [1], [1], [1])]
    if positions is not None:
        for collection, position in zip(bars, positions):
            collection._sort_zpos = position
    fig.canvas.draw()
    assert [round(float(c.do_3d_projection()), 6) for c in bars] == depths
    assert [c.get_zorder() for c in bars] == zorders


def test_the_sort_position_reads_back_as_matplotlib_keeps_it():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    bars = ax.bar3d([0], [0], [0], [1], [1], [1])
    read = [bars._sort_zpos]
    bars._sort_zpos = 2
    read.append(bars._sort_zpos)
    bars.set_zsort("min")
    read.append(bars._sort_zpos)
    bars._sort_zpos = "x"
    read.append(bars._sort_zpos)
    bars.set_3d_properties()
    read.append(bars._sort_zpos)
    # add_collection3d orders a 2-D polygon collection by its lowest z.
    lifted = PolyCollection([[(0, 0), (1, 0), (1, 1)]])
    ax.add_collection3d(lifted, zs=[0.25])
    read.append(lifted._sort_zpos)
    loose = Poly3DCollection([[(0, 0, 0), (1, 0, 0), (1, 1, 1)]])
    loose._sort_zpos = 3
    read.append(loose._sort_zpos)
    ax.add_collection3d(loose)
    read.append(loose._sort_zpos)
    fig.canvas.draw()
    assert [_number(v) for v in read] == [None, 2.0, None, "x", None, 0.25, 3.0, 3.0]


def lookups(work, family):
    """How many times *work* has the font manager look *family* up (its
    message per lookup of a family no font has)."""
    found = []

    class Handler(logging.Handler):
        def emit(self, record):
            message = record.getMessage()
            found.append(family in message and message.endswith("not found."))

    handler = Handler()
    font_manager._log.addHandler(handler)
    try:
        work()
    finally:
        font_manager._log.removeHandler(handler)
    return sum(found)


@pytest.mark.parametrize("kind, expected", [
    ("clipped", 5), ("unclipped", 6), ("out of layout", 5), ("off the log scale", 5),
    ("annotation", 5), ("annotation clipped out", 0), ("hidden", 0), ("drawn before", 7),
    ("hidden axes", 0)])
def test_the_draw_before_a_tight_print_measures_the_texts_its_box_leaves_out(kind, expected):
    # qutip's circuits: a gate's label, clipped to the axes, saved cut to
    # the tight box.
    family = "Corp15Missing" + kind.title().replace(" ", "")

    def work():
        fig, ax = plt.subplots()
        font = {"fontsize": 12, "fontfamily": family}
        if kind in ("clipped", "unclipped", "drawn before", "hidden axes"):
            ax.add_artist(plt.Text(1, 1, "H", clip_on=kind != "unclipped", **font))
        elif kind == "out of layout":
            ax.text(0.5, 0.5, "H", **font).set_in_layout(False)
        elif kind == "off the log scale":
            ax.set_yscale("log")
            ax.text(0.5, -5, "H", **font)
        elif kind == "annotation":
            ax.add_artist(plt.Annotation("H", (0.5, 0.5), **font))
        elif kind == "annotation clipped out":
            ax.annotate("H", (5, 5), annotation_clip=True, **font)
        else:
            ax.text(0.5, 0.5, "H", visible=False, **font)
        if kind == "drawn before":
            fig.canvas.draw()
        ax.set_visible(kind != "hidden axes")
        fig.savefig(io.BytesIO(), format="png", bbox_inches="tight")

    assert lookups(work, family) == expected


@pytest.mark.parametrize("dpi, clipped, calls", [(100, True, 40), (80, False, 41)])
def test_a_tight_box_measures_a_mesh_by_the_corners_of_its_box(dpi, clipped, calls):
    # The what's new of 3.4: a colour bar of a FuncNorm, whose mesh the
    # tight box takes in at 80 dpi (its patch's extents round there), maps
    # the corners of the mesh's box through the program's function once.
    mapped = []

    def forward(x):
        mapped.append(np.size(x))
        return x**2

    fig, ax = plt.subplots(figsize=(5.5, 4.5), dpi=dpi)
    norm = FuncNorm((forward, np.sqrt), vmin=0, vmax=3)
    mesh = ax.pcolormesh(np.linspace(0.1, 2.9, 12).reshape(3, 4), norm=norm)
    colorbar = fig.colorbar(mesh)
    fig.canvas.draw()
    mapped.clear()
    box = fig.get_tightbbox()
    assert bool(colorbar.solids._fully_clipped_to_axes()) is clipped
    assert len(mapped) == calls
    assert np.round(box.extents, 4).tolist()[::2] == ([0.3703, 4.8011] if dpi == 100 else [0.3653, 4.8061])


def test_a_mesh_under_a_program_scale_measures_its_box():
    mapped = []

    def forward(x):
        mapped.append(np.size(x))
        return x**2

    fig, ax = plt.subplots()
    ax.set_yscale("function", functions=(forward, lambda x: np.sqrt(np.abs(x))))
    mesh = ax.pcolormesh(np.arange(5.0), np.arange(1.0, 5.0), np.ones((3, 4)))
    fig.canvas.draw()
    mapped.clear()
    box = mesh.get_datalim(ax.transData)
    assert mapped == []
    assert box.extents.tolist() == [0.0, 1.0, 4.0, 4.0]
    fig.set_dpi(80)
    mapped.clear()
    assert np.round(fig.get_tightbbox().extents, 4).tolist() == [0.4778, 0.2919, 5.8725, 4.2768]
    assert len(mapped) == 78


def _effect_names(artist):
    effects = artist.get_path_effects()
    return None if effects is None else [type(effect).__name__ for effect in effects]


def test_artists_keep_the_path_effects_of_the_rcparam_they_were_made_under():
    # CORP12's leftover: artists made under plt.xkcd() reported the
    # effects in force when read.
    from pltrs import patheffects
    from pltrs.collections import LineCollection
    from pltrs.patches import Rectangle
    stroke = [patheffects.withStroke(linewidth=3, foreground="w")]
    with plt.rc_context({"path.effects": stroke}):
        fig, ax = plt.subplots()
        subfigure = fig.subfigures(1, 2)[0]
        artists = {
            "line": ax.plot([1, 2])[0], "text": ax.text(0, 0, "a"),
            "patch": ax.add_patch(Rectangle((0, 0), 1, 1)), "legend": ax.legend(["a"]),
            "title": ax.set_title("t"), "figure": fig, "axes": ax, "background": ax.patch,
            "label": ax.xaxis.label, "subfigure": subfigure, "its background": subfigure.patch}
        collection = ax.add_collection(LineCollection([[(0, 0), (1, 1)]]))
        fig.canvas.draw()
        tick = ax.xaxis.get_major_ticks()[0].label1
        assert fig.get_path_effects() is ax.get_path_effects()
    later, = plt.figure().add_subplot().plot([1, 2])
    for artist in [*artists.values(), tick]:
        assert _effect_names(artist) == ["withStroke"]
    assert collection.get_path_effects() is None
    with plt.rc_context({"path.effects": [patheffects.Normal()]}):
        assert later.get_path_effects() == []
        late_subfigure = plt.figure().subfigures(1, 2)[0]
    assert _effect_names(late_subfigure) == ["Normal"]


def test_an_rcparams_copy_restores_path_effects():
    # Leaving plt.xkcd() restores the parameters from a copy, which could
    # not spell the program's path effects as rc text.
    from pltrs import patheffects
    plt.rcParams["path.effects"] = [patheffects.Normal()]
    with plt.xkcd():
        line, = plt.plot([1, 2])
        assert [type(e).__name__ for e in plt.rcParams["path.effects"]] == ["withStroke"]
    assert [type(e).__name__ for e in plt.rcParams["path.effects"]] == ["Normal"]
    assert _effect_names(line) == ["withStroke"]
    copy = plt.rcParams.copy()
    plt.rcParams["path.effects"] = []
    plt.rcParams.update(copy)
    assert [type(e).__name__ for e in plt.rcParams["path.effects"]] == ["Normal"]


@pytest.mark.parametrize("drawn_first", [False, True])
def test_a_scatter_legend_handle_shares_the_mapping_of_its_entry(drawn_first):
    # CORP12's leftover: matplotlib's handler copies the values, the
    # colormap and the norm itself (`update_from`), and the colours the
    # entry reports; the handle maps them when it is drawn.
    fig, ax = plt.subplots(figsize=(3, 2))
    dots = ax.scatter([1, 2, 3], [1, 2, 3], c=[0.5, 0.0, 1.0], s=200, label="dots")
    if drawn_first:
        fig.canvas.draw()
    handle = ax.legend(markerscale=3).legend_handles[0]
    assert handle.norm is dots.norm and handle.cmap is dots.cmap
    assert handle.get_array().tolist() == [0.5, 0.0, 1.0]
    reported = [0.128, 0.567, 0.551, 1.0] if drawn_first else [0.122, 0.467, 0.706, 1.0]
    assert np.round(handle.get_facecolor()[0], 3).tolist() == reported
    dots.set_clim(0, 2)
    assert handle.get_clim() == (0.0, 2.0)
    fig.canvas.draw()
    assert len(handle.get_facecolor()) == 3
    assert np.round(handle.get_facecolor()[0], 3).tolist() == [0.23, 0.322, 0.546, 1.0]
    assert np.round(handle.get_edgecolor()[0], 3).tolist() == [0.23, 0.322, 0.546, 1.0]
