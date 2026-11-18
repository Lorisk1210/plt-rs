"""
What the differential tester's programs using the API as documented
(pickling, rcParams, api, orders) found pltrs doing otherwise than
matplotlib 3.11: values typed as matplotlib types them, artists copied
and pickled, layout engines, secondary axes and mathtext's warnings. Each
expected value is recorded from matplotlib.
"""

import copy
import logging
import pickle
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import colors, patches, widgets
from pltrs.transforms import TransformedPatchPath
from build_facts import DEFAULT_INT

X = np.linspace(0, 2 * np.pi, 7)
Y = np.sin(X)
ZZ = np.outer(np.linspace(-1, 1, 9), np.linspace(-1, 1, 11))


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def roundtrip(obj):
    return pickle.loads(pickle.dumps(obj, pickle.HIGHEST_PROTOCOL))


def test_lines_keep_the_clip_path_they_are_made_with():
    fig, ax = plt.subplots()
    circle = patches.Circle((0.5, 0.5), 0.3, transform=ax.transAxes)
    ax.add_patch(circle)
    clip = TransformedPatchPath(circle)
    line = ax.plot([0, 1], [0, 1], clip_path=clip)[0]
    vline = ax.axvline(0.5, 0, 1, clip_path=clip)
    assert line.get_clip_path() is clip and vline.get_clip_path() is clip
    assert line.get_clip_box() is None
    patched = ax.plot([0, 1], [0, 1], clip_path=circle)[0]
    assert type(patched.get_clip_path()).__name__ == "TransformedPatchPath"
    boxed = ax.plot([0, 1], [0, 1], clip_path=patches.Rectangle((0, 0), 1, 1))[0]
    assert boxed.get_clip_path() is None and boxed.get_clip_box() is not None


def test_slider_lines_are_clipped_to_the_track():
    fig = plt.figure(figsize=(4, 3))
    slider = widgets.Slider(fig.add_axes((0.45, 0.05, 0.45, 0.1)), "amp", 0, 2, valinit=1)
    fig.canvas.draw()
    assert slider.vline.get_tightbbox().bounds == (270.0, 22.5, 0.0, 15.0)


def test_scatter_widths_read_back_as_given():
    fig, ax = plt.subplots()
    dots = ax.scatter([0, 1], [0, 1], linewidths=[1, 2])
    assert dots.get_linewidth() == [1, 2]
    assert type(dots.get_linewidth()[0]) is DEFAULT_INT
    assert type(dots.get_dashes()[0][0]) is DEFAULT_INT
    check = widgets.CheckButtons(fig.add_axes((0.1, 0.2, 0.2, 0.15)), ["on"], [True])
    widths = check.ax.collections[0].get_linewidth()
    assert widths.dtype == DEFAULT_INT and list(widths) == [1]


def test_norms_scaled_to_integers_hold_integers():
    fig, ax = plt.subplots()
    mesh = ax.pcolormesh(np.arange(6).reshape(2, 3))
    assert mesh.get_clim() == (0, 5) and type(mesh.get_clim()[0]) is int
    quiver = ax.quiver([0, 1, 2], [0, 1, 0], [1, -1, 0.5], [1, 2, -1], [1, 2, 3])
    assert quiver.get_clim() == (1, 3) and type(quiver.norm.vmin) is int
    assert quiver.get_array().dtype == DEFAULT_INT
    given = ax.quiver([0, 1, 2], [0, 1, 0], [1, -1, 0.5], [1, 2, -1], [1, 2, 3],
                      clim=(0.0, 4))
    assert [type(limit) for limit in given.get_clim()] == [float, float]
    barbs = ax.barbs([1, 2], [3, 3], [10, -20], [15, 5], [1, 2])
    assert barbs.get_clim() == (1, 2) and barbs.get_array().dtype == DEFAULT_INT
    flags = ax.imshow(np.eye(2) > 0)
    assert flags.get_clim() == (False, True)


def test_annotations_outside_their_axes_keep_their_transform():
    fig, ax = plt.subplots()
    ax.plot(X, Y)
    shown = ax.annotate("in", xy=(1, 0), xytext=(3, 0.5))
    hidden = ax.annotate("out", xy=(np.pi / 2, 5), xytext=(3, 0.8))
    fig.canvas.draw()
    assert type(hidden.get_transform()).__name__ == "IdentityTransform"
    assert shown.get_transform() is ax.transData


def test_bar_tick_labels_and_labels_hold_the_values_given():
    fig, ax = plt.subplots()
    bars = ax.bar([1, 2, 3], [3, -1, 2], tick_label=["a", "b", "c"])
    assert ax.get_xticks().dtype == DEFAULT_INT
    assert [t.get_text() for t in ax.get_xticklabels()] == ["a", "b", "c"]
    labels = ax.bar_label(bars)
    assert labels[0].get_position() == (0, 0)
    assert type(labels[0].get_position()[0]) is int
    assert labels[0].xy == (np.float64(1.0), np.float64(3.0))
    assert type(labels[0].xy[0]) is np.float64
    padded = ax.bar_label(bars, padding=np.array([1, 2, 3]))
    assert padded[1].get_position() == (0, -2)
    assert type(padded[1].get_position()[1]) is DEFAULT_INT
    hbars = ax.barh([1, 2], [3, -4])
    assert ax.bar_label(hbars, padding=2.5)[1].get_position() == (-2.5, 0)
    assert ax.bar_label(hbars, label_type="center")[0].xy == (0.5, 0.5)


