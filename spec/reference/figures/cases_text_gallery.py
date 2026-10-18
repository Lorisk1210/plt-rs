"""Text and annotations from the gallery: alignment against a box, text
rotated with its transform, arrow-shaped boxes, the anatomy of an
annotation arrow (connect, clip, shrink, mutate), simple annotations with
relpos, rotated multi-line boxes and annotations of other annotations."""

import numpy as np
from matplotlib import patches

from spec_support import case, rng


@case("text/alignment_against_box", figsize=(6, 4.5))
def _(fig):
    ax = fig.add_axes((0, 0, 1, 1))
    left, width = 0.25, 0.5
    bottom, height = 0.25, 0.5
    right = left + width
    top = bottom + height
    ax.add_patch(patches.Rectangle((left, bottom), width, height, fill=False,
                                   transform=ax.transAxes, clip_on=False))
    kw = dict(transform=ax.transAxes)
    ax.text(left, bottom, "left top", ha="left", va="top", **kw)
    ax.text(left, bottom, "left bottom", ha="left", va="bottom", **kw)
    ax.text(right, top, "right bottom", ha="right", va="bottom", **kw)
    ax.text(right, top, "right top", ha="right", va="top", **kw)
    ax.text(right, bottom, "center top", ha="center", va="top", **kw)
    ax.text(left, 0.5 * (bottom + top), "right center", ha="right", va="center",
            rotation="vertical", **kw)
    ax.text(left, 0.5 * (bottom + top), "left center", ha="left", va="center",
            rotation="vertical", **kw)
    ax.text(0.5 * (left + right), 0.5 * (bottom + top), "middle", ha="center", va="center",
            fontsize=20, color="red", **kw)
    ax.text(right, 0.5 * (bottom + top), "centered", ha="center", va="center",
            rotation="vertical", **kw)
    ax.text(left, top, "rotated\nwith newlines", ha="center", va="center", rotation=45, **kw)
    ax.set_axis_off()


