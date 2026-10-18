"""Offset boxes and small API gaps: anchored texts and boxes, packers,
padded and auxiliary-transform boxes, annotation boxes holding texts,
images and drawing areas, the ``axes_grid1`` anchored artists,
``Axes.arrow``, ``Axes.ecdf``, ``pyplot.figtext`` and ``pyplot.xkcd``."""

import matplotlib
import matplotlib.pyplot as plt
import numpy as np
from matplotlib.font_manager import FontProperties
from matplotlib.offsetbox import (AnchoredOffsetbox, AnchoredText, AnnotationBbox,
                                  AuxTransformBox, DrawingArea, HPacker, OffsetImage,
                                  PaddedBox, TextArea, VPacker)
from matplotlib.patches import Circle, Ellipse, Rectangle
from mpl_toolkits.axes_grid1.anchored_artists import (AnchoredAuxTransformBox,
                                                      AnchoredDirectionArrows,
                                                      AnchoredDrawingArea, AnchoredSizeBar)

from spec_support import case, rng

T = np.linspace(0, 10, 40)

# The keys ``plt.xkcd()`` sets.
XKCD_KEYS = (
    "font.family", "font.size", "path.sketch", "path.effects", "axes.linewidth",
    "lines.linewidth", "figure.facecolor", "grid.linewidth", "axes.grid",
    "axes.unicode_minus", "axes.edgecolor", "xtick.major.size", "xtick.major.width",
    "ytick.major.size", "ytick.major.width",
)


def _xkcd_rc(**sketch):
    """The rcParams ``plt.xkcd(**sketch)`` sets, as a case ``rc``: the
    framework builds and draws the figure inside it, so the figure patch,
    the lazily created ticks and every artist pick the style up."""
    with plt.xkcd(**sketch):
        return {key: matplotlib.rcParams[key] for key in XKCD_KEYS}


def _unit_axes(fig):
    ax = fig.subplots()
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 1)
    return ax


def _gradient(rows, cols):
    """A smooth scalar field for images."""
    y, x = np.mgrid[0:rows, 0:cols]
    return np.sin(x / 3.0) + np.cos(y / 4.0)


# --- anchored boxes -----------------------------------------------------------


@case("offsetbox/anchored_text")
def _(fig):
    ax = fig.subplots()
    ax.plot(T, np.sin(T))
    at = AnchoredText("Figure 1a", prop=dict(size=15), frameon=True, loc="upper left")
    at.patch.set_boxstyle("round,pad=0.,rounding_size=0.2")
    ax.add_artist(at)
    ax.add_artist(AnchoredText("lower right\ntwo lines", loc="lower right",
                               prop=dict(color="tab:red")))
    ax.add_artist(AnchoredText("no frame", loc="center", frameon=False, pad=0.1, borderpad=0.2))
    ax.add_artist(AnchoredText("centred, wide pads", loc="upper center",
                               prop=dict(ha="center", size=8), borderpad=(2.0, 0.3)))
    ax.add_artist(AnchoredText("anchored at (0.6, 0.3)", loc="lower left",
                               bbox_to_anchor=(0.6, 0.3), bbox_transform=ax.transAxes,
                               borderpad=0.0))


@case("offsetbox/anchored_packers")
def _(fig):
    ax = fig.subplots()
    ax.plot(T, np.cos(T), color="0.6")
    box1 = TextArea(" Test: ", textprops=dict(color="k"))
    box2 = DrawingArea(60, 20, 0, 0)
    for x, angle, color in [(10, 30, "r"), (30, 170, "g"), (50, 230, "b")]:
        box2.add_artist(Ellipse((x, 10), width=16, height=5, angle=angle, fc=color))
    box = HPacker(children=[box1, box2], align="center", pad=0, sep=5)
    ax.add_artist(AnchoredOffsetbox(loc="lower left", child=box, pad=0., frameon=True,
                                    bbox_to_anchor=(0., 1.02), bbox_transform=ax.transAxes,
                                    borderpad=0.))
    rows = []
    for label, color in [("alpha", "tab:blue"), ("beta, longer", "tab:orange")]:
        swatch = DrawingArea(20, 10, 0, 0)
        swatch.add_artist(Rectangle((0, 0), 20, 10, fc=color))
        rows.append(HPacker(children=[swatch, TextArea(label)], align="baseline", pad=0, sep=4))
    title = TextArea("Key", textprops=dict(weight="bold"))
    column = VPacker(children=[title, *rows], align="right", pad=2, sep=3)
    ax.add_artist(AnchoredOffsetbox(loc="upper right", child=column, pad=0.3))
    wide = VPacker(children=[TextArea("expand mode"),
                             HPacker(children=[TextArea("a"), TextArea("b"), TextArea("c")],
                                     mode="expand", pad=0, sep=0)],
                   align="center", pad=0, sep=2, width=120)
    ax.add_artist(AnchoredOffsetbox(loc="lower right", child=wide, frameon=True))


