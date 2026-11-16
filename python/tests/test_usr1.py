"""
What the written family (``spec/diff/written``, task USR1: the short
programs users write for an ordinary chart) showed pltrs doing otherwise
than matplotlib 3.11, reduced to matplotlib's API: the rotation a label of
``pie(rotatelabels=True)`` reads back, a colorbar's tickers put back by
``update_normal``, an alpha taken from the property cycle, the colour the
redundant-colour warning names, the attributes of an SVG text element and
the extents of a curved path. Every expected value was recorded from
matplotlib.
"""

import io
import warnings

from PIL import Image

import numpy as np
import pytest
from cycler import cycler

import pltrs
import pltrs.pyplot as plt
from pltrs import ticker
from pltrs.colors import LogNorm
from pltrs.patches import Circle, Ellipse, Rectangle
from pltrs.path import Path


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def test_rotations_read_back_in_one_turn():
    fig, ax = plt.subplots()
    _, texts = ax.pie([40, 25, 12, 8, 5, 10], labels=list("ABCDEF"), rotatelabels=True)
    assert [t.get_rotation() for t in texts] == pytest.approx(
        [72.0, 9.0, 75.59999999999997, 291.6, 315.0, 342.0], abs=1e-12)
    text = ax.text(0, 0, "x")
    text.set_rotation(-90)
    assert text.get_rotation() == 270.0


def _labels(cb):
    cb.ax.figure.canvas.draw()
    return [t.get_text() for t in cb.ax.get_yticklabels()], len(cb.ax.yaxis.get_minorticklocs())


def test_update_normal_puts_the_colorbars_own_tickers_back():
    fig, ax = plt.subplots()
    im = ax.imshow(np.arange(4.).reshape(2, 2))
    cb = fig.colorbar(im)
    cb.set_ticks([0, 1, 2], labels=["lo", "mid", "hi"])
    assert _labels(cb) == (["lo", "mid", "hi"], 0)
    cb.update_normal(im)
    assert _labels(cb) == (["0", "1", "2"], 0)
    assert type(cb.locator).__name__ == "FixedLocator"
    cb.set_ticks([0.5, 2.5])
    cb.minorticks_on()
    assert _labels(cb) == (["0.5", "2.5"], 5)
    im.set_clim(0, 4)
    assert _labels(cb) == (["0.5", "2.5"], 0)
    cb.set_ticklabels(["a", "b"])
    cb.minorticks_off()
    im.set_cmap("magma")
    assert _labels(cb) == (["0.5", "2.5"], 0)
    cb.formatter = ticker.FormatStrFormatter("%.2f")
    im.set_clim(0, 3)
    assert _labels(cb) == (["0.50", "2.50"], 0)
    im.set_norm(LogNorm(1, 3))
    assert _labels(cb)[1] == 16
    assert type(cb.locator).__name__ == "LogLocator"


def test_cycled_alphas_read_back_as_the_cycle_holds_them():
    fig, ax = plt.subplots()
    ax.set_prop_cycle(cycler(alpha=np.array([0.25, 0.5])) + cycler(color=["r", "b"]))
    first, = ax.plot([0, 1])
    given, = ax.plot([0, 1], alpha=0.75)
    third, = ax.plot([1, 0])
    bar = ax.bar([0], [1])[0]
    polygon = ax.fill([0, 1, 1], [0, 0, 1])[0]
    text = ax.text(0, 0, "t", alpha=0.25)
    assert [type(a.get_alpha()) for a in (first, given, third, polygon, text)] == [
        np.float64, float, np.float64, np.float64, float]
    assert [first.get_alpha(), third.get_alpha(), polygon.get_alpha()] == [0.25, 0.25, 0.5]
    assert bar.get_alpha() is None


@pytest.mark.parametrize("fmt, kwargs, shown", [
    ("r", {"color": "b"}, "color=(1.0, 0.0, 0.0, 1)"),
    ("C2", {"color": "b"}, "color=(0.17254901960784313, 0.6274509803921569, 0.17254901960784313, 1)"),
    ("g--", {"color": "b"}, "color='g'"),
    ("0.5", {"color": "b"}, "color=(0.5, 0.5, 0.5, 1.0)"),
    ("o-", {"marker": "s"}, "marker='o'"),
])
def test_redundant_format_warnings_name_the_formats_value(fmt, kwargs, shown):
    fig, ax = plt.subplots()
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        ax.plot([0, 1], fmt, **kwargs)
    assert [str(w.message) for w in caught] == [
        f"{shown.split('=')[0]} is redundantly defined by the '{shown.split('=')[0]}' keyword "
        f'argument and the fmt string "{fmt}" (-> {shown}). The keyword argument will take '
        "precedence."]


def test_svg_text_elements_write_their_style_first():
    with pltrs.rc_context({"svg.fonttype": "none", "svg.hashsalt": "s"}):
        fig, ax = plt.subplots()
        ax.text(0.5, 0.5, "label")
        ax.text(0.2, 0.2, "turned", rotation=30)
        buf = io.StringIO()
        fig.savefig(buf, format="svg", metadata={"Date": None})
    elements = [line.strip() for line in buf.getvalue().splitlines()
                if "label</text>" in line or "turned</text>" in line]
    assert [e[:20] + "..." + e[-30:] for e in elements] == [
        '<text style="font-si... 236.16 174.528)">label</text>',
        '<text style="font-si...53) rotate(-30)">turned</text>']
    label = elements[0]
    assert label.index(" style=") < label.index(" x=") < label.index(" y=") < label.index(" transform=")


def test_curved_extents_agree_to_the_last_bit():
    verts = [(0, -1), (-1.5, 0.3), (-0.8, 1.4), (0, 0.6), (0.8, 1.4), (1.5, 0.3), (0, -1), (0, -1)]
    path = Path(verts, [1, 4, 4, 4, 4, 4, 4, 79])
    assert path.get_extents().get_points().tolist() == [
        [-0.8816326530612244, -1.0], [0.8816326530612246, 0.8712802768166089]]
    fig, ax = plt.subplots()
    ax.add_patch(Ellipse((1, 2), 3, 1, angle=30))
    ax.autoscale_view()
    assert ax.dataLim.get_points().tolist() == [
        [-0.299038105676658, 1.1634836962621922], [2.299038105676658, 2.836516303737808]]


def test_update_from_keeps_the_patchs_own_linestyle():
    dashed = Rectangle((0, 0), 1, 1, ls="--", lw=2)
    results = []
    for own in (None, ":", "-."):
        circle = Circle((0, 0), 1) if own is None else Circle((0, 0), 1, ls=own)
        circle.update_from(dashed)
        results.append(circle.get_linestyle())
    solid_source = Circle((0, 0), 1, ls="--")
    solid_source.update_from(Rectangle((0, 0), 1, 1))
    assert results + [solid_source.get_linestyle()] == ["solid", ":", "-.", "--"]


def test_png_keeps_every_metadata_key_as_text():
    fig, ax = plt.subplots(figsize=(1, 1), dpi=20)
    buf = io.BytesIO()
    fig.savefig(buf, format="png", metadata={
        "Title": "T", "Author": "QA", "Description": "three points", "Software": None,
        "Experiment": "run-42", "Source": "model"})
    buf.seek(0)
    with Image.open(buf) as im:
        text = {k: v for k, v in im.info.items() if isinstance(v, str)}
    assert text == {"Title": "T", "Author": "QA", "Description": "three points",
                    "Experiment": "run-42", "Source": "model"}
