"""
The subjects of the sweep: one object of every kind of artist, and of the
helper objects artists hand out, each in a small figure of its own. All
data is computed, so that both libraries build the same scene.
"""

import numpy as np

import pltrs.pyplot as plt
from pltrs import collections, image, lines, offsetbox, patches, text, transforms, tri
from pltrs.path import Path

#: Every subject: its name and the function building it, given a figure
#: and its Axes (None for the subjects that make their own).
SUBJECTS = {}

#: The projection of the Axes a subject is built in (absent: rectilinear;
#: None: no Axes).
_PROJECTIONS = {}

X = np.linspace(0.0, 3.0, 7)
Y = np.sin(X) + 1.5
Z = np.add.outer(np.linspace(-1.0, 1.0, 5), np.linspace(-1.0, 2.0, 6))
VALUES = np.array([1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0])

#: The names of the arrays above, which the runner puts back after each
#: row: an artist that keeps the array it is given hands it out to be
#: changed.
DATA = ("X", "Y", "Z", "VALUES")


def subject(function=None, *, axes="rectilinear"):
    """Register *function* as the builder of the subject of its name."""
    def register(function):
        SUBJECTS[function.__name__] = function
        _PROJECTIONS[function.__name__] = axes
        return function
    return register if function is None else register(function)


def figure_and_axes(name="line"):
    """A new small figure and the Axes the subject *name* is built in."""
    fig = plt.figure(figsize=(3, 3), dpi=40)
    projection = _PROJECTIONS.get(name, "rectilinear")
    if projection is None:
        return fig, None
    ax = fig.add_subplot(projection=None if projection == "rectilinear" else projection)
    return fig, ax


def build(name):
    """A new figure with the subject *name*: the figure, the subject."""
    fig, ax = figure_and_axes(name)
    return fig, SUBJECTS[name](fig, ax)


def frame(ax):
    """Fix the view of *ax*, so that nothing a row changes rescales it."""
    ax.set_xlim(-1, 4)
    ax.set_ylim(-1, 4)


@subject(axes=None)
def figure(fig, ax):
    ax = fig.add_subplot()
    ax.plot(X, Y, label="sin")
    fig.suptitle("super")
    fig.text(0.1, 0.9, "note")
    fig.legend()
    return fig


@subject(axes=None)
def subfigure(fig, ax):
    sub = fig.subfigures(1, 2)[0]
    sub.subplots().plot(X, Y)
    return sub


@subject
def axes(fig, ax):
    ax.plot(X, Y, label="sin")
    ax.bar([0, 1], [1, 2])
    ax.scatter(X, Y)
    ax.imshow(Z, extent=(0, 1, 0, 1))
    ax.text(1, 1, "text")
    ax.legend()
    ax.set_title("title")
    return ax


@subject(axes="polar")
def axes_polar(fig, ax):
    ax.plot(X, VALUES)
    return ax


@subject(axes="3d")
def axes3d(fig, ax):
    ax.plot(X, Y, VALUES)
    return ax


@subject
def xaxis(fig, ax):
    ax.plot(X, Y)
    return ax.xaxis


@subject
def major_tick(fig, ax):
    ax.plot(X, Y)
    return ax.xaxis.get_major_ticks()[1]


@subject
def tick_label(fig, ax):
    ax.plot(X, Y)
    fig.canvas.draw()
    return ax.get_xticklabels()[1]


@subject
def spine(fig, ax):
    ax.plot(X, Y)
    return ax.spines["left"]


@subject
def line(fig, ax):
    frame(ax)
    return ax.plot(X.copy(), Y.copy(), "o-", label="sin")[0]


@subject
def line_loose(fig, ax):
    frame(ax)
    line = lines.Line2D(np.array([0.0, 1.0, 2.0]), np.array([2.0, 0.0, 1.0]), color="r")
    ax.add_line(line)
    return line


@subject
def axline(fig, ax):
    frame(ax)
    return ax.axline((0, 0), (1, 2))