@case("offsetbox/padded_and_aux_boxes")
def _(fig):
    ax = fig.subplots()
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 5)
    padded = PaddedBox(TextArea("padded box"), pad=6, draw_frame=True,
                       patch_attrs=dict(facecolor="lightyellow", edgecolor="tab:brown"))
    ax.add_artist(AnchoredOffsetbox(loc="upper left", child=padded, frameon=False))
    aux = AuxTransformBox(ax.transData)
    aux.add_artist(Rectangle((0, 0), 2, 1, fc="tab:blue"))
    aux.add_artist(Ellipse((3, 0.5), 1.5, 1, fc="tab:green", alpha=0.7))
    ax.add_artist(AnchoredOffsetbox(loc="lower right", child=aux, pad=0.2))
    clipped = DrawingArea(40, 30, 0, 0, clip=True)
    clipped.add_artist(Circle((40, 30), 25, fc="tab:purple"))
    ax.add_artist(AnchoredOffsetbox(loc="center", child=clipped, frameon=True))
    unclipped = DrawingArea(40, 30, 5, 5)
    unclipped.add_artist(Circle((40, 30), 25, fc="tab:purple"))
    ax.add_artist(AnchoredOffsetbox(loc="center left", child=unclipped, frameon=True))


# --- annotation boxes ---------------------------------------------------------


@case("offsetbox/annotation_bbox_text")
def _(fig):
    ax = _unit_axes(fig)
    xy = (0.5, 0.7)
    ax.plot(*xy, ".r")
    ax.add_artist(AnnotationBbox(TextArea("Test 1"), xy, xybox=(-20, 40), xycoords="data",
                                 boxcoords="offset points", arrowprops=dict(arrowstyle="->"),
                                 bboxprops=dict(boxstyle="sawtooth")))
    ax.add_artist(AnnotationBbox(TextArea("Test 2"), xy, xybox=(1.02, xy[1]), xycoords="data",
                                 boxcoords=("axes fraction", "data"), box_alignment=(0., 0.5),
                                 arrowprops=dict(arrowstyle="->")))
    ax.add_artist(AnnotationBbox(TextArea("fontsize 14,\nround box"), (0.2, 0.2),
                                 xybox=(0.45, 0.35), fontsize=14, pad=0.6,
                                 bboxprops=dict(boxstyle="round", fc="lavender"),
                                 arrowprops=dict(arrowstyle="-|>", connectionstyle="arc3,rad=0.3",
                                                 color="tab:red")))
    ax.add_artist(AnnotationBbox(TextArea("hidden: point outside"), (1.5, 0.5),
                                 xybox=(0.5, 0.1)))
    ax.add_artist(AnnotationBbox(TextArea("shown: annotation_clip=False"), (1.5, 0.1),
                                 xybox=(0.7, 0.05), annotation_clip=False, frameon=False,
                                 arrowprops=dict(arrowstyle="-[", relpos=(1.0, 0.5))))


