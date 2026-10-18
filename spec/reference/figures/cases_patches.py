"""Patches and collections: every matplotlib.patches class, hatching, edge
styles, alpha/zorder, the collection classes, broken_barh/fill and clip
paths."""

import numpy as np
from matplotlib import collections, patches, tri
from matplotlib.path import Path

from spec_support import case, rng

BOXSTYLES = ["square", "round", "round4", "sawtooth", "roundtooth",
             "circle", "ellipse", "darrow", "larrow", "rarrow"]

HATCHES = ["/", "\\", "|", "-", "+", "x", "o", "O", ".", "*"]

HATCH_COMBOS = ["//", "///", "\\\\\\", "||", "--", "++", "xx", "oo", "OO", "..", "**",
                "/o", "\\|", "|*", "-\\", "x*", "o-", "O."]


def _canvas(fig, xlim=(0, 10), ylim=(0, 10), equal=True):
    ax = fig.subplots()
    ax.set_xlim(*xlim)
    ax.set_ylim(*ylim)
    if equal:
        ax.set_aspect("equal")
    return ax


# --- simple shapes

@case("patches/rectangle_default")
def _(fig):
    ax = fig.subplots()
    ax.add_patch(patches.Rectangle((0.2, 0.3), 0.5, 0.4))


@case("patches/rectangle_angle_rotation_point", figsize=(5, 4))
def _(fig):
    ax = _canvas(fig)
    for rp, color in [("xy", "C0"), ("center", "C1"), ((5, 5), "C2")]:
        ax.add_patch(patches.Rectangle((3, 4), 4, 2, angle=30, rotation_point=rp,
                                       fc=color, alpha=0.5, ec="k"))
    ax.add_patch(patches.Rectangle((3, 4), 4, 2, fill=False, ls="--"))
    ax.plot([3, 5, 5], [4, 5, 5], "k.")


@case("patches/circle_ellipse", figsize=(5, 4))
def _(fig):
    ax = _canvas(fig)
    ax.add_patch(patches.Circle((2.5, 7), 2))
    ax.add_patch(patches.Circle((7.5, 7), 1.5, fc="none", ec="tab:red", lw=3))
    ax.add_patch(patches.Ellipse((3, 2.5), 5, 2, angle=20, fc="tab:green"))
    ax.add_patch(patches.Ellipse((7.5, 2.5), 1.5, 3.5, angle=-30, fc="gold", ec="k"))


@case("patches/wedge_width", figsize=(5, 4))
def _(fig):
    ax = _canvas(fig)
    ax.add_patch(patches.Wedge((2.5, 7.5), 2, 30, 270))
    ax.add_patch(patches.Wedge((7.5, 7.5), 2, 0, 360, width=0.7, fc="tab:orange"))
    ax.add_patch(patches.Wedge((2.5, 2.5), 2, -45, 45, width=1.2, fc="tab:green", ec="k"))
    ax.add_patch(patches.Wedge((7.5, 2.5), 2, 300, 60, fc="tab:purple", alpha=0.6))


@case("patches/arc", figsize=(5, 4))
def _(fig):
    ax = _canvas(fig)
    ax.add_patch(patches.Arc((2.5, 7.5), 4, 3, theta1=0, theta2=270, lw=2))
    ax.add_patch(patches.Arc((7.5, 7.5), 4, 2, angle=30, color="tab:red", lw=3))
    ax.add_patch(patches.Arc((2.5, 2.5), 4, 4, theta1=200, theta2=20, ls="--"))
    ax.add_patch(patches.Arc((7.5, 2.5), 3, 4, angle=-45, theta1=45, theta2=135, lw=4,
                             color="tab:green", capstyle="round"))


