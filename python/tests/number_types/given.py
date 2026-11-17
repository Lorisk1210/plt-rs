"""
G: what getters return of the numbers a program gave, by hand: ints,
numpy ints and floats of other widths given to the artists, read back.
"""

import numpy as np

import pltrs.pyplot as plt
from pltrs import collections, colors, patches


def lines(rows):
    """Lines made of ints, int32 and float32 data."""
    fig, ax = plt.subplots(figsize=(4, 3), dpi=72)
    ln, = ax.plot([1, 2, 3], [4, 5, 6], lw=2, ms=7, alpha=1, zorder=3, markevery=2,
                  marker="o", mew=1)
    for name in ("get_xdata", "get_ydata"):
        rows.row("G Line2D ints", f"{name}()", lambda n=name: getattr(ln, n)())
        rows.row("G Line2D ints", f"{name}(orig=False)",
                 lambda n=name: getattr(ln, n)(orig=False))
    for name in ("get_linewidth", "get_markersize", "get_alpha", "get_zorder",
                 "get_markevery", "get_markeredgewidth"):
        rows.row("G Line2D ints", f"{name}()", lambda n=name: getattr(ln, n)())
    rows.row("G Line2D ints", "get_xydata()", ln.get_xydata)
    rows.row("G Line2D ints", "get_data()", ln.get_data)
    ln2, = ax.plot(np.arange(3), np.arange(3, dtype=np.int32))
    rows.row("G Line2D int32", "get_ydata()", ln2.get_ydata)
    rows.row("G Line2D int32", "get_ydata(orig=False)", lambda: ln2.get_ydata(orig=False))
    ln3, = ax.plot(np.float32([1, 2]), np.float32([3, 4]))
    rows.row("G Line2D float32", "get_xdata()", ln3.get_xdata)
    rows.row("G Line2D float32", "get_xydata()", ln3.get_xydata)
    ln.set_dashes([4, 2])
    rows.row("G Line2D", "get_dashes via _dash_pattern", lambda: ln._dash_pattern)
    rows.row("G Line2D", "set_data(int scalar list) get_xdata",
             lambda: (ln.set_data([5], [6]), ln.get_xdata())[1])
    rows.row("G Line2D", "axhline get_ydata", lambda: ax.axhline(2).get_ydata())
    rows.row("G Line2D", "axvline get_xdata", lambda: ax.axvline(3).get_xdata())


def shapes(rows):
    """Patches whose geometry is given in ints."""
    fig, ax = plt.subplots()
    r = patches.Rectangle((1, 2), 3, 4, angle=30, lw=2)
    ax.add_patch(r)
    for name in ("get_x", "get_y", "get_width", "get_height", "get_xy", "get_angle",
                 "get_linewidth", "get_center"):
        rows.row("G Rectangle ints", f"{name}()", lambda n=name: getattr(r, n)())
    rows.row("G Rectangle ints", "get_bbox()", r.get_bbox)
    rows.row("G Rectangle ints", "get_corners()", r.get_corners)
    c = patches.Circle((1, 1), 2)
    for name in ("get_radius", "get_center", "get_width"):
        rows.row("G Circle ints", f"{name}()", lambda n=name: getattr(c, n)())
    e = patches.Ellipse((0, 0), 4, 2, angle=45)
    for name in ("get_width", "get_height", "get_angle", "get_center"):
        rows.row("G Ellipse ints", f"{name}()", lambda n=name: getattr(e, n)())
    w = patches.Wedge((0, 0), 1, 30, 60, width=1)
    for name in ("theta1", "theta2", "r", "width", "center"):
        rows.row("G Wedge ints", name, lambda n=name: getattr(w, n))
    p = patches.Polygon([[0, 0], [1, 0], [1, 1]])
    rows.row("G Polygon ints", "get_xy()", p.get_xy)
    arc = patches.Arc((0, 0), 2, 2, theta1=0, theta2=90)
    rows.row("G Arc ints", "theta1, theta2", lambda: (arc.theta1, arc.theta2))
    fa = patches.FancyArrowPatch((0, 0), (1, 1), shrinkA=3, shrinkB=6, mutation_scale=10)
    rows.row("G FancyArrowPatch ints", "shrinkA, shrinkB", lambda: (fa.shrinkA, fa.shrinkB))
    rows.row("G FancyArrowPatch ints", "get_mutation_scale()", fa.get_mutation_scale)
    fb = patches.FancyBboxPatch((1, 2), 3, 4, boxstyle="round,pad=1")
    for name in ("get_x", "get_y", "get_width", "get_height"):
        rows.row("G FancyBboxPatch ints", f"{name}()", lambda n=name: getattr(fb, n)())
    rows.row("G FancyBboxPatch", "get_boxstyle().pad", lambda: fb.get_boxstyle().pad)
    rows.row("G ArrowStyle", "ArrowStyle('->', head_length=1).head_length",
             lambda: patches.ArrowStyle("->", head_length=1).head_length)
    rows.row("G RegularPolygon ints", "numvertices, radius",
             lambda: (lambda q: (q.numvertices, q.radius))(
                 patches.RegularPolygon((0, 0), 5, radius=2)))
    r2 = patches.Rectangle((0, 0), 1, 1)
    r2.update_from(patches.Circle((0, 0), 1, fc="y", ec="k", lw=2))
    rows.row("G Rectangle after update_from(Circle(fc='y'))",
             "get_facecolor(), get_edgecolor()",
             lambda: (r2.get_facecolor(), r2.get_edgecolor()))


