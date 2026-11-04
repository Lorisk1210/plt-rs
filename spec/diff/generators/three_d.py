"""The 3-D toolkit (``mpl_toolkits.mplot3d``): its plotting methods with
argument variants, the view and the box, zooming, panning and rotating
through mouse events and the toolbar, and the public names of
``art3d``, ``proj3d`` and ``axis3d``.

Every program starts on a fresh 3-D axes. The plotting programs read the
limits, ticks and artists after each variant, draw, and read them again;
the others read, after each change, what matplotlib's tests of
``test_axes3d.py`` and ``test_art3d.py`` read. Every figure is compared as
an image at the end.
"""

from __future__ import annotations

from typing import Iterator

from . import common

#: The statements making a fresh 3-D axes.
AXES = [
    "from mpl_toolkits.mplot3d import Axes3D, axes3d, art3d, proj3d, axis3d",
    "fig = plt.figure()",
    "ax = fig.add_subplot(projection='3d')",
]

#: Grids the plotting variants share.
GRIDS = [
    "X, Y = np.meshgrid(np.linspace(-1, 1, 6), np.linspace(-2, 2, 5))",
    "Z = np.cos(X) * np.sin(Y)",
    "tri_x = np.array([0, 1, 0, 1, 0.5])",
    "tri_y = np.array([0, 0, 1, 1, 0.5])",
    "tri_z = np.array([1, 2, 3, 4, 0])",
    "filled = np.zeros((3, 3, 3), dtype=bool)",
    "filled[0, 0, 0] = filled[1, 1, 1] = filled[2, 1, 0] = True",
]

#: method -> argument lists (the text between the call's parentheses)
VARIANTS: dict[str, list[str]] = {
    "plot": ["x, y, values", "x, y", "x, y, 2, zdir='y'", "x, y, values, 'r--'", "x, y, zs=values, lw=3",
             "[0, 1], [0, 1], [0, 1], marker='o', label='l'", "x, y, values, axlim_clip=True"],
    "scatter": ["x, y, values", "x, y, values, c=values, cmap='plasma'", "x, y, zs=1, zdir='x', s=50",
                "x, y, values, depthshade=False, marker='^'", "x, y, values, s=values * 10, alpha=0.5",
                "[0, 1], [0, 1], [0, 1], c=[1, 2]", "x, y, values, depthshade_minalpha=0.6"],
    "plot_surface": ["X, Y, Z", "X, Y, Z, cmap='viridis'", "X, Y, Z, rstride=2, cstride=2, color='r'",
                     "X, Y, Z, rcount=3, ccount=3, shade=False", "X, Y, Z, facecolors=plt.cm.jet(Z / 2 + 0.5)",
                     "X, Y, Z, cmap='coolwarm', vmin=-0.5, vmax=0.5, linewidth=1, edgecolor='k'"],
    "plot_wireframe": ["X, Y, Z", "X, Y, Z, rstride=2, cstride=0", "X, Y, Z, rcount=2, ccount=4, color='g'"],
    "plot_trisurf": ["tri_x, tri_y, tri_z", "tri_x, tri_y, tri_z, cmap='magma'",
                     "tri_x, tri_y, tri_z, color='r', shade=False"],
    "contour": ["X, Y, Z", "X, Y, Z, levels=3, zdir='x', offset=-1", "X, Y, Z, extend3d=True, stride=2",
                "X, Y, Z, colors='k'"],
    "contourf": ["X, Y, Z", "X, Y, Z, levels=4, zdir='z', offset=-1", "X, Y, Z, cmap='jet'"],
    "tricontour": ["tri_x, tri_y, tri_z", "tri_x, tri_y, tri_z, levels=2, zdir='y', offset=2"],
    "tricontourf": ["tri_x, tri_y, tri_z", "tri_x, tri_y, tri_z, levels=3"],
    "bar": ["[1, 2, 3], [3, 1, 2]", "[1, 2, 3], [3, 1, 2], zs=1, zdir='y'", "[1, 2], [2, 3], [0, 1], color='r'"],
    "bar3d": ["[0, 1], [0, 1], [0, 0], 0.5, 0.5, [1, 2]", "[0, 1], [0, 1], [0, 0], 0.5, 0.5, [1, 2], shade=False,"
              " color='g'", "[0], [0], [0], 1, 1, 1, color=['r', 'g', 'b', 'c', 'm', 'y']"],
    "quiver": ["[0], [0], [0], [1], [1], [1]", "x, y, values, y, x, y, length=0.5, normalize=True",
               "[0, 1], [0, 1], [0, 1], [1, 0], [0, 1], [1, 1], pivot='tip', arrow_length_ratio=0.5, colors='r'"],
    "voxels": ["filled", "filled, facecolors='r', edgecolors='k'", "np.indices((4, 4, 4))[0], "
               "np.indices((4, 4, 4))[1], np.indices((4, 4, 4))[2], filled"],
    "stem": ["x, y, values", "x, y, values, bottom=1, orientation='x'", "x, y, values, linefmt='r-', "
             "markerfmt='D', basefmt='k:'"],
    "text": ["0, 0, 0, 'origin'", "1, 1, 1, 'dir', zdir='x'", "0.5, 0.5, 0.5, 'tuple', zdir=(1, 1, 0)",
             "0, 1, 0, 'clipped', axlim_clip=True, color='r'"],
    "text2D": ["0.05, 0.95, '2D', transform=ax.transAxes"],
    "errorbar": ["x, y, values, zerr=0.5", "x, y, values, xerr=0.1, yerr=0.2, zerr=0.3, errorevery=2",
                 "x, y, values, zerr=0.5, zuplims=True, zlolims=True, capsize=3"],
    "fill_between": ["x, y, 0, x, y, values", "x, y, values, x, y, 0, mode='quad'",
                     "x, y, values, x, y, 0, where=y > 0, facecolor='r'"],
    "add_collection3d": ["collections.PolyCollection([[(0, 0), (1, 0), (1, 1)]]), zs=1",
                         "collections.LineCollection([[(0, 0), (1, 1)]]), zs=[0.5], zdir='y'",
                         "art3d.Poly3DCollection([[(0, 0, 0), (1, 0, 0), (1, 1, 1)]], alpha=0.5)"],
}