def test_box_plots_at_integer_positions():
    fig, ax = plt.subplots()
    values = np.array([3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5])
    parts = ax.boxplot([values, values * 2], showmeans=True)
    whisker_x = parts["whiskers"][2].get_xdata()
    assert whisker_x.dtype == DEFAULT_INT and list(whisker_x) == [2, 2]
    assert parts["means"][0].get_xdata().dtype == DEFAULT_INT
    assert parts["caps"][0].get_xdata().dtype == np.float64


def test_contours_of_given_colours_index_them_with_integers():
    fig, ax = plt.subplots(figsize=(5, 2.5))
    zz = ZZ + np.sin(np.linspace(0, 3, 11))
    lines = ax.contour(zz, levels=[-1, -0.5, 0, 0.5, 1], colors="k")
    assert lines.cvalues == [0, 1, 2, 3, 4]
    assert lines.get_array().dtype == DEFAULT_INT
    labels = ax.clabel(lines, fmt="%.1f")
    assert [type(value) for value in labels[0].get_color()] == [np.float64] * 4
    assert type(labels[0].get_position()[0]) is np.float64
    assert type(lines.labelCValues[0]) is DEFAULT_INT
    filled = ax.contourf(zz, levels=[-1, 0, 1], colors=["r", "g", "b", "k"], extend="both")
    assert filled.cvalues == [-1, 0, 1, 2]
    mapped = ax.contourf(zz, levels=6)
    assert mapped.get_array().dtype == np.float64


def test_clipped_texts_measure_inside_their_clip_box():
    fig, ax = plt.subplots(figsize=(5, 2.5))
    clipped = ax.text(0.98, 0.5, "long text here", clip_on=True)
    loose = ax.text(0.98, 0.3, "long text here")
    fig.canvas.draw()
    assert clipped.get_tightbbox().x1 == 450.0
    assert loose.get_tightbbox().x1 == pytest.approx(540.6875)


def test_empty_contour_levels_share_one_path():
    fig, ax = plt.subplots()
    lines = ax.contour(ZZ, levels=[-5, 0, 5, 6])
    paths = lines.get_paths()
    assert [len(p.vertices) for p in paths] == [0, 18, 0, 0]
    assert paths[0] is paths[2] is paths[3] and paths[1] is not paths[0]


def test_colorbar_extensions_and_ticks_as_matplotlib_makes_them():
    fig, (ax, ax2) = plt.subplots(1, 2, figsize=(5, 2.5))
    image = ax.imshow(ZZ + 1.5, norm=colors.LogNorm(), cmap="plasma")
    bar = fig.colorbar(image, ax=ax, extend="both")
    assert [type(p).__name__ for p in bar.ax.patches] == ["PathPatch", "PathPatch"]
    assert [type(p.get_zorder()) for p in bar.ax.patches] == [np.float64] * 2
    mesh = ax2.pcolormesh(ZZ, norm=colors.BoundaryNorm([-1, 0, 0.5, 1], 3))
    ticked = fig.colorbar(mesh, ax=ax2, orientation="horizontal", ticks=[-1, 0, 1])
    assert ticked.get_ticks().dtype == DEFAULT_INT
    assert ticked.ax.get_xticks().dtype == DEFAULT_INT


def test_axes_specs_twins_and_grids_as_matplotlib_keeps_them():
    fig, axs = plt.subplots(2, 2, sharex=True, sharey="row", figsize=(4, 3))
    twin = axs[0, 0].twinx()
    assert twin.get_subplotspec() is axs[0, 0].get_subplotspec()
    mosaic_fig = plt.figure(layout="constrained", figsize=(5, 3))
    mosaic_fig.subplot_mosaic("AAB;CDB", width_ratios=[1, 2, 1])
    mosaic_fig.canvas.draw()
    for figure in (mosaic_fig, roundtrip(mosaic_fig)):
        spec = figure.axes[0].get_subplotspec()
        assert spec is figure.axes[0].get_subplotspec()
        assert spec.get_gridspec().get_width_ratios() == [1, 2, 1]
        assert type(spec.get_gridspec().get_width_ratios()[0]) is int
    geometry = roundtrip(fig).axes[0].get_subplotspec().get_geometry()
    assert [type(value) for value in geometry] == [int, int, np.int64, np.int64]


def test_colorbars_anchor_3d_axes_and_give_their_axes_a_transform():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    x, y = np.meshgrid(np.linspace(-1, 1, 6), np.linspace(-1, 1, 5))
    surface = ax.plot_surface(x, y, x * y, cmap="coolwarm")
    fig.colorbar(surface, shrink=0.6)
    assert ax.get_anchor() == (1.0, 0.5)
    constrained = plt.figure(layout="constrained")
    sub = constrained.subfigures(1, 2)[0]
    bar = sub.colorbar(sub.add_subplot().imshow(np.eye(2)))
    assert not bar.ax.is_transform_set()
    constrained.canvas.draw()
    assert bar.ax.is_transform_set()
    assert type(bar.ax.get_transform()).__name__ == "BboxTransformTo"