def texts(rows):
    """Texts and annotations placed at ints."""
    fig, ax = plt.subplots()
    t = ax.text(1, 2, "t", rotation=30, fontsize=12, linespacing=2)
    for name in ("get_position", "get_rotation", "get_fontsize", "get_linespacing",
                 "get_unitless_position"):
        rows.row("G Text ints", f"{name}()", lambda n=name: getattr(t, n)())
    an = ax.annotate("a", (1, 2), (3, 4), arrowprops=dict(arrowstyle="->"))
    rows.row("G Annotation ints", "xy", lambda: an.xy)
    rows.row("G Annotation ints", "xyann", lambda: an.xyann)
    rows.row("G Annotation ints", "get_position()", an.get_position)
    rows.row("G Annotation", "arrow_patch.shrinkA", lambda: an.arrow_patch.shrinkA)


def axes(rows):
    """Limits, ticks and margins set with ints."""
    fig, ax = plt.subplots(figsize=(4, 3), dpi=72)
    ax.plot([1, 2, 3], [4, 5, 6])
    rows.row("G Axes", "set_xlim(1, 5) returns", lambda: ax.set_xlim(1, 5))
    rows.row("G Axes", "get_xlim() after ints", ax.get_xlim)
    rows.row("G Axes", "get_xbound()", ax.get_xbound)
    rows.row("G Axes", "set_ylim(np.int64) returns", lambda: ax.set_ylim(np.int64(0), np.int64(9)))
    rows.row("G Axes", "set_xticks([1, 2, 3]) get_xticks",
             lambda: (ax.set_xticks([1, 2, 3]), ax.get_xticks())[1])
    rows.row("G Axes", "get_xticks(minor=True)", lambda: ax.get_xticks(minor=True))
    rows.row("G Axes", "margins()", ax.margins)
    rows.row("G Axes", "set_aspect(2) get_aspect", lambda: (ax.set_aspect(2), ax.get_aspect())[1])
    rows.row("G Axes", "set_box_aspect(1) get_box_aspect",
             lambda: (ax.set_box_aspect(1), ax.get_box_aspect())[1])
    rows.row("G Axes", "get_position().bounds", lambda: ax.get_position().bounds)
    rows.row("G Axes", "get_zorder()", ax.get_zorder)
    rows.row("G Axes", "get_xmargin()", ax.get_xmargin)
    rows.row("G Axes", "get_rasterization_zorder() after 2",
             lambda: (ax.set_rasterization_zorder(2), ax.get_rasterization_zorder())[1])
    rows.row("G Axes", "dataLim.bounds", lambda: ax.dataLim.bounds)
    rows.row("G Axes", "viewLim.intervalx", lambda: ax.viewLim.intervalx)
    ax.set_aspect("auto")
    ax.set_box_aspect(None)
    fig.canvas.draw()
    rows.row("G Axes after draw", "get_xticks()", ax.get_xticks)
    rows.row("G Axes after draw", "get_yticks()", ax.get_yticks)
    rows.row("G Axes after draw", "xaxis.get_majorticklocs()", ax.xaxis.get_majorticklocs)
    rows.row("G Axes after draw", "xaxis.get_view_interval()", ax.xaxis.get_view_interval)
    rows.row("G Axes after draw", "xaxis.get_data_interval()", ax.xaxis.get_data_interval)
    ax.tick_params(axis="x", width=2, length=8, pad=6, labelsize=7, labelrotation=45)
    fig.canvas.draw()
    tk = ax.xaxis.get_major_ticks()[0]
    rows.row("G Tick after tick_params ints", "get_pad()", tk.get_pad)
    rows.row("G Tick after tick_params ints", "tick1line.get_markersize()",
             tk.tick1line.get_markersize)
    rows.row("G Tick after tick_params ints", "tick1line.get_markeredgewidth()",
             tk.tick1line.get_markeredgewidth)
    rows.row("G Tick after tick_params ints", "label1.get_fontsize()", tk.label1.get_fontsize)
    rows.row("G Tick after tick_params ints", "label1.get_rotation()", tk.label1.get_rotation)
    rows.row("G Tick", "get_loc()", tk.get_loc)
    rows.row("G Axis", "_major_tick_kw['width']", lambda: ax.xaxis._major_tick_kw.get("width"))
    rows.row("G Axis", "get_tick_space()", ax.xaxis.get_tick_space)
    ax.spines["bottom"].set_bounds(1, 4)
    rows.row("G Spine", "get_bounds() after ints", ax.spines["bottom"].get_bounds)
    ax.spines["left"].set_position(("outward", 10))
    rows.row("G Spine", "get_position() outward 10", ax.spines["left"].get_position)
    rows.row("G Spine", "get_linewidth()", ax.spines["left"].get_linewidth)