#: What a plotting program reads after each variant.
AFTER = [
    "ax.get_xlim(), ax.get_ylim(), ax.get_zlim()",
    "ax.xy_dataLim.bounds, ax.zz_dataLim.bounds",
    "ax.get_xticks(), ax.get_zticks()",
    "[t.get_text() for t in ax.get_zticklabels()]",
    "[type(c).__name__ for c in ax.get_children()]",
    "len(ax.lines), len(ax.patches), len(ax.collections), len(ax.texts)",
    "ax.has_data()",
]


def _plotting() -> Iterator[tuple[str, list[str]]]:
    """One program per plotting method of `Axes3D`."""
    for method, variants in VARIANTS.items():
        statements = list(GRIDS)
        for arguments in variants:
            statements += [
                "fig = plt.figure()",
                "ax = fig.add_subplot(projection='3d')",
                f"result = ax.{method}({arguments})",
                "type(result).__name__",
                *AFTER,
                "fig.canvas.draw()",
                *AFTER,
            ]
        yield f"plotting_{method}", statements


def _view() -> Iterator[str]:
    yield "ax.plot(x, y, values)"
    yield "ax.M, ax.invM"
    yield "ax.get_axis_position()"
    yield "ax.format_coord(0.01, 0.01)"
    yield "ax.elev, ax.azim, ax.roll, ax.initial_elev, ax.initial_azim"
    yield "ax.get_proj()"
    yield "fig.canvas.draw()"
    yield "ax.M, ax.invM"
    yield "ax.get_axis_position()"
    yield "ax.format_coord(0.01, 0.01)"
    yield "ax.format_coord(-0.05, 0.02)"
    yield "ax._calc_coord(0.01, 0.01)"
    yield "ax.view_init(20, 30)"
    yield "ax.elev, ax.azim, ax.roll"
    yield "ax.get_axis_position()"
    yield "ax.get_proj()"
    yield "ax.view_init(10, 20, 30, vertical_axis='y')"
    yield "ax.get_proj()"
    yield "ax._vertical_axis"
    yield "ax.view_init(elev=-90, azim=0, vertical_axis='x')"
    yield "ax.get_proj()"
    yield "ax.view_init()"
    yield "ax.elev, ax.azim, ax.roll"
    yield "ax.view_init(vertical_axis='w')"
    yield "ax.elev = 45"
    yield "ax.azim = 120.5"
    yield "ax.elev, ax.azim, ax.get_proj()"
    yield "fig.canvas.draw()"
    yield "ax.get_axis_position()"
    yield "ax.set_proj_type('ortho')"
    yield "ax.get_proj()"
    yield "ax._focal_length"
    yield "ax.set_proj_type('persp', focal_length=0.2)"
    yield "ax.get_proj(), ax._focal_length"
    yield "ax.set_proj_type('persp', focal_length=0)"
    yield "ax.set_proj_type('ortho', focal_length=1)"
    yield "ax.set_proj_type('fisheye')"
    yield "ax.get_box_aspect()"
    yield "ax.set_box_aspect((1, 2, 3))"
    yield "ax.get_box_aspect()"
    yield "ax.set_box_aspect(None, zoom=2)"
    yield "ax.get_box_aspect(), ax.get_proj()"
    yield "ax.set_box_aspect((1, 1), zoom=1)"
    yield "ax.set_box_aspect((1, 1, 1), zoom=0)"
    yield "fig.canvas.draw()"
    for adjustable in ("box", "datalim"):
        for aspect in ("equal", "equalxy", "equalxz", "equalyz", "auto"):
            yield f"ax.set_aspect('{aspect}', adjustable='{adjustable}')"
            yield "ax.get_aspect(), ax.get_adjustable()"
            yield "ax.get_xlim(), ax.get_ylim(), ax.get_zlim()"
            yield "ax.get_box_aspect()"
    yield "ax.set_aspect('equal', adjustable='figure')"
    yield "ax.set_adjustable('datalim')"
    yield "ax.get_adjustable()"
    yield "ax.get_position().bounds, ax.get_position(original=True).bounds"
    yield "ax.bbox.bounds, ax.transAxes.get_matrix()"
    yield "ax.set_anchor('W')"
    yield "fig.canvas.draw()"
    yield "ax.get_position().bounds, ax.get_anchor()"
    yield "ax.axison, ax._axis3don"
    yield "ax.set_axis_off()"
    yield "ax.axison, ax._axis3don"
    yield "ax.axis('on')"
    yield "ax.axison, ax._axis3don"
    yield "ax.axis(False)"
    yield "ax.axison, ax._axis3don"
    yield "ax.set_axis_on()"
    yield "ax.grid(False)"
    yield "ax._draw_grid"
    yield "ax.grid(True)"
    yield "ax.computed_zorder"
    yield "ax.computed_zorder = False"
    yield "fig.canvas.draw()"
    yield "ax.set_top_view()"
    yield "ax.get_xlim(), ax.viewLim.bounds"
    yield "ax.get_tightbbox().bounds"
    yield "ax.get_subplotspec() is ax.get_subplotspec()"
    yield "ax.can_zoom(), ax.can_pan()"


