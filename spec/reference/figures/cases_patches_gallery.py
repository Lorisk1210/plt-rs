"""Patches and collections from the gallery: collections with offsets and
their own transforms, ellipse collections in x units, multicoloured lines,
connection patches between coordinate systems, arrows between patches,
hatched collections and arcs on unequal axes."""

import numpy as np
from matplotlib import collections, patches, transforms
from matplotlib.colors import BoundaryNorm, ListedColormap, Normalize

from spec_support import case, rng

CYCLE = ["#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd",
         "#8c564b", "#e377c2", "#7f7f7f", "#bcbd22", "#17becf"]


@case("patches/gallery_collections")
def _(fig):
    nverts = 50
    npts = 100
    r = np.arange(nverts)
    theta = np.linspace(0, 2 * np.pi, nverts)
    xx = r * np.sin(theta)
    yy = r * np.cos(theta)
    spiral = np.column_stack([xx, yy])
    g = rng(19680801)
    xyo = np.column_stack([g.normal(npts), g.normal(npts)])
    (ax1, ax2), (ax3, ax4) = fig.subplots(2, 2)
    fig.subplots_adjust(top=0.92, left=0.07, right=0.97, hspace=0.3, wspace=0.3)

    col = collections.LineCollection([spiral], offsets=xyo, offset_transform=ax1.transData)
    col.set_transform(fig.dpi_scale_trans + transforms.Affine2D().scale(1.0 / 72.0))
    ax1.add_collection(col, autolim=True)
    col.set_color(CYCLE)
    ax1.autoscale_view()
    ax1.set_title("LineCollection using offsets")

    col = collections.PolyCollection([spiral], offsets=xyo, offset_transform=ax2.transData)
    col.set_transform(transforms.Affine2D().scale(fig.dpi / 72.0))
    ax2.add_collection(col, autolim=True)
    col.set_color(CYCLE)
    ax2.autoscale_view()
    ax2.set_title("PolyCollection using offsets")

    col = collections.RegularPolyCollection(
        7, sizes=np.abs(xx) * 10.0, offsets=xyo, offset_transform=ax3.transData)
    col.set_transform(transforms.Affine2D().scale(fig.dpi / 72.0))
    ax3.add_collection(col, autolim=True)
    col.set_color(CYCLE)
    ax3.autoscale_view()
    ax3.set_title("RegularPolyCollection using offsets")

    nverts = 60
    ncurves = 20
    yy = np.linspace(0, 2 * np.pi, nverts)
    ym = np.max(yy)
    xx = (0.2 + (ym - yy) / ym) ** 2 * np.cos(yy - 0.4) * 0.5
    segs = [np.column_stack([xx + 0.02 * g.normal(nverts), yy * 100]) for _ in range(ncurves)]
    col = collections.LineCollection(segs, offsets=(0.1, 0.0))
    ax4.add_collection(col, autolim=True)
    col.set_color(CYCLE)
    ax4.autoscale_view()
    ax4.set_title("Successive data offsets")
    ax4.set_xlabel("Zonal velocity component (m/s)")
    ax4.set_ylabel("Depth (m)")
    ax4.set_ylim(ax4.get_ylim()[::-1])


@case("patches/ellipsecollection_x_units_colorbar", figsize=(6, 4))
def _(fig):
    x = np.arange(10)
    y = np.arange(15)
    xx, yy = np.meshgrid(x, y)
    xy = np.column_stack((xx.ravel(), yy.ravel()))
    ax = fig.subplots()
    ec = collections.EllipseCollection(xx / 10.0, yy / 15.0, xx * 9, units="x", offsets=xy,
                                       offset_transform=ax.transData)
    ec.set_array((xx + yy).ravel())
    ax.add_collection(ec)
    ax.autoscale_view()
    ax.set_xlabel("X")
    ax.set_ylabel("y")
    cbar = fig.colorbar(ec)
    cbar.set_label("X+Y")


@case("patches/multicolored_line")
def _(fig):
    x = np.linspace(0, 3 * np.pi, 500)
    y = np.sin(x)
    dydx = np.cos(0.5 * (x[:-1] + x[1:]))
    points = np.array([x, y]).T.reshape(-1, 1, 2)
    segments = np.concatenate([points[:-1], points[1:]], axis=1)
    axs = fig.subplots(2, 1, sharex=True, sharey=True)
    norm = Normalize(dydx.min(), dydx.max())
    lc = collections.LineCollection(segments, cmap="viridis", norm=norm)
    lc.set_array(dydx)
    lc.set_linewidth(2)
    line = axs[0].add_collection(lc)
    fig.colorbar(line, ax=axs[0])
    cmap = ListedColormap(["r", "g", "b"])
    norm = BoundaryNorm([-1, -0.5, 0.5, 1], cmap.N)
    lc = collections.LineCollection(segments, cmap=cmap, norm=norm)
    lc.set_array(dydx)
    lc.set_linewidth(np.linspace(0.5, 4, len(segments)))
    line = axs[1].add_collection(lc)
    fig.colorbar(line, ax=axs[1])
    axs[0].set_xlim(x.min(), x.max())
    axs[0].set_ylim(-1.1, 1.1)