@case("offsetbox/annotation_bbox_image")
def _(fig):
    ax = _unit_axes(fig)
    xy = (0.3, 0.55)
    ax.plot(*xy, "o", color="tab:red")
    image = OffsetImage(np.arange(100).reshape((10, 10)), zoom=2)
    image.image.axes = ax
    ax.add_artist(AnnotationBbox(image, xy, xybox=(-50., 50.), xycoords="data",
                                 boxcoords="offset points", pad=0.3,
                                 arrowprops=dict(arrowstyle="->")))
    field = _gradient(24, 32)
    small = OffsetImage(field, zoom=1.5, cmap="plasma", interpolation="bilinear")
    ax.add_artist(AnnotationBbox(small, (0.75, 0.3), frameon=False))
    rgb = np.dstack([np.linspace(0, 1, 16)[None, :].repeat(12, 0),
                     np.linspace(1, 0, 12)[:, None].repeat(16, 1),
                     np.full((12, 16), 0.5)])
    ax.add_artist(AnnotationBbox(OffsetImage(rgb, zoom=3), (0.8, 0.8), xybox=(0.55, 0.85),
                                 box_alignment=(1.0, 1.0), bboxprops=dict(edgecolor="tab:blue",
                                                                         linewidth=2),
                                 arrowprops=dict(arrowstyle="fancy", color="0.5")))


@case("offsetbox/annotation_bbox_drawing_area")
def _(fig):
    ax = _unit_axes(fig)
    xy = (0.6, 0.4)
    ax.plot(*xy, "s", color="k")
    da = DrawingArea(20, 20, 0, 0)
    da.add_artist(Circle((10, 10), 10))
    ax.add_artist(AnnotationBbox(da, xy, xybox=(1., xy[1]), xycoords="data",
                                 boxcoords=("axes fraction", "data"), box_alignment=(0.2, 0.5),
                                 arrowprops=dict(arrowstyle="->"), bboxprops=dict(alpha=0.5)))
    bars = DrawingArea(36, 24, 0, 0)
    for i, height in enumerate([8, 20, 14]):
        bars.add_artist(Rectangle((2 + 12 * i, 0), 8, height, fc=f"C{i}"))
    ax.add_artist(AnnotationBbox(bars, xy, xybox=(0.2, 0.8), xycoords="data",
                                 boxcoords="axes fraction",
                                 arrowprops=dict(arrowstyle="-|>",
                                                 connectionstyle="angle,angleA=0,angleB=90,rad=10",
                                                 shrinkA=0, shrinkB=5, mutation_scale=20)))
    ax.add_artist(AnnotationBbox(TextArea("figure pixels"), (0.6, 0.4),
                                 xybox=(80, 60), boxcoords="figure pixels",
                                 box_alignment=(0, 0), arrowprops=dict(arrowstyle="wedge")))


# --- axes_grid1 anchored artists ------------------------------------------------


@case("offsetbox/anchored_size_bar")
def _(fig):
    ax = fig.subplots()
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    ax.add_artist(AnchoredSizeBar(ax.transData, 1, "1 unit", "lower left"))
    ax.add_artist(AnchoredSizeBar(ax.transData, 3, "3 units", "upper right", pad=0.5, sep=5,
                                  borderpad=0.5, frameon=False, size_vertical=0.5,
                                  color="tab:red", label_top=True,
                                  fontproperties=FontProperties(size=14)))
    ax.add_artist(AnchoredSizeBar(ax.transData, 2, "2 units, unfilled", "lower right",
                                  size_vertical=0.4, fill_bar=False, color="tab:blue"))
    ax.add_artist(AnchoredSizeBar(ax.transAxes, 0.25, "quarter axes", "center left",
                                  size_vertical=0.01))


@case("offsetbox/anchored_direction_arrows")
def _(fig):
    ax = fig.subplots()
    ax.imshow(_gradient(30, 40), cmap="gray")
    ax.add_artist(AnchoredDirectionArrows(ax.transAxes, "X", "Y"))
    ax.add_artist(AnchoredDirectionArrows(ax.transAxes, "East", "South", loc="lower left",
                                          color="k", aspect_ratio=-1, sep_x=0.02, sep_y=-0.01,
                                          text_props={"ec": "w", "fc": "k"}))
    ax.add_artist(AnchoredDirectionArrows(ax.transAxes, "a", "b", loc="upper right", angle=30,
                                          color="tab:orange", frameon=True, length=0.1,
                                          fontsize=0.05, back_length=0.3, head_width=6,
                                          head_length=8, tail_width=1, alpha=0.8))