def _limits() -> Iterator[str]:
    yield "ax.scatter([0, 1, 2], [-1, 0, 1], [10, 20, 30])"
    yield "ax.get_xlim(), ax.get_ylim(), ax.get_zlim()"
    yield "ax.get_w_lims()"
    yield "ax.get_autoscale_on(), ax.get_autoscalez_on()"
    yield "ax.margins()"
    yield "ax.margins(0.1, 0.2, 0.3)"
    yield "ax.margins()"
    yield "ax.margins(0.1, 0.2)"
    yield "ax.margins(0.2, x=0.1)"
    yield "ax.margins(-0.6)"
    yield "ax.margins(tight=False)"
    yield "ax.set_zmargin(0.5)"
    yield "ax.get_zlim(), ax.get_zmargin()"
    yield "ax.autoscale(axis='z', tight=True)"
    yield "ax.get_zlim()"
    yield "ax.autoscale(enable=False)"
    yield "ax.get_autoscale_on(), ax.get_autoscalex_on()"
    yield "ax.autoscale(axis='w')"
    yield "ax.auto_scale_xyz([0, 5], [0, 6], [0, 7])"
    yield "ax.get_w_lims()"
    yield "ax.autoscale_view(tight=True, scalez=False)"
    yield "ax.set_xlim3d(-1, 2, view_margin=0.1)"
    yield "ax.get_xlim3d()"
    yield "ax.set_xlim(xmin=0)"
    yield "ax.set_xlim(0, xmax=1)"
    yield "ax.set_ylim3d(2, -2)"
    yield "ax.get_ylim(), ax.yaxis_inverted()"
    yield "ax.set_zlim((0, np.inf))"
    yield "ax.set_zlim(np.nan, 1)"
    yield "ax.set_zlim(1, 1)"
    yield "ax.get_zlim()"
    yield "ax.set_xbound(-3, 3)"
    yield "ax.get_xbound(), ax.get_xlim()"
    yield "ax.set_zbound(upper=5, view_margin=0.2)"
    yield "ax.get_zbound()"
    yield "ax.invert_zaxis()"
    yield "ax.zaxis_inverted(), ax.get_zlim(), ax.get_zbound()"
    yield "ax.set_zinverted(False)"
    yield "ax.get_zinverted()"
    yield "ax.set_zscale('log')"
    yield "ax.get_zscale(), ax.get_zlim()"
    yield "ax.set_zlim(-1, 100)"
    yield "ax.get_zlim()"
    yield "ax.set_zlim(0, 0)"
    yield "ax.set_xscale('symlog', linthresh=0.5)"
    yield "ax.get_xscale(), ax.get_xticks()"
    yield "ax.set_yscale('logit')"
    yield "ax.get_ylim()"
    yield "ax.set_yscale('asinh')"
    yield "ax.set_zscale('function', functions=(np.sqrt, np.square))"
    yield "ax.get_zscale()"
    yield "fig.canvas.draw()"
    yield "ax.get_zticks(), [t.get_text() for t in ax.get_zticklabels()]"
    yield "ax.set_zscale('linear')"
    yield "ax.set_zticks([10, 20, 30], ['a', 'b', 'c'])"
    yield "ax.get_zticks(), [t.get_text() for t in ax.get_zticklabels()]"
    yield "ax.set_zticks([5.5], minor=True)"
    yield "ax.get_zticks(minor=True)"
    yield "ax.zaxis.get_ticklocs(minor=True)"
    yield "ax.set_zticklabels(['x'])"
    yield "ax.set_zlabel('depth', labelpad=12)"
    yield "ax.get_zlabel(), ax.zaxis.labelpad"
    yield "ax.tick_params(axis='z', labelsize=7, colors='r')"
    yield "ax.tick_params(axis='both', which='minor', length=2)"
    yield "ax.locator_params(axis='z', nbins=3)"
    yield "ax.ticklabel_format(axis='z', style='sci', scilimits=(0, 0))"
    yield "fig.canvas.draw()"
    yield "ax.get_zticks(), [t.get_text() for t in ax.get_zticklabels()]"
    yield "ax.zaxis.get_offset_text().get_text()"
    yield "ax.format_zdata(12.3456)"
    yield "ax.fmt_zdata = lambda z: f'{z:.1f}z'"
    yield "ax.format_zdata(12.3456)"
    yield "other = fig.add_subplot(2, 2, 4, projection='3d', sharez=ax, shareview=ax)"
    yield "other.get_zlim() == ax.get_zlim()"
    yield "ax.set_zlim(0, 2)"
    yield "other.get_zlim()"
    yield "ax.view_init(5, 6, 7)"
    yield "other.elev, other.azim, other.roll"
    yield "ax.sharez(other)"
    yield "third = fig.add_subplot(2, 2, 1, projection='3d')"
    yield "third.shareview(ax)"
    yield "third.elev, third.azim"
    yield "ax.clear()"
    yield "ax.get_xlim(), ax.get_zlim(), ax.get_zscale(), ax.has_data()"