@case("patches/connectionpatch_coordinate_systems", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    xy_a, xy_b = (0.2, 0.2), (0.8, 0.8)
    con = patches.ConnectionPatch(xy_a, xy_b, "data", "data", arrowstyle="-|>", shrinkA=5,
                                  shrinkB=5, mutation_scale=20, fc="w")
    ax1.plot([xy_a[0], xy_b[0]], [xy_a[1], xy_b[1]], "o")
    ax1.add_artist(con)
    xy = (0.3, 0.2)
    con = patches.ConnectionPatch(xyA=xy, coordsA=ax2.transData, xyB=xy, coordsB=ax1.transData,
                                  arrowstyle="->", shrinkB=5)
    fig.add_artist(con)
    con = patches.ConnectionPatch(xyA=(0.6, 1.0), coordsA=ax2.transAxes, xyB=(0.0, 0.2),
                                  coordsB=ax2.get_yaxis_transform(), arrowstyle="-")
    ax2.add_artist(con)
    ax1.set_xlim(0, 1)
    ax1.set_ylim(0, 1)
    ax2.set_xlim(0, 0.5)
    ax2.set_ylim(0, 0.5)


@case("patches/fancyarrowpatch_between_patches", figsize=(6, 4))
def _(fig):
    ax = fig.subplots()
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 6)
    ax.set_aspect("equal")
    styles = [
        ("arc3,rad=0.3", "fancy", 1.0),
        ("angle3,angleA=90,angleB=0", "simple", 0.6),
        ("angle,angleA=-90,angleB=180,rad=8", "-|>", 1.0),
        ("arc,angleA=-90,angleB=0,armA=30,armB=30,rad=6", "->", 1.0),
        ("bar,fraction=-0.2", "<|-|>", 0.5),
    ]
    for i, (connection, arrow, aspect) in enumerate(styles):
        x = 1 + 2 * i
        a = patches.Circle((x - 0.4, 1.2), 0.4, fc="lightsteelblue", ec="k")
        b = patches.Circle((x + 0.4, 4.5), 0.5, fc="moccasin", ec="k")
        ax.add_patch(a)
        ax.add_patch(b)
        ax.add_patch(patches.FancyArrowPatch(
            (x - 0.4, 1.2), (x + 0.4, 4.5), patchA=a, patchB=b, connectionstyle=connection,
            arrowstyle=arrow, mutation_scale=15, mutation_aspect=aspect, shrinkA=2, shrinkB=4,
            fc="tab:red", ec="k", lw=1))


@case("patches/collection_hatch_alpha_and_edges", figsize=(6, 3))
def _(fig):
    ax = fig.subplots()
    squares = [[(x, 0.5), (x + 2, 0.5), (x + 2, 2.5), (x, 2.5)] for x in (0.5, 3.0, 5.5, 8.0)]
    ax.add_collection(collections.PolyCollection([squares[0]], facecolors="lightblue",
                                                 hatch="//", alpha=0.5))
    ax.add_collection(collections.PolyCollection([squares[1]], facecolors="w",
                                                 edgecolors="tab:red", hatch="xx",
                                                 linewidths=2))
    ax.add_collection(collections.PolyCollection([squares[2]], facecolors="none",
                                                 edgecolors="tab:green", hatch="..", alpha=0.6))
    ax.add_collection(collections.PatchCollection(
        [patches.Circle((9, 1.5), 0.9), patches.Rectangle((8.2, 2.7), 1.6, 0.6)],
        facecolors=["gold", "tab:purple"], edgecolors="k", hatch="\\\\", alpha=0.7))
    ax.set_xlim(0, 10.5)
    ax.set_ylim(0, 3.5)


@case("patches/collection_hatch_rc_color", figsize=(5, 3),
      rc={"hatch.color": "tab:purple", "hatch.linewidth": 1.5})
def _(fig):
    ax = fig.subplots()
    squares = [[(x, 0.5), (x + 2, 0.5), (x + 2, 2.5), (x, 2.5)] for x in (0.5, 3.0, 5.5)]
    ax.add_collection(collections.PolyCollection([squares[0]], facecolors="w", hatch="//"))
    ax.add_collection(collections.PolyCollection([squares[1]], facecolors="w",
                                                 edgecolors="k", hatch="++", alpha=0.4))
    ax.add_collection(collections.PolyCollection([squares[2]], facecolors="0.9",
                                                 edgecolors="tab:orange", hatch="o"))
    ax.set_xlim(0, 8)
    ax.set_ylim(0, 3)


@case("patches/arcs_wedges_unequal_aspect", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    for ax in (ax1, ax2):
        ax.set_xlim(-2, 2)
        ax.set_ylim(-1, 1)
    ax1.add_patch(patches.Arc((0, 0), 3, 1.5, angle=20, theta1=30, theta2=300, lw=3,
                              color="tab:blue"))
    ax1.add_patch(patches.Arc((0, 0), 1.5, 1.5, theta1=-45, theta2=45, lw=2, color="tab:red",
                              capstyle="round"))
    ax1.add_patch(patches.Ellipse((1.2, 0.5), 1.0, 0.4, angle=-30, fc="none", ec="tab:green",
                                  lw=2, ls="--"))
    ax2.add_patch(patches.Wedge((0, 0), 0.9, 300, 60, width=0.3, fc="tab:orange", ec="k"))
    ax2.add_patch(patches.Wedge((-1, -0.2), 0.6, 90, 450, fc="tab:cyan", alpha=0.6))
    ax2.add_patch(patches.Wedge((1.2, 0.3), 0.5, -30, 30, width=0.5, hatch="//", fc="w",
                                ec="tab:purple"))