@case("offsetbox/anchored_drawing_area_and_aux")
def _(fig):
    ax = fig.subplots()
    ax.plot(T, T * 0.1, color="0.7")
    ada = AnchoredDrawingArea(40, 20, 0, 0, loc="upper right", pad=0., frameon=False)
    ada.drawing_area.add_artist(Circle((10, 10), 10, fc="tab:blue"))
    ada.drawing_area.add_artist(Circle((30, 10), 5, fc="tab:red"))
    ax.add_artist(ada)
    box = AnchoredAuxTransformBox(ax.transData, loc="upper left")
    box.drawing_area.add_artist(Ellipse((0, 0), width=1.5, height=0.4, angle=30))
    ax.add_artist(box)
    # matplotlib < 3.8's AnchoredEllipse(ax.transData, 1.0, 0.3, 0.0, "lower right"):
    # an AnchoredAuxTransformBox holding an Ellipse, with pad and borderpad 0.1.
    ellipse = AnchoredAuxTransformBox(ax.transData, loc="lower right", pad=0.1, borderpad=0.1)
    ellipse.drawing_area.add_artist(Ellipse((0, 0), width=1.0, height=0.3, angle=0.0))
    ax.add_artist(ellipse)


# --- Axes.arrow, Axes.ecdf, pyplot.figtext ------------------------------------


@case("offsetbox/axes_arrow")
def _(fig):
    ax = fig.subplots()
    ax.arrow(0, 0, 0.5, 0.5, head_width=0.05, head_length=0.1, fc="k", ec="k")
    ax.arrow(0.1, 0.8, 0.6, 0, width=0.02, length_includes_head=True, color="tab:red")
    ax.arrow(0.9, 0.1, 0, 0.5, width=0.03, shape="left", overhang=0.3, color="tab:green")
    ax.arrow(0.5, 0.1, 0.3, 0.2, width=0.01, shape="right", head_starts_at_zero=True,
             head_width=0.08, alpha=0.6)
    ax.arrow(0.2, 0.6, -0.15, -0.3, width=0.015, head_length=0.05, overhang=-0.4,
             fill=False, linewidth=2, edgecolor="tab:purple")


@case("offsetbox/ecdf", figsize=(9, 3.5))
def _(fig):
    ax1, ax2, ax3 = fig.subplots(1, 3)
    data = rng(301).normal(150, 0.0, 1.0)
    ax1.ecdf(data, label="CDF")
    ax1.ecdf(data, complementary=True, label="CCDF")
    ax1.legend()
    weights = rng(302).uniform(150, 0.2, 2.0)
    ax2.ecdf(data, weights=weights, orientation="horizontal", color="tab:green", lw=2)
    ax2.ecdf(data, orientation="horizontal", complementary=True, linestyle="--")
    counts = np.array([1, 2, 2, 3, 5, 5, 5, 8, 8, 13], dtype=float)
    ax3.ecdf(counts, compress=True, marker="o")
    # Tied values carry equal weights, so the order argsort gives ties is moot.
    ax3.ecdf(counts, weights=counts, compress=True, complementary=True)


@case("offsetbox/figtext")
def _(fig):
    ax = fig.subplots()
    ax.plot(T, np.sin(T) ** 2)
    fig.subplots_adjust(bottom=0.2)
    plt.figtext(0.5, 0.03, "a caption centred below the axes", ha="center", fontsize=12)
    plt.figtext(0.02, 0.98, "top left", va="top", color="tab:blue")
    plt.figtext(0.98, 0.5, "rotated on the right", rotation=90, ha="right", va="center",
                bbox=dict(boxstyle="round", fc="0.9"))


# --- pyplot.xkcd ----------------------------------------------------------------


@case("offsetbox/xkcd_bacon", rc=_xkcd_rc())
def _(fig):
    ax = fig.add_axes((0.1, 0.2, 0.8, 0.7))
    ax.spines[["top", "right"]].set_visible(False)
    ax.set_xticks([])
    ax.set_yticks([])
    ax.set_ylim(-30, 10)
    data = np.ones(100)
    data[70:] -= np.arange(30)
    ax.annotate("THE DAY I REALIZED\nI COULD COOK BACON\nWHENEVER I WANTED",
                xy=(70, 1), arrowprops=dict(arrowstyle="->"), xytext=(15, -10))
    ax.plot(data)
    ax.set_xlabel("time")
    ax.set_ylabel("my overall health")
    plt.figtext(0.5, 0.05, '"Stove Ownership" from xkcd by Randall Munroe', ha="center")