def _mouse() -> Iterator[str]:
    yield "from matplotlib.backend_bases import MouseButton, MouseEvent, NavigationToolbar2"
    yield "ax.scatter(0, 0, 0)"
    yield "ax._on_move(MouseEvent('motion_notify_event', fig.canvas, 300, 250))"
    yield "MouseEvent._from_ax_coords('button_press_event', ax, (0, 0), MouseButton.LEFT)._process()"
    yield "ax.button_pressed"
    yield "MouseEvent._from_ax_coords('motion_notify_event', ax, (0.01, 0.01), MouseButton.LEFT)._process()"
    yield "ax.elev, ax.azim, ax.roll"
    yield "MouseEvent._from_ax_coords('button_release_event', ax, (0.01, 0.01), MouseButton.LEFT)._process()"
    yield "fig.canvas.draw()"
    yield "ax.get_xlim(), ax.get_ylim(), ax.get_zlim()"
    for button in ("LEFT", "MIDDLE", "RIGHT"):
        yield f"MouseEvent._from_ax_coords('button_press_event', ax, (0, 0), MouseButton.{button})._process()"
        yield "ax.button_pressed, ax._sx, ax._sy"
        yield "ax.format_coord(0.01, 0.01)"
        yield f"MouseEvent._from_ax_coords('motion_notify_event', ax, (0.02, 0.03), MouseButton.{button})._process()"
        yield "ax.elev, ax.azim, ax.roll"
        yield "ax.get_xlim(), ax.get_ylim(), ax.get_zlim()"
        yield f"MouseEvent._from_ax_coords('button_release_event', ax, (0.02, 0.03), MouseButton.{button})._process()"
        yield "ax.button_pressed"
    for style in ("azel", "trackball", "sphere", "arcball"):
        yield f"matplotlib.rcParams['axes3d.mouserotationstyle'] = '{style}'"
        yield "ax.view_init(0, 0, 30)"
        yield "fig.canvas.draw()"
        yield "MouseEvent._from_ax_coords('button_press_event', ax, (0, 0), MouseButton.LEFT)._process()"
        yield ("MouseEvent._from_ax_coords('motion_notify_event', ax, (0.25 * ax._pseudo_w, 0.1 * ax._pseudo_h), "
               "MouseButton.LEFT)._process()")
        yield "ax.elev, ax.azim, ax.roll"
        yield "MouseEvent._from_ax_coords('button_release_event', ax, (0, 0), MouseButton.LEFT)._process()"
    yield "matplotlib.rcParams['axes3d.mouserotationstyle'] = 'azel'"
    yield "ax.view_init(11, 23)"
    yield "MouseEvent._from_ax_coords('button_press_event', ax, (0, 0), MouseButton.LEFT, key='control')._process()"
    yield ("MouseEvent._from_ax_coords('motion_notify_event', ax, (0.013, 0.007), MouseButton.LEFT, "
           "key='control')._process()")
    yield "ax.elev, ax.azim, ax.roll"
    yield "MouseEvent._from_ax_coords('button_release_event', ax, (0, 0), MouseButton.LEFT)._process()"
    yield "ax.mouse_init(rotate_btn=3, pan_btn=1, zoom_btn=[2])"
    yield "ax._rotate_btn, ax._pan_btn, ax._zoom_btn"
    yield "MouseEvent._from_ax_coords('button_press_event', ax, (0, 0), MouseButton.LEFT)._process()"
    yield "MouseEvent._from_ax_coords('motion_notify_event', ax, (0.05, 0.05), MouseButton.LEFT)._process()"
    yield "ax.get_xlim(), ax.get_ylim(), ax.get_zlim()"
    yield "MouseEvent._from_ax_coords('button_release_event', ax, (0.05, 0.05), MouseButton.LEFT)._process()"
    yield "ax.disable_mouse_rotation()"
    yield "ax._rotate_btn, ax._pan_btn, ax._zoom_btn"
    yield "ax.mouse_init()"
    yield "view = ax._get_view()"
    yield "view"
    yield "ax._set_view_from_bbox((150, 150, 300, 300))"
    yield "ax.get_xlim(), ax.get_ylim(), ax.get_zlim()"
    yield "ax._set_view_from_bbox((150, 150, 300, 300), direction='out', mode='x')"
    yield "ax.get_xlim(), ax.get_ylim(), ax.get_zlim()"
    yield "ax._set_view(view)"
    yield "ax.get_xlim(), ax.elev, ax.azim"
    yield "ax._zoom_data_limits(0.5, 0.5, 0.5)"
    yield "ax.get_xlim(), ax.get_ylim(), ax.get_zlim()"
    yield "ax._zoom_data_limits(2, 1, 0.5)"
    yield "ax.get_xlim(), ax.get_ylim(), ax.get_zlim()"
    yield "ax.start_pan(200, 200, 1)"
    yield "ax.drag_pan(1, None, 210, 205)"
    yield "ax.get_xlim(), ax.get_ylim(), ax.get_zlim()"
    yield "ax.drag_pan(1, 'x', 230, 215)"
    yield "ax.get_xlim(), ax.get_ylim(), ax.get_zlim()"
    yield "ax.end_pan()"
    for tool, button, key in (("zoom", "LEFT", None), ("zoom", "LEFT", "'x'"), ("zoom", "RIGHT", None),
                              ("pan", "LEFT", None), ("pan", "LEFT", "'y'")):
        yield "fig = plt.figure()"
        yield "ax = fig.add_subplot(projection='3d')"
        yield "ax.scatter(0, 0, 0)"
        yield "fig.canvas.draw()"
        yield "s0 = ax.transData.transform((0, 0)).astype(int)"
        yield "s1 = ax.transData.transform((1, 1)).astype(int)"
        yield "tb = NavigationToolbar2(fig.canvas)"
        yield f"tb.{tool}()"
        yield f"MouseEvent('button_press_event', fig.canvas, *s0, MouseButton.{button}, key={key})._process()"
        yield (f"MouseEvent('motion_notify_event', fig.canvas, *s1, MouseButton.{button}, key={key}, "
               f"buttons={{MouseButton.{button}}})._process()")
        yield f"MouseEvent('button_release_event', fig.canvas, *s1, MouseButton.{button}, key={key})._process()"
        yield "ax.get_xlim3d(), ax.get_ylim3d(), ax.get_zlim3d()"
        yield "tb.back()"
        yield "ax.get_xlim3d(), ax.get_zlim3d()"
        yield "tb.forward()"
        yield "ax.get_xlim3d(), ax.get_zlim3d()"
        yield "tb.home()"
        yield "ax.get_xlim3d(), ax.get_zlim3d()"


