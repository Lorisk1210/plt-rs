"""
R, values matplotlib computes in the toolkits and composite artists: 3-D
axes and their projections, triangulations, legends, offset boxes,
tables, quiver keys and colour bars of each norm, from ints.
"""

import numpy as np

import pltrs.pyplot as plt
from pltrs import colors, tri
from pltrs.mpl_toolkits.mplot3d import proj3d
from pltrs.offsetbox import AnchoredText, DrawingArea, HPacker, TextArea


def three_d(rows):
    """3-D plotting methods and projections of ints."""
    ax = plt.figure(figsize=(3, 3), dpi=72).add_subplot(projection="3d")
    line, = ax.plot([1, 2], [3, 4], [5, 6])
    rows.row("R plot3D ints", "get_data_3d()", line.get_data_3d)
    points = ax.scatter([1, 2], [3, 4], [5, 6])
    rows.row("R scatter3D ints", "_offsets3d", lambda: points._offsets3d)
    rows.row("R scatter3D ints", "get_offsets()", points.get_offsets)
    rows.row("R bar3d ints", "get_paths()[0].vertices dtype",
             lambda: ax.bar3d([1], [1], [0], 1, 1, [3]).get_paths()[0].vertices)
    surface = ax.plot_surface(*np.meshgrid([0, 1], [0, 1]), np.array([[0, 1], [1, 2]]))
    rows.row("R plot_surface ints", "_vec dtype", lambda: surface._vec)
    rows.row("R Axes3D ints", "get_xlim3d()", ax.get_xlim3d)
    rows.row("R Axes3D ints", "get_w_lims()", ax.get_w_lims)
    rows.row("R Axes3D ints", "zz_viewLim.intervalx", lambda: ax.zz_viewLim.intervalx)
    ax.figure.canvas.draw()
    rows.row("R Axes3D ints", "get_proj()", ax.get_proj)
    rows.row("R Axes3D ints", "M", lambda: ax.M)
    rows.row("R Axes3D ints", "get_zticks()", ax.get_zticks)
    rows.row("R proj3d", "proj_transform(ints)", lambda: proj3d.proj_transform(1, 2, 3, ax.M))
    rows.row("R proj3d", "proj_transform(int arrays)",
             lambda: proj3d.proj_transform(np.array([1, 2]), np.array([3, 4]), np.array([5, 6]), ax.M))
    rows.row("R proj3d", "world_transformation(ints)",
             lambda: proj3d.world_transformation(0, 1, 0, 1, 0, 1))
    rows.row("R Axes3D", "format_coord(10, 10)", lambda: ax.format_coord(10, 10))
    rows.row("R Line3D", "get_xdata() after draw", line.get_xdata)
    rows.row("R Path3DCollection", "get_offsets() after draw", points.get_offsets)


def triangulations(rows):
    """Triangulations of int points and what is computed on them."""
    triangulation = tri.Triangulation([0, 1, 0, 1], [0, 0, 1, 1])
    for name in ("x", "y", "triangles", "edges", "neighbors", "mask", "is_delaunay"):
        rows.row("R Triangulation(ints)", name, lambda n=name: getattr(triangulation, n))
    rows.row("R Triangulation(ints)", "calculate_plane_coefficients(ints)",
             lambda: triangulation.calculate_plane_coefficients(np.array([0, 1, 2, 3])))
    rows.row("R Triangulation(ints)", "get_masked_triangles()", triangulation.get_masked_triangles)
    finder = triangulation.get_trifinder()
    rows.row("R TriFinder", "__call__(0.2, 0.1)", lambda: finder(0.2, 0.1))
    rows.row("R TriFinder", "__call__(arrays)", lambda: finder(np.array([0.2, 2]), np.array([0.1, 2])))
    linear = tri.LinearTriInterpolator(triangulation, np.array([0, 1, 2, 3]))
    rows.row("R LinearTriInterpolator(int z)", "__call__(0.5, 0.5)", lambda: linear(0.5, 0.5))
    rows.row("R LinearTriInterpolator(int z)", "gradient(0.5, 0.5)", lambda: linear.gradient(0.5, 0.5))
    rows.row("R CubicTriInterpolator(int z)", "__call__(0.5, 0.5)",
             lambda: tri.CubicTriInterpolator(triangulation, np.array([0, 1, 2, 3]))(0.5, 0.5))
    rows.row("R TriAnalyzer", "scale_factors", lambda: tri.TriAnalyzer(triangulation).scale_factors)
    rows.row("R TriAnalyzer", "circle_ratios()", lambda: tri.TriAnalyzer(triangulation).circle_ratios())
    refined = tri.UniformTriRefiner(triangulation).refine_triangulation()
    rows.row("R UniformTriRefiner", "x, triangles", lambda: (refined.x, refined.triangles))
    fig, ax = plt.subplots()
    rows.row("R tripcolor ints", "get_array()",
             lambda: ax.tripcolor([0, 1, 0, 1], [0, 0, 1, 1], [0, 1, 2, 3]).get_array())
    rows.row("R triplot ints", "lines get_xdata()",
             lambda: ax.triplot([0, 1, 0, 1], [0, 0, 1, 1])[0].get_xdata())
    rows.row("R tricontourf ints", "zmin, zmax",
             lambda: (lambda c: (c.zmin, c.zmax))(ax.tricontourf([0, 1, 0, 1], [0, 0, 1, 1], [0, 1, 2, 3])))