def collections_(rows):
    """Collections, bars and labels made of ints."""
    fig2, ax2 = plt.subplots(figsize=(3, 3), dpi=72)
    sc = ax2.scatter([1, 2, 3], [4, 5, 6], s=[10, 20, 30], c=[1, 2, 3], linewidths=2, alpha=1)
    for name in ("get_offsets", "get_sizes", "get_linewidths", "get_array", "get_alpha",
                 "get_clim"):
        rows.row("G PathCollection ints", f"{name}()", lambda n=name: getattr(sc, n)())
    sc2 = ax2.scatter(np.float32([1, 2]), np.float32([3, 4]))
    rows.row("G PathCollection float32", "get_offsets()", sc2.get_offsets)
    lc = collections.LineCollection([[(0, 0), (1, 1)]], linewidths=[2])
    rows.row("G LineCollection ints", "get_segments()", lc.get_segments)
    rows.row("G LineCollection ints", "get_linewidths()", lc.get_linewidths)
    pc = collections.PolyCollection([[(0, 0), (1, 0), (1, 1)]], array=np.array([3]))
    rows.row("G PolyCollection int array", "get_array()", pc.get_array)
    rows.row("G PolyCollection", "get_paths()[0].vertices", lambda: pc.get_paths()[0].vertices)
    ec = collections.EventCollection([1, 2, 3], lineoffset=2, linelength=1)
    rows.row("G EventCollection ints", "get_positions()", ec.get_positions)
    rows.row("G EventCollection ints", "get_lineoffset(), get_linelength()",
             lambda: (ec.get_lineoffset(), ec.get_linelength()))
    fbc = ax2.fill_between([0, 1, 2], [0, 1, 0])
    rows.row("G FillBetween", "sticky_edges.y", lambda: fbc.sticky_edges.y)
    bars = ax2.bar([1, 2], [3, 4], width=1)
    rows.row("G Bar Rectangle", "sticky_edges.y", lambda: bars[0].sticky_edges.y)
    rows.row("G Bar Rectangle", "get_height(), get_width(), get_x()",
             lambda: (bars[0].get_height(), bars[0].get_width(), bars[0].get_x()))
    rows.row("G BarContainer", "datavalues", lambda: bars.datavalues)
    hb = ax2.bar([1, 2], [3.5, 4.5])
    rows.row("G BarContainer floats", "datavalues", lambda: hb.datavalues)
    barh = ax2.barh([1, 2], [3, 4])
    rows.row("G BarContainer barh", "datavalues", lambda: barh.datavalues)
    labels = ax2.bar_label(bars)
    rows.row("G bar_label", "get_text()s", lambda: [x.get_text() for x in labels])
    rows.row("G Legend", "_loc for 'upper left'",
             lambda: ax2.legend(["x"], loc="upper left")._loc)
    rows.row("G Legend", "_ncols, borderpad",
             lambda: (lambda lg: (lg._ncols, lg.borderpad))(
                 ax2.legend(["x"], ncols=2, borderpad=1)))