def _art3d() -> Iterator[str]:
    yield "art3d.get_dir_vector('x'), art3d.get_dir_vector(None), art3d.get_dir_vector((1, 2, 3))"
    yield "art3d.get_dir_vector('w')"
    yield "art3d.juggle_axes([1], [2], [3], 'x'), art3d.juggle_axes([1], [2], [3], '-y')"
    yield "art3d.rotate_axes([1], [2], [3], 'x'), art3d.rotate_axes([1], [2], [3], '-x')"
    yield "line = art3d.Line3D([0, 1], [0, 1], [0, 1], color='r')"
    yield "line.get_data_3d(), line.get_color(), line.get_data()"
    yield "line.set_data_3d([0, 2], [1, 3], [2, 4])"
    yield "line.get_data_3d()"
    yield "line.set_data_3d(np.array([[0, 1], [1, 0], [2, 2]]))"
    yield "line.get_data_3d()"
    yield "line.set_data_3d(1, 2, 3)"
    yield "ax.add_line(line)"
    yield "line.get_data_3d(), line.get_data()"
    yield "line.set_3d_properties(zs=2, zdir='y')"
    yield "line.get_data_3d()"
    yield "line2d = ax.plot([0, 1], [1, 0])[0]"
    yield "art3d.line_2d_to_3d(line2d, zs=[1, 2])"
    yield "type(line2d).__name__, line2d.get_data_3d()"
    yield "lines = art3d.Line3DCollection([[(0, 0, 0), (1, 1, 1)], [(0, 1, 0), (1, 0, 1)]], colors=['r', 'b'])"
    yield "ax.add_collection3d(lines)"
    yield "lines.get_segments(), lines.get_colors()"
    yield "lc = collections.LineCollection([[(0, 0), (1, 1)]])"
    yield "art3d.line_collection_2d_to_3d(lc, zs=1, zdir='x')"
    yield "type(lc).__name__"
    yield ("poly = art3d.Poly3DCollection([[(0, 0, 0), (1, 0, 0), (1, 1, 0)], [(0, 0, 1), (1, 0, 1), (1, 1, 1)]], "
           "facecolors=['r', 'g'], edgecolors='k', zsort='min')")
    yield "ax.add_collection3d(poly)"
    yield "poly.get_facecolor(), poly.get_edgecolor()"
    yield "poly.get_paths()"
    yield "poly.set_zsort('max')"
    yield "poly.set_zsort('mean')"
    yield "poly.set_sort_zpos(3)"
    yield "poly.set_alpha(0.3)"
    yield "poly.get_facecolor(), poly.get_alpha()"
    yield "poly.set_facecolor('b')"
    yield "poly.get_facecolor()"
    yield "poly.set_verts([[(0, 0, 0), (0, 1, 0), (0, 1, 1), (0, 0, 1)]])"
    yield "poly.set_verts_and_codes([[(0, 0, 0), (0, 1, 0), (0, 1, 1)]], [[1, 2, 79]])"
    yield "art3d.Poly3DCollection([[(0, 0), (1, 1)]])"
    yield "art3d.Poly3DCollection([[(0, 0, 0), (1, 1, 1), (0, 1, 1)]], shade=True)"
    yield "shaded = art3d.Poly3DCollection([[(0, 0, 0), (1, 1, 1), (0, 1, 1)]], shade=True, facecolors='r')"
    yield "shaded.get_facecolor()"
    yield "pc = collections.PolyCollection([[(0, 0), (1, 0), (1, 1)]], facecolors='g')"
    yield "art3d.poly_collection_2d_to_3d(pc, zs=2)"
    yield "ax.add_collection3d(pc)"
    yield "type(pc).__name__, pc.get_facecolor()"
    yield "patch = patches.Circle((0.5, 0.5), 0.3)"
    yield "ax.add_patch(patch)"
    yield "art3d.patch_2d_to_3d(patch, z=1, zdir='y')"
    yield "type(patch).__name__"
    yield "rect = art3d.Patch3D(zs=[0, 1, 2, 3], zdir='z', facecolor='r')"
    yield "type(rect).__name__"
    yield "pp = patches.PathPatch(path.Path([(0, 0), (1, 0), (1, 1)], [1, 2, 2]))"
    yield "ax.add_patch(pp)"
    yield "art3d.pathpatch_2d_to_3d(pp, z=0.5, zdir='x')"
    yield "type(pp).__name__, pp.get_path()"
    yield "sc = ax.scatter([0, 1, 2], [2, 1, 0], [1, 1, 1], c=[1, 2, 3])"
    yield "sc._offsets3d, sc.get_offsets()"
    yield "sc.get_depthshade()"
    yield "sc.set_depthshade(False)"
    yield "sc.get_depthshade()"
    yield "sc._offsets3d = ([0, 1], [0, 1], [0, 1])"
    yield "sc._offsets3d"
    yield "sc.set_3d_properties([5, 6], 'z')"
    yield "sc._offsets3d"
    yield "patch_col = collections.PatchCollection([patches.Circle((0, 0), 0.1), patches.Circle((1, 1), 0.2)])"
    yield "art3d.patch_collection_2d_to_3d(patch_col, zs=[0, 1], zdir='y', depthshade=False)"
    yield "ax.add_collection3d(patch_col)"
    yield "type(patch_col).__name__, patch_col.get_depthshade()"
    yield "t = ax.text(1, 2, 3, 'three', zdir='y')"
    yield "type(t).__name__, t.get_position_3d(), t.get_position()"
    yield "t.set_position_3d((4, 5, 6), zdir='x')"
    yield "t.get_position_3d()"
    yield "t.set_z(7)"
    yield "t.get_position_3d()"
    yield "t.set_3d_properties(z=1, zdir=(1, 1, 1))"
    yield "t.get_position_3d()"
    yield "t2 = ax.text2D(0.1, 0.1, 'flat')"
    yield "art3d.text_2d_to_3d(t2, z=2, zdir='z')"
    yield "type(t2).__name__, t2.get_position_3d()"
    yield "t.get_tightbbox()"
    yield "fig.canvas.draw()"
    yield "poly.get_facecolor(), poly.get_edgecolor()"
    yield "line.get_data()"
    yield "lines.do_3d_projection()"
    yield "sc.get_offsets(), sc.get_facecolor()"
    yield "t.get_position()"
    yield "art3d._all_points_on_plane([0, 1, 0], [0, 0, 1], [0, 0, 0])"
    yield "art3d._all_points_on_plane([0, 1, 0, 1], [0, 0, 1, 1], [0, 0, 0, 1])"