def boxes(rows):
    """Legends, offset boxes, tables and quiver keys placed with ints."""
    fig, ax = plt.subplots(figsize=(4, 3), dpi=72)
    ax.plot([1, 2], [3, 4], label="a")
    legend = ax.legend(loc=(0, 0), borderaxespad=1, handlelength=2, markerscale=2)
    fig.canvas.draw()
    rows.row("R Legend", "get_bbox_to_anchor()", legend.get_bbox_to_anchor)
    rows.row("R Legend", "_loc", lambda: legend._loc)
    rows.row("R Legend", "markerscale, handlelength", lambda: (legend.markerscale, legend.handlelength))
    rows.row("R Legend", "get_window_extent()", legend.get_window_extent)
    rows.row("R Legend", "legend_handles[0].get_xdata()", lambda: legend.legend_handles[0].get_xdata())
    rows.row("R Legend", "get_lines()[0].get_xdata()", lambda: legend.get_lines()[0].get_xdata())
    rows.row("R Legend", "_legend_box.get_offset()", lambda: legend._legend_box.get_offset())
    anchored = AnchoredText("t", loc=2, pad=1, borderpad=1)
    ax.add_artist(anchored)
    area = DrawingArea(20, 10, 1, 2)
    packed = HPacker(children=[TextArea("x"), area], pad=1, sep=2)
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    rows.row("R AnchoredText", "get_window_extent()", lambda: anchored.get_window_extent(renderer))
    rows.row("R AnchoredText", "pad, borderpad", lambda: (anchored.pad, anchored.borderpad))
    rows.row("R DrawingArea(ints)", "width, height, xdescent", lambda: (area.width, area.height, area.xdescent))
    rows.row("R DrawingArea(ints)", "get_bbox(renderer)", lambda: area.get_bbox(renderer))
    rows.row("R HPacker", "get_bbox(renderer)", lambda: packed.get_bbox(renderer))
    rows.row("R HPacker", "pad, sep", lambda: (packed.pad, packed.sep))
    table = ax.table(cellText=[["1", "2"]], colWidths=[1, 2], loc="bottom")
    rows.row("R Table colWidths=ints", "cell get_width()", lambda: table[0, 1].get_width())
    rows.row("R Table", "cell get_xy()", lambda: table[0, 0].get_xy())
    fig.canvas.draw()
    rows.row("R Table drawn", "cell get_xy()", lambda: table[0, 0].get_xy())
    q = ax.quiver([1], [1], [1], [1])
    key = ax.quiverkey(q, 0.5, 0.5, 2, "k")
    fig.canvas.draw()
    rows.row("R QuiverKey ints", "U, X, Y", lambda: (key.U, key.X, key.Y))
    rows.row("R Axes", "get_tightbbox()", lambda: ax.get_tightbbox(renderer))
    rows.row("R Figure", "get_tightbbox()", lambda: fig.get_tightbbox(renderer))


def colorbars(rows):
    """Colour bars of each norm over int data."""
    data = np.arange(1, 10).reshape(3, 3)
    for label, norm in [("Normalize", colors.Normalize()), ("LogNorm", colors.LogNorm()),
                        ("SymLogNorm", colors.SymLogNorm(1)), ("TwoSlopeNorm", colors.TwoSlopeNorm(5)),
                        ("BoundaryNorm", colors.BoundaryNorm([1, 3, 6, 9], 256)),
                        ("NoNorm", colors.NoNorm()), ("PowerNorm", colors.PowerNorm(2))]:
        fig, ax = plt.subplots(figsize=(3, 3), dpi=72)
        cb = fig.colorbar(ax.imshow(data, norm=norm))
        fig.canvas.draw()
        for name in ("_boundaries", "_values", "vmin", "vmax"):
            rows.row(f"R Colorbar {label} of ints", name, lambda c=cb, n=name: getattr(c, n))
        rows.row(f"R Colorbar {label} of ints", "get_ticks()", cb.get_ticks)
        rows.row(f"R Colorbar {label} of ints", "ax.get_ylim()", cb.ax.get_ylim)
        rows.row(f"R Colorbar {label} of ints", "norm.vmin, norm.vmax", lambda c=cb: (c.norm.vmin, c.norm.vmax))
    fig, ax = plt.subplots()
    cb = fig.colorbar(ax.contourf(data, levels=[1, 4, 9]), extend="both")
    fig.canvas.draw()
    rows.row("R Colorbar contourf int levels", "_boundaries", lambda: cb._boundaries)
    rows.row("R Colorbar contourf int levels", "_values", lambda: cb._values)
    rows.row("R Colorbar contourf int levels", "get_ticks()", cb.get_ticks)
    cb2 = fig.colorbar(ax.imshow(data), ticks=[2, 5, 8])
    fig.canvas.draw()
    rows.row("R Colorbar ticks=ints", "get_ticks()", cb2.get_ticks)


SCENES = (three_d, triangulations, boxes, colorbars)