@subject
def step_line(fig, ax):
    frame(ax)
    return ax.step(X, Y, where="mid")[0]


@subject
def text_plain(fig, ax):
    return ax.text(0.5, 0.5, "hello", ha="center")


@subject
def text_boxed(fig, ax):
    return ax.text(0.2, 0.3, "boxed", bbox=dict(boxstyle="round", fc="y"))


@subject
def title(fig, ax):
    return ax.set_title("title")


@subject
def annotation(fig, ax):
    return ax.annotate("note", xy=(0.2, 0.2), xytext=(0.6, 0.7), arrowprops=dict(arrowstyle="->"))


def _patch(ax, patch):
    """*patch*, added to *ax* with a fixed view."""
    frame(ax)
    ax.add_patch(patch)
    return patch


@subject
def rectangle(fig, ax):
    return _patch(ax, patches.Rectangle((0.1, 0.2), 1.5, 1.3, fc="r", angle=10))


@subject
def bar_patch(fig, ax):
    frame(ax)
    return ax.bar([0, 1, 2], [3, 1, 2])[1]


@subject
def circle(fig, ax):
    return _patch(ax, patches.Circle((1.5, 1.5), 1.0))


@subject
def ellipse(fig, ax):
    return _patch(ax, patches.Ellipse((1.5, 1.5), 2.0, 1.0, angle=30))


@subject
def polygon(fig, ax):
    return _patch(ax, patches.Polygon(np.array([[0.0, 0.0], [3.0, 0.0], [1.5, 3.0]]), closed=True))


@subject
def wedge(fig, ax):
    return _patch(ax, patches.Wedge((1.5, 1.5), 1.2, 30, 120, width=0.5))


@subject
def arc(fig, ax):
    return _patch(ax, patches.Arc((1.5, 1.5), 2.0, 1.5, theta1=10, theta2=200))


@subject
def regular_polygon(fig, ax):
    return _patch(ax, patches.RegularPolygon((1.5, 1.5), 6, radius=1.0))


@subject
def annulus(fig, ax):
    return _patch(ax, patches.Annulus((1.5, 1.5), 1.2, 0.4))


@subject
def fancy_box(fig, ax):
    return _patch(ax, patches.FancyBboxPatch((0.5, 0.5), 1.5, 1.2, boxstyle="round,pad=0.2"))


@subject
def fancy_arrow_patch(fig, ax):
    return _patch(ax, patches.FancyArrowPatch((0.2, 0.2), (2.8, 2.5), arrowstyle="-|>",
                                              mutation_scale=20))


@subject
def fancy_arrow(fig, ax):
    return _patch(ax, patches.FancyArrow(0.2, 0.2, 2.0, 1.5, width=0.3))


@subject
def arrow(fig, ax):
    return _patch(ax, patches.Arrow(0.2, 0.2, 2.0, 1.5))


@subject
def path_patch(fig, ax):
    return _patch(ax, patches.PathPatch(Path([(0, 0), (3, 0), (3, 3), (0, 0)], [1, 2, 2, 79])))


@subject
def connection_patch(fig, ax):
    return _patch(ax, patches.ConnectionPatch((0.2, 0.2), (2.9, 2.9), "data"))


@subject
def step_patch(fig, ax):
    frame(ax)
    return ax.stairs([1, 3, 2], [0, 1, 2, 3], fill=True)


@subject
def pie_wedge(fig, ax):
    return ax.pie([1, 2, 3])[0][1]


@subject
def scatter(fig, ax):
    frame(ax)
    return ax.scatter(X, Y, s=VALUES * 30, c=VALUES, linewidths=[1, 2, 3], edgecolors="k")


@subject
def scatter_plain(fig, ax):
    frame(ax)
    return ax.scatter(X, Y, s=VALUES * 30, color=list("rgbcmyk"), linewidths=2, edgecolors="k")