def _proj3d() -> Iterator[str]:
    yield "M = proj3d.world_transformation(0, 1, 0, 2, 0, 3)"
    yield "M"
    yield "proj3d.world_transformation(0, 1, 0, 2, 0, 3, pb_aspect=(1, 2, 3))"
    yield "proj3d.proj_transform(1, 1, 1, M)"
    yield "proj3d.proj_transform(np.array([0, 1]), np.array([0, 2]), np.array([0, 3]), M)"
    yield "proj3d.inv_transform(1, 0.5, 1 / 3, M)"
    yield "proj3d.proj_transform_clip(1, 1, 1, M)"
    yield "proj3d.proj_transform(1, 1, 1, ax.M)"
    yield "proj3d.proj_transform([1.5], [1.5], [1.5], ax.get_proj())"
    yield "fig.canvas.draw()"
    yield "proj3d.proj_transform(1, 1, 1, ax.M)"
    yield "proj3d.inv_transform(0.01, 0.02, -0.1, ax.invM)"
    yield "proj3d._persp_transformation(-1, 1, 1)"
    yield "proj3d._ortho_transformation(-1, 1)"
    yield "proj3d._view_axes(np.array([1, 1, 1]), np.array([0, 0, 0]), np.array([0, 0, 1]), 0.3)"