@case("offsetbox/xkcd_ticks", rc=_xkcd_rc(scale=2, length=50, randomness=4))
def _(fig):
    ax = fig.subplots()
    ax.plot(T, np.sin(T), label="sine")
    ax.plot(T, 0.5 * np.cos(T), label="half cosine")
    ax.bar([2, 5, 8], [0.4, -0.3, 0.6], width=1.2, color="tab:orange", label="bars")
    ax.set_title("sketchy")
    ax.set_xlabel("x")
    ax.legend(loc="lower left")


# --- more anchored boxes, packers and annotation boxes ---------------------------


@case("offsetbox/anchored_text_location_codes")
def _(fig):
    ax = _unit_axes(fig)
    for code in range(1, 11):
        ax.add_artist(AnchoredText(f"loc={code}", loc=code, prop=dict(size=8), pad=0.2,
                                   borderpad=0.3))
    styled = AnchoredText("styled frame", loc="center", prop=dict(size=11, color="tab:brown"),
                          bbox_to_anchor=(0.5, 0.3), bbox_transform=ax.transAxes)
    styled.patch.set(facecolor="lightyellow", edgecolor="tab:orange", linewidth=1.5)
    styled.patch.set_boxstyle("round4,pad=0.2")
    ax.add_artist(styled)


@case("offsetbox/packer_alignments_and_modes")
def _(fig):
    ax = _unit_axes(fig)

    def texts(*sizes):
        return [TextArea(f"{s}pt", textprops=dict(size=s)) for s in sizes]

    for loc, align in [("upper left", "top"), ("upper right", "bottom"),
                       ("center left", "center"), ("center right", "baseline")]:
        box = HPacker(children=texts(8, 14, 20), align=align, pad=2, sep=6)
        ax.add_artist(AnchoredOffsetbox(loc=loc, child=box, frameon=True))
    for loc, align in [("lower left", "left"), ("lower right", "right")]:
        column = VPacker(children=texts(8, 12, 16), align=align, pad=2, sep=2)
        ax.add_artist(AnchoredOffsetbox(loc=loc, child=column, frameon=True))
    equal = HPacker(children=texts(8, 10, 12), mode="equal", sep=4, width=150)
    ax.add_artist(AnchoredOffsetbox(loc="lower center", child=equal, frameon=True))
    fixed = VPacker(children=texts(9, 9), mode="fixed", sep=10, align="center")
    ax.add_artist(AnchoredOffsetbox(loc="upper center", child=fixed, frameon=True))


@case("offsetbox/drawing_area_descents_in_packer")
def _(fig):
    ax = _unit_axes(fig)
    shifted = DrawingArea(30, 20, xdescent=5, ydescent=8)
    shifted.add_artist(Rectangle((-5, -8), 30, 20, fc="tab:cyan"))
    shifted.add_artist(Circle((0, 0), 3, fc="k"))
    row = HPacker(children=[TextArea("before"), shifted, TextArea("after")],
                  align="baseline", pad=0, sep=4)
    anchored = AnchoredOffsetbox(loc="upper left", child=row, frameon=True)
    anchored.patch.set_boxstyle("round,pad=0.1")
    ax.add_artist(anchored)
    clipped = DrawingArea(50, 30, 10, 10, clip=True)
    clipped.add_artist(Ellipse((15, 5), 60, 40, fc="tab:pink", ec="k"))
    ax.add_artist(AnchoredOffsetbox(loc="center", child=clipped, frameon=True, pad=0.0))
    top = DrawingArea(40, 25, 0, 0)
    top.add_artist(Rectangle((0, 0), 40, 25, fc="tab:olive", alpha=0.5))
    column = VPacker(children=[top, TextArea("under the area")], align="center", pad=0, sep=2)
    ax.add_artist(AnchoredOffsetbox(loc="lower right", child=column, frameon=False))