@subject
def line_collection(fig, ax):
    frame(ax)
    lc = collections.LineCollection([[(0, 0), (1, 1)], [(0, 1), (1, 0)]], linewidths=[1, 3],
                                    colors=["r", "b"])
    ax.add_collection(lc)
    return lc


@subject
def poly_collection(fig, ax):
    frame(ax)
    pc = collections.PolyCollection([[(0, 0), (2, 0), (1, 2)], [(1, 1), (3, 1), (2, 3)]],
                                    facecolors=["r", "g"], edgecolors="k")
    ax.add_collection(pc)
    return pc


@subject
def fill_between(fig, ax):
    frame(ax)
    return ax.fill_between(X, Y, 0.5)


@subject
def violin_body(fig, ax):
    frame(ax)
    return ax.violinplot([VALUES])["bodies"][0]


@subject
def quadmesh(fig, ax):
    return ax.pcolormesh(Z)


@subject
def pcolor(fig, ax):
    return ax.pcolor(Z)


@subject
def hexbin(fig, ax):
    return ax.hexbin(X, Y, gridsize=4)


@subject
def event_collection(fig, ax):
    frame(ax)
    return ax.eventplot([1, 2, 3.5])[0]


@subject
def stem_lines(fig, ax):
    frame(ax)
    return ax.stem([0, 1, 2], [1, 2, 1]).stemlines


@subject
def errorbar_lines(fig, ax):
    frame(ax)
    return ax.errorbar([0, 1, 2], [1, 2, 1], yerr=0.4).lines[2][0]


@subject
def quiver(fig, ax):
    return ax.quiver([0, 1], [0, 1], [1, -1], [1, 2])


@subject
def barbs(fig, ax):
    return ax.barbs([0, 1], [0, 1], [10, -20], [15, 5])


@subject
def ellipse_collection(fig, ax):
    frame(ax)
    ec = collections.EllipseCollection([1, 2], [0.5, 1], [0, 30], units="xy",
                                       offsets=[[0.5, 0.5], [2, 2]],
                                       offset_transform=ax.transData)
    ax.add_collection(ec)
    return ec


@subject
def circle_collection(fig, ax):
    frame(ax)
    cc = collections.CircleCollection([200, 400], offsets=[[0.5, 0.5], [2, 2]],
                                      offset_transform=ax.transData)
    ax.add_collection(cc)
    return cc


@subject
def regular_poly_collection(fig, ax):
    frame(ax)
    rc = collections.RegularPolyCollection(5, sizes=[300, 500], offsets=[[0.5, 0.5], [2, 2]],
                                           offset_transform=ax.transData)
    ax.add_collection(rc)
    return rc


@subject
def patch_collection(fig, ax):
    frame(ax)
    pc = collections.PatchCollection([patches.Circle((1, 1), 1), patches.Rectangle((1, 1), 1, 2)],
                                     match_original=True)
    ax.add_collection(pc)
    return pc


@subject
def path_collection(fig, ax):
    frame(ax)
    pc = collections.PathCollection([Path([(0, 0), (3, 0), (1.5, 3), (0, 0)])],
                                    facecolors="c", transform=ax.transData)
    ax.add_collection(pc)
    return pc


@subject
def tri_mesh(fig, ax):
    return ax.tripcolor([0.0, 1.0, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0], [1.0, 2.0, 3.0, 4.0],
                        shading="gouraud")


@subject
def tripcolor(fig, ax):
    return ax.tripcolor([0.0, 1.0, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0], [1.0, 2.0, 3.0, 4.0])


@subject
def image_scalar(fig, ax):
    return ax.imshow(Z)


@subject
def image_rgb(fig, ax):
    return ax.imshow(np.dstack([(Z + 1) / 3] * 3).clip(0, 1))


@subject(axes=None)
def figimage(fig, ax):
    return fig.figimage(Z * 40)


@subject
def pcolor_image(fig, ax):
    return ax.pcolorfast(np.arange(7.0), np.arange(6.0), Z)