@case("patches/polygon_closed_open", figsize=(5, 4))
def _(fig):
    ax = _canvas(fig)
    pts = np.array([[1, 1], [4, 2], [3, 4.5], [1.5, 3.5]])
    ax.add_patch(patches.Polygon(pts, closed=True, fc="tab:blue", ec="k"))
    ax.add_patch(patches.Polygon(pts + [5, 0], closed=False, fill=False, lw=3))
    star = [[7.5 + 2 * np.cos(t) * (1 if i % 2 == 0 else 0.4), 7 + 2 * np.sin(t) * (1 if i % 2 == 0 else 0.4)]
            for i, t in enumerate(np.linspace(np.pi / 2, np.pi / 2 + 2 * np.pi, 11)[:-1])]
    ax.add_patch(patches.Polygon(star, fc="gold", ec="tab:red", lw=2, joinstyle="miter"))
    ax.add_patch(patches.Polygon([[1, 6], [4, 9], [1, 9], [4, 6]], fc="tab:green", alpha=0.6))


@case("patches/regular_and_circle_polygon", figsize=(5, 4))
def _(fig):
    ax = _canvas(fig)
    ax.add_patch(patches.RegularPolygon((2, 7.5), 3, radius=1.5))
    ax.add_patch(patches.RegularPolygon((5, 7.5), 5, radius=1.5, orientation=np.pi / 5, fc="C1"))
    ax.add_patch(patches.RegularPolygon((8, 7.5), 6, radius=1.5, fc="C2", ec="k"))
    ax.add_patch(patches.CirclePolygon((2, 2.5), 1.5, resolution=8, fc="C3"))
    ax.add_patch(patches.CirclePolygon((5, 2.5), 1.5, fc="C4"))
    ax.add_patch(patches.CirclePolygon((8, 2.5), 1.5, resolution=5, fill=False, lw=2))


@case("patches/annulus", figsize=(5, 4))
def _(fig):
    ax = _canvas(fig)
    ax.add_patch(patches.Annulus((3, 5), 2.5, 0.8))
    ax.add_patch(patches.Annulus((7.5, 5), (2, 1), 0.5, angle=30, fc="tab:red", ec="k",
                                 hatch="//"))


@case("patches/pathpatch_curves", figsize=(5, 4))
def _(fig):
    ax = _canvas(fig)
    verts = [(1, 1), (2, 5), (5, 5), (5, 2), (4, 1), (2.5, 0.5), (1, 1), (1, 1)]
    codes = [Path.MOVETO, Path.CURVE4, Path.CURVE4, Path.CURVE4, Path.LINETO,
             Path.CURVE3, Path.CURVE3, Path.CLOSEPOLY]
    ax.add_patch(patches.PathPatch(Path(verts, codes), fc="tab:orange", ec="k", lw=2))
    outer = [(6, 6), (9, 6), (9, 9), (6, 9), (6, 6)]
    inner = [(7, 7), (7, 8), (8, 8), (8, 7), (7, 7)]
    hole = Path(outer + inner, [Path.MOVETO] + [Path.LINETO] * 3 + [Path.CLOSEPOLY]
                + [Path.MOVETO] + [Path.LINETO] * 3 + [Path.CLOSEPOLY])
    ax.add_patch(patches.PathPatch(hole, fc="tab:blue", ec="k"))
    ax.add_patch(patches.PathPatch(Path([(6, 1), (9, 4), (6, 4)], [Path.MOVETO, Path.CURVE3,
                                                                   Path.CURVE3]),
                                   fill=False, lw=3, ec="tab:green"))