def images(rows):
    """Images of int data, and their norms."""
    fig2, ax2 = plt.subplots(figsize=(3, 3), dpi=72)
    im = ax2.imshow(np.arange(6).reshape(2, 3), extent=(0, 3, 0, 2))
    rows.row("G AxesImage int data", "get_extent()", im.get_extent)
    rows.row("G AxesImage int data", "get_array()", im.get_array)
    rows.row("G AxesImage int data", "get_clim()", im.get_clim)
    rows.row("G AxesImage int data", "norm.vmin, norm.vmax", lambda: (im.norm.vmin, im.norm.vmax))
    im.set_clim(1, 4)
    rows.row("G AxesImage", "get_clim() after ints", im.get_clim)
    im2 = ax2.imshow(np.zeros((2, 2)))
    rows.row("G AxesImage default", "get_extent()", im2.get_extent)
    pf = ax2.pcolorfast((0, 8), (3, 6), np.arange(6).reshape(2, 3))
    rows.row("G pcolorfast image", "get_extent()", pf.get_extent)
    qm = ax2.pcolormesh(np.arange(4).reshape(2, 2))
    rows.row("G QuadMesh", "get_array()", qm.get_array)
    rows.row("G QuadMesh", "get_coordinates()", qm.get_coordinates)


def norms(rows):
    """Norms and colormaps given ints."""
    made = [("Normalize(0, 10)", lambda: colors.Normalize(0, 10)),
            ("Normalize(np.int64)", lambda: colors.Normalize(np.int64(0), np.int64(10))),
            ("LogNorm(1, 100)", lambda: colors.LogNorm(1, 100)),
            ("TwoSlopeNorm(0, -1, 1)", lambda: colors.TwoSlopeNorm(0, -1, 1)),
            ("BoundaryNorm([0, 1, 5])", lambda: colors.BoundaryNorm([0, 1, 5], 10))]
    for label, make in made:
        rows.row(f"G {label}", "vmin, vmax", lambda m=make: (lambda n: (n.vmin, n.vmax))(m()))
        rows.row(f"G {label}", "call(5)", lambda m=make: m()(5))
        rows.row(f"G {label}", "call([1, 2])", lambda m=make: m()([1, 2]))
        rows.row(f"G {label}", "call(np.int32 array)",
                 lambda m=make: m()(np.array([1, 2], np.int32)))
    rows.row("G BoundaryNorm", "boundaries", lambda: colors.BoundaryNorm([0, 1, 5], 10).boundaries)
    rows.row("G Normalize", "inverse(0.5)", lambda: colors.Normalize(0, 10).inverse(0.5))
    rows.row("G Colormap", "cmap(0.5)", lambda: plt.get_cmap("viridis")(0.5))
    rows.row("G Colormap", "cmap(3)", lambda: plt.get_cmap("viridis")(3))
    rows.row("G Colormap", "cmap([0.5], bytes=True)",
             lambda: plt.get_cmap("viridis")([0.5], bytes=True))
    rows.row("G colors", "to_rgba('r')", lambda: colors.to_rgba("r"))
    rows.row("G colors", "to_rgba((1, 0, 0))", lambda: colors.to_rgba((1, 0, 0)))
    rows.row("G colors", "to_rgb((1, 0, 0, 1))", lambda: colors.to_rgb((1, 0, 0, 1)))