@case("text/rotation_relative_to_line", figsize=(6, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot(range(0, 8), range(0, 8))
    ax.set_xlim([-10, 10])
    ax.text(-8, 0, "text 45° in screen coordinates", fontsize=14, rotation=45,
            rotation_mode="anchor")
    ax.text(0, 0, "text 45° in data coordinates", fontsize=14, rotation=45,
            rotation_mode="anchor", transform_rotates_text=True)
    ax.text(-9, 6, "-30° in data", fontsize=10, rotation=-30, transform_rotates_text=True,
            bbox=dict(fc="w", ec="tab:blue"))


@case("text/arrow_boxes_rotated", figsize=(5, 5))
def _(fig):
    ax = fig.subplots()
    ax.set_aspect(1)
    g = rng(2)
    ax.scatter(-1 + g.normal(100), -1 + g.normal(100), color="r")
    ax.scatter(1 + g.normal(100), 1 + g.normal(100), color="g")
    bbox_props = dict(boxstyle="round", fc="w", ec="0.5", alpha=0.9)
    ax.text(-2, -2, "Sample A", ha="center", va="center", size=20, bbox=bbox_props)
    ax.text(2, 2, "Sample B", ha="center", va="center", size=20, bbox=bbox_props)
    t = ax.text(0, 0, "Direction", ha="center", va="center", rotation=45, size=15,
                bbox=dict(boxstyle="rarrow", fc=(0.8, 0.9, 0.9), ec="b", lw=2))
    t.get_bbox_patch().set_boxstyle("rarrow", pad=0.6)
    ax.text(2.5, -2.5, "Back", ha="center", va="center", rotation=-30, size=12,
            bbox=dict(boxstyle="larrow,pad=0.4", fc="lightyellow", ec="k"))
    ax.text(-2.5, 2.5, "Both", ha="center", va="center", rotation=90, size=12,
            bbox=dict(boxstyle="darrow,pad=0.3", fc="mistyrose", ec="tab:red", lw=1.5))
    ax.set_xlim(-4, 4)
    ax.set_ylim(-4, 4)


@case("text/annotation_arrow_anatomy", figsize=(6, 6))
def _(fig):
    axs = fig.subplots(2, 2)
    x1, y1 = 0.3, 0.3
    x2, y2 = 0.7, 0.7
    for ax, (label, style, patch_b, shrink) in zip(axs.flat, [
        ("connect", "-", False, 0),
        ("clip", "-", True, 0),
        ("shrink", "-", True, 5),
        ("mutate", "fancy", True, 5),
    ]):
        ax.plot([x1, x2], [y1, y2], ".")
        el = patches.Ellipse((x1, y1), 0.3, 0.4, angle=30, alpha=0.2)
        ax.add_artist(el)
        ax.annotate("", xy=(x1, y1), xycoords="data", xytext=(x2, y2), textcoords="data",
                    arrowprops=dict(arrowstyle=style, color="0.5",
                                    patchB=el if patch_b else None, shrinkB=shrink,
                                    connectionstyle="arc3,rad=0.3"))
        ax.text(0.05, 0.95, label, transform=ax.transAxes, ha="left", va="top")
        ax.set(xlim=(0, 1), ylim=(0, 1), xticks=[], yticks=[], aspect=1)


@case("text/annotate_simple_relpos", figsize=(6, 3))
def _(fig):
    axs = fig.subplots(1, 3)
    for ax in axs:
        ax.set(xlim=(0, 1), ylim=(0, 1), xticks=[], yticks=[])
    axs[0].annotate("Test", xy=(0.2, 0.2), xycoords="data", xytext=(0.8, 0.8),
                    textcoords="data", size=20, va="center", ha="center",
                    arrowprops=dict(arrowstyle="simple", connectionstyle="arc3,rad=-0.2"))
    axs[1].annotate("Test", xy=(0.2, 0.2), xycoords="data", xytext=(0.8, 0.8),
                    textcoords="data", size=20, va="center", ha="center",
                    bbox=dict(boxstyle="round4", fc="w"),
                    arrowprops=dict(arrowstyle="-|>", connectionstyle="arc3,rad=-0.2", fc="w"))
    axs[2].annotate("Test", xy=(0.2, 0.2), xycoords="data", xytext=(0.8, 0.8),
                    textcoords="data", size=20, va="center", ha="center",
                    bbox=dict(boxstyle="round", fc="w"),
                    arrowprops=dict(arrowstyle="-|>", connectionstyle="arc3,rad=0.2",
                                    relpos=(0.0, 0.0), fc="w"))
    axs[2].annotate("Test", xy=(0.2, 0.2), xycoords="data", xytext=(0.8, 0.8),
                    textcoords="data", size=20, va="center", ha="center",
                    bbox=dict(boxstyle="round", fc="w"),
                    arrowprops=dict(arrowstyle="-|>", connectionstyle="arc3,rad=-0.2",
                                    relpos=(1.0, 0.0), fc="w"))


@case("text/rotated_multiline_boxes", figsize=(6, 4))
def _(fig):
    ax = fig.subplots()
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 6)
    text = "first line\nsecond, longer line\n3"
    for x, (rotation, mode, align) in zip([1.5, 5, 8.5], [
        (20, "default", "left"),
        (-35, "anchor", "center"),
        (90, "default", "right"),
    ]):
        ax.plot(x, 3, "k+", ms=12)
        ax.text(x, 3, text, rotation=rotation, rotation_mode=mode, multialignment=align,
                ha="center", va="center", linespacing=1.5,
                bbox=dict(boxstyle="round,pad=0.4", fc="lavender", ec="slateblue"))


@case("text/annotate_annotation_chain", figsize=(6, 4))
def _(fig):
    ax = fig.subplots()
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 6)
    an1 = ax.annotate("Test 1", xy=(0.5, 0.5), xycoords="data", va="center", ha="center",
                      bbox=dict(boxstyle="round", fc="w"))
    an2 = ax.annotate("Test 2", xy=(1, 0.5), xycoords=an1, xytext=(30, 0),
                      textcoords="offset points", va="center", ha="left",
                      bbox=dict(boxstyle="round", fc="w"),
                      arrowprops=dict(arrowstyle="->"))
    ax.annotate("Test 3", xy=(0.5, 1), xycoords=an2, xytext=(0.5, 1.1), textcoords=(an1, "axes fraction"),
                va="bottom", ha="center", bbox=dict(boxstyle="sawtooth", fc="w"),
                arrowprops=dict(patchB=an2.get_bbox_patch(), connectionstyle="arc3,rad=0.2",
                                arrowstyle="->"))
    an4 = ax.annotate("Test 4", xy=(6, 2), xytext=(8, 5), va="center", ha="center",
                      bbox=dict(boxstyle="circle,pad=0.5", fc="0.9"),
                      arrowprops=dict(arrowstyle="wedge,tail_width=0.7", fc="tab:orange",
                                      ec="none", connectionstyle="arc3,rad=-0.3"))
    ax.annotate("from 4", xy=(0, 0.5), xycoords=an4, xytext=(-60, -40),
                textcoords="offset points", ha="right",
                arrowprops=dict(arrowstyle="<|-", shrinkA=0, shrinkB=0))


@case("text/annotate_offset_from", figsize=(4, 3))
def _(fig):
    # After the gallery's annotate_simple_coord03 (text.OffsetFrom).
    from matplotlib.text import OffsetFrom
    ax = fig.subplots()
    an1 = ax.annotate("Test 1", xy=(0.5, 0.5), xycoords="data", va="center", ha="center",
                      bbox=dict(boxstyle="round", fc="w"))
    ax.annotate("Test 2", xy=(0.1, 0.1), xycoords="data", xytext=(0, -10),
                textcoords=OffsetFrom(an1, (0.5, 0)), va="top", ha="center",
                bbox=dict(boxstyle="round", fc="w"), arrowprops=dict(arrowstyle="->"))
    ax.annotate("Test 3", xy=(0.9, 0.8), xytext=(5, 5),
                textcoords=OffsetFrom(an1, (1, 1), unit="pixels"), va="bottom", ha="left",
                arrowprops=dict(arrowstyle="->"))
