"""The variants of the toolkit subjects of the ``api`` generator: a second
object of each toolkit subject's class (``subjects.SUBJECTS``,
``artists.SUBJECTS``), made with other arguments, on which the base
subject's callables run again (``api._variants``).

The objects are those of other layouts than the base subject's: 3-D axes
in another projection, view and vertical axis, and their artists made with
other directions and styles; axisartist axes on curvilinear and floating
grid helpers, their axis artists and the parts of those; axes_grid1 grids,
dividers, host and parasite axes, inset locators and anchored artists with
other layouts; images of each kind other than ``AxesImage`` with other
interpolations, origins, colour maps and data; geographic axes on another
centre and grid.
"""

from __future__ import annotations

from .subjects import AXES, CURVILINEAR

#: A 3-D axes in orthographic projection, seen from another side and rolled.
ORTHO = ["fig = plt.figure()", "ax = fig.add_subplot(projection='3d', proj_type='ortho', elev=50, azim=-120, "
         "roll=20, box_aspect=(2, 1, 1), computed_zorder=False)"]
#: A 3-D axes with the y axis vertical, a short focal length and inverted x.
VERTICAL_Y = ["fig = plt.figure()", "ax = fig.add_subplot(projection='3d', focal_length=0.4)",
              "ax.view_init(elev=15, azim=60, vertical_axis='y')", "ax.scatter(x, y, values)",
              "ax.set_xlim(7, -1)"]
#: Curvilinear axes on a rotated, skewed grid, drawn once.
SKEWED = ["from mpl_toolkits.axisartist import grid_helper_curvelinear, grid_finder, angle_helper",
          "from mpl_toolkits.axisartist import SubplotHost",
          "fig = plt.figure()",
          "skew = transforms.Affine2D().skew_deg(20, 0).rotate_deg(15)",
          "helper = grid_helper_curvelinear.GridHelperCurveLinear(skew, grid_locator1=grid_finder.MaxNLocator(5), "
          "grid_locator2=grid_finder.FixedLocator([-2, 0, 1, 3]))",
          "ax = fig.add_subplot(axes_class=SubplotHost, grid_helper=helper)",
          "ax.set_xlim(-3, 4)", "ax.set_ylim(-2, 3)", "ax.plot([-2, 3], [-1, 2])",
          "ax.axis['top'].major_ticklabels.set_visible(True)", "fig.canvas.draw()"]
#: A quarter annulus of polar axes on a floating grid helper.
FLOATING_POLAR = ["from mpl_toolkits.axisartist import floating_axes, angle_helper, grid_finder",
                  "fig = plt.figure()",
                  "polar = transforms.Affine2D().scale(np.pi / 180, 1) + "
                  "matplotlib.projections.polar.PolarTransform()",
                  "helper = floating_axes.GridHelperCurveLinear(polar, extremes=(0, 90, 2, 6), "
                  "grid_locator1=angle_helper.LocatorDMS(4), tick_formatter1=angle_helper.FormatterDMS(), "
                  "grid_locator2=grid_finder.MaxNLocator(4))",
                  "ax = fig.add_subplot(axes_class=floating_axes.FloatingAxes, grid_helper=helper)"]
#: axisartist axes with a logarithmic y axis and an inverted x axis.
AXISARTIST_LOG = ["from mpl_toolkits import axisartist", "fig = plt.figure()",
                  "ax = fig.add_subplot(axes_class=axisartist.Axes)", "ax.plot(x + 1, values)",
                  "ax.set_yscale('log')", "ax.invert_xaxis()"]