def test_axisartist_parts_are_no_children():
    from pltrs.mpl_toolkits import axisartist
    fig = plt.figure()
    ax = fig.add_subplot(121, axes_class=axisartist.Axes)
    curved = fig.add_subplot(122, axes_class=axisartist.Axes)
    curved.axis["lat"] = curved.new_floating_axis(0, 60)
    curved.grid(True)
    assert not ax.has_data() and ax._children == []
    assert not curved.has_data()
    assert ax.plot([0, 1])[0].get_label() == "_child0"


def test_wrapping_switches_texts_to_the_anchor_rotation_mode():
    fig, ax = plt.subplots(figsize=(5, 2.5))
    wrapped = ax.text(0.1, 0.5, "wrapped text that is long enough to wrap", wrap=True)
    plain = ax.text(0.1, 0.2, "plain", rotation=45)
    assert wrapped.get_rotation_mode() == "default"
    fig.canvas.draw()
    assert wrapped.get_rotation_mode() == "anchor"
    assert plain.get_rotation_mode() == "default"
    wrapped.set_rotation_mode("default")
    assert wrapped.get_rotation_mode() == "default"


def test_the_tight_engine_warns_at_each_draw():
    fig = plt.figure(figsize=(5, 3))
    fig.subplot_mosaic("AAB;CDB", gridspec_kw=dict(hspace=0.1))
    fig.set_layout_engine("tight")
    for _ in range(2):
        with pytest.warns(UserWarning, match="not compatible with tight_layout"):
            fig.canvas.draw()


def test_secondary_axes_warn_of_identical_limits_where_set():
    fig, ax = plt.subplots()
    ax.plot(X, Y)
    ax.set_xscale("function", functions=(np.square, np.sqrt))
    secondary = ax.secondary_xaxis("top", functions=(np.square, np.sqrt))
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        fig.canvas.draw()
    identical = [w for w in caught if "identical low and high xlims" in str(w.message)]
    assert len(identical) == 3
    for measure in (secondary.get_tightbbox, ax.get_tightbbox):
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            measure()
        assert sum("identical" in str(w.message) for w in caught) == 1


def test_shallow_copies_are_artists_of_their_own():
    fig, ax = plt.subplots(figsize=(3, 2))
    line, = ax.plot(X, Y, label="line")
    shallow = copy.copy(line)
    shallow.set_color("r")
    shallow.set_data([0, 1], [1, 0])
    assert shallow.get_color() == "r" and line.get_color() != "r"
    assert list(line.get_xdata()[:2]) == [0, X[1]]
    assert shallow.axes is ax and shallow.figure is fig and shallow not in ax.lines
    rect = ax.add_patch(patches.Rectangle((1, 0), 1, 0.5))
    rect_copy = copy.copy(rect)
    rect_copy.set_width(3)
    assert (rect.get_width(), rect_copy.get_width()) == (1, 3)
    label = ax.text(1, 0.5, "text")
    text_copy = copy.copy(label)
    text_copy.set_text("copied")
    assert (label.get_text(), text_copy.get_text(), text_copy in ax.texts) == (
        "text", "copied", False)
    handle = copy.copy(line)
    handle.update_from(ax.plot(X, -Y, "r--")[0])
    assert (handle.get_linestyle(), line.get_linestyle()) == ("--", "-")
    fig.canvas.draw()


def test_pickled_artists_keep_the_transforms_of_their_axes():
    from pltrs.offsetbox import AnchoredText
    fig, ax = plt.subplots()
    ax.add_artist(AnchoredText("anchored", loc="upper right"))
    fig.canvas.draw()
    fig2 = roundtrip(fig)
    assert fig2.axes[0].artists[0].get_transform() is fig2.axes[0].transData


def test_missing_mathtext_glyphs_are_logged(caplog):
    pltrs.rcParams["mathtext.fontset"] = "custom"
    pltrs.rcParams["mathtext.default"] = "tt"
    pltrs.rcParams["mathtext.fallback"] = None
    fig, ax = plt.subplots()
    ax.text(0.1, 0.5, r"$\aleph$")
    with caplog.at_level(logging.WARNING):
        fig.canvas.draw()
    assert ("Font 'tt' does not have a glyph for '\\\\aleph' [U+2135], substituting "
            "with a dummy symbol.") in caplog.messages


def test_deep_copies_copy_their_figure():
    fig, ax = plt.subplots()
    line, = ax.plot(X, Y)
    ax.text(1, 0.5, "text")
    deep = copy.deepcopy(line)
    assert deep.axes is not ax and type(deep.figure).__name__ == "Figure"
    assert deep.axes.lines[0] is deep
    fig2 = copy.deepcopy(fig)
    assert [type(a).__name__ for a in fig2.axes] == ["Axes"]
    assert [t.get_text() for t in fig2.axes[0].texts] == ["text"]
