"""matplotlib's non-image tests from ``tests/test_tightlayout.py`` that lay
a figure out and then check where the axes went, or only draw: their
figures, drawn here in the test suite's default style, record every
axes' position. One case per figure a test makes (``_0`` .. ``_2``, or
the label).

Adaptations: the tests run with text drawn as rectangles
(``text_placeholders``); these draw real text, so the positions are
matplotlib's for real text and the tests' literals do not apply; the
figures of pyplot calls are made on the case's figure, and the
warnings the layouts raise are silenced; ``test_tight_pads`` and
``test_tight_kwargs`` make the same figure (an engine with ``pad=0.15``),
drawn once as ``tight_pads``.
"""

import warnings

import matplotlib.pyplot as plt
from matplotlib.offsetbox import AnchoredOffsetbox, DrawingArea
from matplotlib.patches import Rectangle

from figures._mpl_tests import group_case

GROUP = "mpl_tests_tightlayout"
mpl_test = group_case(GROUP)


@mpl_test("outward_ticks")
def _(fig):
    ax = fig.add_subplot(221)
    ax.xaxis.set_tick_params(tickdir='out', length=16, width=3)
    ax.yaxis.set_tick_params(tickdir='out', length=16, width=3)
    ax.xaxis.set_tick_params(tickdir='out', length=32, width=3, tick1On=True, which='minor')
    ax.yaxis.set_tick_params(tickdir='out', length=32, width=3, tick1On=True, which='minor')
    ax.xaxis.set_ticks([0], minor=True)
    ax.yaxis.set_ticks([0], minor=True)
    ax = fig.add_subplot(222)
    ax.xaxis.set_tick_params(tickdir='in', length=32, width=3)
    ax.yaxis.set_tick_params(tickdir='in', length=32, width=3)
    ax = fig.add_subplot(223)
    ax.xaxis.set_tick_params(tickdir='inout', length=32, width=3)
    ax.yaxis.set_tick_params(tickdir='inout', length=32, width=3)
    ax = fig.add_subplot(224)
    ax.xaxis.set_tick_params(tickdir='out', length=32, width=3)
    ax.yaxis.set_tick_params(tickdir='out', length=32, width=3)
    fig.tight_layout()


def _add_offsetboxes(ax, size=10, margin=.1, color='black'):
    """The test's ``add_offsetboxes``: boxes anchored around ``ax``."""
    m, mp = margin, 1 + margin
    anchor_points = [(-m, -m), (-m, .5), (-m, mp), (.5, mp), (mp, mp), (mp, .5), (mp, -m),
                     (.5, -m)]
    for point in anchor_points:
        da = DrawingArea(size, size)
        background = Rectangle((0, 0), width=size, height=size, facecolor=color,
                               edgecolor='None', linewidth=0, antialiased=False)
        da.add_artist(background)
        anchored_box = AnchoredOffsetbox(loc='center', child=da, pad=0., frameon=False,
                                         bbox_to_anchor=point, bbox_transform=ax.transAxes,
                                         borderpad=0.)
        ax.add_artist(anchored_box)


def _offsetboxes(fig, with_boxes, hidden=False):
    axs = fig.subplots(2, 2)
    for ax, color in zip(axs.flat, ['red', 'blue', 'green', 'yellow']):
        ax.plot([0, 1], [0, 1], color=color)
        if with_boxes:
            _add_offsetboxes(ax, 20, color=color)
    if hidden:
        for ax in axs.flat:
            for child in ax.get_children():
                if isinstance(child, AnchoredOffsetbox):
                    child.set_visible(False)
    fig.tight_layout()


@mpl_test("tight_layout_offsetboxes_0")
def _(fig):
    _offsetboxes(fig, False)


@mpl_test("tight_layout_offsetboxes_1")
def _(fig):
    _offsetboxes(fig, True)


@mpl_test("tight_layout_offsetboxes_2")
def _(fig):
    _offsetboxes(fig, True, hidden=True)


for _label in ["xlabel", "ylabel"]:
    mpl_test(f"verybig_decorators_{_label}", figsize=(3, 2))(
        lambda fig, label=_label: fig.subplots().set(**{label: 'a' * 100}))


@mpl_test("big_decorators_horizontal", figsize=(3, 2))
def _(fig):
    axs = fig.subplots(1, 2)
    axs[0].set_xlabel('a' * 30)
    axs[1].set_xlabel('b' * 30)


@mpl_test("big_decorators_vertical", figsize=(3, 2))
def _(fig):
    axs = fig.subplots(2, 1)
    axs[0].set_ylabel('a' * 20)
    axs[1].set_ylabel('b' * 20)


@mpl_test("badsubplotgrid")
def _(fig):
    plt.subplot2grid((4, 5), (0, 0), fig=fig)
    plt.subplot2grid((5, 5), (0, 3), colspan=3, rowspan=5, fig=fig)
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", UserWarning)
        fig.tight_layout()


@mpl_test("collapsed")
def _(fig):
    fig.set_layout_engine("tight")
    ax = fig.subplots()
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)
    ax.annotate('BIG LONG STRING', xy=(1.25, 2), xytext=(10.5, 1.75), annotation_clip=False)
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", UserWarning)
        fig.tight_layout()
        fig.tight_layout(rect=[0, 0, 0.8, 0.8])


@mpl_test("suptitle")
def _(fig):
    fig.set_layout_engine("tight")
    ax = fig.subplots()
    fig.suptitle("foo")
    ax.set_title("bar")


@mpl_test("manual_colorbar")
def _(fig):
    axes = fig.subplots(1, 2)
    pts = axes[1].scatter([0, 1], [0, 1], c=[1, 5])
    ax_rect = axes[1].get_position()
    cax = fig.add_axes([ax_rect.x1 + 0.005, ax_rect.y0, 0.015, ax_rect.height])
    fig.colorbar(pts, cax=cax)
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", UserWarning)
        fig.tight_layout()


@mpl_test("tight_pads")
def _(fig):
    fig.set_layout_engine("tight", pad=0.15)
    fig.subplots()