@subject
def non_uniform_image(fig, ax):
    im = image.NonUniformImage(ax, extent=(0, 4, 0, 4))
    im.set_data(np.array([0.0, 1.0, 3.0]), np.array([0.0, 2.0, 3.0]), Z[:3, :3])
    ax.add_image(im)
    frame(ax)
    return im


@subject
def bbox_image(fig, ax):
    im = image.BboxImage(transforms.Bbox([[10, 10], [80, 80]]), data=Z)
    ax.add_artist(im)
    return im


@subject
def contour(fig, ax):
    return ax.contour(Z, levels=3)


@subject
def contourf(fig, ax):
    return ax.contourf(Z, levels=3)


@subject
def contour_labeled(fig, ax):
    cs = ax.contour(Z, levels=3)
    cs.clabel()
    return cs


@subject
def tricontour(fig, ax):
    return ax.tricontour([0, 1, 0, 1, 0.5], [0, 0, 1, 1, 0.5], [1, 2, 3, 4, 0])


@subject
def legend(fig, ax):
    ax.plot(X, Y, label="one")
    ax.bar([1], [2], label="two")
    return ax.legend()


@subject
def colorbar_axes(fig, ax):
    return fig.colorbar(ax.imshow(Z)).ax


@subject
def table(fig, ax):
    return ax.table([["a", "b"], ["c", "d"]], loc="center")


@subject
def table_cell(fig, ax):
    return ax.table([["a", "b"], ["c", "d"]], loc="center")[0, 1]


@subject
def anchored_text(fig, ax):
    box = offsetbox.AnchoredText("anchored", loc="upper left")
    ax.add_artist(box)
    return box


@subject
def annotation_bbox(fig, ax):
    box = offsetbox.AnnotationBbox(offsetbox.TextArea("box"), (0.5, 0.5))
    ax.add_artist(box)
    return box


@subject
def offset_image(fig, ax):
    im = offsetbox.OffsetImage(Z, zoom=4)
    ax.add_artist(offsetbox.AnnotationBbox(im, (0.5, 0.5)))
    return im


@subject
def drawing_area(fig, ax):
    area = offsetbox.DrawingArea(40, 40)
    area.add_artist(patches.Circle((20, 20), 15))
    ax.add_artist(offsetbox.AnchoredOffsetbox("center", child=area))
    return area


@subject
def inset_axes(fig, ax):
    ax.plot(X, Y)
    inset = ax.inset_axes([0.5, 0.5, 0.4, 0.4])
    inset.plot(X, Y)
    return inset


@subject
def twin_axes(fig, ax):
    ax.plot(X, Y)
    twin = ax.twinx()
    twin.plot(X, VALUES)
    return twin


@subject(axes="3d")
def line3d(fig, ax):
    return ax.plot(X, Y, VALUES)[0]


@subject(axes="3d")
def scatter3d(fig, ax):
    return ax.scatter(X, Y, VALUES, c=VALUES)


@subject(axes="3d")
def surface3d(fig, ax):
    xs, ys = np.meshgrid(np.linspace(-1.0, 2.0, 6), np.linspace(-1.0, 1.0, 5))
    return ax.plot_surface(xs, ys, Z)


@subject(axes="3d")
def wireframe3d(fig, ax):
    xs, ys = np.meshgrid(np.linspace(-1.0, 2.0, 6), np.linspace(-1.0, 1.0, 5))
    return ax.plot_wireframe(xs, ys, Z)


@subject(axes="3d")
def text3d(fig, ax):
    return ax.text(0.5, 0.5, 0.5, "deep")


@subject
def triangulation_plot(fig, ax):
    triangulation = tri.Triangulation(np.array([0.0, 1.0, 0.0, 1.0]), np.array([0.0, 0.0, 1.0, 1.0]))
    ax.triplot(triangulation)
    return triangulation


@subject
def offset_from(fig, ax):
    anchor = ax.text(0.5, 0.5, "anchor")
    return text.OffsetFrom(anchor, (0.5, 0))
