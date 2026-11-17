# A full polar curvilinear grid (no extremes) with ExtremeFinderCycle and LocatorDMS/FormatterDMS, plus floating axes along r and theta.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.projections import PolarAxes
from matplotlib.transforms import Affine2D
from mpl_toolkits.axisartist import Axes, GridHelperCurveLinear, HostAxes
import mpl_toolkits.axisartist.angle_helper as angle_helper

tr = Affine2D().scale(np.pi / 180, 1) + PolarAxes.PolarTransform(apply_theta_transforms=False)
finder = angle_helper.ExtremeFinderCycle(nx=20, ny=20, lon_cycle=360, lat_cycle=None, lon_minmax=None, lat_minmax=(0, np.inf))
helper = GridHelperCurveLinear(tr, extreme_finder=finder, grid_locator1=angle_helper.LocatorDMS(12),
                               tick_formatter1=angle_helper.FormatterDMS())
fig = plt.figure(figsize=(5, 5), dpi=80)
ax = fig.add_subplot(axes_class=HostAxes, grid_helper=helper)
ax.axis["right"].major_ticklabels.set_visible(True)
ax.axis["top"].major_ticklabels.set_visible(True)
ax.axis["right"].get_helper().nth_coord_ticks = 0
ax.axis["bottom"].get_helper().nth_coord_ticks = 1
ax.set_aspect(1)
ax.set_xlim(-5, 12)
ax.set_ylim(-5, 10)
ax.axis["lat"] = ax.new_floating_axis(0, 60)
ax.axis["lon"] = ax.new_floating_axis(1, 6)
ax.axis["lat"].label.set_text("theta = 60")
ax.grid(True)
aux = ax.get_aux_axes(tr)
aux.plot(np.linspace(0, 90, 10), np.linspace(1, 8, 10), "o-")
fig.canvas.draw()
print(sorted(ax.axis.keys()))
print(finder(lambda x, y: tr.inverted().transform(np.c_[x, y]).T, -5, -5, 12, 10))
print(angle_helper.LocatorDMS(12)(0, 360)[0][:5], angle_helper.LocatorHMS(6)(0, 360)[0][:4])
print(angle_helper.FormatterDMS()("bottom", 1, [0, 15, 30.5]))
print(angle_helper.FormatterHMS()("bottom", 1, [0, 15, 30]))
print(angle_helper.select_step(0, 90, 6), angle_helper.select_step24(0, 24, 6)[:2])
fig.savefig("polar_curvilinear.png")