def _axis3d() -> Iterator[str]:
    yield "ax.plot([0, 1, 2], [0, 2, 4], [1, 2, 3])"
    for name in ("xaxis", "yaxis", "zaxis"):
        yield f"axis = ax.{name}"
        yield "type(axis).__name__, axis.axis_name, axis.get_rotate_label('long label')"
        yield "axis.get_ticks_position(), axis.get_label_position()"
        yield "axis.active_pane()"
        yield "axis.get_view_interval(), axis.get_data_interval(), axis.get_minpos()"
        yield "axis.get_inverted(), axis.get_scale()"
        yield "type(axis.line).__name__, axis.line.get_data(), axis.line.get_color()"
        yield "type(axis.pane).__name__, axis.pane.get_facecolor()"
        yield "type(axis.gridlines).__name__"
        yield "axis.label.get_text(), axis.offsetText.get_text()"
    yield "fig.canvas.draw()"
    for name in ("xaxis", "yaxis", "zaxis"):
        yield f"axis = ax.{name}"
        yield "axis.active_pane()"
        yield "axis.line.get_data()"
        yield "axis.get_tightbbox().bounds"
        yield "axis.get_tick_space()"
        yield "axis.get_ticklocs(), axis.get_ticklocs(minor=True)"
        yield "[t.get_text() for t in axis.get_ticklabels()]"
        yield "[t.get_position() for t in axis.get_ticklabels()]"
        yield "axis._get_tickdir('default'), axis._get_tickdir('upper')"
    yield "axis = ax.zaxis"
    yield "axis.set_pane_color('r')"
    yield "axis.pane.get_facecolor()"
    yield "axis.set_pane_color((0, 0, 1), alpha=0.25)"
    yield "axis.pane.get_facecolor(), axis.pane.get_edgecolor()"
    yield "axis.set_rotate_label(False)"
    yield "axis.get_rotate_label('long label')"
    yield "axis.set_ticks_position('upper')"
    yield "axis.get_ticks_position()"
    yield "axis.set_ticks_position('top')"
    yield "axis.set_label_position('both')"
    yield "axis.get_label_position()"
    yield "axis.set_label_position('left')"
    yield "axis.line.set_color('r')"
    yield "axis.line.set_linewidth(3)"
    yield "axis.line.get_color(), axis.line.get_linewidth()"
    yield "axis.line.set_visible(False)"
    yield "axis.line.get_visible()"
    yield "axis.set_view_interval(-1, 5)"
    yield "axis.get_view_interval(), ax.get_zlim()"
    yield "axis.set_view_interval(0, 2, ignore=True)"
    yield "ax.get_zlim()"
    yield "axis.set_data_interval(-2, 8)"
    yield "axis.get_data_interval()"
    yield "axis.set_inverted(True)"
    yield "axis.get_inverted(), ax.get_zlim()"
    yield "axis.set_major_locator(ticker.MultipleLocator(0.5))"
    yield "axis.set_major_formatter('{x:.2f}')"
    yield "axis.set_minor_locator(ticker.AutoMinorLocator())"
    yield "fig.canvas.draw()"
    yield "axis.get_ticklocs(), [t.get_text() for t in axis.get_ticklabels()]"
    yield "len(axis.get_minorticklocs())"
    yield "len(axis.get_major_ticks()), len(axis.get_minor_ticks())"
    yield "axis.get_major_ticks()[0].get_loc()"
    yield "ax.get_zgridlines(), ax.get_zticklines()"
    yield "ax.xaxis.set_visible(False)"
    yield "ax.get_tightbbox().bounds"


AREAS = {
    "view": _view,
    "limits": _limits,
    "mouse": _mouse,
    "art3d": _art3d,
    "proj3d": _proj3d,
    "axis3d": _axis3d,
}


def programs() -> Iterator[tuple[str, list[str]]]:
    """The plotting programs, then one program per area."""
    header = common.HEADER + common.DATA
    for name, statements in _plotting():
        yield name, header + AXES + statements
    for name, statements in AREAS.items():
        yield name, header + AXES + list(statements())