@case("offsetbox/annotation_bbox_packed_children")
def _(fig):
    ax = _unit_axes(fig)
    xy = (0.3, 0.4)
    ax.plot(*xy, "o", color="tab:red")
    bars = DrawingArea(36, 24, 0, 0)
    for i, height in enumerate([10, 24, 16]):
        bars.add_artist(Rectangle((2 + 12 * i, 0), 8, height, fc=f"C{i}"))
    card = VPacker(children=[TextArea("sales", textprops=dict(weight="bold")), bars],
                   align="center", pad=0, sep=3)
    ax.add_artist(AnnotationBbox(card, xy, xybox=(60, 50), boxcoords="offset points",
                                 arrowprops=dict(arrowstyle="->",
                                                 connectionstyle="arc3,rad=-0.2")))
    swatch = DrawingArea(12, 12, 0, 0)
    swatch.add_artist(Circle((6, 6), 6, fc="tab:green"))
    row = HPacker(children=[swatch, TextArea("green marker")], align="center", pad=0, sep=4)
    ax.add_artist(AnnotationBbox(row, (0.75, 0.25), xybox=(0.95, 0.6),
                                 box_alignment=(1.0, 0.0),
                                 bboxprops=dict(boxstyle="round", fc="0.95"),
                                 arrowprops=dict(arrowstyle="-|>", color="tab:green")))
    ax.plot(0.75, 0.25, "s", color="tab:green")


@case("offsetbox/annotation_bbox_aux_and_padded")
def _(fig):
    ax = fig.subplots()
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 5)
    aux = AuxTransformBox(ax.transData)
    aux.add_artist(Rectangle((0, 0), 1.5, 0.6, angle=20, fc="tab:blue"))
    aux.add_artist(Circle((2.2, 0.3), 0.3, fc="tab:orange"))
    ax.add_artist(AnnotationBbox(aux, (2, 1), xybox=(6, 3.5),
                                 arrowprops=dict(arrowstyle="->")))
    padded = PaddedBox(TextArea("padded text"), pad=5, draw_frame=True,
                       patch_attrs=dict(facecolor="honeydew", edgecolor="tab:green"))
    ax.add_artist(AnnotationBbox(padded, (8, 1), xybox=(8, 2.5), frameon=False,
                                 arrowprops=dict(arrowstyle="-[")))
    ax.plot([2, 8], [1, 1], "k.")


@case("offsetbox/annotation_bbox_mixed_coords")
def _(fig):
    ax = fig.subplots()
    ax.plot(T, np.sin(T))
    ax.add_artist(AnnotationBbox(TextArea("blended: x data, y axes"), (5, 0.9),
                                 xycoords=("data", "axes fraction"), xybox=(2, 0.6),
                                 boxcoords=("data", "axes fraction"),
                                 arrowprops=dict(arrowstyle="->")))
    ax.add_artist(AnnotationBbox(TextArea("figure fraction"), (0.5, 0.05),
                                 xycoords="figure fraction", xybox=(0.8, 0.2),
                                 boxcoords="axes fraction", annotation_clip=False,
                                 arrowprops=dict(arrowstyle="wedge,tail_width=0.5",
                                                 fc="0.8", ec="0.4")))
    ax.add_artist(AnnotationBbox(TextArea("axes pixels"), (8, -0.9), xybox=(300, 40),
                                 boxcoords="axes pixels", pad=0.8,
                                 bboxprops=dict(boxstyle="larrow", fc="lavender"),
                                 arrowprops=dict(arrowstyle="->", shrinkB=8)))


@case("offsetbox/textarea_multiline_baseline")
def _(fig):
    ax = _unit_axes(fig)
    row = HPacker(children=[TextArea("one\ntwo\nthree", multilinebaseline=True),
                            TextArea("base"),
                            TextArea("left\nlines", multilinebaseline=False),
                            TextArea("right\naligned lines", textprops=dict(ha="right"))],
                  align="baseline", pad=0, sep=8)
    ax.add_artist(AnchoredOffsetbox(loc="center", child=row, frameon=True))
    ax.axhline(0.5, color="0.8")


@case("offsetbox/anchored_text_boxstyles")
def _(fig):
    ax = _unit_axes(fig)
    styles = ["round4,pad=0.3", "sawtooth", "larrow", "roundtooth", "circle"]
    for i, style in enumerate(styles):
        at = AnchoredText(style, loc="center", bbox_to_anchor=(0.2 + 0.15 * i, 0.15 + 0.17 * i),
                          bbox_transform=ax.transAxes, prop=dict(size=9))
        at.patch.set_boxstyle(style)
        at.patch.set(facecolor=f"C{i}", alpha=0.4, hatch="//" if i % 2 else None)
        ax.add_artist(at)