def colorbars(rows):
    """Colour bars of int boundaries."""
    fig3, ax3 = plt.subplots(figsize=(3, 3), dpi=72)
    im3 = ax3.imshow(np.arange(4).reshape(2, 2), norm=colors.BoundaryNorm([0, 1, 2, 4], 256))
    cb = fig3.colorbar(im3)
    fig3.canvas.draw()
    rows.row("G Colorbar BoundaryNorm ints", "get_ticks()", cb.get_ticks)
    rows.row("G Colorbar BoundaryNorm ints", "_boundaries", lambda: cb._boundaries)
    rows.row("G Colorbar BoundaryNorm ints", "_values", lambda: cb._values)
    rows.row("G Colorbar BoundaryNorm ints", "vmin, vmax", lambda: (cb.vmin, cb.vmax))
    cb2 = fig3.colorbar(ax3.imshow(np.arange(4).reshape(2, 2)), boundaries=[0, 1, 2, 3],
                        values=[0, 1, 2])
    fig3.canvas.draw()
    rows.row("G Colorbar boundaries=ints", "_boundaries", lambda: cb2._boundaries)
    rows.row("G Colorbar boundaries=ints", "get_ticks()", cb2.get_ticks)
    rows.row("G Colorbar", "ax.get_ylim()", cb.ax.get_ylim)


def projections(rows):
    """Polar and 3-D axes set with ints."""
    pax = plt.figure().add_subplot(projection="polar")
    rows.row("G PolarAxes", "set_rlim(1, 10) returns", lambda: pax.set_rlim(1, 10))
    rows.row("G PolarAxes", "get_rmax(), get_rmin()", lambda: (pax.get_rmax(), pax.get_rmin()))
    rows.row("G PolarAxes", "set_rorigin(-1) get_rorigin",
             lambda: (pax.set_rorigin(-1), pax.get_rorigin())[1])
    rows.row("G PolarAxes", "set_theta_offset(1) get_theta_offset",
             lambda: (pax.set_theta_offset(1), pax.get_theta_offset())[1])
    rows.row("G PolarAxes", "set_thetamin(30) get_thetamin",
             lambda: (pax.set_thetamin(30), pax.get_thetamin())[1])
    rows.row("G PolarAxes", "set_rlabel_position(45) get_rlabel_position",
             lambda: (pax.set_rlabel_position(45), pax.get_rlabel_position())[1])
    plog = plt.figure().add_subplot(projection="polar")
    plog.set_rscale("log")
    rows.row("G PolarAxes log", "set_rlim(1, 10) returns", lambda: plog.set_rlim(1, 10))
    rows.row("G PolarAxes log", "get_ylim()", plog.get_ylim)
    ax3d = plt.figure().add_subplot(projection="3d")
    ax3d.view_init(30, 45, 0)
    rows.row("G Axes3D view_init ints", "elev, azim, roll",
             lambda: (ax3d.elev, ax3d.azim, ax3d.roll))
    rows.row("G Axes3D", "set_zlim(1, 5) returns", lambda: ax3d.set_zlim(1, 5))
    rows.row("G Axes3D", "get_zlim()", ax3d.get_zlim)
    rows.row("G Axes3D", "get_xlim3d() default", ax3d.get_xlim3d)
    rows.row("G Axes3D", "get_box_aspect()", ax3d.get_box_aspect)


def figures(rows):
    """Figures, tables and their sizes given in ints."""
    fg = plt.figure(figsize=(4, 3), dpi=100)
    rows.row("G Figure ints", "get_size_inches()", fg.get_size_inches)
    rows.row("G Figure ints", "get_dpi(), dpi", lambda: (fg.get_dpi(), fg.dpi))
    rows.row("G Figure ints", "get_figwidth()", fg.get_figwidth)
    rows.row("G Figure", "subplotpars.left", lambda: fg.subplotpars.left)
    rows.row("G Figure", "bbox.bounds", lambda: fg.bbox.bounds)
    rows.row("G Figure", "canvas.get_width_height()", fg.canvas.get_width_height)
    tb = plt.figure().add_subplot().table(cellText=[["a", "b"]], loc="center")
    rows.row("G Table", "cell get_width/get_height",
             lambda: (tb[0, 0].get_width(), tb[0, 0].get_height()))


SCENES = (lines, shapes, texts, axes, collections_, images, norms, colorbars, projections,
          figures)