#: (name, base subject, statements leaving the object in ``obj``)
VARIANTS: list[tuple[str, str, list[str]]] = [
    # mplot3d
    ("axes3d_variant", "axes3d", ORTHO + ["ax.plot_surface(X, Y, Z, cmap='viridis')", "obj = ax"]),
    ("axes3d_variant_vertical", "axes3d", VERTICAL_Y + ["obj = ax"]),
    ("xaxis3d_variant", "xaxis3d", VERTICAL_Y + ["obj = ax.xaxis"]),
    ("yaxis3d_variant", "yaxis3d", ORTHO + ["ax.plot(x, y, values)", "obj = ax.yaxis"]),
    ("zaxis3d_variant", "zaxis3d", VERTICAL_Y + ["ax.set_zlim(2, -2)", "obj = ax.zaxis"]),
    ("line3d_collection_variant", "line3d_collection",
     ORTHO + ["from mpl_toolkits.mplot3d import art3d",
              "obj = art3d.Line3DCollection([[(0, 0, 0), (1, 2, 1)], [(1, 0, 2), (0, 1, 0), (2, 2, 2)]], "
              "colors=['r', 'b'], linewidths=[1, 3], linestyles='dashed')",
              "ax.add_collection3d(obj, autolim=True)"]),
    ("patch3d_collection_variant", "patch3d_collection",
     VERTICAL_Y + ["from mpl_toolkits.mplot3d import art3d",
                   "obj = art3d.Patch3DCollection([patches.Rectangle((0, 0), 1, 2), patches.Circle((1, 1), 0.5)], "
                   "zs=[0, 1], zdir='x', depthshade=False, facecolors=['r', 'b'], edgecolors='k')",
                   "ax.add_collection3d(obj)"]),
    ("path3d_collection_variant", "path3d_collection",
     ORTHO + ["obj = ax.scatter(x, y, values, s=values * 20, marker='^', zdir='y', depthshade=False, "
              "c=values, cmap='plasma', edgecolors='k')"]),
    ("patch3d_variant", "patch3d",
     ORTHO + ["from mpl_toolkits.mplot3d import art3d", "obj = patches.Rectangle((0.2, 0.1), 0.5, 0.3, fc='g')",
              "ax.add_patch(obj)", "art3d.patch_2d_to_3d(obj, z=1, zdir='x')"]),
    ("surface3d_variant", "surface3d",
     VERTICAL_Y + ["obj = ax.plot_surface(X, Y, Z, rstride=2, cstride=2, color='r', shade=False, "
                   "edgecolor='k', linewidth=0.5)"]),
    ("line3d_variant", "line3d", ORTHO + ["obj = ax.plot(x, y, values, 'o--', zdir='x', markevery=2)[0]"]),
    ("text3d_variant", "text3d", VERTICAL_Y + ["obj = ax.text(1, 0, 2, 'diagonal', zdir=(1, 1, 0), "
                                               "ha='right', color='b')"]),
    ("path_patch3d_variant", "path_patch3d",
     ORTHO + ["from mpl_toolkits.mplot3d import art3d",
              "obj = patches.PathPatch(path.Path([(0, 0), (2, 1), (1, 2), (0, 0)], [1, 2, 2, 79]), fc='y')",
              "ax.add_patch(obj)", "art3d.pathpatch_2d_to_3d(obj, z=0.5, zdir='y')"]),
    ("contour3d_variant", "contour3d",
     VERTICAL_Y + ["obj = ax.contour(X, Y, Z, levels=[-0.5, 0, 0.5], zdir='x', offset=-1, cmap='coolwarm')"]),
    # axisartist
    ("axisartist_axes_variant", "axisartist_axes", CURVILINEAR + ["ax.plot([2, 8], [1, 6])", "obj = ax"]),
    ("axisartist_axes_variant_log", "axisartist_axes", AXISARTIST_LOG + ["obj = ax"]),
    ("axisartist_axes_zero_variant", "axisartist_axes_zero",
     ["from mpl_toolkits import axisartist", "fig = plt.figure()",
      "obj = fig.add_subplot(axes_class=axisartist.AxesZero)", "obj.plot(x - 3, y)",
      "obj.axis['xzero'].set_visible(True)", "obj.axis['yzero'].set_axisline_style('-|>')"]),
    ("axis_artist_variant", "axis_artist",
     CURVILINEAR + ["obj = ax.new_floating_axis(1, 6)", "ax.axis['radius'] = obj", "obj.label.set_text('r')"]),
    ("axis_artist_variant_floating", "axis_artist", FLOATING_POLAR + ["obj = ax.axis['left']"]),
    ("axis_artist_ticks_variant", "axis_artist_ticks",
     SKEWED + ["obj = ax.axis['left'].major_ticks", "obj.set_tick_direction('inout')"]),
    ("axis_artist_ticklabels_variant", "axis_artist_ticklabels", SKEWED + ["obj = ax.axis['top'].major_ticklabels"]),
    ("axis_artist_ticklabels_variant_floating", "axis_artist_ticklabels",
     FLOATING_POLAR + ["fig.canvas.draw()", "obj = ax.axis['bottom'].major_ticklabels"]),
    ("axis_artist_label_variant", "axis_artist_label",
     FLOATING_POLAR + ["ax.axis['left'].label.set_text('radius')", "fig.canvas.draw()",
                       "obj = ax.axis['left'].label"]),
    ("gridlines_collection_variant", "gridlines_collection", SKEWED + ["obj = ax.gridlines"]),
    ("fixed_axis_helper_variant", "fixed_axis_helper", AXISARTIST_LOG + ["obj = ax.axis['top']._axis_artist_helper"]),
    ("floating_axis_helper_variant", "floating_axis_helper",
     AXISARTIST_LOG + ["obj = ax.new_floating_axis(0, 3, axis_direction='top')._axis_artist_helper"]),
    ("curvilinear_helper_variant", "curvilinear_helper", SKEWED + ["obj = helper"]),
    ("curvilinear_floating_helper_variant", "curvilinear_floating_helper",
     CURVILINEAR + ["obj = ax.new_floating_axis(1, 6)._axis_artist_helper"]),
    ("curvilinear_fixed_helper_variant", "curvilinear_fixed_helper",
     SKEWED + ["obj = helper.new_fixed_axis('bottom', axes=ax)._axis_artist_helper"]),
    ("grid_finder_variant", "grid_finder", SKEWED + ["obj = helper.grid_finder"]),
    ("floating_axes_variant", "floating_axes", FLOATING_POLAR + ["ax.plot([30, 60], [3, 5])", "obj = ax"]),
    ("floating_fixed_helper_variant", "floating_fixed_helper", FLOATING_POLAR + ["obj = ax.axis['top']._axis_artist_helper"]),
    ("floating_grid_helper_variant", "floating_grid_helper", FLOATING_POLAR + ["obj = helper"]),
    ("grid_maxn_locator_variant", "grid_maxn_locator",
     ["from mpl_toolkits.axisartist import grid_finder",
      "obj = grid_finder.MaxNLocator(nbins=7, steps=[1, 2, 5, 10], integer=True, symmetric=True)"]),
    ("locator_d_variant", "locator_d",
     ["from mpl_toolkits.axisartist import angle_helper", "obj = angle_helper.LocatorD(3, include_last=False)"]),
    # axes_grid1
    ("divider_variant", "divider",
     AXES + ["from mpl_toolkits.axes_grid1 import Divider, Size",
             "obj = Divider(fig, (0.05, 0.1, 0.9, 0.8), [Size.Fixed(0.5), Size.AxesX(ax), Size.Scaled(1)], "
             "[Size.Fraction(0.2, Size.AxesY(ax)), Size.Scaled(1)], aspect=True, anchor='NE')",
             "ax.set_axes_locator(obj.new_locator(1, 1))"]),
    ("subplot_divider_variant", "subplot_divider",
     ["from mpl_toolkits.axes_grid1.axes_divider import SubplotDivider", "from mpl_toolkits.axes_grid1 import Size",
      "fig = plt.figure()",
      "obj = SubplotDivider(fig, 2, 2, 3, horizontal=[Size.Fixed(1), Size.Scaled(1)], "
      "vertical=[Size.Scaled(1), Size.Fixed(0.4)], aspect=True, anchor='S')"]),
    ("axes_divider_variant", "axes_divider",
     AXES + ["from mpl_toolkits.axes_grid1 import make_axes_locatable", "ax.imshow(z)",
             "obj = make_axes_locatable(ax)", "obj.append_axes('top', size='20%', pad=0.1)",
             "obj.append_axes('left', 1.2, pad=0.3, sharey=ax)"]),
    ("image_grid_variant", "image_grid",
     ["from mpl_toolkits.axes_grid1 import ImageGrid", "fig = plt.figure()",
      "obj = ImageGrid(fig, 211, (3, 1), direction='column', axes_pad=(0.3, 0.1), share_all=True, aspect=False, "
      "label_mode='all', cbar_mode='edge', cbar_location='top', cbar_size='10%', cbar_pad=0.2)",
      "obj[0].imshow(z)"]),
    ("cbar_axes_variant", "cbar_axes",
     ["from mpl_toolkits.axes_grid1 import ImageGrid", "fig = plt.figure()",
      "grid = ImageGrid(fig, 111, (2, 2), cbar_mode='single', cbar_location='bottom', cbar_pad=0.3)",
      "im = grid[0].imshow(z, cmap='magma')", "obj = grid.cbar_axes[0]", "obj.colorbar(im)"]),
    ("host_axes_variant", "host_axes",
     ["from mpl_toolkits.axes_grid1 import host_subplot", "from mpl_toolkits import axisartist",
      "fig = plt.figure()", "obj = host_subplot(121, axes_class=axisartist.Axes, figure=fig)",
      "obj.plot(x, y)", "par = obj.twiny()", "par.plot(values, x)",
      "aux = obj.twin(transforms.Affine2D().scale(2, 1))"]),
    ("parasite_axes_variant", "parasite_axes",
     ["from mpl_toolkits.axes_grid1 import host_subplot", "fig = plt.figure()",
      "host = host_subplot(111, figure=fig)", "host.plot(x, y)",
      "obj = host.get_aux_axes(transforms.Affine2D().scale(1, 2), viewlim_mode='transform')",
      "obj.plot(x, y, 'r')"]),
    ("simple_axis_artist_variant", "simple_axis_artist",
     ["from mpl_toolkits.axes_grid1 import mpl_axes", "fig = plt.figure()",
      "a = fig.add_subplot(axes_class=mpl_axes.Axes)", "a.plot(days, np.arange(7))", "a.set_yscale('log')",
      "obj = a.axis['top']"]),
    ("anchored_size_locator_variant", "anchored_size_locator",
     AXES + ["from mpl_toolkits.axes_grid1 import inset_locator",
             "ins = inset_locator.inset_axes(ax, width=1.3, height='40%', loc='lower left', "
             "bbox_to_anchor=(0.1, 0.1, 0.5, 0.5), bbox_transform=ax.transAxes, borderpad=0)",
             "obj = ins.get_axes_locator()"]),
    ("anchored_zoom_locator_variant", "anchored_zoom_locator",
     AXES + ["from mpl_toolkits.axes_grid1 import inset_locator", "ax.plot(x, y)",
             "ins = inset_locator.zoomed_inset_axes(ax, 0.5, loc='lower right', bbox_to_anchor=(1, 0), "
             "bbox_transform=ax.transAxes, borderpad=1)",
             "obj = ins.get_axes_locator()"]),
    ("bbox_connector_variant", "bbox_connector",
     AXES + ["from mpl_toolkits.axes_grid1 import inset_locator", "ax.plot(x, y)",
             "ins = inset_locator.zoomed_inset_axes(ax, 2, loc='upper right')", "ins.set_xlim(1, 2)",
             "ins.set_ylim(0.5, 1)",
             "obj = inset_locator.BboxConnector(ins.bbox, transforms.TransformedBbox(ins.viewLim, ax.transData), "
             "2, 4, ec='r', ls='--')",
             "ax.add_patch(obj)"]),
    ("axes_size_fraction_variant", "axes_size_fraction",
     AXES + ["from mpl_toolkits.axes_grid1 import Size", "ax.set_aspect(2)",
             "obj = Size.Fraction(2, Size.AxesY(ax, aspect='axes', ref_ax=ax))"]),
    # images other than AxesImage
    ("bbox_image_variant", "bbox_image",
     AXES + ["obj = image.BboxImage(transforms.TransformedBbox(transforms.Bbox.from_bounds(0.2, 0.2, 0.5, 0.5), "
             "ax.transAxes), data=z, cmap='gray', interpolation='bilinear', origin='lower')",
             "ax.add_artist(obj)"]),
    ("non_uniform_image_variant", "non_uniform_image",
     AXES + ["obj = image.NonUniformImage(ax, interpolation='bilinear', extent=(-1, 5, -1, 4), cmap='RdBu')",
             "obj.set_data([0, 0.5, 2, 4], [0, 1, 3], z[:3, :4])", "ax.add_image(obj)", "ax.set_xlim(-1, 5)",
             "ax.set_ylim(-1, 4)"]),
    ("pcolor_image_variant", "pcolor_image",
     AXES + ["obj = image.PcolorImage(ax, np.array([0.0, 1, 3, 4, 7, 8, 9]), np.array([0.0, 2, 3, 5, 6, 8]), "
             "rgba[:, :, :3].repeat(3, axis=0)[:5].repeat(2, axis=1), cmap='magma')",
             "ax.add_image(obj)", "ax.set_xlim(0, 9)", "ax.set_ylim(0, 8)"]),
    ("offset_image_variant", "offset_image",
     AXES + ["obj = offsetbox.OffsetImage(rgba, zoom=10, interpolation='bilinear', origin='lower', dpi_cor=False)",
             "ax.add_artist(offsetbox.AnnotationBbox(obj, (0.3, 0.6), xycoords='axes fraction', frameon=False))"]),
    ("figimage_variant", "figimage",
     ["fig = plt.figure()", "obj = fig.figimage(z, xo=20, yo=30, origin='lower', cmap='gray', alpha=0.5)"]),
    # geographic axes
    ("aitoff_axes_variant", "aitoff_axes",
     ["fig = plt.figure()", "obj = fig.add_subplot(projection='lambert', center_longitude=0.5, center_latitude=0.3)",
      "obj.plot(x - 3, y)", "obj.set_longitude_grid(45)", "obj.set_latitude_grid(20)", "obj.grid(True)"]),
]