@case("patches/steppatch_and_stairs", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    edges = np.arange(7)
    ax.add_patch(patches.StepPatch([1, 3, 2, 5, 4, 2], edges, baseline=0.5, fill=True,
                                   alpha=0.4))
    ax.stairs([2, 4, 3, 6, 5, 3], edges, color="tab:red", lw=2)
    ax.stairs([4, 1, 2, 3, 1, 5], edges, baseline=None, orientation="vertical", ls="--")
    ax.autoscale_view()


@case("patches/shadow", figsize=(5, 4))
def _(fig):
    ax = _canvas(fig)
    rect = patches.Rectangle((1, 5), 3, 3, fc="tab:blue")
    circ = patches.Circle((7, 3), 2, fc="tab:orange")
    for p in (rect, circ):
        ax.add_patch(p)
        ax.add_patch(patches.Shadow(p, 0.3, -0.3))
    ax.add_patch(patches.Shadow(patches.Rectangle((1, 1), 2, 2, fc="tab:green"), 0.5, -0.5,
                                shade=0.3))


# --- fancy boxes and arrows

@case("patches/fancybbox_boxstyles", figsize=(6, 4))
def _(fig):
    ax = _canvas(fig, xlim=(0, 20), ylim=(0, 12))
    for i, style in enumerate(BOXSTYLES):
        x, y = 1.2 + 4 * (i % 5), 7 - 5.5 * (i // 5)
        ax.add_patch(patches.FancyBboxPatch((x, y), 1.6, 0.8, boxstyle=style + ",pad=0.2",
                                            fc="lightsteelblue", ec="navy"))
        ax.text(x + 0.8, y + 2.8, style, ha="center", fontsize=8)


@case("patches/fancybbox_mutation", figsize=(6, 3))
def _(fig):
    ax = _canvas(fig, equal=False)
    ax.add_patch(patches.FancyBboxPatch((1, 3), 2, 4, boxstyle="round,pad=0.3"))
    ax.add_patch(patches.FancyBboxPatch((4, 3), 2, 4, boxstyle="round,pad=0.3",
                                        mutation_scale=2, fc="C1"))
    ax.add_patch(patches.FancyBboxPatch((7, 3), 2, 4, boxstyle="round,pad=0.3",
                                        mutation_aspect=3, fc="C2"))


@case("patches/fancyarrow", figsize=(5, 4))
def _(fig):
    ax = _canvas(fig)
    ax.add_patch(patches.FancyArrow(1, 8, 6, 0, width=0.3))
    ax.add_patch(patches.FancyArrow(1, 6, 6, 0, width=0.3, head_width=1.2, head_length=1.5,
                                    length_includes_head=True, fc="C1"))
    ax.add_patch(patches.FancyArrow(1, 4, 6, 0, width=0.3, shape="left", overhang=0.4, fc="C2"))
    ax.add_patch(patches.FancyArrow(1, 2, 6, 0, width=0.3, shape="right", fc="C3",
                                    head_starts_at_zero=True))
    ax.add_patch(patches.FancyArrow(9, 1, 0, 7, width=0.2, fc="C4", ec="k"))


@case("patches/arrow", figsize=(5, 4))
def _(fig):
    ax = _canvas(fig)
    ax.add_patch(patches.Arrow(1, 1, 5, 5))
    ax.add_patch(patches.Arrow(2, 8, 6, 0, width=2, fc="tab:red"))
    ax.add_patch(patches.Arrow(9, 1, -3, 3, width=0.5, fc="tab:green", ec="k"))


@case("patches/fancyarrowpatch_styles", figsize=(6, 5))
def _(fig):
    ax = _canvas(fig, equal=False)
    ax.set_axis_off()
    styles = ["-", "->", "-[", "|-|", "-|>", "<-", "<->", "<|-", "<|-|>", "fancy",
              "simple", "wedge", "]-[", "]->"]
    for i, style in enumerate(styles):
        x, y = 0.5 + 5 * (i // 7), 9.3 - 1.4 * (i % 7)
        ax.add_patch(patches.FancyArrowPatch((x, y), (x + 4, y), arrowstyle=style,
                                             mutation_scale=15, fc="tab:cyan"))


@case("patches/fancyarrowpatch_connections", figsize=(6, 4))
def _(fig):
    ax = _canvas(fig, equal=False)
    styles = ["arc3,rad=0.4", "arc3,rad=-0.4", "angle3,angleA=0,angleB=90",
              "angle,angleA=0,angleB=90,rad=10", "arc,angleA=0,angleB=0,armA=20,armB=20,rad=5",
              "bar,fraction=-0.3"]
    for i, style in enumerate(styles):
        x, y = 0.5 + 3.3 * (i % 3), 5.5 - 5 * (i // 3)
        ax.plot([x, x + 2.5], [y, y + 3.5], "k.")
        ax.add_patch(patches.FancyArrowPatch((x, y), (x + 2.5, y + 3.5), arrowstyle="-|>",
                                             connectionstyle=style, mutation_scale=12))


@case("patches/fancyarrowpatch_path_shrink", figsize=(5, 4))
def _(fig):
    ax = _canvas(fig)
    path = Path([(1, 1), (5, 10), (9, 1)], [Path.MOVETO, Path.CURVE3, Path.CURVE3])
    ax.add_patch(patches.FancyArrowPatch(path=path, arrowstyle="fancy", mutation_scale=25,
                                         fc="tab:red", alpha=0.7))
    ax.add_patch(patches.FancyArrowPatch((1, 9), (9, 9), arrowstyle="<->", shrinkA=20,
                                         shrinkB=40, mutation_scale=15))
    ax.plot([1, 9], [9, 9], "k.")


@case("patches/connectionpatch_two_axes", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    ax1.plot([0, 1], [0, 1], "o-")
    ax2.plot([0, 1], [1, 0], "s-")
    ax2.add_artist(patches.ConnectionPatch(xyA=(1, 1), xyB=(0, 1), coordsA="data",
                                           coordsB="data", axesA=ax1, axesB=ax2,
                                           arrowstyle="->", color="tab:red", lw=2))
    fig.add_artist(patches.ConnectionPatch(xyA=(0.5, 0.2), xyB=(0.5, 0.2),
                                           coordsA=ax1.transAxes, coordsB=ax2.transAxes,
                                           connectionstyle="arc3,rad=-0.3", ls="--"))


# --- hatching

@case("patches/hatches_basic", figsize=(6, 3))
def _(fig):
    ax = _canvas(fig, xlim=(0, 10), ylim=(0, 4), equal=False)
    for i, hatch in enumerate(HATCHES):
        x, y = 0.2 + 2 * (i % 5), 2.2 - 2 * (i // 5)
        ax.add_patch(patches.Rectangle((x, y), 1.6, 1.6, fill=False, hatch=hatch))
        ax.text(x + 0.8, y + 1.65, repr(hatch), ha="center", fontsize=8)


@case("patches/hatches_density_combos", figsize=(6, 4))
def _(fig):
    ax = _canvas(fig, xlim=(0, 12), ylim=(0, 6), equal=False)
    for i, hatch in enumerate(HATCH_COMBOS):
        x, y = 0.2 + 2 * (i % 6), 4.1 - 2 * (i // 6)
        ax.add_patch(patches.Rectangle((x, y), 1.6, 1.5, fc="lightyellow", ec="k", hatch=hatch))
        ax.text(x + 0.8, y + 1.55, repr(hatch), ha="center", fontsize=7)


@case("patches/hatch_linewidth_color_rc", figsize=(5, 3),
      rc={"hatch.linewidth": 2.5, "hatch.color": "tab:red"})
def _(fig):
    ax = _canvas(fig, equal=False)
    ax.add_patch(patches.Rectangle((0.5, 1), 4, 8, hatch="/", fc="w", ec="k"))
    ax.add_patch(patches.Circle((7.5, 5), 2.5, hatch="x", fill=False))


@case("patches/hatch_edge_and_hatchcolor", figsize=(5, 3))
def _(fig):
    ax = _canvas(fig, equal=False)
    ax.add_patch(patches.Rectangle((0.5, 1), 2.5, 8, hatch="//", fc="w", ec="tab:blue"))
    ax.add_patch(patches.Rectangle((3.75, 1), 2.5, 8, hatch="\\\\", fc="0.9", ec="k",
                                   hatchcolor="tab:green"))
    rect = patches.Rectangle((7, 1), 2.5, 8, hatch="+", fc="w", ec="k", lw=1)
    rect.set_hatch_linewidth(3)
    ax.add_patch(rect)


@case("patches/hatched_bars_alpha", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.bar([1, 2, 3], [3, 1, 2], hatch=["/", "o", "x"], color="tab:blue", alpha=0.5,
           edgecolor="k")


# --- edge styles, alpha, zorder

@case("patches/edge_styles", figsize=(6, 4))
def _(fig):
    ax = _canvas(fig, equal=False)
    for i, ls in enumerate(["-", "--", ":", "-.", (0, (5, 1, 1, 1))]):
        ax.add_patch(patches.Rectangle((0.3 + 1.9 * i, 6.5), 1.5, 3, fill=False, lw=2, ls=ls))
    for i, js in enumerate(["miter", "round", "bevel"]):
        ax.add_patch(patches.Polygon([[0.5 + 3.3 * i, 3.5], [1.5 + 3.3 * i, 5.8],
                                      [2.5 + 3.3 * i, 3.5]], closed=False, fill=False, lw=8,
                                     joinstyle=js, capstyle=["butt", "round", "projecting"][i]))
    ax.add_patch(patches.Rectangle((0.5, 0.5), 9, 2, fc="w", ec="tab:red", lw=3, ls="--",
                                   edgegapcolor="tab:blue"))


@case("patches/alpha_zorder", figsize=(4, 4))
def _(fig):
    ax = _canvas(fig)
    ax.add_patch(patches.Circle((4, 6), 3, fc="tab:red", alpha=0.5, zorder=3))
    ax.add_patch(patches.Circle((6, 6), 3, fc="tab:green", alpha=0.5, zorder=1))
    ax.add_patch(patches.Circle((5, 4), 3, fc="tab:blue", alpha=0.5, zorder=2))
    ax.plot([0, 10], [5, 5], color="k", lw=4, zorder=2.5)
    ax.add_patch(patches.Rectangle((1, 1), 8, 1, fc=(1, 0.5, 0, 0.3), ec=(0, 0, 0, 1), lw=2))


# --- collections

@case("patches/patchcollection_cmap", figsize=(5, 4))
def _(fig):
    ax = _canvas(fig)
    shapes = [patches.Circle((2, 2), 1.5), patches.Rectangle((5, 1), 3, 2),
              patches.Wedge((3, 7), 2, 30, 300), patches.RegularPolygon((7.5, 7), 6, radius=1.8),
              patches.Ellipse((5, 4.5), 2, 1, angle=45)]
    pc = collections.PatchCollection(shapes, cmap="viridis", alpha=0.8, edgecolor="k")
    pc.set_array(np.arange(5.0))
    ax.add_collection(pc)


@case("patches/patchcollection_match_original", figsize=(4, 4))
def _(fig):
    ax = _canvas(fig)
    shapes = [patches.Circle((3, 3), 2, fc="tab:red", hatch="//"),
              patches.Rectangle((5, 5), 3, 3, fc="tab:blue", ec="k", lw=3)]
    ax.add_collection(collections.PatchCollection(shapes, match_original=True))


@case("patches/linecollection_styles", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0.0, 4 * np.pi, 60)
    segs = [np.column_stack([x, np.sin(x) + i]) for i in range(5)]
    lc = collections.LineCollection(segs, colors=["C0", "C1", "C2", "C3", "C4"],
                                    linewidths=[0.5, 1, 2, 3, 4],
                                    linestyles=["-", "--", ":", "-.", "-"])
    ax.add_collection(lc)
    ax.autoscale_view()


@case("patches/linecollection_array", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0.0, 4 * np.pi, 100)
    pts = np.column_stack([x, np.sin(x)])
    segs = np.stack([pts[:-1], pts[1:]], axis=1)
    lc = collections.LineCollection(segs, cmap="plasma", linewidth=3)
    lc.set_array(np.cos(x[:-1]))
    ax.add_collection(lc)
    ax.autoscale_view()


@case("patches/polycollection", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    verts = [[(0, 0), (1, 0), (0.5, 1)], [(1, 1), (2, 1), (2, 2), (1, 2)],
             [(2.5, 0), (3.5, 0.5), (3, 1.5), (2.2, 1)]]
    pc = collections.PolyCollection(verts, facecolors=["tab:red", "tab:green", "tab:blue"],
                                    edgecolors="k", linewidths=2, alpha=0.6)
    ax.add_collection(pc)
    ax.autoscale_view()



@case("patches/polycollection_nan_vertices", figsize=(4, 3))
def _(fig):
    # NaN vertices make the extents NaN, which turns off the single-path
    # (draw_markers) shortcut.
    ax = fig.subplots()
    x = np.linspace(0.0, 1.0, 20)
    y = 0.5 + 0.3 * np.sin(2 * np.pi * x)
    verts = np.column_stack([np.r_[np.nan, x, np.nan], np.r_[np.nan, y, np.nan]])
    ax.add_collection(collections.PolyCollection([verts], facecolors="tab:orange",
                                                 edgecolors="k", linewidths=0.5))
    ax.set_xlim(-0.1, 1.1)
    ax.set_ylim(0.0, 1.0)

@case("patches/ellipsecollection_units", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    xy = np.array([[x, y] for x in range(4) for y in range(3)], dtype=float)
    for ax, units in [(ax1, "xy"), (ax2, "points")]:
        widths = 0.3 + 0.1 * xy[:, 0] if units == "xy" else 10 + 5 * xy[:, 0]
        ec = collections.EllipseCollection(widths, widths / 2, 30 * xy[:, 1], units=units,
                                           offsets=xy, offset_transform=ax.transData,
                                           cmap="viridis")
        ec.set_array(xy[:, 0] + xy[:, 1])
        ax.add_collection(ec)
        ax.set_xlim(-1, 4)
        ax.set_ylim(-1, 3)
        ax.set_title(units)


@case("patches/circlecollection", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    g = rng(51)
    offsets = np.column_stack([g.uniform(20), g.uniform(20)])
    cc = collections.CircleCollection(g.uniform(20, 20.0, 400.0), offsets=offsets,
                                      offset_transform=ax.transData, facecolors="tab:blue",
                                      edgecolors="k", alpha=0.6)
    ax.add_collection(cc)


@case("patches/regularpolycollection", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    offsets = np.array([[i % 4, i // 4] for i in range(12)], dtype=float)
    rc = collections.RegularPolyCollection(5, rotation=0.3, sizes=50 + 30 * np.arange(12),
                                           offsets=offsets, offset_transform=ax.transData,
                                           cmap="cool", edgecolors="k")
    rc.set_array(np.arange(12.0))
    ax.add_collection(rc)
    ax.set_xlim(-1, 4)
    ax.set_ylim(-1, 3)


@case("patches/eventcollection", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    g = rng(52)
    ax.add_collection(collections.EventCollection(g.uniform(30, 0.0, 10.0), lineoffset=1,
                                                  linelength=0.8))
    ax.add_collection(collections.EventCollection(g.uniform(20, 0.0, 10.0), lineoffset=3,
                                                  linelength=1.5, color="tab:red",
                                                  linewidth=2, linestyle="--"))
    ax.add_collection(collections.EventCollection(g.uniform(15, 0.0, 4.0), orientation="vertical",
                                                  lineoffset=8, linelength=1, color="tab:green"))
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 4.5)


@case("patches/trimesh", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    g = rng(53)
    x, y = g.uniform(40), g.uniform(40)
    t = tri.Triangulation(x, y)
    mesh = collections.TriMesh(t, cmap="viridis")
    mesh.set_array(np.sin(4 * x) * np.cos(3 * y))
    ax.add_collection(mesh)
    ax.autoscale_view()


@case("patches/broken_barh", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    ax.broken_barh([(1, 2), (4, 1), (6, 3)], (1, 2))
    ax.broken_barh([(0, 3), (5, 4)], (4, 2), facecolors=["tab:red", "tab:green"],
                   edgecolor="k", hatch="//")
    ax.set_ylim(0, 7)


@case("patches/fill_polygons", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    t = np.linspace(0.0, 2 * np.pi, 7)
    ax.fill(np.cos(t), np.sin(t), "tab:blue", np.cos(t) + 2.5, np.sin(t), "tab:orange", alpha=0.7)
    ax.fill([0, 1, 0.5], [-2, -2, -1], fill=False, hatch="x", ec="k")


# --- clipping

@case("patches/clip_path_artists", figsize=(4, 4))
def _(fig):
    ax = _canvas(fig)
    clip = patches.Circle((5, 5), 3.5, transform=ax.transData)
    ax.add_patch(patches.Circle((5, 5), 3.5, fill=False, ls=":"))
    for i in range(10):
        line, = ax.plot([0, 10], [i, 10 - i + 3], lw=3)
        line.set_clip_path(clip)
    rect = patches.Rectangle((0, 0), 6, 6, fc="tab:gray", alpha=0.5)
    ax.add_patch(rect)
    rect.set_clip_path(clip)
